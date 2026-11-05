use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard};

use alacritty_terminal::event::{Event, EventListener, OnResize, WindowSize};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Boundary, Column, Direction, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::search::RegexSearch;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::tty;
use alacritty_terminal::vte::ansi::Handler;
use barrier_kit::platform::WakeHandle;

use crate::io::{self as pty_io, Extras};

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
    pub fn fitting(width: f32, height: f32, cell_width: f32, cell_height: f32) -> Self {
        Self {
            columns: (width / cell_width).floor().max(1.0) as u16,
            lines: (height / cell_height).floor().max(1.0) as u16,
            cell_width,
            cell_height,
        }
    }

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

pub enum MarkJump {
    Latest,
    Previous,
    Next,
}

pub struct Launch {
    pub profile: String,
    pub command: Option<String>,
    pub directory: Option<PathBuf>,
}

pub struct Session {
    pub id: SessionId,
    pub profile: String,
    pub term: Arc<FairMutex<Term<Listener>>>,
    extras: Arc<Mutex<Extras>>,
    pty: RefCell<tty::Pty>,
    sender: Sender<Cow<'static, [u8]>>,
    size: Cell<GridSize>,
    area: Cell<Option<(f32, f32)>>,
    shell_pid: u32,
}

impl Session {
    pub fn spawn(
        id: SessionId,
        size: GridSize,
        launch: Launch,
        listener: Listener,
    ) -> io::Result<Self> {
        let options = tty::Options {
            shell: launch
                .command
                .map(|command| tty::Shell::new("/bin/sh".into(), vec!["-c".into(), command])),
            working_directory: launch.directory,
            drain_on_exit: false,
            env: HashMap::from([
                ("TERM".into(), "xterm-256color".into()),
                ("COLORTERM".into(), "truecolor".into()),
                ("TERM_PROGRAM".into(), "CurseTerm".into()),
            ]),
        };
        let mut pty = tty::new(&options, size.window_size(), id)?;
        let shell_pid = pty.child().id();
        let term = Arc::new(FairMutex::new(Term::new(
            Config::default(),
            &size.term_size(),
            listener.clone(),
        )));
        let extras = Arc::new(Mutex::new(Extras {
            cell: (size.cell_width, size.cell_height),
            ..Extras::default()
        }));
        let sender = pty_io::start(&mut pty, term.clone(), listener, extras.clone())?;
        Ok(Self {
            id,
            profile: launch.profile,
            term,
            extras,
            pty: RefCell::new(pty),
            sender,
            size: Cell::new(size),
            area: Cell::new(None),
            shell_pid,
        })
    }

    pub fn write(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return;
        }
        self.term.lock().scroll_display(Scroll::Bottom);
        if let Err(error) = self.sender.send(bytes) {
            tracing::warn!(%error, session = self.id, "the shell no longer reads input");
        }
    }

    pub fn size(&self) -> GridSize {
        self.size.get()
    }

    pub fn working_directory(&self) -> Option<PathBuf> {
        let reported = self.extras().directory.clone();
        reported.or_else(|| std::fs::read_link(format!("/proc/{}/cwd", self.shell_pid)).ok())
    }

    pub fn extras(&self) -> MutexGuard<'_, Extras> {
        self.extras
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn fit(&self, width: f32, height: f32, cell_width: f32, cell_height: f32) {
        self.area.set(Some((width, height)));
        self.resize(GridSize::fitting(width, height, cell_width, cell_height));
    }

    pub fn refit(&self, cell_width: f32, cell_height: f32) {
        if let Some((width, height)) = self.area.get() {
            self.resize(GridSize::fitting(width, height, cell_width, cell_height));
        }
    }

    fn resize(&self, size: GridSize) {
        if size == self.size.get() || size.columns == 0 || size.lines == 0 {
            return;
        }
        self.size.set(size);
        let mut term = self.term.lock();
        let mut extras = self.extras();
        pty_io::reflow(&mut term, &mut extras, |term| term.resize(size.term_size()));
        extras.cell = (size.cell_width, size.cell_height);
        drop((extras, term));
        self.pty.borrow_mut().on_resize(size.window_size());
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

    pub fn reset(&self) {
        self.term.lock().reset_state();
        self.forget_everything();
    }

    pub fn clear_scrollback(&self) {
        let mut term = self.term.lock();
        let history = term.history_size() as i64;
        term.grid_mut().clear_history();
        self.extras().forget_lines_above(history);
    }

    pub fn clear_buffer(&self) {
        let mut term = self.term.lock();
        let cursor = term.grid().cursor.point;
        term.scroll_up(cursor.line.0.max(0) as usize);
        term.grid_mut().clear_history();
        term.goto(0, cursor.column.0);
        self.forget_everything();
    }

    fn forget_everything(&self) {
        let mut extras = self.extras();
        extras.marks.clear();
        extras.images.clear();
    }

    pub fn set_mark(&self) {
        let line = pty_io::absolute_line(&self.term.lock());
        self.extras().add_mark(line);
    }

    pub fn has_marks(&self) -> bool {
        !self.extras().marks.is_empty()
    }

    pub fn jump_to_mark(&self, jump: MarkJump) {
        let mut term = self.term.lock();
        let history = term.history_size() as i64;
        let offset = term.grid().display_offset() as i64;
        let top = history - offset;
        let extras = self.extras();
        let marks = &extras.marks;
        let target = match jump {
            MarkJump::Latest => marks.last(),
            MarkJump::Previous => marks.iter().rev().find(|mark| **mark < top),
            MarkJump::Next => marks.iter().find(|mark| **mark > top),
        };
        if let Some(line) = target {
            let wanted = (history - line).clamp(0, history);
            term.scroll_display(Scroll::Delta((wanted - offset) as i32));
        }
    }

    pub fn find(&self, query: &str, direction: Direction) -> bool {
        let Ok(mut regex) = RegexSearch::new(&literal_pattern(query)) else {
            return false;
        };
        let mut term = self.term.lock();
        let current = term
            .selection
            .as_ref()
            .and_then(|selection| selection.to_range(&term));
        let origin = match (current, direction) {
            (Some(range), Direction::Right) => range.end.add(&*term, Boundary::Grid, 1),
            (Some(range), Direction::Left) => range.start.sub(&*term, Boundary::Grid, 1),
            (None, _) => term.grid().cursor.point,
        };
        let side = match direction {
            Direction::Right => Side::Left,
            Direction::Left => Side::Right,
        };
        let Some(found) = term.search_next(&mut regex, origin, direction, side, None) else {
            return false;
        };
        term.scroll_to_point(*found.start());
        let mut selection = Selection::new(SelectionType::Simple, *found.start(), Side::Left);
        selection.update(*found.end(), Side::Right);
        term.selection = Some(selection);
        true
    }

    pub fn has_scrollback(&self) -> bool {
        self.term.lock().history_size() > 0
    }

    pub fn display_offset(&self) -> usize {
        self.term.lock().grid().display_offset()
    }
}

fn literal_pattern(query: &str) -> String {
    query
        .chars()
        .flat_map(|c| match c {
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$'
            | '#' | '&' | '-' | '~' => vec!['\\', c],
            c => vec![c],
        })
        .collect()
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
