//! Clay's unified AI terminal.
//!
//! One timeline of blocks, where the AI agent is the default input and `!` is the explicit
//! shell escape. See `CLAY.md` for the design and the reasoning behind that ordering.
//!
//! This crate is deliberately separate from `terminal` and `agent_ui`: it composes both rather
//! than living inside either, which keeps Clay's additions out of the crates that see the most
//! upstream churn.

pub mod input;
pub mod timeline;
pub mod view;

pub use input::{COMMAND_SIGIL, Route, SHELL_SIGIL, looks_like_shell_command, route};
pub use view::{AiTerminalView, OpenAiTerminal, init};
pub use timeline::{AgentBlock, Block, BlockId, BlockKind, CommandBlock, ShellBlock, Timeline};
