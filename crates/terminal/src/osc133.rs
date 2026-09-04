//! Reading OSC 133 shell-integration marks out of a PTY byte stream.
//!
//! OSC 133 is how a shell tells the terminal where commands begin and end. Without it a terminal
//! sees only characters, which is why Clay's terminal has had no concept of a command: no
//! boundaries, no exit codes, no way to attribute output to what produced it.
//!
//! The marks, as emitted by the shell's prompt hooks:
//!
//! | Sequence | Meaning |
//! |---|---|
//! | `OSC 133 ; A ST` | a prompt is about to be drawn |
//! | `OSC 133 ; B ST` | the prompt has finished; what follows is what the user typed |
//! | `OSC 133 ; C ST` | the command is running; what follows is its output |
//! | `OSC 133 ; D ; <code> ST` | the command finished, with an exit code |
//!
//! This scans for them rather than parsing the whole stream, because `vte` — a plain crates.io
//! dependency — drops OSC sequences it does not recognise and offers no hook, and
//! `alacritty_terminal`'s event loop owns the PTY read. Scanning a copy of the bytes on their way
//! to the parser is what keeps this in-tree, with no fork of either crate.

/// A shell-integration mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellMark {
    /// A prompt is about to be drawn.
    PromptStart,
    /// The prompt has finished; the user's typing follows.
    CommandStart,
    /// The command is running; its output follows.
    OutputStart,
    /// The command finished. `exit_code` is absent when the shell did not report one.
    CommandFinished { exit_code: Option<i32> },
}

/// Scans a byte stream for OSC 133 marks.
///
/// Stateful because a PTY read can split a sequence anywhere — a 4 KiB boundary falling between
/// the `133` and the `;` is not unusual under load, and losing a `D` mark there would mean a
/// command that never appears to finish.
#[derive(Debug, Default)]
pub struct Osc133Scanner {
    state: State,
    /// The bytes of the sequence seen so far, after `ESC ]`. Bounded: a runaway OSC must not
    /// grow this without limit.
    payload: Vec<u8>,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum State {
    /// Ordinary output.
    #[default]
    Text,
    /// Seen `ESC`, waiting to see whether `]` follows.
    Escape,
    /// Inside an OSC payload.
    Osc,
    /// Inside an OSC payload, having just seen `ESC` — the start of a `ESC \` terminator.
    OscEscape,
}

/// The longest OSC payload worth keeping.
///
/// OSC 133 payloads are a handful of bytes. Anything longer is a different sequence — a title, a
/// hyperlink, a clipboard write — and holding on to it would let a large paste grow this buffer.
const MAX_PAYLOAD: usize = 64;

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;

impl Osc133Scanner {
    /// Feeds bytes through the scanner, returning any marks they completed.
    ///
    /// The bytes are not modified or consumed: callers pass the same slice on to the real parser.
    /// The marks are a side channel, which is what lets this sit alongside `vte` rather than
    /// replacing it.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<ShellMark> {
        let mut marks = Vec::new();
        for &byte in bytes {
            match self.state {
                State::Text => {
                    if byte == ESC {
                        self.state = State::Escape;
                    }
                }
                State::Escape => {
                    if byte == b']' {
                        self.state = State::Osc;
                        self.payload.clear();
                    } else if byte == ESC {
                        // A second escape restarts the sequence rather than aborting it.
                        self.state = State::Escape;
                    } else {
                        self.state = State::Text;
                    }
                }
                State::Osc => match byte {
                    BEL => {
                        if let Some(mark) = parse_payload(&self.payload) {
                            marks.push(mark);
                        }
                        self.finish();
                    }
                    ESC => self.state = State::OscEscape,
                    _ => {
                        // Past the limit the payload is discarded but the scan continues, so the
                        // terminator still returns us to text rather than leaving us stuck.
                        if self.payload.len() < MAX_PAYLOAD {
                            self.payload.push(byte);
                        }
                    }
                },
                State::OscEscape => {
                    if byte == b'\\' {
                        if let Some(mark) = parse_payload(&self.payload) {
                            marks.push(mark);
                        }
                        self.finish();
                    } else {
                        // Not a terminator after all; the escape was part of the payload.
                        if self.payload.len() < MAX_PAYLOAD {
                            self.payload.push(ESC);
                        }
                        self.state = State::Osc;
                    }
                }
            }
        }
        marks
    }

    fn finish(&mut self) {
        self.state = State::Text;
        self.payload.clear();
    }
}

