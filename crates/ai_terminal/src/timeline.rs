//! The block timeline.
//!
//! One ordered list of blocks, whatever produced them: a shell command, an agent turn, or a
//! command Clay handled itself. Keeping them in one timeline rather than separate panes is the
//! point of the design — the agent and the shell are two ways of doing the same job, and their
//! results belong in the same history.

use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Identifies a block within one timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(pub usize);

/// A shell command and what became of it.
#[derive(Clone, Debug, PartialEq)]
pub struct ShellBlock {
    /// Exactly what the user typed after `!`, unmodified.
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub output: String,
    /// `None` while still running.
    pub exit_code: Option<i32>,
}

impl ShellBlock {
    pub fn succeeded(&self) -> Option<bool> {
        self.exit_code.map(|code| code == 0)
    }

    /// What to tell the agent about a failure.
    ///
    /// A failed command is not a dead end: rather than leaving the user to copy the error out
    /// and explain it, the command, its directory, its exit code and its output go to the agent
    /// as context so it can suggest or issue a correction. This returns `None` for anything that
    /// has not actually failed, so the caller cannot accidentally narrate a success.
    ///
    /// The output is truncated from the *end* — a failing command's last lines carry the error,
    /// while its first lines are usually progress noise.
    pub fn failure_context(&self, max_output_bytes: usize) -> Option<String> {
        let exit_code = self.exit_code?;
        if exit_code == 0 {
            return None;
        }

        let mut context = format!("The shell command `{}` failed", self.command);
        if let Some(cwd) = &self.cwd {
            context.push_str(&format!(" in `{}`", cwd.display()));
        }
        context.push_str(&format!(" with exit code {exit_code}."));

        let output = self.output.trim();
        if !output.is_empty() {
            context.push_str("\n\nOutput:\n");
            context.push_str(&tail(output, max_output_bytes));
        }
        Some(context)
    }
}

/// The last `max_bytes` of `text`, on a line boundary, noting what was dropped.
fn tail(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    // Start from a char boundary, then move forward to a line break so the excerpt does not
    // begin mid-line and read as corrupt.
    let mut start = text.len() - max_bytes;
    while start < text.len() && !text.is_char_boundary(start) {
        start += 1;
    }
    let kept = match text[start..].find('\n') {
        Some(newline) => &text[start + newline + 1..],
        None => &text[start..],
    };
    format!("[earlier output omitted]\n{kept}")
}

/// An agent turn.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentBlock {
    pub prompt: String,
    /// The thread this turn belongs to, so a block can be traced back to the conversation.
    pub session_id: Option<String>,
}

/// A command Clay handled itself, which never reaches the shell.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandBlock {
    pub name: String,
    pub args: String,
    /// What Clay did, or why it could not.
    pub outcome: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BlockKind {
    Shell(ShellBlock),
    Agent(AgentBlock),
    Command(CommandBlock),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub id: BlockId,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub kind: BlockKind,
}

impl Block {
    /// How long the block took, or has been running.
    pub fn duration(&self, now: DateTime<Utc>) -> chrono::TimeDelta {
        self.finished_at.unwrap_or(now) - self.started_at
    }

