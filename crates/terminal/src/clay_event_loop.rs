//! Clay's own PTY read loop, so shell-integration marks can be seen.
//!
//! `alacritty_terminal::event_loop::EventLoop` owns the PTY read and feeds the parser on its own
//! thread, which means Clay never sees the bytes. `vte` drops OSC sequences it does not
//! recognise and offers no hook, so OSC 133 is invisible from outside. This loop does the same
//! job, and tees a copy of every byte through [`Osc133Scanner`] on the way to the parser.
//!
//! It reuses alacritty's PTY entirely — `EventedPty` and `EventedReadWrite` are public traits —
//! so what is reimplemented here is only the polling, the write queue and the exit handling.
//! Modelled closely on the original for that reason: the semantics around hangup, drain-on-exit
//! and synchronized updates are easy to get subtly wrong.
//!
//! Windows only for now. The poll tokens are `pub` in `tty::windows` but `pub(crate)` on Unix,
//! and their values differ between the two (1/2 against 0/1), so there is no correct way to
//! reference them off Windows without duplicating private constants by value. Other platforms
//! keep alacritty's loop.

use crate::osc133::{Osc133Scanner, ShellMark};
use alacritty_terminal::event::{Event as AlacTermEvent, EventListener, OnResize, WindowSize};
use alacritty_terminal::term::Term;
use alacritty_terminal::tty::windows::{PTY_CHILD_EVENT_TOKEN, PTY_READ_WRITE_TOKEN};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::tty::{ChildEvent, EventedPty};
use futures::channel::mpsc::UnboundedSender;
use polling::{Event as PollingEvent, Events, PollMode, Poller};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::Instant;
use vte::ansi::{Processor, StdSyncHandler};

/// Matches alacritty's own read buffer. Large enough that a burst of output is a handful of
/// reads rather than hundreds.
const READ_BUFFER_SIZE: usize = 0x10_0000;

/// How many bytes to parse while holding the terminal lock before letting the UI in.
const MAX_LOCKED_READ: usize = u16::MAX as usize;

/// What the terminal asks of the loop.
pub(super) enum LoopMsg {
    Input(Cow<'static, [u8]>),
    Resize(WindowSize),
    Shutdown,
}

/// The terminal's handle on a running loop.
pub(super) struct LoopSender {
    tx: Sender<LoopMsg>,
    /// Needed because the loop is usually parked in `Poller::wait`; without waking it, a write
    /// would sit in the channel until the PTY happened to produce output.
    poller: Arc<Poller>,
}

impl LoopSender {
    pub(super) fn send(&self, msg: LoopMsg) -> Result<(), &'static str> {
        self.tx.send(msg).map_err(|_| "pty loop is gone")?;
        // A failure here means the loop cannot be woken, which is worse than a dropped message,
        // so it is reported rather than ignored.
        self.poller.notify().map_err(|_| "could not wake pty loop")
    }
}

/// Starts the loop on its own thread.
pub(super) fn spawn<T, L>(
    term: Arc<FairMutex<Term<L>>>,
    listener: L,
    mut pty: T,
    drain_on_exit: bool,
    marks: Option<UnboundedSender<ShellMark>>,
) -> std::io::Result<LoopSender>
where
    T: EventedPty + OnResize + Send + 'static,
    L: EventListener + Send + Clone + 'static,
{
    let poller = Arc::new(Poller::new()?);
    let (tx, rx) = channel();
    let sender = LoopSender {
        tx,
        poller: poller.clone(),
    };

    std::thread::Builder::new()
        .name("Clay PTY reader".into())
        .spawn(move || {
            let mut loop_state = LoopState {
                parser: Processor::<StdSyncHandler>::new(),
                scanner: Osc133Scanner::default(),
                writes: VecDeque::new(),
                written: 0,
                marks,
            };
            run(
                &mut loop_state,
                term,
                listener,
                &mut pty,
                &poller,
                rx,
                drain_on_exit,
            );
        })?;

    Ok(sender)
}

