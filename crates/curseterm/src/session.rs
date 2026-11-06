use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{self, Read};
use std::ops::RangeInclusive;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use alacritty_terminal::event::{Event, EventListener, OnResize, WindowSize};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Boundary, Column, Direction, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::{Cell as GridCell, Flags};
use alacritty_terminal::term::search::{Match, RegexSearch};
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::tty;
use alacritty_terminal::vte::ansi::{CursorShape, CursorStyle, Handler, Processor};
use barrier_kit::platform::WakeHandle;
use barrier_kit_next::tokens::GLOBAL_SEARCH;

use regex::Regex;
use rustix::process::{Pid, Signal, kill_process_group};

use crate::activity::Activity;
use crate::composer::Composer;
use crate::config::{APP_NAME, CursorType, Profile, SCROLLBACK_LIMIT};
use crate::hints;
use crate::io::{self as pty_io, Extras};
use crate::jobs;
use crate::key_report::{KITTY_FLAGS, KeyMode};
use crate::keys::CopyCommand;
use crate::links;
use crate::profile_switching::{Place, Switcher};
use crate::semantic_history::{self, Action, SemanticHistory, Target};
use crate::session_note::SessionNote;
use crate::smart_selection::{self, Precision};
use crate::status;

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

const PLACE_CHECK_INTERVAL: Duration = Duration::from_millis(500);

pub struct Launch {
    pub profile: String,
    pub command: Option<String>,
    pub directory: Option<PathBuf>,
}

pub struct Session {
    pub id: SessionId,
    profile: RefCell<String>,
    pub term: Arc<FairMutex<Term<Listener>>>,
    extras: Arc<Mutex<Extras>>,
    pty: RefCell<tty::Pty>,
    sender: Sender<Cow<'static, [u8]>>,
    size: Cell<GridSize>,
    area: Cell<Option<(f32, f32)>>,
    pub origin: Cell<(f32, f32)>,
    shell_pid: u32,
    runs_command: bool,
    pub started: Instant,
    ended: Cell<bool>,
    options: Cell<(usize, CursorStyle, bool)>,
    coprocess: RefCell<Option<Child>>,
    switcher: RefCell<Switcher>,
    semantic_history: RefCell<SemanticHistory>,
    place_checked: Cell<Instant>,
    pub place_check_queued: Cell<bool>,
    pub composer: RefCell<Composer>,
    pub note: RefCell<SessionNote>,
    pub activity: Cell<Activity>,
    pub idle_armed: Cell<bool>,
}

// Autocomplete.m searches back from the cursor; this many lines is enough for a prompt.
const AUTOCOMPLETE_LINES: usize = 500;

thread_local! {
    static SEARCH: RefCell<Option<(String, RegexSearch)>> = const { RefCell::new(None) };
}

fn with_search<R>(pattern: &str, search: impl FnOnce(&mut RegexSearch) -> R) -> Option<R> {
    SEARCH.with(|cached| {
        let mut cached = cached.borrow_mut();
        if cached.as_ref().is_none_or(|(known, _)| known != pattern) {
            *cached = Some((pattern.to_string(), RegexSearch::new(pattern).ok()?));
        }
        cached.as_mut().map(|(_, regex)| search(regex))
    })
}

impl Session {
    pub fn spawn(
        id: SessionId,
        size: GridSize,
        launch: Launch,
        listener: Listener,
    ) -> io::Result<Self> {
        let runs_command = launch.command.is_some();
        let options = tty::Options {
            shell: launch
                .command
                .map(|command| tty::Shell::new("/bin/sh".into(), vec!["-c".into(), command])),
            working_directory: launch.directory,
            drain_on_exit: false,
            env: HashMap::from([
                ("TERM".into(), "xterm-256color".into()),
                ("COLORTERM".into(), "truecolor".into()),
                ("TERM_PROGRAM".into(), APP_NAME.into()),
            ]),
        };
        let mut pty = tty::new(&options, size.window_size(), id)?;
        let shell_pid = pty.child().id();
        let term = Arc::new(FairMutex::new(Term::new(
            Config::default(),
            &size.term_size(),
            listener.clone(),
        )));
        let extras = Arc::new(Mutex::new(Extras::new((size.cell_width, size.cell_height))));
        let sender = pty_io::start(&mut pty, term.clone(), listener, extras.clone())?;
        Ok(Self {
            id,
            profile: RefCell::new(launch.profile),
            term,
            extras,
            pty: RefCell::new(pty),
            sender,
            size: Cell::new(size),
            area: Cell::new(None),
            origin: Cell::new((0.0, 0.0)),
            shell_pid,
            runs_command,
            started: Instant::now(),
            ended: Cell::new(false),
            options: Cell::new((
                Config::default().scrolling_history,
                CursorStyle::default(),
                false,
            )),
            coprocess: RefCell::new(None),
            switcher: RefCell::default(),
            semantic_history: RefCell::default(),
            place_checked: Cell::new(Instant::now() - PLACE_CHECK_INTERVAL),
            place_check_queued: Cell::new(false),
            composer: RefCell::default(),
            note: RefCell::default(),
            activity: Cell::default(),
            idle_armed: Cell::new(false),
        })
    }

    pub fn profile(&self) -> String {
        self.profile.borrow().clone()
    }