    pub fn is_running(&self) -> bool {
        self.finished_at.is_none()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Timeline {
    blocks: Vec<Block>,
    next_id: usize,
}

impl Timeline {
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn get(&self, id: BlockId) -> Option<&Block> {
        self.blocks.iter().find(|block| block.id == id)
    }

    pub fn get_mut(&mut self, id: BlockId) -> Option<&mut Block> {
        self.blocks.iter_mut().find(|block| block.id == id)
    }

    /// Appends a block and returns its id.
    pub fn push(&mut self, kind: BlockKind, started_at: DateTime<Utc>) -> BlockId {
        let id = BlockId(self.next_id);
        self.next_id += 1;
        self.blocks.push(Block {
            id,
            started_at,
            finished_at: None,
            kind,
        });
        id
    }

    /// Marks a block finished. Ignores an unknown id and one already finished, so a duplicate
    /// completion event cannot rewrite history.
    pub fn finish(&mut self, id: BlockId, finished_at: DateTime<Utc>) {
        if let Some(block) = self.get_mut(id) {
            if block.finished_at.is_none() {
                block.finished_at = Some(finished_at);
            }
        }
    }

    /// The most recent block still running, if any.
    pub fn running(&self) -> Option<&Block> {
        self.blocks.iter().rev().find(|block| block.is_running())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(unix: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(unix, 0).unwrap()
    }

    fn failed(command: &str, output: &str, exit_code: i32) -> ShellBlock {
        ShellBlock {
            command: command.to_string(),
            cwd: Some(PathBuf::from("/work")),
            output: output.to_string(),
            exit_code: Some(exit_code),
        }
    }

    #[test]
    fn ids_are_stable_and_ordered() {
        let mut timeline = Timeline::default();
        let first = timeline.push(
            BlockKind::Agent(AgentBlock {
                prompt: "hello".into(),
                session_id: None,
            }),
            at(0),
        );
        let second = timeline.push(
            BlockKind::Command(CommandBlock {
                name: "open".into(),
                args: String::new(),
                outcome: None,
            }),
            at(1),
        );
        assert_ne!(first, second);
        assert_eq!(timeline.blocks().len(), 2);
        assert_eq!(timeline.get(first).map(|block| block.id), Some(first));
    }

    #[test]
    fn finishing_twice_does_not_rewrite_the_first_time() {
        // Completion can arrive by more than one route; the first one is the truth.
        let mut timeline = Timeline::default();
        let id = timeline.push(
            BlockKind::Shell(failed("false", "", 1)),
            at(0),
        );
        timeline.finish(id, at(10));
        timeline.finish(id, at(99));
        assert_eq!(timeline.get(id).unwrap().finished_at, Some(at(10)));
    }

    #[test]
    fn a_running_block_reports_elapsed_time() {
        let mut timeline = Timeline::default();
        let id = timeline.push(
            BlockKind::Agent(AgentBlock {
                prompt: "think".into(),
                session_id: None,
            }),
            at(100),
        );
        let block = timeline.get(id).unwrap();
        assert!(block.is_running());
        assert_eq!(block.duration(at(130)).num_seconds(), 30);

        timeline.finish(id, at(160));
        let block = timeline.get(id).unwrap();
        assert!(!block.is_running());
        // Once finished, later clock readings must not stretch the duration.
        assert_eq!(block.duration(at(999)).num_seconds(), 60);
    }

    #[test]
    fn running_finds_the_latest_unfinished_block() {
        let mut timeline = Timeline::default();
        let first = timeline.push(BlockKind::Shell(failed("a", "", 1)), at(0));
        let second = timeline.push(BlockKind::Shell(failed("b", "", 1)), at(1));
        timeline.finish(second, at(2));
        assert_eq!(timeline.running().map(|block| block.id), Some(first));
        timeline.finish(first, at(3));
        assert!(timeline.running().is_none());
    }

    #[test]
    fn a_failure_becomes_agent_context() {
        let block = failed("cargo build", "error[E0308]: mismatched types", 101);
        let context = block.failure_context(1024).unwrap();
        assert!(context.contains("cargo build"));
        assert!(context.contains("/work"));
        assert!(context.contains("101"));
        assert!(context.contains("E0308"));
    }

    #[test]
    fn a_success_is_never_narrated_as_a_failure() {
        let mut block = failed("ls", "a\nb", 0);
        assert_eq!(block.failure_context(1024), None);
        // Nor is something still running.
        block.exit_code = None;
        assert_eq!(block.failure_context(1024), None);
    }

    #[test]
    fn long_output_keeps_the_end_where_the_error_is() {
        let output = format!("{}\nerror: the actual problem", "progress\n".repeat(500));
        let block = failed("make", &output, 2);
        let context = block.failure_context(200).unwrap();
        assert!(context.contains("error: the actual problem"));
        assert!(context.contains("[earlier output omitted]"));
        // Truncation should hold roughly to the budget rather than shipping the whole log.
        assert!(context.len() < 600, "context was {} bytes", context.len());
    }

    #[test]
    fn truncation_does_not_split_a_character() {
        // A multi-byte character straddling the cut must not produce invalid output.
        let output = "é".repeat(400);
        let block = failed("build", &output, 1);
        let context = block.failure_context(100).unwrap();
        assert!(context.contains("[earlier output omitted]"));
    }

    #[test]
    fn succeeded_is_unknown_while_running() {
        let mut block = failed("sleep 1", "", 0);
        assert_eq!(block.succeeded(), Some(true));
        block.exit_code = Some(1);
        assert_eq!(block.succeeded(), Some(false));
        block.exit_code = None;
        assert_eq!(block.succeeded(), None);
    }
}