struct LoopState {
    parser: Processor<StdSyncHandler>,
    scanner: Osc133Scanner,
    /// Pending writes to the PTY, oldest first.
    writes: VecDeque<Cow<'static, [u8]>>,
    /// How much of the front write has gone out. A PTY write is not guaranteed to take
    /// everything, so a partially written chunk has to be remembered rather than resent.
    written: usize,
    marks: Option<UnboundedSender<ShellMark>>,
}

impl LoopState {
    fn wants_write(&self) -> bool {
        !self.writes.is_empty()
    }
}

fn run<T, L>(
    state: &mut LoopState,
    term: Arc<FairMutex<Term<L>>>,
    listener: L,
    pty: &mut T,
    poller: &Arc<Poller>,
    rx: Receiver<LoopMsg>,
    drain_on_exit: bool,
) where
    T: EventedPty + OnResize,
    L: EventListener,
{
    let mut buf = vec![0u8; READ_BUFFER_SIZE];
    let poll_opts = PollMode::Level;
    let mut interest = PollingEvent::readable(0);

    // SAFETY: the pty outlives this function, which is the whole body of the thread, and it is
    // deregistered on the way out.
    if let Err(error) = unsafe { pty.register(poller, interest, poll_opts) } {
        log::error!("clay pty loop: could not register the pty: {error}");
        return;
    }

    let mut events = Events::with_capacity(NonZeroUsize::new(1024).unwrap());

    'outer: loop {
        // A synchronized update holds output back until its terminator or a timeout, so the loop
        // has to wake for that even with nothing to read.
        let timeout = state
            .parser
            .sync_timeout()
            .sync_timeout()
            .map(|deadline| deadline.saturating_duration_since(Instant::now()));

        events.clear();
        if let Err(error) = poller.wait(&mut events, timeout) {
            match error.kind() {
                ErrorKind::Interrupted => continue,
                _ => {
                    log::error!("clay pty loop: polling failed: {error}");
                    break 'outer;
                }
            }
        }

        // The channel is drained first, and before the empty-events check below, because
        // `Poller::notify` wakes `wait` with *no* events. Checking for emptiness first would
        // treat every queued write as a synchronized-update timeout and `continue` past it, so
        // nothing the user typed would ever reach the shell.
        let mut had_message = false;
        loop {
            match rx.try_recv() {
                Ok(LoopMsg::Input(bytes)) => {
                    state.writes.push_back(bytes);
                    had_message = true;
                }
                Ok(LoopMsg::Resize(size)) => {
                    pty.on_resize(size);
                    had_message = true;
                }
                Ok(LoopMsg::Shutdown) => break 'outer,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break 'outer,
            }
        }

        // Genuinely nothing to do, so this is the synchronized-update timeout firing.
        if events.is_empty() && !had_message {
            state.parser.stop_sync(&mut *term.lock());
            listener.send_event(AlacTermEvent::Wakeup);
            continue;
        }

        for event in events.iter() {
            match event.key {
                PTY_CHILD_EVENT_TOKEN => {
                    if let Some(ChildEvent::Exited(status)) = pty.next_child_event() {
                        if let Some(status) = status {
                            listener.send_event(AlacTermEvent::ChildExit(status));
                        }
                        if drain_on_exit {
                            // Whatever the command printed just before exiting would otherwise
                            // be lost, which is exactly the output the user wants to read.
                            let _ = pty_read(state, &term, &listener, pty, &mut buf);
                        }
                        term.lock().exit();
                        listener.send_event(AlacTermEvent::Wakeup);
                        break 'outer;
                    }
                }
                PTY_READ_WRITE_TOKEN => {
                    if event.is_interrupt() {
                        // Do not do I/O on a dead PTY; loop round for the Exited event.
                        continue;
                    }
                    if event.readable {
                        if let Err(error) = pty_read(state, &term, &listener, pty, &mut buf) {
                            log::error!("clay pty loop: read failed: {error}");
                            break 'outer;
                        }
                    }
                    if event.writable {
                        if let Err(error) = pty_write(state, pty) {
                            log::error!("clay pty loop: write failed: {error}");
                            break 'outer;
                        }
                    }
                }
                _ => {}
            }
        }

        // A `notify` wake carries no poll event, so a queued write would otherwise sit until the
        // PTY happened to produce output. Attempt it directly.
        if state.wants_write() {
            if let Err(error) = pty_write(state, pty) {
                log::error!("clay pty loop: write failed: {error}");
                break 'outer;
            }
        }

        // Ask for writability only while there is something left over. Asking always would spin
        // the loop, since a PTY is almost always writable.
        let wants_write = state.wants_write();
        if wants_write != interest.writable {
            interest.writable = wants_write;
            if let Err(error) = pty.reregister(poller, interest, poll_opts) {
                log::error!("clay pty loop: could not update interest: {error}");
                break 'outer;
            }
        }
    }

    let _ = pty.deregister(poller);
}

