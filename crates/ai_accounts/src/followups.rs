//! Follow-up messages waiting to be sent.
//!
//! When a turn stops on a usage limit, or the user schedules one by hand, the prompt to send
//! later is recorded here. A five-hour window comfortably outlives most editor sessions, so this
//! is on disk rather than in memory: a resume that evaporates when Clay restarts would be worse
//! than useless, because the user would have stopped watching for it.
//!
//! This module deliberately knows nothing about threads or agents beyond opaque identifiers.
//! Reopening a thread and sending the prompt is the caller's job; keeping that out of here is
//! what makes the timing rules testable.

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Why a follow-up is pending, so the UI can explain itself and so an automatic resume can be
/// distinguished from one the user asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowUpReason {
    /// The user scheduled it.
    #[default]
    Manual,
    /// A turn stopped on a usage limit and this resumes it.
    UsageLimit,
}

/// Enough to find the conversation a follow-up belongs to.
///
/// Both identifiers are kept because either can be the one available: a thread known to this
/// machine has a `thread_id`, while a session shared or imported from elsewhere is only known by
/// `session_id`. Storing both means a restart can still find the thread whichever route it
/// arrived by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl ThreadRef {
    pub fn is_empty(&self) -> bool {
        self.thread_id.is_none() && self.session_id.is_none()
    }
}

/// A message to send once `resume_at` arrives.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FollowUp {
    pub id: String,
    pub thread: ThreadRef,
    /// The prompt to send. For a usage-limit resume this is the "carry on" instruction rather
    /// than a replay of the original turn, which the agent still has in its own context.
    pub prompt: String,
    pub resume_at: DateTime<Utc>,
    #[serde(default)]
    pub reason: FollowUpReason,
    /// Bind this account before sending. Set when the policy chose to wait on a *different*
    /// account than the one that hit the limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// A human-readable note about why this exists, shown in the UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl FollowUp {
    pub fn new(thread: ThreadRef, prompt: impl Into<String>, resume_at: DateTime<Utc>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            thread,
            prompt: prompt.into(),
            resume_at,
            reason: FollowUpReason::Manual,
            account_id: None,
            note: None,
        }
    }

    pub fn reason(mut self, reason: FollowUpReason) -> Self {
        self.reason = reason;
        self
    }

    pub fn account(mut self, account_id: impl Into<String>) -> Self {
        self.account_id = Some(account_id.into());
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Everything currently scheduled.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FollowUps {
    #[serde(default)]
    pub pending: Vec<FollowUp>,
}

impl FollowUps {
    /// The follow-ups due at or before `now`, oldest first.
    ///
    /// Ordered by `resume_at` so a backlog after a long shutdown replays in the order it was
    /// scheduled rather than an arbitrary one.
    pub fn due(&self, now: DateTime<Utc>) -> Vec<&FollowUp> {
        let mut due: Vec<&FollowUp> = self
            .pending
            .iter()
            .filter(|followup| followup.resume_at <= now)
            .collect();
        due.sort_by_key(|followup| followup.resume_at);
        due
    }

    /// When the next follow-up falls due, if any is still in the future.
    pub fn next_wake(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.pending
            .iter()
            .map(|followup| followup.resume_at)
            .filter(|resume_at| *resume_at > now)
            .min()
    }

    /// Replaces any follow-up already scheduled for the same thread.
    ///
    /// One pending resume per conversation: hitting a limit twice in a row, or scheduling by
    /// hand over an automatic resume, should move the appointment rather than stack up several
    /// messages that all fire into the same thread.
    pub fn schedule(&mut self, followup: FollowUp) {
        if !followup.thread.is_empty() {
            self.pending
                .retain(|existing| existing.thread != followup.thread);
        }
        self.pending.push(followup);
    }

    pub fn remove(&mut self, id: &str) -> Option<FollowUp> {
        let index = self.pending.iter().position(|pending| pending.id == id)?;
        Some(self.pending.remove(index))
    }

    pub fn for_thread(&self, thread: &ThreadRef) -> Option<&FollowUp> {
        self.pending
            .iter()
            .find(|pending| &pending.thread == thread)
    }
}

fn followups_path() -> PathBuf {
    paths::config_dir().join("scheduled_followups.json")
}

/// Reads the scheduled follow-ups, or an empty set if there are none.
///
/// A corrupt file logs and yields an empty set rather than propagating: a scheduling file that
/// cannot be parsed should not stop the editor from starting.
pub fn load() -> FollowUps {
    let path = followups_path();
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                log::warn!("ai_accounts: could not read {}: {error}", path.display());
            }
            return FollowUps::default();
        }
    };

    parse(&contents).unwrap_or_else(|error| {
        log::error!(
            "ai_accounts: {} is not readable, ignoring it: {error}",
            path.display()
        );
        FollowUps::default()
    })
}

/// Reads the file's contents, tolerating a byte-order mark.
///
/// Windows tooling readily writes UTF-8 with a BOM and `serde_json` rejects one, so without this
/// a scheduled message would vanish because something put three bytes on the front of the file.
fn parse(contents: &str) -> Result<FollowUps, serde_json::Error> {
    let contents = contents.strip_prefix('\u{feff}').unwrap_or(contents);
    serde_json::from_str(contents)
}

