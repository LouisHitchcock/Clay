//! Deciding what to do when an account hits its usage limit.
//!
//! Switching to another account is preferred over waiting, but not blindly: moving onto an
//! account that is nearly exhausted only moves the problem, and burns a second allowance doing
//! it. So a candidate has to have real headroom in its five-hour window, and if none does, the
//! work is scheduled against whichever account frees up soonest.

use crate::{AiAccount, AiAccountsIndex, AccountUsage, SESSION_WINDOW, read_usage};
use chrono::{DateTime, Utc};

/// The share of an account's five-hour window that, once consumed, disqualifies it as a switch
/// target. Switching onto an account with only a fifth of its window left tends to hit the same
/// wall again within minutes.
pub const DEFAULT_HEADROOM_THRESHOLD: f64 = 0.8;

/// What to do about a turn that stopped on a usage limit.
#[derive(Clone, Debug, PartialEq)]
pub enum LimitResponse {
    /// Bind this account instead and carry on immediately.
    Switch { account: AiAccount },
    /// No account has headroom. Resume against `account` once its window rolls.
    Wait {
        account: AiAccount,
        resume_at: DateTime<Utc>,
    },
}

/// An account paired with what is known about its usage, so callers can explain a decision.
#[derive(Clone, Debug)]
pub struct AccountWithUsage {
    pub account: AiAccount,
    pub usage: AccountUsage,
}

/// Reads usage for every account registered to `agent_id`.
pub fn accounts_with_usage(index: &AiAccountsIndex, agent_id: &str) -> Vec<AccountWithUsage> {
    index
        .for_agent(agent_id)
        .map(|account| AccountWithUsage {
            usage: read_usage(&account.config_dir),
            account: account.clone(),
        })
        .collect()
}

