//! Reading an account's recent usage, so a switch can avoid a nearly-exhausted account.
//!
//! Claude Code writes `usage-history.jsonl` into its config directory: one object per line,
//! sampled roughly once a minute, holding the fraction of the five-hour window and of the
//! weekly allowance that has been consumed. Because each Clay account is its own
//! `CLAUDE_CONFIG_DIR`, each account gets its own file and usage is genuinely per-account.

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use std::path::Path;

/// The rolling window Claude Code's `session` figure covers.
pub const SESSION_WINDOW: Duration = Duration::hours(5);

/// The file Claude Code samples usage into, relative to a config directory.
const USAGE_HISTORY: &str = "usage-history.jsonl";

/// One sample as written by Claude Code.
#[derive(Debug, Deserialize)]
struct UsageSample {
    /// Unix seconds.
    ts: f64,
    /// Fraction of the five-hour window consumed, 0.0 to 1.0.
    session: f64,
    /// Fraction of the weekly allowance consumed, 0.0 to 1.0.
    weekly: f64,
}

/// What is known about an account's remaining allowance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AccountUsage {
    /// No sample was found — the account has never run under Clay, so nothing is known.
    Unknown,
    Known {
        /// Fraction of the five-hour window consumed, 0.0 to 1.0.
        session: f64,
        /// Fraction of the weekly allowance consumed, 0.0 to 1.0.
        weekly: f64,
        /// When the sample was taken.
        sampled_at: DateTime<Utc>,
    },
}

impl AccountUsage {
    /// Whether this account is worth switching *to*.
    ///
    /// `Unknown` counts as usable: refusing to switch for lack of data would make the feature
    /// useless on a freshly imported account, and the decision self-corrects — an account that
    /// is actually exhausted reports a limit immediately, which falls back to scheduling.
    pub fn has_headroom(&self, threshold: f64, now: DateTime<Utc>) -> bool {
        match self.session_fraction(now) {
            Some(session) => session < threshold,
            None => true,
        }
    }

    /// The consumed fraction of the five-hour window, as best it can be known now.
    ///
    /// A sample older than the window means the window has rolled, so nothing of it is
    /// consumed. Otherwise the sample is returned as-is: usage cannot rise while the account is
    /// idle, and the file is only written while it runs, so the figure can only be an
    /// overstatement. That errs towards refusing a switch rather than making a bad one.
    pub fn session_fraction(&self, now: DateTime<Utc>) -> Option<f64> {
        match self {
            AccountUsage::Unknown => None,
            AccountUsage::Known {
                session,
                sampled_at,
                ..
            } => {
                if now.signed_duration_since(*sampled_at) >= SESSION_WINDOW {
                    Some(0.0)
                } else {
                    Some(*session)
                }
            }
        }
    }

    /// When the five-hour window containing the last sample runs out.
    ///
    /// Used to pick which account to wait on when none has headroom. `None` when nothing is
    /// known, or when the window has already rolled and there is nothing to wait for.
    pub fn session_resets_at(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            AccountUsage::Unknown => None,
            AccountUsage::Known { sampled_at, .. } => {
                let resets_at = *sampled_at + SESSION_WINDOW;
                (resets_at > now).then_some(resets_at)
            }
        }
    }

    pub fn weekly_fraction(&self) -> Option<f64> {
        match self {
            AccountUsage::Unknown => None,
            AccountUsage::Known { weekly, .. } => Some(*weekly),
        }
    }
}

/// Reads the most recent usage sample from an account's config directory.
///
/// A missing, empty or malformed file is [`AccountUsage::Unknown`] rather than an error: usage
/// is an optimisation for choosing between accounts, and failing to read it must never be what
/// stops a turn from continuing.
pub fn read_usage(config_dir: &Path) -> AccountUsage {
    let path = config_dir.join(USAGE_HISTORY);
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                log::warn!(
                    "ai_accounts: could not read {}: {error}",
                    path.display()
                );
            }
            return AccountUsage::Unknown;
        }
    };

    latest_sample(&contents)
}

/// The last parseable sample in a `usage-history.jsonl` body.
///
/// Scans from the end, because the newest sample is the last line and the file grows without
/// bound — thousands of lines after a few days of use.
fn latest_sample(contents: &str) -> AccountUsage {
    for line in contents.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(sample) = serde_json::from_str::<UsageSample>(line) else {
            continue;
        };
        let Some(sampled_at) = DateTime::from_timestamp(sample.ts as i64, 0) else {
            continue;
        };
        return AccountUsage::Known {
            session: sample.session,
            weekly: sample.weekly,
            sampled_at,
        };
    }
    AccountUsage::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(unix: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(unix, 0).unwrap()
    }

    #[test]
    fn reads_the_newest_sample() {
        let body = concat!(
            "{\"ts\":1000,\"session\":0.10,\"weekly\":0.01}\n",
            "{\"ts\":2000,\"session\":0.42,\"weekly\":0.05}\n",
        );
        assert_eq!(
            latest_sample(body),
            AccountUsage::Known {
                session: 0.42,
                weekly: 0.05,
                sampled_at: at(2000),
            }
        );
    }

    #[test]
    fn skips_trailing_junk_rather_than_giving_up() {
        // The file is appended to by a running process, so the last line can be a partial write.
        let body = concat!(
            "{\"ts\":2000,\"session\":0.42,\"weekly\":0.05}\n",
            "{\"ts\":3000,\"sessio\n",
        );
        assert_eq!(
            latest_sample(body),
            AccountUsage::Known {
                session: 0.42,
                weekly: 0.05,
                sampled_at: at(2000),
            }
        );
    }

    #[test]
    fn no_samples_is_unknown() {
        assert_eq!(latest_sample(""), AccountUsage::Unknown);
        assert_eq!(latest_sample("\n \n"), AccountUsage::Unknown);
    }

    #[test]
    fn unknown_usage_counts_as_usable() {
        // Otherwise a freshly imported account could never be switched to.
        assert!(AccountUsage::Unknown.has_headroom(0.8, at(0)));
        assert_eq!(AccountUsage::Unknown.session_fraction(at(0)), None);
    }

    #[test]
    fn the_threshold_is_exclusive_at_the_top() {
        let usage = AccountUsage::Known {
            session: 0.8,
            weekly: 0.0,
            sampled_at: at(1_000),
        };
        assert!(!usage.has_headroom(0.8, at(1_100)));

        let usage = AccountUsage::Known {
            session: 0.79,
            weekly: 0.0,
            sampled_at: at(1_000),
        };
        assert!(usage.has_headroom(0.8, at(1_100)));
    }

    #[test]
    fn a_sample_older_than_the_window_means_it_has_rolled() {
        let usage = AccountUsage::Known {
            session: 1.0,
            weekly: 0.2,
            sampled_at: at(0),
        };
        // Five hours and a second later, the window that sample belonged to is gone.
        let now = at(SESSION_WINDOW.num_seconds() + 1);
        assert_eq!(usage.session_fraction(now), Some(0.0));
        assert!(usage.has_headroom(0.8, now));
        assert_eq!(usage.session_resets_at(now), None);
    }

    #[test]
    fn a_recent_sample_is_taken_at_face_value() {
        let usage = AccountUsage::Known {
            session: 1.0,
            weekly: 0.2,
            sampled_at: at(1_000),
        };
        let now = at(1_060);
        assert_eq!(usage.session_fraction(now), Some(1.0));
        assert!(!usage.has_headroom(0.8, now));
        assert_eq!(
            usage.session_resets_at(now),
            Some(at(1_000 + SESSION_WINDOW.num_seconds()))
        );
    }
}