    // PTYSession.m -setProfile:preservingName:: the session takes another profile's settings.
    pub fn change_profile(&self, profile: &Profile) {
        *self.profile.borrow_mut() = profile.name.clone();
        self.apply_profile(profile);
    }

    pub fn apply_profile(&self, profile: &Profile) {
        {
            let mut extras = self.extras();
            extras.set_triggers(&profile.triggers);
            extras.set_smart_selection_rules(&profile.smart_selection_rules);
            *self.semantic_history.borrow_mut() = profile.semantic_history.clone();
            extras.apps_change_key_mode = profile.apps_change_key_reporting;
            // PTYSession.m -setProfile: a profile asking for CSI u sets the session's mode.
            match (profile.report_keys_csi_u, extras.key_mode) {
                (true, KeyMode::Standard) => extras.key_mode = KeyMode::CsiU,
                (false, KeyMode::CsiU) => extras.key_mode = KeyMode::Standard,
                _ => {}
            }
        }
        let lines = profile.scrollback_lines.min(SCROLLBACK_LIMIT) as usize;
        let shape = match profile.cursor {
            CursorType::Box => CursorShape::Block,
            CursorType::VerticalBar => CursorShape::Beam,
            CursorType::Underline => CursorShape::Underline,
        };
        let cursor = CursorStyle {
            shape,
            blinking: profile.blinking_cursor,
        };
        let kitty = profile.apps_change_key_reporting;
        if self.options.replace((lines, cursor, kitty)) != (lines, cursor, kitty) {
            self.term.lock().set_options(Config {
                scrolling_history: lines,
                default_cursor_style: cursor,
                kitty_keyboard: kitty,
                ..Config::default()
            });
        }
    }

    pub fn key_mode(&self) -> KeyMode {
        self.extras().key_mode
    }

    pub fn set_key_mode(&self, mode: KeyMode) {
        self.extras().key_mode = mode;
    }

    // PTYSession.m -terminalStateToggle…: tags 12 to 16 flip one kitty flag, CSI = flags ; 1 u.
    pub fn toggle_kitty_flag(&self, flag: TermMode) {
        let mode = self.mode();
        let bits = KITTY_FLAGS
            .iter()
            .enumerate()
            .filter(|(_, known)| mode.contains(**known) != (**known == flag))
            .map(|(index, _)| 1u8 << index)
            .sum::<u8>();
        self.feed(format!("\x1b[={bits};1u").as_bytes());
    }