/// Chooses between switching accounts and waiting for a reset.
///
/// `exhausted_id` is the account that just hit the limit, and `reset_hint` is the reset time
/// parsed out of the agent's limit message when there was one — more trustworthy than anything
/// inferred from a usage sample, because the agent is quoting the server.
///
/// Returns `None` only when there is no account to act on at all.
pub fn choose_after_limit(
    accounts: &[AccountWithUsage],
    exhausted_id: &str,
    reset_hint: Option<DateTime<Utc>>,
    threshold: f64,
    now: DateTime<Utc>,
) -> Option<LimitResponse> {
    // Prefer a switch. Known headroom beats unknown usage: an account Clay has never run has no
    // sample, and while that is treated as usable rather than blocking the feature, a measured
    // account is the better bet when both are available.
    let mut candidates: Vec<&AccountWithUsage> = accounts
        .iter()
        .filter(|candidate| candidate.account.id != exhausted_id)
        .filter(|candidate| candidate.usage.has_headroom(threshold, now))
        .collect();

    candidates.sort_by(|a, b| {
        let rank = |candidate: &AccountWithUsage| match candidate.usage.session_fraction(now) {
            // Known usage sorts first, least-used ahead of most-used.
            Some(session) => (0, session),
            None => (1, 0.0),
        };
        let (a_group, a_session) = rank(a);
        let (b_group, b_session) = rank(b);
        a_group
            .cmp(&b_group)
            .then(a_session.total_cmp(&b_session))
            .then_with(|| a.account.display_name.cmp(&b.account.display_name))
    });

    if let Some(candidate) = candidates.first() {
        return Some(LimitResponse::Switch {
            account: candidate.account.clone(),
        });
    }

    // Nothing has headroom, so wait on whichever account frees up first.
    let mut soonest: Option<(DateTime<Utc>, &AiAccount)> = None;
    for candidate in accounts {
        let resets_at = if candidate.account.id == exhausted_id {
            // The agent told us when this one resets; trust that over the sample.
            reset_hint.or_else(|| candidate.usage.session_resets_at(now))
        } else {
            candidate.usage.session_resets_at(now)
        };
        let Some(resets_at) = resets_at else {
            continue;
        };
        if soonest.is_none_or(|(best, _)| resets_at < best) {
            soonest = Some((resets_at, &candidate.account));
        }
    }

    if let Some((resume_at, account)) = soonest {
        return Some(LimitResponse::Wait {
            account: account.clone(),
            resume_at,
        });
    }

    // Nothing is known about any window. Fall back to the exhausted account and a full window,
    // which is the longest it can possibly need — better a late resume than none.
    let exhausted = accounts
        .iter()
        .find(|candidate| candidate.account.id == exhausted_id)
        .or_else(|| accounts.first())?;
    Some(LimitResponse::Wait {
        account: exhausted.account.clone(),
        resume_at: reset_hint.unwrap_or(now + SESSION_WINDOW),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn at(unix: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(unix, 0).unwrap()
    }

    fn account(id: &str, name: &str) -> AiAccount {
        AiAccount {
            id: id.to_string(),
            agent_id: "claude-acp".to_string(),
            display_name: name.to_string(),
            config_dir: PathBuf::from(format!("/tmp/{id}")),
            state: crate::AccountState::Authenticated,
            created_at: None,
            last_used_at: None,
        }
    }

    fn known(id: &str, name: &str, session: f64, sampled_at: i64) -> AccountWithUsage {
        AccountWithUsage {
            account: account(id, name),
            usage: AccountUsage::Known {
                session,
                weekly: 0.1,
                sampled_at: at(sampled_at),
            },
        }
    }

    fn unknown(id: &str, name: &str) -> AccountWithUsage {
        AccountWithUsage {
            account: account(id, name),
            usage: AccountUsage::Unknown,
        }
    }

    #[test]
    fn switches_to_an_account_with_headroom() {
        let now = at(10_000);
        let accounts = vec![
            known("a", "work", 1.0, 9_900),
            known("b", "personal", 0.2, 9_900),
        ];
        assert_eq!(
            choose_after_limit(&accounts, "a", None, DEFAULT_HEADROOM_THRESHOLD, now),
            Some(LimitResponse::Switch {
                account: account("b", "personal")
            })
        );
    }

    #[test]
    fn prefers_the_least_used_account() {
        let now = at(10_000);
        let accounts = vec![
            known("a", "exhausted", 1.0, 9_900),
            known("b", "busy", 0.6, 9_900),
            known("c", "fresh", 0.05, 9_900),
        ];
        let Some(LimitResponse::Switch { account }) =
            choose_after_limit(&accounts, "a", None, DEFAULT_HEADROOM_THRESHOLD, now)
        else {
            panic!("expected a switch");
        };
        assert_eq!(account.display_name, "fresh");
    }

    #[test]
    fn a_measured_account_beats_one_with_no_data() {
        let now = at(10_000);
        let accounts = vec![
            known("a", "exhausted", 1.0, 9_900),
            unknown("b", "never used"),
            known("c", "measured", 0.5, 9_900),
        ];
        let Some(LimitResponse::Switch { account }) =
            choose_after_limit(&accounts, "a", None, DEFAULT_HEADROOM_THRESHOLD, now)
        else {
            panic!("expected a switch");
        };
        assert_eq!(account.display_name, "measured");
    }

    #[test]
    fn never_switches_back_to_the_exhausted_account() {
        let now = at(10_000);
        // Its own sample says it has headroom, but the agent just told us otherwise.
        let accounts = vec![known("a", "exhausted", 0.1, 9_900)];
        let response =
            choose_after_limit(&accounts, "a", Some(at(20_000)), DEFAULT_HEADROOM_THRESHOLD, now);
        assert_eq!(
            response,
            Some(LimitResponse::Wait {
                account: account("a", "exhausted"),
                resume_at: at(20_000),
            })
        );
    }

    #[test]
    fn waits_on_the_soonest_reset_when_nothing_has_headroom() {
        let now = at(10_000);
        let window = SESSION_WINDOW.num_seconds();
        let accounts = vec![
            // Resets last.
            known("a", "exhausted", 1.0, 9_900),
            // Sampled earlier, so its window rolls sooner.
            known("b", "also full", 0.95, 9_000),
        ];
        let Some(LimitResponse::Wait { account, resume_at }) =
            choose_after_limit(&accounts, "a", None, DEFAULT_HEADROOM_THRESHOLD, now)
        else {
            panic!("expected a wait");
        };
        assert_eq!(account.display_name, "also full");
        assert_eq!(resume_at, at(9_000 + window));
    }

    #[test]
    fn the_agents_own_reset_time_wins_over_the_sample() {
        let now = at(10_000);
        // The sample would put the reset far later than the agent says.
        let accounts = vec![known("a", "exhausted", 1.0, 9_999)];
        let hint = at(10_500);
        let Some(LimitResponse::Wait { resume_at, .. }) =
            choose_after_limit(&accounts, "a", Some(hint), DEFAULT_HEADROOM_THRESHOLD, now)
        else {
            panic!("expected a wait");
        };
        assert_eq!(resume_at, hint);
    }

    #[test]
    fn with_nothing_known_it_still_schedules_a_resume() {
        let now = at(10_000);
        let accounts = vec![unknown("a", "only account")];
        // The only account is the exhausted one, so there is nothing to switch to and no sample
        // to date a window from. A full window is the longest it could need.
        let Some(LimitResponse::Wait { account, resume_at }) =
            choose_after_limit(&accounts, "a", None, DEFAULT_HEADROOM_THRESHOLD, now)
        else {
            panic!("expected a wait");
        };
        assert_eq!(account.display_name, "only account");
        assert_eq!(resume_at, now + SESSION_WINDOW);
    }

    #[test]
    fn no_accounts_at_all_is_no_decision() {
        assert_eq!(
            choose_after_limit(&[], "a", None, DEFAULT_HEADROOM_THRESHOLD, at(0)),
            None
        );
    }
}