/// Reads what the PTY has, tees it through the scanner, and feeds the parser.
fn pty_read<T, L>(
    state: &mut LoopState,
    term: &Arc<FairMutex<Term<L>>>,
    listener: &L,
    pty: &mut T,
    buf: &mut [u8],
) -> std::io::Result<()>
where
    T: EventedPty,
    L: EventListener,
{
    let mut unprocessed = 0;
    let mut processed = 0;
    // Reserving the next lock for this read is what stops the UI thread starving the reader
    // under heavy output; alacritty's `FairMutex` exists for precisely this.
    let _lease = Some(term.lease());
    let mut terminal = None;

    loop {
        match pty.reader().read(&mut buf[unprocessed..]) {
            // Windows reports this when the PTY has nothing more.
            Ok(0) if unprocessed == 0 => break,
            Ok(got) => unprocessed += got,
            Err(error) => match error.kind() {
                ErrorKind::Interrupted | ErrorKind::WouldBlock => {
                    if unprocessed == 0 {
                        break;
                    }
                }
                _ => return Err(error),
            },
        }

        let terminal = match &mut terminal {
            Some(terminal) => terminal,
            None => terminal.insert(match term.try_lock_unfair() {
                // Block rather than grow the buffer past its limit.
                None if unprocessed >= READ_BUFFER_SIZE => term.lock_unfair(),
                None => continue,
                Some(terminal) => terminal,
            }),
        };

        // The tee: the scanner sees a copy, the parser gets the bytes unchanged.
        let marks = state.scanner.feed(&buf[..unprocessed]);
        if let Some(sender) = &state.marks {
            for mark in marks {
                // A closed receiver is not fatal — shell integration going quiet should never
                // stop the terminal working.
                let _ = sender.unbounded_send(mark);
            }
        }

        state.parser.advance(&mut **terminal, &buf[..unprocessed]);

        processed += unprocessed;
        unprocessed = 0;

        if processed >= MAX_LOCKED_READ {
            break;
        }
    }

    // Bytes swallowed by a synchronized update have not been displayed, so they are not a reason
    // to redraw.
    if state.parser.sync_bytes_count() < processed && processed > 0 {
        listener.send_event(AlacTermEvent::Wakeup);
    }

    Ok(())
}

/// Writes as much of the pending queue as the PTY will take.
fn pty_write<T>(state: &mut LoopState, pty: &mut T) -> std::io::Result<()>
where
    T: EventedPty,
{
    while let Some(front) = state.writes.front() {
        match pty.writer().write(&front[state.written..]) {
            Ok(0) => break,
            Ok(wrote) => {
                state.written += wrote;
                if state.written >= front.len() {
                    state.writes.pop_front();
                    state.written = 0;
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::Interrupted | ErrorKind::WouldBlock => break,
                _ => return Err(error),
            },
        }
    }
    Ok(())
}
