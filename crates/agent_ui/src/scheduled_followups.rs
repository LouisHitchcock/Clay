//! Sending scheduled follow-ups when their time comes.
//!
//! The store and the timing rules live in `ai_accounts::followups`; this is the runtime that
//! acts on them — arming a timer, reopening the thread and sending the prompt.
//!
//! A single global rather than one per window: a follow-up belongs to a conversation, not to a
//! window, and several windows each firing the same one would send the prompt twice. The global
//! re-reads the file immediately before firing and removes the entry *before* sending, so a
//! second window that wakes at the same moment finds nothing to do.

use crate::{Agent, AgentPanel, AgentThreadSource, ThreadId};
use agent_client_protocol::schema::v1 as acp;
use ai_accounts::{FollowUp, FollowUpReason, FollowUps, followups};
use chrono::Utc;
use gpui::{App, AppContext as _, AsyncApp, BorrowAppContext as _, Global, Task, WeakEntity};
use std::time::Duration;
use workspace::Workspace;

/// How long to keep looking for the thread after asking the panel to open it.
///
/// Opening can go to storage, so the view is not always available on the next tick. This is
/// generous because the alternative — giving up — loses the resume the user was waiting hours
/// for.
const THREAD_OPEN_TIMEOUT: Duration = Duration::from_secs(20);
const THREAD_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// The longest a single sleep runs before re-checking.
///
/// A five-hour wait is not spent in one `timer` call: a machine that suspends mid-wait would
/// otherwise overshoot by however long it was asleep. Re-checking against the wall clock keeps
/// the firing time honest.
const MAX_SLEEP: Duration = Duration::from_secs(60);

struct ScheduledFollowUps {
    followups: FollowUps,
    /// The window to resume into. Whichever workspace was most recently opened; a follow-up has
    /// no window of its own, so it goes wherever the user currently is.
    workspace: Option<WeakEntity<Workspace>>,
    _timer: Option<Task<()>>,
}

impl Global for ScheduledFollowUps {}

pub fn init(cx: &mut App) {
    cx.set_global(ScheduledFollowUps {
        followups: followups::load(),
        workspace: None,
        _timer: None,
    });

    cx.observe_new::<Workspace>(|_, window, cx| {
        if window.is_none() {
            return;
        }
        let workspace = cx.entity().downgrade();
        cx.update_global::<ScheduledFollowUps, _>(|this, _| {
            this.workspace = Some(workspace);
        });
        arm(cx);
    })
    .detach();
}

/// Schedules `followup`, replacing any already pending for the same thread.
pub fn schedule(followup: FollowUp, cx: &mut App) {
    cx.update_global::<ScheduledFollowUps, _>(|this, _| {
        this.followups = followups::load();
        this.followups.schedule(followup);
        if let Err(error) = followups::save(&this.followups) {
            log::error!("scheduled_followups: could not save: {error:#}");
        }
    });
    arm(cx);
}

pub fn cancel(id: &str, cx: &mut App) {
    cx.update_global::<ScheduledFollowUps, _>(|this, _| {
        this.followups = followups::load();
        this.followups.remove(id);
        if let Err(error) = followups::save(&this.followups) {
            log::error!("scheduled_followups: could not save: {error:#}");
        }
    });
    arm(cx);
}

pub fn pending(cx: &App) -> Vec<FollowUp> {
    cx.try_global::<ScheduledFollowUps>()
        .map(|this| this.followups.pending.clone())
        .unwrap_or_default()
}

/// (Re)starts the timer for whatever is next.
fn arm(cx: &mut App) {
    let Some(this) = cx.try_global::<ScheduledFollowUps>() else {
        return;
    };
    let now = Utc::now();
    let has_due = !this.followups.due(now).is_empty();
    let next_wake = this.followups.next_wake(now);

    if !has_due && next_wake.is_none() {
        cx.update_global::<ScheduledFollowUps, _>(|this, _| this._timer = None);
        return;
    }

    let timer = cx.spawn(async move |cx: &mut AsyncApp| {
        loop {
            let Some((due, next_wake)) = cx.update(|cx| {
                let this = cx.try_global::<ScheduledFollowUps>()?;
                let now = Utc::now();
                Some((
                    this.followups.due(now).into_iter().cloned().collect::<Vec<_>>(),
                    this.followups.next_wake(now),
                ))
            }) else {
                return;
            };

            for followup in due {
                // Removed before sending, so a crash mid-send cannot leave a follow-up that
                // fires again on every startup for ever.
                if let Some(followup) = cx.update(|cx| take(&followup.id, cx)) {
                    resume(followup, cx).await;
                }
            }

            let Some(next_wake) = next_wake else {
                return;
            };
            let remaining = next_wake
                .signed_duration_since(Utc::now())
                .to_std()
                .unwrap_or(Duration::ZERO);
            if remaining.is_zero() {
                continue;
            }
            cx.update(|cx| {
                cx.background_executor()
                    .timer(remaining.min(MAX_SLEEP))
            })
            .await;
        }
    });

    cx.update_global::<ScheduledFollowUps, _>(|this, _| this._timer = Some(timer));
}

