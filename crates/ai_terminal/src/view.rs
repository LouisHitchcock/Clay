//! The unified terminal's pane item: one timeline, one input.
//!
//! The input is an editor rather than a shell prompt, which is what makes agent-first work at
//! all — the shell never sees a keystroke until a `!` line is submitted, so there is no readline
//! fighting for the same keys.

use crate::{BlockKind, Route, Timeline, input, timeline};
use chrono::Utc;
use editor::{Editor, EditorEvent};
use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, Render, Task, WeakEntity, Window,
    actions, prelude::*,
};
use ui::{Label, prelude::*, v_flex};
use workspace::{Item, Workspace};

actions!(
    ai_terminal,
    [
        /// Opens the unified AI terminal in the active pane.
        OpenAiTerminal,
        /// Submits the current input.
        Submit,
    ]
);

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _window, _cx| {
        workspace.register_action(|workspace, _: &OpenAiTerminal, window, cx| {
            let view = cx.new(|cx| AiTerminalView::new(window, cx));
            workspace.add_item_to_active_pane(Box::new(view), None, true, window, cx);
        });
    })
    .detach();
}

pub struct AiTerminalView {
    timeline: Timeline,
    input: Entity<Editor>,
    focus_handle: FocusHandle,
    /// Set while the input would more likely have been meant for the shell, so the hint can be
    /// shown without ever acting on it.
    shell_hint: bool,
    _subscriptions: Vec<gpui::Subscription>,
}

impl AiTerminalView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            let mut editor = Editor::auto_height(1, 10, window, cx);
            editor.set_placeholder_text("Ask the agent, or ! to run a shell command", window, cx);
            editor
        });

        let subscription = cx.subscribe(&input, |this, _, event: &EditorEvent, cx| {
            if matches!(event, EditorEvent::BufferEdited) {
                this.update_hint(cx);
            }
        });

        Self {
            timeline: Timeline::default(),
            input,
            focus_handle: cx.focus_handle(),
            shell_hint: false,
            _subscriptions: vec![subscription],
        }
    }

    fn update_hint(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text(cx);
        let hint = input::looks_like_shell_command(&text);
        if hint != self.shell_hint {
            self.shell_hint = hint;
            cx.notify();
        }
    }

    fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text(cx);
        let route = input::route(&text);
        if matches!(route, Route::Empty) {
            return;
        }

        self.input.update(cx, |input, cx| {
            input.clear(window, cx);
        });
        self.shell_hint = false;

        let now = Utc::now();
        match route {
            Route::Empty => {}
            Route::Agent(prompt) => {
                self.timeline.push(
                    BlockKind::Agent(timeline::AgentBlock {
                        prompt,
                        session_id: None,
                    }),
                    now,
                );
            }
            Route::Command { name, args } => {
                let outcome = self.run_command(&name, &args, cx);
                let id = self.timeline.push(
                    BlockKind::Command(timeline::CommandBlock {
                        name,
                        args,
                        outcome: Some(outcome),
                    }),
                    now,
                );
                // In-process commands finish as soon as they are run; nothing is awaited.
                self.timeline.finish(id, Utc::now());
            }
            Route::Shell(command) => {
                self.timeline.push(
                    BlockKind::Shell(timeline::ShellBlock {
                        command,
                        cwd: None,
                        output: String::new(),
                        exit_code: None,
                    }),
                    now,
                );
            }
        }
        cx.notify();
    }

    /// Runs a command Clay handles itself. Unknown names are reported rather than swallowed, so
    /// a typo does not look like a command that silently did nothing.
    fn run_command(&mut self, name: &str, _args: &str, _cx: &mut Context<Self>) -> String {
        match name {
            "clear" => {
                self.timeline = Timeline::default();
                "Cleared the timeline.".to_string()
            }
            other => format!("Unknown command `/{other}`."),
        }
    }

    fn render_block(&self, block: &crate::Block, cx: &Context<Self>) -> AnyElement {
        let (icon, heading, body) = match &block.kind {
            BlockKind::Shell(shell) => (
                IconName::Terminal,
                shell.command.clone(),
                match shell.exit_code {
                    None => "Waiting on shell integration — not yet run.".to_string(),
                    Some(0) => shell.output.clone(),
                    Some(code) => format!("exit {code}\n{}", shell.output),
                },
            ),
            BlockKind::Agent(agent) => (
                IconName::AiClaude,
                agent.prompt.clone(),
                "Waiting on the agent connection — not yet sent.".to_string(),
            ),
            BlockKind::Command(command) => (
                IconName::Slash,
                format!("/{} {}", command.name, command.args).trim_end().to_string(),
                command.outcome.clone().unwrap_or_default(),
            ),
        };

        v_flex()
            .gap_1()
            .p_2()
            .border_b_1()
            .border_color(cx.theme().colors().border_variant)
            .child(
                h_flex()
                    .gap_2()
                    .child(Icon::new(icon).size(IconSize::Small).color(Color::Muted))
                    .child(Label::new(heading).buffer_font(cx)),
            )
            .when(!body.is_empty(), |this| {
                this.child(
                    div().pl_5().child(
                        Label::new(body)
                            .size(LabelSize::Small)
                            .color(Color::Muted)
                            .buffer_font(cx),
                    ),
                )
            })
            .into_any_element()
    }
}

impl Focusable for AiTerminalView {
    /// The input's handle, not the container's, so activating the tab puts the cursor where the
    /// user is going to type rather than leaving them to click first.
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.focus_handle(cx)
    }
}

impl EventEmitter<()> for AiTerminalView {}

impl Item for AiTerminalView {
    type Event = ();

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        SharedString::from("AI Terminal")
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::Terminal))
    }
}

impl Render for AiTerminalView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let blocks: Vec<AnyElement> = self
            .timeline
            .blocks()
            .iter()
            .map(|block| self.render_block(block, cx))
            .collect();

        v_flex()
            .size_full()
            .key_context("AiTerminal")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::submit))
            .bg(cx.theme().colors().editor_background)
            .child(
                v_flex()
                    .flex_1()
                    .id("ai-terminal-timeline")
                    .overflow_y_scroll()
                    .children(blocks),
            )
            .child(
                v_flex()
                    .border_t_1()
                    .border_color(cx.theme().colors().border)
                    .p_2()
                    .gap_1()
                    .child(self.input.clone())
                    .when(self.shell_hint, |this| {
                        // Offered, never acted on: routing stays entirely in the user's hands.
                        this.child(
                            Label::new("Looks like a shell command — prefix with ! to run it")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                    }),
            )
    }
}
