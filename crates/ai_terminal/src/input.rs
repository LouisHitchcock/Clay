//! Deciding where a line of input goes.
//!
//! The agent is the default and `!` is the explicit shell escape. That ordering is deliberate:
//! natural language is the primary mode, and because the shell is reached only through a sigil
//! there is **no heuristic in the routing at all**. The user always knows which mode they are in
//! from what they typed, so there is no such thing as a silent mis-route — and the `cd`
//! overloading problem that plagues shell-first designs never arises.
//!
//! The one heuristic here, [`looks_like_shell_command`], exists purely to *offer a hint* when
//! someone types `git status` and means the shell. It must never influence routing.

/// Where a line of input should go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    /// Run verbatim in the shell. The `!` has been stripped.
    Shell(String),
    /// A command, handled in-process by Clay when it recognises it and passed to the agent
    /// otherwise — the agent has its own slash commands, so unknown ones are not an error.
    Command { name: String, args: String },
    /// Everything else: send to the agent as an ordinary turn.
    Agent(String),
    /// Nothing to do.
    Empty,
}

/// The sigil that means "run this in the shell, exactly as written".
pub const SHELL_SIGIL: char = '!';
/// The sigil for commands, matching the convention the agent and Claude Code already use.
pub const COMMAND_SIGIL: char = '/';

/// Routes a line of input.
pub fn route(input: &str) -> Route {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Route::Empty;
    }

    if let Some(rest) = trimmed.strip_prefix(SHELL_SIGIL) {
        // Only the sigil is stripped, not the whitespace after it, beyond a single separating
        // space: `!  ls` is a command with odd spacing, and the shell can decide what to make
        // of it. What matters is that nothing else is interpreted.
        let command = rest.trim_start();
        if command.is_empty() {
            return Route::Empty;
        }
        return Route::Shell(command.to_string());
    }

    if let Some(rest) = trimmed.strip_prefix(COMMAND_SIGIL) {
        let rest = rest.trim_start();
        if rest.is_empty() {
            return Route::Empty;
        }
        let (name, args) = match rest.split_once(char::is_whitespace) {
            Some((name, args)) => (name, args.trim_start()),
            None => (rest, ""),
        };
        return Route::Command {
            name: name.to_string(),
            args: args.to_string(),
        };
    }

    Route::Agent(trimmed.to_string())
}

/// Commands common enough that typing one without `!` is more likely a slip than a question.
///
/// Deliberately short and boring. It exists to catch the obvious cases, not to be exhaustive —
/// a longer list would make the hint fire on prose like "cargo is slow today", which is worse
/// than missing a few.
const COMMON_COMMANDS: &[&str] = &[
    "cargo", "cd", "chmod", "clear", "cp", "curl", "docker", "echo", "git", "grep", "kill", "ls",
    "make", "mkdir", "mv", "npm", "nvm", "pnpm", "ps", "pwd", "python", "rm", "rustc", "rustup",
    "ssh", "sudo", "tar", "touch", "which", "yarn",
];

/// Whether this input was probably meant for the shell.
///
/// **For hinting only.** Routing must never consult this: the value of the `!` design is that
/// there is no heuristic between the user and their intent, so acting on a guess here would
/// reintroduce exactly the silent mis-route the design avoids. The caller's job is to *offer*
/// the shell, not to choose it.
pub fn looks_like_shell_command(input: &str) -> bool {
    let trimmed = input.trim();
    // Already routed explicitly, so there is nothing to suggest.
    if trimmed.starts_with(SHELL_SIGIL) || trimmed.starts_with(COMMAND_SIGIL) {
        return false;
    }

    let mut words = trimmed.split_whitespace();
    let Some(first) = words.next() else {
        return false;
    };
    if !COMMON_COMMANDS.contains(&first) {
        return false;
    }

    // A question is a question however it starts: "git blame or git log?" is for the agent.
    if trimmed.ends_with('?') {
        return false;
    }

    // Prose about a command reads differently from an invocation. More than a handful of words,
    // or any capitalisation beyond the first character, suggests a sentence.
    let word_count = trimmed.split_whitespace().count();
    word_count <= 6
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(command: &str) -> Route {
        Route::Shell(command.to_string())
    }

    fn agent(prompt: &str) -> Route {
        Route::Agent(prompt.to_string())
    }

    fn command(name: &str, args: &str) -> Route {
        Route::Command {
            name: name.to_string(),
            args: args.to_string(),
        }
    }

    #[test]
    fn bare_input_goes_to_the_agent() {
        assert_eq!(route("why is this test failing"), agent("why is this test failing"));
        // Including things that look like commands: without the sigil the agent gets it, which
        // is the whole point of there being no heuristic.
        assert_eq!(route("git status"), agent("git status"));
    }

    #[test]
    fn the_sigil_routes_to_the_shell_verbatim() {
        assert_eq!(route("!git status"), shell("git status"));
        assert_eq!(route("! git status"), shell("git status"));
        // Whatever follows is the shell's business, not ours to interpret.
        assert_eq!(
            route("!cd C:\\Users\\Louis && ls | grep foo"),
            shell("cd C:\\Users\\Louis && ls | grep foo")
        );
    }

    #[test]
    fn a_sigil_inside_the_line_is_not_an_escape() {
        // Only a leading sigil routes; otherwise `!` in prose would be unusable.
        assert_eq!(route("what does !! do"), agent("what does !! do"));
        assert_eq!(route("run this!"), agent("run this!"));
    }

    #[test]
    fn slash_routes_to_a_command() {
        assert_eq!(route("/login"), command("login", ""));
        assert_eq!(route("/open crates/ai_terminal"), command("open", "crates/ai_terminal"));
        assert_eq!(route("/  compact"), command("compact", ""));
    }

    #[test]
    fn empty_input_does_nothing() {
        assert_eq!(route(""), Route::Empty);
        assert_eq!(route("   "), Route::Empty);
        // A sigil on its own is not a command yet.
        assert_eq!(route("!"), Route::Empty);
        assert_eq!(route("!   "), Route::Empty);
        assert_eq!(route("/"), Route::Empty);
    }

    #[test]
    fn surrounding_whitespace_is_ignored() {
        assert_eq!(route("  !ls  "), shell("ls"));
        assert_eq!(route("  hello  "), agent("hello"));
    }

    #[test]
    fn hints_at_the_shell_for_a_bare_command() {
        assert!(looks_like_shell_command("git status"));
        assert!(looks_like_shell_command("cargo build -p zed"));
        assert!(looks_like_shell_command("ls"));
    }

    #[test]
    fn does_not_hint_when_the_user_is_clearly_asking() {
        assert!(!looks_like_shell_command("git blame or git log?"));
        assert!(!looks_like_shell_command(
            "cargo build is failing and I cannot work out why"
        ));
        assert!(!looks_like_shell_command("why is git slow"));
    }

    #[test]
    fn does_not_hint_when_already_routed() {
        // The user has said what they want; suggesting it back would be noise.
        assert!(!looks_like_shell_command("!git status"));
        assert!(!looks_like_shell_command("/login"));
    }

    #[test]
    fn does_not_hint_on_empty_input() {
        assert!(!looks_like_shell_command(""));
        assert!(!looks_like_shell_command("   "));
    }
}