    pub fn write(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        let bytes = bytes.into();
        if bytes.is_empty() || self.ended.get() {
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

    // PTYTextView -textViewOpen:workingDirectory: a relative path opens from the shell's folder.
    pub fn open_target(&self, text: &str) -> String {
        self.working_directory()
            .map(|folder| folder.join(text))
            .filter(|path| path.exists())
            .map_or_else(|| text.to_string(), |path| path.display().to_string())
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
        pty_io::resize_keeping_anchors(&mut term, &mut extras, |term| {
            term.resize(size.term_size())
        });
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

    pub fn send_command(&self, command: &str) {
        self.write(format!("{}\r", command.replace('\n', "\r")).into_bytes());
    }

    pub fn paste(&self, text: &str) {
        self.write(self.pasted_bytes(text));
    }

    pub fn pasted_bytes(&self, text: &str) -> Vec<u8> {
        paste_bytes(text, self.mode().contains(TermMode::BRACKETED_PASTE))
    }

    pub fn start_log(&self, path: &std::path::Path) -> io::Result<()> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        self.extras().log = Some(file);
        Ok(())
    }

    pub fn start_coprocess(&self, command: &str) -> io::Result<()> {
        self.stop_coprocess();
        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .current_dir(self.working_directory().unwrap_or_else(|| "/".into()))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()?;
        if let Some(mut output) = child.stdout.take() {
            let sender = self.sender.clone();
            thread::spawn(move || {
                let mut buffer = [0; 4096];
                while let Ok(read @ 1..) = output.read(&mut buffer) {
                    if sender.send(buffer[..read].to_vec().into()).is_err() {
                        break;
                    }
                }
            });
        }
        self.extras().coprocess = child.stdin.take();
        *self.coprocess.borrow_mut() = Some(child);
        Ok(())
    }

    pub fn stop_coprocess(&self) {
        self.extras().coprocess = None;
        if let Some(mut child) = self.coprocess.borrow_mut().take() {
            if let Some(group) = Pid::from_raw(child.id() as i32) {
                let _ = kill_process_group(group, Signal::TERM);
            }
            let _ = child.wait();
        }
    }

    pub fn coprocess_running(&self) -> bool {
        let mut coprocess = self.coprocess.borrow_mut();
        let ended = coprocess
            .as_mut()
            .is_some_and(|child| !matches!(child.try_wait(), Ok(None)));
        if ended {
            *coprocess = None;
            self.extras().coprocess = None;
        }
        coprocess.is_some()
    }

    pub fn stop_log(&self) {
        self.extras().log = None;
    }

    pub fn logging(&self) -> bool {
        self.extras().log.is_some()
    }

    pub fn contents(&self) -> String {
        let term = self.term.lock();
        let top = Point::new(Line(-(term.history_size() as i32)), Column(0));
        let bottom = Point::new(
            Line(term.screen_lines() as i32 - 1),
            Column(term.columns().saturating_sub(1)),
        );
        let text = term.bounds_to_string(top, bottom);
        format!("{}\n", text.trim_end())
    }

    // iTermSelection.m -enumerateSelectedAbsoluteRanges: each sub-selection ends its own line.
    pub fn selected_text(&self) -> Option<String> {
        let term = self.term.lock();
        if let Some(text) = term.selection_to_string() {
            return Some(text);
        }
        let extras = self.extras();
        let history = term.history_size() as i64;
        let point = |(line, column): (i64, usize)| {
            Point::new(Line((line - history) as i32), Column(column))
        };
        (!extras.sub_selections.is_empty()).then(|| {
            extras
                .sub_selections
                .iter()
                .map(|(start, end)| term.bounds_to_string(point(*start), point(*end)))
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    // PTYTextView+ARC.m -convertMatchesToSelections
    pub fn select_matches(&self, pattern: &str) {
        let mut term = self.term.lock();
        let start = Point::new(Line(-(term.history_size() as i32)), Column(0));
        let end = Point::new(
            Line(term.screen_lines() as i32 - 1),
            Column(term.columns() - 1),
        );
        let history = term.history_size() as i64;
        let absolute = |point: &Point| (history + i64::from(point.line.0), point.column.0);
        let ranges = matches_between(&term, pattern, start, end)
            .iter()
            .map(|found| (absolute(found.start()), absolute(found.end())))
            .collect();
        term.selection = None;
        self.extras().sub_selections = ranges;
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
        self.extras().sub_selections.clear();
    }

    pub fn extend_selection(&self, point: Point, side: Side) {
        if let Some(selection) = self.term.lock().selection.as_mut() {
            selection.update(point, side);
        }
    }

    pub fn clear_selection(&self) {
        let mut term = self.term.lock();
        term.selection = None;
        self.extras().sub_selections.clear();
    }

    pub fn reset(&self) {
        self.term.lock().reset_state();
        self.extras().forget_everything();
    }

    pub fn clear_scrollback(&self) {
        let mut term = self.term.lock();
        let history = term.history_size() as i64;
        term.grid_mut().clear_history();
        self.extras().forget_lines(0..history);
    }

    pub fn clear_buffer(&self) {
        let mut term = self.term.lock();
        let cursor = term.grid().cursor.point;
        term.scroll_up(cursor.line.0.max(0) as usize);
        term.grid_mut().clear_history();
        term.goto(0, cursor.column.0);
        self.extras().forget_everything();
    }

    pub fn last_mark_above_cursor(&self) -> Option<i64> {
        let cursor = pty_io::absolute_line(&self.term.lock());
        self.extras()
            .marks
            .iter()
            .rev()
            .find(|mark| **mark < cursor)
            .copied()
    }

    pub fn clear_to_last_mark(&self) {
        let Some(mark) = self.last_mark_above_cursor() else {
            return;
        };
        let cursor = pty_io::absolute_line(&self.term.lock());
        // VT100Screen.m clearToLastMark: keep the marked line unless it is just above the cursor.
        self.clear_from(if mark == cursor - 1 { mark } else { mark + 1 });
    }

    pub fn selection_start(&self) -> Option<i64> {
        let term = self.term.lock();
        let range = term.selection.as_ref()?.to_range(&term)?;
        Some(term.history_size() as i64 + i64::from(range.start.line.0))
    }

    pub fn clear_from(&self, line: i64) {
        let mut term = self.term.lock();
        let history = term.history_size() as i64;
        let cursor = pty_io::absolute_line(&term);
        let start = line.max(history);
        if start >= cursor {
            return;
        }
        let column = term.grid().cursor.point.column.0;
        let top = (start - history) as i32;
        term.selection = None;
        term.goto(top, 0);
        term.delete_lines((cursor - start) as usize);
        term.goto(top, column);
        self.extras().forget_lines(start..cursor);
    }

    pub fn set_mark(&self) {
        let line = pty_io::absolute_line(&self.term.lock());
        self.extras().add_mark(line);
    }

    // PTYSession.m -saveScrollPositionWithName: marks the top visible line and labels it with an
    // annotation across the whole line.
    pub fn set_named_mark(&self, name: &str, label: String) {
        let term = self.term.lock();
        let line = term.history_size() as i64 - term.grid().display_offset() as i64;
        let columns = term.columns();
        drop(term);
        let mut extras = self.extras();
        extras.add_mark(line);
        extras.named_marks.push((line, name.to_string()));
        extras.annotate((line, 0), (line, columns));
        if let Some(annotation) = extras.annotations.last_mut() {
            annotation.text = label;
            annotation.focus = false;
        }
    }

    pub fn named_marks(&self) -> Vec<String> {
        self.extras()
            .named_marks
            .iter()
            .map(|(_, name)| name.clone())
            .collect()
    }

    pub fn jump_to_named_mark(&self, index: usize) {
        let line = self.extras().named_marks.get(index).map(|(line, _)| *line);
        if let Some(line) = line {
            self.scroll_into_view(line..=line);
        }
    }

    // PTYTextView ⌘-click → iTermSemanticHistoryController.m -openPath:…: a link opens; a file
    // under the pointer does what the profile's Semantic History says.
    pub fn open_at(&self, point: Point) -> bool {
        let term = self.term.lock();
        let link = term.grid()[point]
            .hyperlink()
            .map(|link| link.uri().to_string());
        let (text, columns) = pty_io::trimmed_text_and_byte_columns(&term, point.line);
        drop(term);
        if let Some(url) = link.or_else(|| links::url_at(&text, &columns, point.column.0)) {
            links::open(&url);
            return true;
        }
        let Some(span) = links::word_span(&text, &columns, point.column.0) else {
            return false;
        };
        let raw = &text[span.clone()];
        let (file, line, column) = semantic_history::split_position(raw);
        let directory = self.working_directory().unwrap_or_default();
        let path = directory.join(file);
        let history = self.semantic_history.borrow().clone();
        let target = Target {
            path: if history.action == Action::RawCommand {
                raw.to_string()
            } else {
                path.display().to_string()
            },
            line: line.to_string(),
            column: column.to_string(),
            prefix: text[..span.start].to_string(),
            suffix: text[span.end..].to_string(),
            directory: directory.display().to_string(),
        };
        let command = || semantic_history::substitute(&history.text, &target, shell_escaped);
        match history.action {
            Action::RawCommand => pty_io::run_command(&command()),
            _ if !path.exists() => return false,
            Action::Command => pty_io::run_command(&command()),
            Action::Coprocess => {
                if let Err(error) = self.start_coprocess(&command()) {
                    tracing::warn!(%error, "the Semantic History coprocess could not start");
                }
            }
            Action::SendText => self.write(command().into_bytes()),
            Action::Url if !path.is_dir() => links::open(&semantic_history::substitute(
                &history.text,
                &target,
                semantic_history::percent_encoded,
            )),
            Action::Url | Action::DefaultApp => links::open(&target.path),
        }
        true
    }

    pub fn smart_select(&self, point: Point) {
        let rules = self.extras().smart_selection.clone();
        let mut term = self.term.lock();
        let Some((first, last)) = smart_span(&term, point, &rules) else {
            return;
        };
        let mut selection = Selection::new(SelectionType::Simple, first, Side::Left);
        selection.update(last, Side::Right);
        term.selection = Some(selection);
    }

    // The order of the tries follows PTYSession.m -addNoteAtCursor.
    pub fn annotate_cursor(&self) {
        let rules = self.extras().smart_selection.clone();
        let term = self.term.lock();
        let cursor = term.grid().cursor.point;
        let line = pty_io::absolute_line(&term);
        let around = [cursor.column.0, cursor.column.0 + 1];
        let span = around
            .into_iter()
            .chain(cursor.column.0.checked_sub(1))
            .filter(|column| *column < term.columns())
            .find_map(|column| smart_span(&term, Point::new(cursor.line, Column(column)), &rules));
        let history = term.history_size() as i64;
        let absolute = |point: Point| (history + i64::from(point.line.0), point.column.0);
        let (start, end) = span.map_or(((line, 0), (line, term.columns())), |(first, last)| {
            let (row, column) = absolute(last);
            (absolute(first), (row, column + 1))
        });
        drop(term);
        self.extras().annotate(start, end);
    }

    pub fn annotate_selection(&self) {
        let term = self.term.lock();
        let Some(range) = term
            .selection
            .as_ref()
            .and_then(|selection| selection.to_range(&term))
        else {
            return;
        };
        let history = term.history_size() as i64;
        let absolute = |point: Point| history + i64::from(point.line.0);
        let start = (absolute(range.start), range.start.column.0);
        let end = (absolute(range.end), range.end.column.0 + 1);
        drop(term);
        self.extras().annotate(start, end);
    }

    pub fn has_annotations(&self) -> bool {
        !self.extras().annotations.is_empty()
    }

    pub fn toggle_annotations(&self) {
        let mut extras = self.extras();
        let hide = extras
            .annotations
            .iter()
            .any(|annotation| !annotation.hidden);
        let now = Instant::now();
        for annotation in &mut extras.annotations {
            if annotation.hidden != hide {
                annotation.hidden = hide;
                annotation.toggled = Some(now);
            }
        }
    }

    pub fn step_annotation(&self, forward: bool) {
        let mut extras = self.extras();
        let mut order: Vec<_> = extras
            .annotations
            .iter()
            .map(|annotation| (annotation.start, annotation.id))
            .collect();
        order.sort_unstable();
        let current = extras
            .current_annotation
            .and_then(|id| order.iter().find(|(_, other)| *other == id))
            .map(|(start, _)| *start);
        let target = match (current, forward) {
            (Some(at), true) => order.iter().find(|(start, _)| *start > at),
            (Some(at), false) => order.iter().rev().find(|(start, _)| *start < at),
            (None, _) => None,
        }
        .or(if forward { order.first() } else { order.last() });
        let Some(&(_, id)) = target else {
            return;
        };
        extras.current_annotation = Some(id);
        let Some(annotation) = extras.annotation(id) else {
            return;
        };
        let now = Instant::now();
        if annotation.hidden {
            annotation.hidden = false;
            annotation.toggled = Some(now);
        }
        annotation.highlighted = Some(now);
        let lines = annotation.start.0..=annotation.end.0;
        drop(extras);
        self.scroll_into_view(lines);
    }

    // PTYTextView.m -scrollLineNumberRangeIntoView: centers a range that is not on screen.
    fn scroll_into_view(&self, lines: RangeInclusive<i64>) {
        let mut term = self.term.lock();
        let history = term.history_size() as i64;
        let offset = term.grid().display_offset() as i64;
        let screen = term.screen_lines() as i64;
        let top = history - offset;
        if *lines.start() >= top && *lines.end() < top + screen {
            return;
        }
        let middle = (lines.start() + lines.end()) / 2;
        let wanted = (history - (middle - screen / 2)).clamp(0, history);
        term.scroll_display(Scroll::Delta((wanted - offset) as i32));
    }

    pub fn set_annotation_text(&self, id: pty_io::AnnotationId, text: String) {
        if let Some(annotation) = self.extras().annotation(id) {
            annotation.text = text;
        }
    }

    pub fn annotation_focused(&self, id: pty_io::AnnotationId) {
        if let Some(annotation) = self.extras().annotation(id) {
            annotation.focus = false;
        }
    }

    pub fn remove_annotation(&self, id: pty_io::AnnotationId) {
        self.extras()
            .annotations
            .retain(|annotation| annotation.id != id);
    }

    #[cfg(feature = "kit-parcheado")]
    pub fn cursor_cell(&self) -> (usize, i32) {
        let term = self.term.lock();
        let cursor = term.grid().cursor.point;
        (
            cursor.column.0,
            cursor.line.0 + term.grid().display_offset() as i32,
        )
    }

    pub fn jobs(&self) -> Vec<jobs::Job> {
        jobs::of_session(self.shell_pid)
    }

    pub fn has_output(&self) -> bool {
        self.extras().output.is_some()
    }

    pub fn select_output(&self) {
        let Some([start, end]) = self.extras().output else {
            return;
        };
        let mut term = self.term.lock();
        let history = term.history_size() as i64;
        let point = |line: i64| Point::new(Line((line - history) as i32), Column(0));
        let mut selection = Selection::new(SelectionType::Lines, point(start), Side::Left);
        selection.update(point(end), Side::Right);
        term.selection = Some(selection);
        term.scroll_to_point(point(start));
    }

    // PTYSession.m -textViewRangeOfCurrentCommand: at a prompt, what has been typed after it;
    // otherwise the last command.
    fn current_command(&self) -> Option<((i64, usize), (i64, usize))> {
        // The reader thread takes the terminal before the extras, so the extras guard must be
        // gone before the terminal is locked.
        let (command_start, command) = {
            let extras = self.extras();
            (extras.command_start, extras.command)
        };
        match command_start {
            Some(start) => {
                let term = self.term.lock();
                Some((
                    start,
                    (
                        pty_io::absolute_line(&term),
                        term.grid().cursor.point.column.0,
                    ),
                ))
            }
            None => command,
        }
        .filter(|(start, end)| end > start)
    }

    // PTYSession.m -isAtShellPrompt: between OSC 133;B and 133;C.
    pub fn at_prompt(&self) -> bool {
        self.extras().command_start.is_some()
    }

    // PTYSession.m -currentCommand: what is typed at the prompt so far.
    pub fn typed_command(&self) -> Option<String> {
        let start = self.extras().command_start?;
        let term = self.term.lock();
        let history = term.history_size() as i64;
        let end = (
            pty_io::absolute_line(&term),
            term.grid().cursor.point.column.0,
        );
        if end <= start {
            return Some(String::new());
        }
        let point = |(line, column): (i64, usize)| {
            Point::new(Line((line - history) as i32), Column(column))
        };
        Some(
            term.bounds_to_string(point(start), point(end).sub(&*term, Boundary::None, 1))
                .replace('\n', ""),
        )
    }

    // Autocomplete.m: the word before the cursor and the lines from the cursor upwards.
    pub fn autocomplete_context(&self) -> (String, Vec<String>) {
        let term = self.term.lock();
        let cursor = term.grid().cursor.point;
        let line_start = Point::new(cursor.line, Column(0));
        let before = if cursor.column.0 > 0 {
            term.bounds_to_string(line_start, Point::new(cursor.line, cursor.column - 1))
        } else {
            String::new()
        };
        let prefix = before
            .rsplit(char::is_whitespace)
            .next()
            .unwrap_or_default()
            .to_string();
        let top = -(term.history_size() as i32);
        let lines = (top..=cursor.line.0)
            .rev()
            .take(AUTOCOMPLETE_LINES)
            .map(|line| {
                let line = Line(line);
                term.bounds_to_string(
                    Point::new(line, Column(0)),
                    Point::new(line, Column(term.columns() - 1)),
                )
            })
            .collect();
        (prefix, lines)
    }

    pub fn has_current_command(&self) -> bool {
        self.current_command().is_some()
    }

    pub fn select_current_command(&self) {
        let Some((start, end)) = self.current_command() else {
            return;
        };
        let mut term = self.term.lock();
        let history = term.history_size() as i64;
        let point = |(line, column): (i64, usize)| {
            Point::new(Line((line - history) as i32), Column(column))
        };
        let mut selection = Selection::new(SelectionType::Simple, point(start), Side::Left);
        selection.update(point(end), Side::Left);
        term.selection = Some(selection);
    }

    // AsyncFilter.swift: whole logical lines that match, keeping as many of the newest as the
    // screen holds.
    pub fn filtered_rows(&self, pattern: &str) -> Vec<Vec<GridCell>> {
        let Ok(regex) = regex::Regex::new(pattern) else {
            return Vec::new();
        };
        let term = self.term.lock();
        let mut rows = Vec::new();
        let mut logical: Vec<Vec<GridCell>> = Vec::new();
        let mut text = String::new();
        let top = -(term.history_size() as i32);
        for line in top..term.screen_lines() as i32 {
            let row: Vec<GridCell> = term.grid()[Line(line)].into_iter().cloned().collect();
            let wraps = row
                .last()
                .is_some_and(|cell| cell.flags.contains(Flags::WRAPLINE));
            text.extend(
                row.iter()
                    .filter(|cell| !cell.flags.contains(Flags::WIDE_CHAR_SPACER))
                    .map(|cell| cell.c),
            );
            logical.push(row);
            if !wraps {
                if regex.is_match(&text) {
                    rows.append(&mut logical);
                } else {
                    logical.clear();
                }
                text.clear();
            }
        }
        let keep = term.screen_lines();
        if rows.len() > keep {
            rows.drain(..rows.len() - keep);
        }
        rows
    }

    pub fn copy_mode(&self) -> bool {
        self.mode().contains(TermMode::VI)
    }

    pub fn toggle_copy_mode(&self) {
        self.term.lock().toggle_vi_mode();
    }

    pub fn copy_mode_step(&self, command: CopyCommand) {
        let mut term = self.term.lock();
        match command {
            CopyCommand::Move(motion) => term.vi_motion(motion),
            CopyCommand::Top => {
                let top = Point::new(Line(-(term.history_size() as i32)), Column(0));
                term.vi_goto_point(top);
            }
            CopyCommand::Bottom => {
                let bottom = Point::new(Line(term.screen_lines() as i32 - 1), Column(0));
                term.vi_goto_point(bottom);
            }
            CopyCommand::PageUp => term.scroll_display(Scroll::PageUp),
            CopyCommand::PageDown => term.scroll_display(Scroll::PageDown),
            CopyCommand::Select(kind) => toggle_vi_selection(&mut term, kind),
            CopyCommand::Copy | CopyCommand::Exit => term.toggle_vi_mode(),
        }
        let cursor = term.vi_mode_cursor.point;
        term.scroll_to_point(cursor);
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

    pub fn find(&self, pattern: &str, direction: Direction) -> bool {
        with_search(pattern, |regex| self.find_with(regex, direction)).unwrap_or(false)
    }

    fn find_with(&self, regex: &mut RegexSearch, direction: Direction) -> bool {
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
        let Some(found) = term.search_next(regex, origin, direction, side, None) else {
            return false;
        };
        term.scroll_to_point(*found.start());
        let mut selection = Selection::new(SelectionType::Simple, *found.start(), Side::Left);
        selection.update(*found.end(), Side::Right);
        term.selection = Some(selection);
        true
    }

    // iTermGlobalSearchEngineCursor.m -snippetFromExtractor:result:, over the whole history.
    pub fn snippets(&self, pattern: &str) -> Vec<Snippet> {
        let term = self.term.lock();
        let start = Point::new(Line(-(term.history_size() as i32)), Column(0));
        let end = Point::new(
            Line(term.screen_lines() as i32 - 1),
            Column(term.columns() - 1),
        );
        let history = term.history_size() as i64;
        let absolute = |point: &Point| (history + i64::from(point.line.0), point.column.0);
        matches_between(&term, pattern, start, end)
            .iter()
            .map(|found| Snippet {
                start: absolute(found.start()),
                end: absolute(found.end()),
                parts: snippet(&term, found),
            })
            .collect()
    }

    pub fn reveal_match(&self, start: (i64, usize), end: (i64, usize)) {
        let mut term = self.term.lock();
        let history = term.history_size() as i64;
        let point = |(line, column): (i64, usize)| {
            Point::new(Line((line - history) as i32), Column(column))
        };
        term.scroll_to_point(point(start));
        let mut selection = Selection::new(SelectionType::Simple, point(start), Side::Left);
        selection.update(point(end), Side::Right);
        term.selection = Some(selection);
    }

    pub fn find_position(&self, pattern: &str) -> Option<(usize, usize)> {
        let term = self.term.lock();
        let start = Point::new(Line(-(term.history_size() as i32)), Column(0));
        let end = Point::new(
            Line(term.screen_lines() as i32 - 1),
            Column(term.columns() - 1),
        );
        let all = matches_between(&term, pattern, start, end);
        let selected = term
            .selection
            .as_ref()
            .and_then(|selection| selection.to_range(&term))
            .map(|range| range.start);
        let current = all
            .iter()
            .position(|found| Some(*found.start()) == selected)?;
        Some((current + 1, all.len()))
    }

    // PTYSession.m -tryAutoProfileSwitchWithHostname:…: where the session is now; without shell
    // integration, this machine and its user.
    pub fn place(&self) -> Place {
        let (username, hostname) = self.extras().remote_host.clone().unwrap_or_else(|| {
            (
                std::env::var("USER").unwrap_or_default(),
                status::host_name().unwrap_or_default(),
            )
        });
        let group = rustix::termios::tcgetpgrp(self.pty.borrow().file()).ok();
        let read = |file: &str| {
            group.and_then(|group| {
                std::fs::read(format!("/proc/{}/{file}", group.as_raw_nonzero())).ok()
            })
        };
        let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).trim().replace('\0', " ");
        Place {
            hostname,
            username,
            path: self
                .working_directory()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            job: read("comm").map(text).unwrap_or_default(),
            command_line: read("cmdline").map(text).unwrap_or_default(),
        }
    }

    // At most twice a second, since it reads /proc; a check asked for sooner waits for the rest.
    pub fn place_check_wait(&self) -> Option<Duration> {
        PLACE_CHECK_INTERVAL.checked_sub(self.place_checked.get().elapsed())
    }

    pub fn switch_profile_for_place(&self, profiles: &[Profile]) -> Option<String> {
        self.place_checked.set(Instant::now());
        self.place_check_queued.set(false);
        let place = self.place();
        let current = self.profile();
        self.switcher.borrow_mut().update(profiles, &current, place)
    }

    pub fn foreground_name(&self) -> Option<String> {
        let group = rustix::termios::tcgetpgrp(self.pty.borrow().file()).ok()?;
        let name =
            std::fs::read_to_string(format!("/proc/{}/comm", group.as_raw_nonzero())).ok()?;
        Some(name.trim().to_string())
    }

    pub fn has_jobs(&self) -> bool {
        let foreground = rustix::termios::tcgetpgrp(self.pty.borrow().file());
        foreground.is_ok_and(|group| {
            self.runs_command || group.as_raw_nonzero().get() as u32 != self.shell_pid
        })
    }

    pub fn show_ended(&self, notice: &str) {
        self.ended.set(true);
        self.feed(format!("\r\n{notice}\r\n").as_bytes());
    }

    pub fn reset_character_set(&self) {
        self.feed(b"\x1b(B\x1b)B\x0f");
    }

    pub fn has_state(&self, state: TerminalState) -> bool {
        self.mode().intersects(state.mode())
    }

    pub fn toggle_state(&self, state: TerminalState) {
        self.feed(state.sequence(!self.has_state(state)));
    }

    fn feed(&self, bytes: &[u8]) {
        let mut processor: Processor = Processor::new();
        processor.advance(&mut *self.term.lock(), bytes);
    }

    pub fn has_scrollback(&self) -> bool {
        self.term.lock().history_size() > 0
    }

    pub fn display_offset(&self) -> usize {
        self.term.lock().grid().display_offset()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop_coprocess();
    }
}

pub fn visible_found(term: &Term<Listener>, pattern: &str) -> Vec<hints::Found> {
    let Ok(mut regex) = RegexSearch::new(pattern) else {
        return Vec::new();
    };
    let offset = term.grid().display_offset() as i32;
    let start = Point::new(Line(-offset), Column(0));
    let end = Point::new(
        Line(term.screen_lines() as i32 - 1 - offset),
        Column(term.columns() - 1),
    );
    let history = term.history_size() as i64;
    let absolute = |point: &Point| (history + i64::from(point.line.0), point.column.0);
    matches_with(term, &mut regex, start, end)
        .into_iter()
        .map(|found| {
            let text = term.bounds_to_string(*found.start(), *found.end());
            (absolute(found.start()), absolute(found.end()), text)
        })
        .collect()
}

pub fn visible_matches(term: &Term<Listener>, pattern: &str) -> Vec<Match> {
    let offset = term.grid().display_offset() as i32;
    let start = Point::new(Line(-offset), Column(0));
    let end = Point::new(
        Line(term.screen_lines() as i32 - 1 - offset),
        Column(term.columns() - 1),
    );
    matches_between(term, pattern, start, end)
}

#[derive(Clone, PartialEq)]
pub struct Snippet {
    pub start: (i64, usize),
    pub end: (i64, usize),
    pub parts: [String; 3],
}

// iTermTextExtractor.m -attributedStringForSnippetForRange:...: the match with some of its logical
// line before and after it.
fn snippet(term: &Term<Listener>, found: &Match) -> [String; 3] {
    let line_start = term.line_search_left(*found.start());
    let line_end = term.line_search_right(*found.end());
    let before = if *found.start() > line_start {
        let text = term.bounds_to_string(line_start, found.start().sub(term, Boundary::None, 1));
        let skip = text
            .chars()
            .count()
            .saturating_sub(GLOBAL_SEARCH.prefix_chars);
        text.chars()
            .skip(skip)
            .collect::<String>()
            .trim_start()
            .to_string()
    } else {
        String::new()
    };
    let mut after = if *found.end() < line_end {
        let text = term.bounds_to_string(found.end().add(term, Boundary::None, 1), line_end);
        text.trim_end().to_string()
    } else {
        String::new()
    };
    if after.chars().count() > GLOBAL_SEARCH.suffix_chars {
        after = after.chars().take(GLOBAL_SEARCH.suffix_chars).collect();
        after.push('…');
    }
    let matched = term
        .bounds_to_string(*found.start(), *found.end())
        .replace('\n', " ");
    [before, matched, after]
}

fn matches_between(term: &Term<Listener>, pattern: &str, start: Point, end: Point) -> Vec<Match> {
    with_search(pattern, |regex| matches_with(term, regex, start, end)).unwrap_or_default()
}

fn matches_with<T: EventListener>(
    term: &Term<T>,
    regex: &mut RegexSearch,
    start: Point,
    end: Point,
) -> Vec<Match> {
    let mut found = Vec::new();
    let mut origin = start;
    while let Some(next) = term.regex_search_right(regex, origin, end) {
        let last = *next.end() >= end;
        origin = next.end().add(term, Boundary::None, 1);
        found.push(next);
        if last {
            break;
        }
    }
    found
}

fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    if bracketed {
        let clean = text.replace('\x1b', "");
        format!("\x1b[200~{clean}\x1b[201~").into_bytes()
    } else {
        text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalState {
    AlternateScreen,
    FocusReporting,
    MouseReporting,
    BracketedPaste,
    ApplicationCursor,
    ApplicationKeypad,
}

impl TerminalState {
    pub const ALL: [Self; 6] = [
        Self::AlternateScreen,
        Self::FocusReporting,
        Self::MouseReporting,
        Self::BracketedPaste,
        Self::ApplicationCursor,
        Self::ApplicationKeypad,
    ];

    fn mode(self) -> TermMode {
        match self {
            Self::AlternateScreen => TermMode::ALT_SCREEN,
            Self::FocusReporting => TermMode::FOCUS_IN_OUT,
            Self::MouseReporting => TermMode::MOUSE_MODE,
            Self::BracketedPaste => TermMode::BRACKETED_PASTE,
            Self::ApplicationCursor => TermMode::APP_CURSOR,
            Self::ApplicationKeypad => TermMode::APP_KEYPAD,
        }
    }

    fn sequence(self, on: bool) -> &'static [u8] {
        match (self, on) {
            (Self::AlternateScreen, true) => b"\x1b[?1049h",
            (Self::AlternateScreen, false) => b"\x1b[?1049l",
            (Self::FocusReporting, true) => b"\x1b[?1004h",
            (Self::FocusReporting, false) => b"\x1b[?1004l",
            (Self::MouseReporting, true) => b"\x1b[?1000h",
            (Self::MouseReporting, false) => b"\x1b[?1000l\x1b[?1002l\x1b[?1003l",
            (Self::BracketedPaste, true) => b"\x1b[?2004h",
            (Self::BracketedPaste, false) => b"\x1b[?2004l",
            (Self::ApplicationCursor, true) => b"\x1b[?1h",
            (Self::ApplicationCursor, false) => b"\x1b[?1l",
            (Self::ApplicationKeypad, true) => b"\x1b=",
            (Self::ApplicationKeypad, false) => b"\x1b>",
        }
    }
}

pub fn shell_escaped(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if " \t!\"#$&'()*,;<>?[\\]^`{|}~".contains(c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

fn toggle_vi_selection(term: &mut Term<Listener>, kind: SelectionType) {
    let same = term
        .selection
        .as_ref()
        .is_some_and(|selection| selection.ty == kind && !selection.is_empty());
    term.selection = (!same).then(|| {
        let mut selection = Selection::new(kind, term.vi_mode_cursor.point, Side::Left);
        selection.include_all();
        selection
    });
}

// iTermTextExtractor.m -smartSelectionAt:…: the profile's rules over the clicked logical line,
// and word selection when none matches.
fn smart_span(
    term: &Term<Listener>,
    point: Point,
    rules: &[(Regex, Precision)],
) -> Option<(Point, Point)> {
    let history = term.history_size() as i64;
    let row = history + i64::from(point.line.0);
    let (text, cells) = pty_io::logical_text(term, pty_io::logical_line_at(term, row));
    let target = cells.iter().position(|cell| *cell == (row, point.column.0));
    let to_point =
        |(row, column): (i64, usize)| Point::new(Line((row - history) as i32), Column(column));
    if let Some(span) = target.and_then(|target| smart_selection::best_match(&text, target, rules))
    {
        return Some((to_point(cells[span.start]), to_point(cells[span.end - 1])));
    }
    let (text, columns) = pty_io::trimmed_text_and_byte_columns(term, point.line);
    let span = links::word_span(&text, &columns, point.column.0)?;
    Some((
        Point::new(point.line, Column(columns[span.start])),
        Point::new(point.line, Column(columns[span.end - 1])),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_special_characters_get_a_backslash_and_letters_do_not() {
        assert_eq!(
            shell_escaped("mi archivo (1).txt"),
            r"mi\ archivo\ \(1\).txt"
        );
        assert_eq!(shell_escaped("a$b"), r"a\$b");
        assert_eq!(shell_escaped("ñandú"), "ñandú");
    }

    #[test]
    fn a_bracketed_paste_is_wrapped_and_cannot_close_the_bracket_early() {
        assert_eq!(
            paste_bytes("ls\x1b[201~rm", true),
            b"\x1b[200~ls[201~rm\x1b[201~".to_vec()
        );
    }

    #[test]
    fn a_pattern_that_matches_nothing_still_ends_the_search() {
        use alacritty_terminal::event::VoidListener;
        let mut term = Term::new(Config::default(), &TermSize::new(10, 3), VoidListener);
        Processor::<alacritty_terminal::vte::ansi::StdSyncHandler>::new()
            .advance(&mut term, b"aaa\r\nbbb");
        let (start, end) = (
            Point::new(Line(0), Column(0)),
            Point::new(Line(2), Column(9)),
        );
        for pattern in ["(?i)a*", "(?i)^", "(?i)x?"] {
            let mut regex = RegexSearch::new(pattern).expect("pattern");
            assert!(matches_with(&term, &mut regex, start, end).len() <= 30);
        }
    }

    #[test]
    fn a_plain_paste_turns_newlines_into_carriage_returns() {
        assert_eq!(paste_bytes("a\nb\r\nc", false), b"a\rb\rc".to_vec());
    }
}
