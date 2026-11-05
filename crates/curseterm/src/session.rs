use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::tty;
use barrier_kit::platform::WakeHandle;

pub type SessionId = u64;

#[derive(Clone)]
pub struct Listener {
    id: SessionId,
    events: Sender<(SessionId, Event)>,
    wake: WakeHandle,
}

impl Listener {
    pub fn new(id: SessionId, events: Sender<(SessionId, Event)>, wake: WakeHandle) -> Self {
        Self { id, events, wake }
    }
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        if self.events.send((self.id, event)).is_ok() {
            self.wake.wake();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSize {
    pub columns: u16,
    pub lines: u16,
    pub cell_width: f32,
    pub cell_height: f32,
}

impl GridSize {
    fn window_size(self) -> WindowSize {
        WindowSize {
            num_lines: self.lines,
            num_cols: self.columns,
            cell_width: self.cell_width.round() as u16,
            cell_height: self.cell_height.round() as u16,
        }
    }

    fn term_size(self) -> TermSize {
        TermSize::new(usize::from(self.columns), usize::from(self.lines))
    }
}

pub struct Session {
    pub id: SessionId,
    pub term: Arc<FairMutex<Term<Listener>>>,
    sender: EventLoopSender,
    size: Cell<GridSize>,
}

impl Session {
    pub fn spawn(id: SessionId, size: GridSize, listener: Listener) -> io::Result<Self> {
        let options = tty::Options {
            shell: None,
            working_directory: None,
            drain_on_exit: false,
            env: HashMap::from([
                ("TERM".into(), "xterm-256color".into()),
                ("COLORTERM".into(), "truecolor".into()),
                ("TERM_PROGRAM".into(), "CurseTerm".into()),
            ]),
        };
        let pty = tty::new(&options, size.window_size(), id)?;
        let term = Arc::new(FairMutex::new(Term::new(
            Config::default(),
            &size.term_size(),
            listener.clone(),
        )));
        let event_loop = EventLoop::new(term.clone(), listener, pty, false, false)?;
        let sender = event_loop.channel();
        event_loop.spawn();
        Ok(Self {
            id,
            term,
            sender,
            size: Cell::new(size),
        })
    }

    pub fn write(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return;
        }
        self.term.lock().scroll_display(Scroll::Bottom);
        if let Err(error) = self.sender.send(Msg::Input(bytes)) {
            tracing::warn!(%error, session = self.id, "the shell no longer reads input");
        }
    }

    pub fn size(&self) -> GridSize {
        self.size.get()
    }

    pub fn resize(&self, size: GridSize) {
        if size == self.size.get() || size.columns == 0 || size.lines == 0 {
            return;
        }
        self.size.set(size);
        self.term.lock().resize(size.term_size());
        let _ = self.sender.send(Msg::Resize(size.window_size()));
    }

    pub fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }

    pub fn scroll(&self, scroll: Scroll) {
        self.term.lock().scroll_display(scroll);
    }

    pub fn paste(&self, text: &str) {
        let bracketed = self.mode().contains(TermMode::BRACKETED_PASTE);
        self.write(paste_bytes(text, bracketed));
    }

    pub fn selected_text(&self) -> Option<String> {
        self.term.lock().selection_to_string()
    }

    pub fn select_all(&self) {
        let mut term = self.term.lock();
        let top = Point::new(Line(-(term.history_size() as i32)), Column(0));
        let bottom = Point::new(
            Line(term.screen_lines() as i32 - 1),
            Column(term.columns().saturating_sub(1)),
        );
        let mut selection = Selection::new(SelectionType::Simple, top, Side::Left);
        selection.update(bottom, Side::Right);
        term.selection = Some(selection);
    }

    pub fn start_selection(&self, kind: SelectionType, point: Point, side: Side) {
        self.term.lock().selection = Some(Selection::new(kind, point, side));
    }

    pub fn extend_selection(&self, point: Point, side: Side) {
        if let Some(selection) = self.term.lock().selection.as_mut() {
            selection.update(point, side);
        }
    }

    pub fn clear_selection(&self) {
        self.term.lock().selection = None;
    }

    pub fn display_offset(&self) -> usize {
        self.term.lock().grid().display_offset()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.sender.send(Msg::Shutdown);
    }
}

fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    if bracketed {
        let clean = text.replace('\x1b', "");
        format!("\x1b[200~{clean}\x1b[201~").into_bytes()
    } else {
        text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::paste_bytes;

    #[test]
    fn a_bracketed_paste_is_wrapped_and_cannot_close_the_bracket_early() {
        assert_eq!(
            paste_bytes("ls\x1b[201~rm", true),
            b"\x1b[200~ls[201~rm\x1b[201~".to_vec()
        );
    }

    #[test]
    fn a_plain_paste_turns_newlines_into_carriage_returns() {
        assert_eq!(paste_bytes("a\nb\r\nc", false), b"a\rb\rc".to_vec());
    }
}
