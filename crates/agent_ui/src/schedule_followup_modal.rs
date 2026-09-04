//! Scheduling a message to the agent by hand.
//!
//! The automatic path covers hitting a usage limit; this is the same machinery driven directly,
//! for "pick this up at 3pm" or "try again in two hours". It attaches to whichever agent thread
//! is in front, because a follow-up with no conversation to land in could never fire — so that
//! case is said plainly rather than accepted and silently dropped later.

use crate::{AgentPanel, scheduled_followups};
use ai_accounts::{FollowUp, ThreadRef, followups};
use gpui::{
    App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, Render, Window,
    actions, prelude::*,
};
use ui::{Modal, ModalFooter, ModalHeader, Section, prelude::*};
use ui_input::InputField;
use workspace::{ModalView, Workspace};

actions!(
    agent,
    [
        /// Schedules the message in the field.
        ConfirmSchedule
    ]
);

pub struct ScheduleFollowUpModal {
    prompt: Entity<InputField>,
    when: Entity<InputField>,
    thread: ThreadRef,
    /// Set when there is no thread to attach to.
    unavailable: Option<SharedString>,
    error: Option<SharedString>,
}

impl ScheduleFollowUpModal {
    /// Registers the action that opens this, so any surface can ask for it without depending on
    /// the agent UI — the title bar does exactly that.
    pub fn register(workspace: &mut Workspace) {
        workspace.register_action(
            |workspace, _: &zed_actions::agent::ScheduleFollowUp, window, cx| {
                let thread = workspace
                    .panel::<AgentPanel>(cx)
                    .and_then(|panel| {
                        let panel = panel.read(cx);
                        let thread_id = panel.active_thread_id(cx)?;
                        Some(ThreadRef {
                            thread_id: Some(thread_id.to_key_string()),
                            session_id: None,
                        })
                    })
                    .unwrap_or_default();

                workspace.toggle_modal(window, cx, |window, cx| {
                    ScheduleFollowUpModal::new(thread, window, cx)
                });
            },
        );
    }

    pub fn new(thread: ThreadRef, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let prompt = cx
            .new(|cx| InputField::new(window, cx, "Carry on with the refactor").label("Message"));
        let when =
            cx.new(|cx| InputField::new(window, cx, "15:00, 3pm, 90m, 2h30m").label("When"));
        let unavailable = thread.is_empty().then(|| {
            SharedString::from(
                "Open an agent thread first — a scheduled message needs a conversation to land in.",
            )
        });

        Self {
            prompt,
            when,
            thread,
            unavailable,
            error: None,
        }
    }

    fn confirm(&mut self, _: &ConfirmSchedule, _window: &mut Window, cx: &mut Context<Self>) {
        if self.unavailable.is_some() {
            return;
        }

        let prompt = self.prompt.read(cx).text(cx).trim().to_string();
        if prompt.is_empty() {
            self.error = Some("Say what to send.".into());
            cx.notify();
            return;
        }

        let typed = self.when.read(cx).text(cx);
        let offset = *chrono::Local::now().offset();
        let Some(resume_at) = followups::parse_when(&typed, chrono::Utc::now(), offset) else {
            self.error = Some("Could not read that time. Try 15:00, 3pm, 90m or 2h30m.".into());
            cx.notify();
            return;
        };

        // Through the runtime rather than straight to the file, so the timer re-arms — writing
        // the file alone would leave the message sitting there until the next window opened.
        scheduled_followups::schedule(FollowUp::new(self.thread.clone(), prompt, resume_at), cx);
        cx.emit(DismissEvent);
    }

    fn cancel(&mut self, _: &menu::Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }
}

impl EventEmitter<DismissEvent> for ScheduleFollowUpModal {}

impl Focusable for ScheduleFollowUpModal {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.prompt.focus_handle(cx)
    }
}

impl ModalView for ScheduleFollowUpModal {}

impl Render for ScheduleFollowUpModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(error) = self.error.clone() {
            let when = self.when.clone();
            when.update(cx, |when, cx| when.set_error(Some(error), cx));
        }

        let body = match self.unavailable.clone() {
            Some(message) => Section::new().child(Label::new(message).color(Color::Muted)),
            None => Section::new()
                .child(self.prompt.clone())
                .child(self.when.clone()),
        };

        v_flex()
            .w(rems(30.))
            .elevation_3(cx)
            .key_context("ScheduleFollowUpModal")
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::cancel))
            .child(
                Modal::new("schedule-followup", None)
                    .header(ModalHeader::new().headline("Schedule a message"))
                    .section(body)
                    .footer(ModalFooter::new().end_slot(
                        Button::new("schedule", "Schedule")
                            .style(ButtonStyle::Filled)
                            .disabled(self.unavailable.is_some())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.confirm(&ConfirmSchedule, window, cx)
                            })),
                    )),
            )
            .into_any_element()
    }
}