/// Reads a mark out of an OSC payload, or `None` if it is not an OSC 133.
fn parse_payload(payload: &[u8]) -> Option<ShellMark> {
    let payload = std::str::from_utf8(payload).ok()?;
    let rest = payload.strip_prefix("133;")?;
    let mut parts = rest.split(';');
    let kind = parts.next()?;

    match kind {
        "A" => Some(ShellMark::PromptStart),
        "B" => Some(ShellMark::CommandStart),
        "C" => Some(ShellMark::OutputStart),
        "D" => {
            // `D` alone means finished without a reported status, which some shells emit when
            // the command was interrupted.
            let exit_code = parts.next().and_then(|code| code.trim().parse().ok());
            Some(ShellMark::CommandFinished { exit_code })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(chunks: &[&[u8]]) -> Vec<ShellMark> {
        let mut scanner = Osc133Scanner::default();
        let mut marks = Vec::new();
        for chunk in chunks {
            marks.extend(scanner.feed(chunk));
        }
        marks
    }

    #[test]
    fn reads_each_mark() {
        assert_eq!(scan(&[b"\x1b]133;A\x07"]), vec![ShellMark::PromptStart]);
        assert_eq!(scan(&[b"\x1b]133;B\x07"]), vec![ShellMark::CommandStart]);
        assert_eq!(scan(&[b"\x1b]133;C\x07"]), vec![ShellMark::OutputStart]);
        assert_eq!(
            scan(&[b"\x1b]133;D;0\x07"]),
            vec![ShellMark::CommandFinished { exit_code: Some(0) }]
        );
    }

    #[test]
    fn accepts_the_string_terminator_as_well_as_bell() {
        // Both terminators are in the wild; zsh tends to use ST, bash BEL.
        assert_eq!(
            scan(&[b"\x1b]133;D;127\x1b\\"]),
            vec![ShellMark::CommandFinished {
                exit_code: Some(127)
            }]
        );
    }

    #[test]
    fn a_sequence_split_across_reads_still_arrives() {
        // The whole reason this is stateful: a PTY read can break anywhere.
        assert_eq!(
            scan(&[b"\x1b", b"]13", b"3;D", b";1", b"\x07"]),
            vec![ShellMark::CommandFinished { exit_code: Some(1) }]
        );
    }

    #[test]
    fn finds_marks_among_ordinary_output() {
        let stream = b"\x1b]133;A\x07user@host$ \x1b]133;B\x07ls\r\n\x1b]133;C\x07a  b\r\n\x1b]133;D;0\x07";
        assert_eq!(
            scan(&[stream]),
            vec![
                ShellMark::PromptStart,
                ShellMark::CommandStart,
                ShellMark::OutputStart,
                ShellMark::CommandFinished { exit_code: Some(0) },
            ]
        );
    }

    #[test]
    fn ignores_other_osc_sequences() {
        // A window title and a hyperlink must not be mistaken for shell integration.
        assert!(scan(&[b"\x1b]0;a title\x07"]).is_empty());
        assert!(scan(&[b"\x1b]8;;https://example.com\x07"]).is_empty());
        // Nor a different OSC that merely starts with the same digits.
        assert!(scan(&[b"\x1b]1337;File=x\x07"]).is_empty());
    }

    #[test]
    fn a_finish_without_a_code_is_still_a_finish() {
        assert_eq!(
            scan(&[b"\x1b]133;D\x07"]),
            vec![ShellMark::CommandFinished { exit_code: None }]
        );
        // As is one with an unreadable code, rather than dropping the mark entirely.
        assert_eq!(
            scan(&[b"\x1b]133;D;wat\x07"]),
            vec![ShellMark::CommandFinished { exit_code: None }]
        );
    }

    #[test]
    fn an_escape_inside_a_payload_does_not_end_it() {
        // ESC followed by something other than backslash is payload, not a terminator.
        assert_eq!(scan(&[b"\x1b]0;a\x1bb\x07\x1b]133;A\x07"]), vec![ShellMark::PromptStart]);
    }

    #[test]
    fn an_overlong_payload_does_not_wedge_the_scanner() {
        // A large OSC must not consume unbounded memory, and must not stop later marks arriving.
        let mut stream = b"\x1b]52;c;".to_vec();
        stream.extend(std::iter::repeat(b'A').take(4096));
        stream.push(BEL);
        stream.extend_from_slice(b"\x1b]133;C\x07");

        let mut scanner = Osc133Scanner::default();
        let marks = scanner.feed(&stream);
        assert_eq!(marks, vec![ShellMark::OutputStart]);
        assert!(scanner.payload.capacity() <= MAX_PAYLOAD * 2);
    }

    #[test]
    fn plain_text_produces_nothing() {
        assert!(scan(&[b"just some output\r\n"]).is_empty());
        assert!(scan(&[b""]).is_empty());
    }
}