pub fn save(followups: &FollowUps) -> Result<()> {
    let path = followups_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(followups).context("serialising follow-ups")?;
    std::fs::write(&path, json).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(unix: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(unix, 0).unwrap()
    }

    fn thread(id: &str) -> ThreadRef {
        ThreadRef {
            thread_id: Some(id.to_string()),
            session_id: None,
        }
    }

    fn followup(thread_id: &str, resume_at: i64) -> FollowUp {
        FollowUp::new(thread(thread_id), "carry on", at(resume_at))
    }

    #[test]
    fn nothing_is_due_before_its_time() {
        let mut followups = FollowUps::default();
        followups.schedule(followup("a", 2_000));
        assert!(followups.due(at(1_999)).is_empty());
        assert_eq!(followups.due(at(2_000)).len(), 1);
    }

    #[test]
    fn a_backlog_replays_in_scheduled_order() {
        // After a long shutdown several can be due at once, and the order they were scheduled in
        // is the order they should fire.
        let mut followups = FollowUps::default();
        followups.schedule(followup("late", 3_000));
        followups.schedule(followup("early", 1_000));
        followups.schedule(followup("middle", 2_000));

        let due: Vec<_> = followups
            .due(at(5_000))
            .into_iter()
            .map(|followup| followup.thread.thread_id.clone().unwrap())
            .collect();
        assert_eq!(due, vec!["early", "middle", "late"]);
    }

    #[test]
    fn the_next_wake_ignores_what_is_already_due() {
        let mut followups = FollowUps::default();
        followups.schedule(followup("overdue", 1_000));
        followups.schedule(followup("later", 5_000));
        assert_eq!(followups.next_wake(at(2_000)), Some(at(5_000)));
    }

    #[test]
    fn no_pending_means_no_wake() {
        assert_eq!(FollowUps::default().next_wake(at(0)), None);
        let mut followups = FollowUps::default();
        followups.schedule(followup("done", 1_000));
        assert_eq!(followups.next_wake(at(2_000)), None);
    }

    #[test]
    fn scheduling_twice_for_one_thread_moves_the_appointment() {
        // Hitting a limit twice must not stack two resumes into the same conversation.
        let mut followups = FollowUps::default();
        followups.schedule(followup("a", 2_000));
        followups.schedule(followup("a", 9_000));
        assert_eq!(followups.pending.len(), 1);
        assert_eq!(followups.pending[0].resume_at, at(9_000));
    }

    #[test]
    fn different_threads_schedule_independently() {
        let mut followups = FollowUps::default();
        followups.schedule(followup("a", 2_000));
        followups.schedule(followup("b", 2_000));
        assert_eq!(followups.pending.len(), 2);
    }

    #[test]
    fn a_followup_without_a_thread_never_displaces_another() {
        // An empty ref matches nothing, so two of them must not cancel each other out.
        let mut followups = FollowUps::default();
        followups.schedule(FollowUp::new(ThreadRef::default(), "one", at(1_000)));
        followups.schedule(FollowUp::new(ThreadRef::default(), "two", at(2_000)));
        assert_eq!(followups.pending.len(), 2);
    }

    #[test]
    fn removing_returns_what_was_removed() {
        let mut followups = FollowUps::default();
        let scheduled = followup("a", 2_000);
        let id = scheduled.id.clone();
        followups.schedule(scheduled);
        assert_eq!(followups.remove(&id).map(|f| f.id), Some(id.clone()));
        assert!(followups.remove(&id).is_none());
        assert!(followups.pending.is_empty());
    }

    #[test]
    fn survives_a_round_trip_through_json() {
        let mut followups = FollowUps::default();
        followups.schedule(
            followup("a", 2_000)
                .reason(FollowUpReason::UsageLimit)
                .account("account-1")
                .note("waiting for personal to reset"),
        );
        let json = serde_json::to_string(&followups).unwrap();
        assert_eq!(
            serde_json::from_str::<FollowUps>(&json).unwrap(),
            followups
        );
    }

    #[test]
    fn a_byte_order_mark_does_not_lose_the_schedule() {
        // PowerShell's Out-File writes UTF-8 with a BOM by default, and serde_json rejects it.
        // This cost a debugging round when the scheduler silently loaded nothing.
        let json = "\u{feff}{\"pending\":[]}";
        assert_eq!(parse(json).unwrap(), FollowUps::default());
    }

    #[test]
    fn unreadable_contents_are_an_error_rather_than_silence() {
        assert!(parse("not json at all").is_err());
    }

    #[test]
    fn reads_a_file_written_before_the_optional_fields_existed() {
        // Forwards compatibility matters here: an older file must not strand a resume.
        let json = r#"{"pending":[{"id":"x","thread":{"thread_id":"a"},"prompt":"carry on","resume_at":"2026-09-04T12:00:00Z"}]}"#;
        let followups: FollowUps = serde_json::from_str(json).unwrap();
        assert_eq!(followups.pending.len(), 1);
        assert_eq!(followups.pending[0].reason, FollowUpReason::Manual);
        assert_eq!(followups.pending[0].account_id, None);
    }
}
