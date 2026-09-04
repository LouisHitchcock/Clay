//! Creating a new AI account, and signing in to it.
//!
//! An account is a config directory the agent is pointed at, so creating one is cheap — the
//! interesting part is authenticating it. For Claude Code that means running `claude /login`
//! with `CLAUDE_CONFIG_DIR` set to the new directory, which this does in a terminal tab so the
//! user can complete the browser flow and see any error the CLI prints.
//!
//! Deliberately a terminal rather than an in-thread `/login`: the agent panel route needs an
//! ACP thread bound to the account before it can send anything, and a terminal is both simpler
//! and closer to what the user would do by hand — which makes it easier to debug when a login
//! misbehaves.

use ai_accounts::{AgentDescriptor, create_account, load_index, save_index};
use gpui::{
    App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, Render, WeakEntity,
    Window, actions, prelude::*,
};
use task::{HideStrategy, RevealStrategy, RevealTarget, SaveStrategy, Shell, SpawnInTerminal, TaskId};
use ui::{Modal, ModalFooter, ModalHeader, Section, prelude::*};
use ui_input::InputField;
use workspace::{ModalView, Workspace};

actions!(
    ai_accounts,
    [
        /// Create the account named in the field and start its login.
        Confirm
    ]
);

pub struct AddAiAccountModal {
    descriptor: &'static AgentDescriptor,
    name: Entity<InputField>,
    workspace: WeakEntity<Workspace>,
    error: Option<SharedString>,
}

impl AddAiAccountModal {
    pub fn new(
        descriptor: &'static AgentDescriptor,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| {
            InputField::new(window, cx, "Work, personal, …").label("Account name")
        });
        Self {
            descriptor,
            name,
            workspace,
            error: None,
        }
    }

    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        let display_name = self.name.read(cx).text(cx).trim().to_string();
        if display_name.is_empty() {
            self.error = Some("Give the account a name.".into());
            cx.notify();
            return;
        }

        // `create_account` validates the name before it creates anything, so a duplicate or
        // otherwise rejected name surfaces here without leaving a directory behind.
        let account = match create_account(self.descriptor.agent_id, &display_name) {
            Ok(account) => account,
            Err(error) => {
                self.error = Some(format!("{error:#}").into());
                cx.notify();
                return;
            }
        };

        // Make it active straight away: the user asked for this account, and the login about to
        // run in the terminal writes into its directory, so anything else would be surprising.
        let mut index = load_index();
        if let Err(error) = index
            .set_default(self.descriptor.agent_id, Some(account.id.clone()))
            .and_then(|()| save_index(&index))
        {
            log::error!("ai_accounts: created {display_name} but could not make it active: {error:#}");
        }

        let config_dir = account.config_dir.display().to_string();
        let mut env = collections::HashMap::default();
        env.insert(self.descriptor.config_dir_env_var.to_string(), config_dir);
        // An ambient API key would let the CLI skip the subscription login entirely, leaving an
        // account that looks signed in but has no credentials of its own.
        for key in self.descriptor.scrub_env {
            env.remove(*key);
        }

        let login = SpawnInTerminal {
            id: TaskId(format!("ai-account-login-{}", account.id)),
            full_label: format!("Sign in to {display_name}"),
            label: format!("Sign in to {display_name}"),
            command: Some("claude".to_string()),
            args: vec!["/login".to_string()],
            command_label: "claude /login".to_string(),
            cwd: None,
            env,
            use_new_terminal: true,
            allow_concurrent_runs: true,
            reveal: RevealStrategy::Always,
            reveal_target: RevealTarget::Dock,
            // Left open on success as well as failure: the CLI prints which account it signed
            // in as, which is the only confirmation the user gets.
            hide: HideStrategy::Never,
            shell: Shell::System,
            show_summary: true,
            show_command: true,
            show_rerun: true,
            save: SaveStrategy::None,
        };

        if let Some(workspace) = self.workspace.upgrade() {
            workspace.update(cx, |workspace, cx| {
                workspace.spawn_in_terminal(login, window, cx).detach();
            });
        }

        cx.emit(DismissEvent);
    }

    fn cancel(&mut self, _: &menu::Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }
}

impl EventEmitter<DismissEvent> for AddAiAccountModal {}

impl Focusable for AddAiAccountModal {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.name.focus_handle(cx)
    }
}

impl ModalView for AddAiAccountModal {}

impl Render for AddAiAccountModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let name = self.name.clone();
        if let Some(error) = self.error.clone() {
            name.update(cx, |name, cx| name.set_error(Some(error), cx));
        }

        v_flex()
            .w(rems(30.))
            .elevation_3(cx)
            .key_context("AddAiAccountModal")
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::cancel))
            .child(
                Modal::new("add-ai-account", None)
                    .header(ModalHeader::new().headline(format!(
                        "Add {} account",
                        self.descriptor.display_name
                    )))
                    .section(Section::new().child(name))
                    .footer(
                        ModalFooter::new().end_slot(
                            Button::new("create", "Create and Sign In")
                                .style(ButtonStyle::Filled)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.confirm(&Confirm, window, cx)
                                })),
                        ),
                    ),
            )
            .into_any_element()
    }
}