/// Removes a follow-up from the store and returns it.
fn take(id: &str, cx: &mut App) -> Option<FollowUp> {
    cx.update_global::<ScheduledFollowUps, _>(|this, _| {
        // Re-read first: another window may have fired this already.
        this.followups = followups::load();
        let taken = this.followups.remove(id);
        if taken.is_some() {
            if let Err(error) = followups::save(&this.followups) {
                log::error!("scheduled_followups: could not save: {error:#}");
            }
        }
        taken
    })
}

async fn resume(followup: FollowUp, cx: &mut AsyncApp) {
    log::info!(
        "scheduled_followups: resuming {} ({:?})",
        followup.id,
        followup.reason
    );

    // Bind the account the policy chose before the agent spawns, so the resumed turn runs
    // against the one that actually has allowance.
    if let Some(account_id) = followup.account_id.as_deref() {
        let mut index = ai_accounts::load_index();
        let agent_id = ai_accounts::CLAUDE_CODE_DESCRIPTOR.agent_id;
        if let Err(error) = index
            .set_default(agent_id, Some(account_id.to_string()))
            .and_then(|()| ai_accounts::save_index(&index))
        {
            log::error!("scheduled_followups: could not bind {account_id}: {error:#}");
        }
    }

    let Some(thread_id) = followup
        .thread
        .thread_id
        .as_deref()
        .and_then(ThreadId::from_key_string)
    else {
        log::error!(
            "scheduled_followups: {} has no thread to resume into; dropping it",
            followup.id
        );
        return;
    };

    let Some(workspace) = cx.update(|cx| {
        cx.try_global::<ScheduledFollowUps>()
            .and_then(|this| this.workspace.clone())
    }) else {
        log::error!("scheduled_followups: no window to resume into");
        return;
    };

    let opened = workspace.update_in(cx, |workspace, window, cx| {
        let Some(panel) = workspace.panel::<AgentPanel>(cx) else {
            return false;
        };
        workspace.focus_panel::<AgentPanel>(window, cx);
        panel.update(cx, |panel, cx| {
            panel.load_agent_thread(
                Agent::NativeAgent,
                thread_id,
                None,
                None,
                true,
                AgentThreadSource::AgentPanel,
                window,
                cx,
            );
        });
        true
    });

    if !matches!(opened, Ok(true)) {
        log::error!(
            "scheduled_followups: could not open the agent panel for {}",
            followup.id
        );
        return;
    }

    // Opening can hit storage, so the view is not necessarily there yet.
    let deadline = std::time::Instant::now() + THREAD_OPEN_TIMEOUT;
    loop {
        let sent = workspace.update_in(cx, |workspace, _window, cx| {
            let panel = workspace.panel::<AgentPanel>(cx)?;
            // The panel owns the thread id; `ThreadView` only knows its session. Waiting for
            // the *right* thread matters because the panel may still be showing the previous
            // one while this loads.
            if panel.read(cx).active_thread_id(cx) != Some(thread_id) {
                return None;
            }
            let thread_view = panel.read(cx).active_thread_view(cx)?;
            let thread = thread_view.read(cx).thread.clone();
            Some(thread.update(cx, |thread, cx| {
                thread.send(
                    vec![acp::ContentBlock::Text(acp::TextContent::new(
                        followup.prompt.clone(),
                    ))],
                    cx,
                )
            }))
        });

        match sent {
            Ok(Some(turn)) => {
                // Detached rather than awaited: the turn can run for minutes, and holding the
                // scheduler loop open would delay every other follow-up behind it.
                cx.update(|cx| cx.background_spawn(turn).detach());
                log::info!("scheduled_followups: sent {}", followup.id);
                return;
            }
            Err(_) => return,
            Ok(None) => {}
        }

        if std::time::Instant::now() >= deadline {
            log::error!(
                "scheduled_followups: thread for {} did not open in time; the follow-up is lost",
                followup.id
            );
            return;
        }
        cx.update(|cx| cx.background_executor().timer(THREAD_POLL_INTERVAL))
            .await;
    }
}

/// The wording sent when resuming after a usage limit.
///
/// Phrased as an instruction to continue rather than a repeat of the original request: the agent
/// still has the conversation, so restating the task would risk it starting over.
pub fn usage_limit_prompt() -> String {
    "Continue from where you left off.".to_string()
}

/// Builds the follow-up for a turn that stopped on a usage limit.
pub fn usage_limit_followup(
    thread: ai_accounts::ThreadRef,
    resume_at: chrono::DateTime<Utc>,
    account_id: Option<String>,
    note: Option<String>,
) -> FollowUp {
    let mut followup = FollowUp::new(thread, usage_limit_prompt(), resume_at)
        .reason(FollowUpReason::UsageLimit);
    if let Some(account_id) = account_id {
        followup = followup.account(account_id);
    }
    if let Some(note) = note {
        followup = followup.note(note);
    }
    followup
}
