//! PTY reading and writing threads (ADR 0005).

use std::borrow::Cow;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Read, Write};
use std::ops::Range;
use std::path::PathBuf;
use std::process::ChildStdin;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::{Cell, Flags, Hyperlink};
use alacritty_terminal::term::{Term, TermMode};
use alacritty_terminal::tty::{EventedReadWrite, Pty};
use alacritty_terminal::vte::ansi::{Color, Handler, Processor};
use freya_engine::prelude::{Data, Image};
use regex::Regex;
use rustix::event::{PollFd, PollFlags, Timespec, poll};

use crate::config::{Trigger, TriggerAction};
use crate::integration::{self, Dimension, InlineImage, Notification, Report};
use crate::key_report::{KeyMode, ModifyOtherKeysWatch};
use crate::osc::{Piece, Splitter};
use crate::session::Listener;
use crate::smart_selection::{self, Precision, Rule};
use crate::triggers::{self, Compiled, PARTIAL_LINE_INTERVAL};

const READ_BUFFER: usize = 1 << 16;

pub struct Placed {
    pub line: i64,
    pub column: usize,
    pub width: f32,
    pub height: f32,
    pub image: Image,
}

// The trigger effects that need the window: PTYSession.m -triggerSideEffect….
pub enum Fired {
    Alert { trigger: usize, message: String },
    Notification(String),
    Coprocess(String),
}

pub type AnnotationId = u64;

#[derive(Clone, PartialEq)]
pub struct Annotation {
    pub id: AnnotationId,
    pub start: (i64, usize),
    pub end: (i64, usize),
    pub text: String,
    pub hidden: bool,
    pub toggled: Option<Instant>,
    pub highlighted: Option<Instant>,
    pub focus: bool,
}

const REPLAY_FRAMES: usize = 120;
const REPLAY_INTERVAL: Duration = Duration::from_millis(500);

pub struct Snapshot {
    pub at: SystemTime,
    pub rows: Vec<Vec<Cell>>,
}

#[derive(Default)]
pub struct Extras {
    pub directory: Option<PathBuf>,
    pub remote_host: Option<(String, String)>,
    pub marks: Vec<i64>,
    pub named_marks: Vec<(i64, String)>,
    pub output: Option<[i64; 2]>,
    output_start: Option<i64>,
    pub images: Vec<Placed>,
    pub annotations: Vec<Annotation>,
    next_annotation: AnnotationId,
    pub current_annotation: Option<AnnotationId>,
    pub cell: (f32, f32),
    pub bell: Option<Instant>,
    pub stamps: VecDeque<(i64, SystemTime)>,
    pub show_timestamps: bool,
    pub cursor_guide: bool,
    pub log: Option<File>,
    pub replay: VecDeque<Snapshot>,
    pub coprocess: Option<ChildStdin>,
    pub cursor_spotlight: Option<Instant>,
    pub triggers_enabled: bool,
    pub triggers_in_interactive_apps: bool,
    pub mark_reported: bool,
    pub last_status: Option<i32>,
    pub command_start: Option<(i64, usize)>,
    pub command: Option<((i64, usize), (i64, usize))>,
    pub alert_on_next_mark: bool,
    pub notifications: Vec<Notification>,
    pub sub_selections: Vec<((i64, usize), (i64, usize))>,
    pub key_mode: KeyMode,
    pub entered: Vec<String>,
    pub apps_change_key_mode: bool,
    triggers: Vec<Trigger>,
    compiled: Vec<Compiled>,
    pub fired: Vec<Fired>,
    last_partial_check: Option<Instant>,
    smart_rules: Vec<Rule>,
    pub smart_selection: Vec<(Regex, Precision)>,
}

impl Extras {
    pub fn new(cell: (f32, f32)) -> Self {
        Self {
            cell,
            triggers_enabled: true,
            ..Self::default()
        }
    }

    pub fn add_mark(&mut self, line: i64) {
        if let Err(at) = self.marks.binary_search(&line) {
            self.marks.insert(at, line);
        }
    }

    pub fn annotate(&mut self, start: (i64, usize), end: (i64, usize)) {
        self.next_annotation += 1;
        self.annotations.push(Annotation {
            id: self.next_annotation,
            start,
            end,
            text: String::new(),
            hidden: false,
            toggled: None,
            highlighted: None,
            focus: true,
        });
    }

    pub fn annotation(&mut self, id: AnnotationId) -> Option<&mut Annotation> {
        self.annotations
            .iter_mut()
            .find(|annotation| annotation.id == id)
    }

    fn end_output(&mut self, line: i64, column: usize) {
        let Some(start) = self.output_start.take() else {
            return;
        };
        let end = if column == 0 { line - 1 } else { line };
        self.output = (end >= start).then_some([start, end]);
    }

    pub fn stamp(&mut self, line: i64, at: SystemTime, keep: usize) {
        match self
            .stamps
            .binary_search_by_key(&line, |(stamped, _)| *stamped)
        {
            Ok(index) => self.stamps[index].1 = at,
            Err(index) => self.stamps.insert(index, (line, at)),
        }
        while self.stamps.len() > keep {
            self.stamps.pop_front();
        }
    }

    fn record(&mut self, term: &Term<Listener>) {
        let due = self
            .replay
            .back()
            .is_none_or(|last| last.at.elapsed().unwrap_or_default() >= REPLAY_INTERVAL);
        if !due {
            return;
        }
        let rows = (0..term.screen_lines() as i32)
            .map(|line| term.grid()[Line(line)].into_iter().cloned().collect())
            .collect();
        self.replay.push_back(Snapshot {
            at: SystemTime::now(),
            rows,
        });
        if self.replay.len() > REPLAY_FRAMES {
            self.replay.pop_front();
        }
    }

    pub fn stamp_of(&self, line: i64) -> Option<SystemTime> {
        let index = self.stamps.partition_point(|(stamped, _)| *stamped < line);
        self.stamps.get(index).map(|(_, at)| *at)
    }

    pub fn set_triggers(&mut self, triggers: &[Trigger]) {
        if self.triggers == triggers {
            return;
        }
        self.triggers = triggers.to_vec();
        self.compiled = triggers::compile(triggers);
    }

    pub fn set_smart_selection_rules(&mut self, rules: &[Rule]) {
        if self.smart_rules != rules {
            self.smart_rules = rules.to_vec();
            self.smart_selection = smart_selection::compile(rules);
        }
    }

    // AlertTrigger.m: Disable This Alert turns the trigger off for this session only.
    pub fn disable_trigger(&mut self, index: usize) {
        for trigger in &mut self.compiled {
            if trigger.index == index {
                trigger.disabled = true;
            }
        }
    }

    fn has_triggers(&self) -> bool {
        self.triggers_enabled && !self.compiled.is_empty()
    }

    // PTYSession.m -triggerEvaluatorShouldUseTriggers: maybe not in the alternate screen.
    fn watching_lines(&self, term: &Term<Listener>) -> bool {
        self.has_triggers()
            && (self.triggers_in_interactive_apps || !term.mode().contains(TermMode::ALT_SCREEN))
    }

    pub fn forget_everything(&mut self) {
        self.marks.clear();
        self.named_marks.clear();
        self.output = None;
        self.output_start = None;
        self.stamps.clear();
        self.images.clear();
        self.annotations.clear();
        self.sub_selections.clear();
    }

    fn anchored_lines(&mut self) -> impl Iterator<Item = &mut i64> {
        let images = self.images.iter_mut().map(|image| &mut image.line);
        let output = self.output.iter_mut().flatten();
        let annotations = self
            .annotations
            .iter_mut()
            .flat_map(|annotation| [&mut annotation.start.0, &mut annotation.end.0]);
        self.marks
            .iter_mut()
            .chain(self.named_marks.iter_mut().map(|(line, _)| line))
            .chain(output)
            .chain(self.output_start.iter_mut())
            .chain(images)
            .chain(annotations)
            .chain(self.stamps.iter_mut().map(|(line, _)| line))
    }

    // A full scrollback dropped its oldest line: what was anchored below it moves up one.
    fn line_scrolled_off(&mut self) {
        let selections = std::mem::take(&mut self.sub_selections);
        self.forget_lines(0..1);
        self.sub_selections = selections
            .into_iter()
            .filter(|(start, _)| start.0 > 0)
            .map(|(start, end)| ((start.0 - 1, start.1), (end.0 - 1, end.1)))
            .collect();
    }

    pub fn forget_lines(&mut self, lines: Range<i64>) {
        let kept = |line: i64| !lines.contains(&line);
        self.marks.retain(|mark| kept(*mark));
        self.named_marks.retain(|(line, _)| kept(*line));
        self.output = self.output.filter(|[start, _]| kept(*start));
        self.output_start = self.output_start.filter(|start| kept(*start));
        self.stamps.retain(|(stamped, _)| kept(*stamped));
        self.images.retain(|image| kept(image.line));
        self.annotations
            .retain(|annotation| kept(annotation.start.0) && kept(annotation.end.0));
        self.sub_selections.clear();
        let removed = lines.end - lines.start;
        self.anchored_lines()
            .filter(|anchored| **anchored >= lines.end)
            .for_each(|anchored| *anchored -= removed);
    }
}

fn cursor_point(term: &Term<Listener>) -> (i64, usize) {
    (absolute_line(term), term.grid().cursor.point.column.0)
}

pub fn absolute_line(term: &Term<Listener>) -> i64 {
    term.history_size() as i64 + i64::from(term.grid().cursor.point.line.0)
}

fn logical_lines(term: &Term<Listener>) -> Vec<i64> {
    let history = term.history_size() as i32;
    let last = Column(term.columns() - 1);
    let mut logical = 0;
    (0..term.total_lines() as i32)
        .map(|row| {
            let current = logical;
            if !term.grid()[Line(row - history)][last]
                .flags
                .contains(Flags::WRAPLINE)
            {
                logical += 1;
            }
            current
        })
        .collect()
}

pub fn resize_keeping_anchors(
    term: &mut Term<Listener>,
    extras: &mut Extras,
    resize: impl FnOnce(&mut Term<Listener>),
) {
    let before = logical_lines(term);
    let logical: Vec<i64> = extras
        .anchored_lines()
        .map(|line| {
            usize::try_from(*line)
                .ok()
                .and_then(|row| before.get(row))
                .copied()
                .unwrap_or(*line)
        })
        .collect();
    resize(term);
    let after = logical_lines(term);
    for (line, logical) in extras.anchored_lines().zip(logical) {
        *line = after.partition_point(|start| *start < logical) as i64;
    }
}

pub fn start(
    pty: &mut Pty,
    term: Arc<FairMutex<Term<Listener>>>,
    listener: Listener,
    extras: Arc<Mutex<Extras>>,
) -> io::Result<Sender<Cow<'static, [u8]>>> {
    let reader = pty.reader().try_clone()?;
    let mut writer = pty.writer().try_clone()?;
    let (sender, inputs) = channel::<Cow<'static, [u8]>>();
    std::thread::Builder::new()
        .name("curseterm-pty-write".into())
        .spawn(move || {
            for bytes in inputs {
                if let Err(error) = write_all(&mut writer, &bytes) {
                    tracing::warn!(%error, "could not write to the shell");
                    break;
                }
            }
        })?;
    std::thread::Builder::new()
        .name("curseterm-pty-read".into())
        .spawn(move || read_loop(reader, term, listener, extras))?;
    Ok(sender)
}

fn wait(file: &File, flags: PollFlags, deadline: Option<Instant>) -> bool {
    let timeout = deadline.map(|deadline| {
        let left = deadline.saturating_duration_since(Instant::now());
        Timespec {
            tv_sec: left.as_secs() as i64,
            tv_nsec: i64::from(left.subsec_nanos()),
        }
    });
    let mut fds = [PollFd::new(file, flags)];
    matches!(poll(&mut fds, timeout.as_ref()), Ok(ready) if ready > 0)
}

fn write_all(file: &mut File, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        match file.write(bytes) {
            Ok(written) => bytes = &bytes[written..],
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                wait(file, PollFlags::OUT, None);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn read_loop(
    mut reader: File,
    term: Arc<FairMutex<Term<Listener>>>,
    listener: Listener,
    extras: Arc<Mutex<Extras>>,
) {
    let mut processor: Processor = Processor::new();
    let mut splitter = Splitter::default();
    let mut modify_other_keys = ModifyOtherKeysWatch::default();
    let mut buffer = vec![0; READ_BUFFER];
    let mut line_has_text = false;
    let mut idle_line_checked = false;
    loop {
        let sync_deadline = processor.sync_timeout().sync_timeout();
        // VT100Screen.m -performPeriodicTriggerCheck: a prompt left waiting still gets its
        // instant triggers once the output pauses.
        let idle_check =
            (line_has_text && !idle_line_checked).then(|| Instant::now() + PARTIAL_LINE_INTERVAL);
        let deadline = match (sync_deadline, idle_check) {
            (Some(sync), Some(idle)) => Some(sync.min(idle)),
            (sync, idle) => sync.or(idle),
        };
        if !wait(&reader, PollFlags::IN, deadline) {
            if sync_deadline.is_some_and(|deadline| deadline <= Instant::now()) {
                processor.stop_sync(&mut *term.lock());
                listener.send_event(Event::Wakeup);
            }
            if idle_check.is_some_and(|deadline| deadline <= Instant::now()) {
                idle_line_checked = true;
                check_partial_line(&mut term.lock(), &extras, &listener);
                listener.send_event(Event::Wakeup);
            }
            continue;
        }
        idle_line_checked = false;
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                continue;
            }
            Err(_) => break,
        };
        tee_output(&extras, &buffer[..read]);
        if let Some(level) = modify_other_keys.feed(&buffer[..read])
            && let Ok(mut extras) = extras.lock()
            && extras.apps_change_key_mode
        {
            extras.key_mode = KeyMode::from_modify_other_keys(level);
        }
        {
            let mut term = term.lock();
            for piece in splitter.feed(&buffer[..read]) {
                match piece {
                    Piece::Bytes(bytes) => feed(
                        &mut processor,
                        &mut term,
                        &bytes,
                        &extras,
                        &listener,
                        &mut line_has_text,
                    ),
                    Piece::Osc(payload) => {
                        if let Some(report) = integration::parse(&payload) {
                            apply(&mut term, &extras, report);
                        }
                    }
                }
            }
            if line_has_text {
                check_partial_line(&mut term, &extras, &listener);
            }
            if let Ok(mut extras) = extras.lock() {
                extras.stamp(absolute_line(&term), SystemTime::now(), term.total_lines());
                extras.record(&term);
            }
        }
        listener.send_event(Event::Wakeup);
    }
    listener.send_event(Event::Exit);
}

fn tee_output(extras: &Mutex<Extras>, bytes: &[u8]) {
    let Ok(mut extras) = extras.lock() else {
        return;
    };
    if let Some(log) = extras.log.as_mut()
        && let Err(error) = log.write_all(bytes)
    {
        tracing::warn!(%error, "could not write the session log; logging stops");
        extras.log = None;
    }
    if let Some(coprocess) = extras.coprocess.as_mut()
        && coprocess.write_all(bytes).is_err()
    {
        extras.coprocess = None;
    }
}

fn is_line_feed(byte: u8) -> bool {
    matches!(byte, b'\n' | 0x0b | 0x0c)
}

// VT100Screen.m -linefeed → PTYTriggerEvaluator -clearTriggerLine: a line's triggers run just
// before the line feed that ends it, so nothing depends on where the line ends up.
fn feed(
    processor: &mut Processor,
    term: &mut Term<Listener>,
    bytes: &[u8],
    extras: &Mutex<Extras>,
    listener: &Listener,
    line_has_text: &mut bool,
) {
    let has_triggers = extras.lock().is_ok_and(|extras| extras.has_triggers());
    for piece in bytes.split_inclusive(|byte| is_line_feed(*byte)) {
        let (body, line_feed) = match piece.split_last() {
            Some((last, body)) if is_line_feed(*last) => (body, Some(*last)),
            _ => (piece, None),
        };
        processor.advance(term, body);
        if has_triggers {
            *line_has_text |= body.iter().any(|byte| *byte >= b' ' && *byte != 0x7f);
        }
        let Some(line_feed) = line_feed else {
            continue;
        };
        // The line's own bytes may have just switched to the alternate screen.
        if std::mem::take(line_has_text)
            && let Ok(mut extras) = extras.lock()
            && extras.watching_lines(term)
        {
            check_line(term, &mut extras, listener, false);
        }
        line_feed_counting_lost_lines(processor, term, line_feed, extras);
    }
}

// alacritty_terminal does not count the lines a full scrollback drops (VT100Screen.m keeps
// totalScrollbackOverflow); seeing the top row land in the history tells it happened.
fn line_feed_counting_lost_lines(
    processor: &mut Processor,
    term: &mut Term<Listener>,
    line_feed: u8,
    extras: &Mutex<Extras>,
) {
    let history = term.history_size();
    let at_bottom =
        history > 0 && term.grid().cursor.point.line.0 == term.screen_lines() as i32 - 1;
    let top = at_bottom.then(|| term.grid()[Line(0)].clone());
    processor.advance(term, &[line_feed]);
    if let Some(top) = top
        && term.history_size() == history
        && term.grid()[Line(-1)] == top
        && let Ok(mut extras) = extras.lock()
    {
        extras.line_scrolled_off();
    }
}

// PTYTriggerEvaluator.m -checkPartialLineTriggers, at most twice a second.
fn check_partial_line(term: &mut Term<Listener>, extras: &Mutex<Extras>, listener: &Listener) {
    let Ok(mut extras) = extras.lock() else {
        return;
    };
    let now = Instant::now();
    let due = extras
        .last_partial_check
        .is_none_or(|last| now.duration_since(last) >= PARTIAL_LINE_INTERVAL);
    if !extras.watching_lines(term)
        || !due
        || !extras.compiled.iter().any(|trigger| trigger.instant)
    {
        return;
    }
    extras.last_partial_check = Some(now);
    check_line(term, &mut extras, listener, true);
}

fn check_line(term: &mut Term<Listener>, extras: &mut Extras, listener: &Listener, partial: bool) {
    let rows = cursor_logical_line(term);
    let (text, cells) = logical_text(term, rows.clone());
    if text.is_empty() {
        return;
    }
    let mut compiled = std::mem::take(&mut extras.compiled);
    'triggers: for trigger in &mut compiled {
        if !trigger.should_try(rows.start, partial) {
            continue;
        }
        let regex = trigger.regex.clone();
        for captures in regex.captures_iter(&text) {
            let Some(whole) = captures.get(0).filter(|whole| !whole.is_empty()) else {
                continue;
            };
            trigger.last_line = Some(rows.start);
            let groups: Vec<&str> = captures
                .iter()
                .map(|group| group.map_or("", |group| group.as_str()))
                .collect();
            let span = (cells[whole.start()], cells[whole.end() - 1]);
            if perform(trigger, term, extras, listener, &groups, span, rows.clone()) {
                break 'triggers;
            }
        }
    }
    extras.compiled = compiled;
}

fn cursor_logical_line(term: &Term<Listener>) -> Range<i64> {
    let history = term.history_size() as i64;
    let last = absolute_line(term);
    let wraps = |row: i64| {
        term.grid()[Line((row - history) as i32)][Column(term.columns() - 1)]
            .flags
            .contains(Flags::WRAPLINE)
    };
    let mut first = last;
    while first > 0 && wraps(first - 1) {
        first -= 1;
    }
    first..last + 1
}

// The wrapped rows around `row`, up to the screen's last row.
pub fn logical_line_at(term: &Term<Listener>, row: i64) -> Range<i64> {
    let history = term.history_size() as i64;
    let bottom = history + term.screen_lines() as i64 - 1;
    let wraps = |row: i64| {
        term.grid()[Line((row - history) as i32)][Column(term.columns() - 1)]
            .flags
            .contains(Flags::WRAPLINE)
    };
    let (mut first, mut last) = (row, row);
    while first > 0 && wraps(first - 1) {
        first -= 1;
    }
    while last < bottom && wraps(last) {
        last += 1;
    }
    first..last + 1
}

// iTermStringLine: the wrapped rows joined, with each byte's cell.
pub fn logical_text(term: &Term<Listener>, rows: Range<i64>) -> (String, Vec<(i64, usize)>) {
    let history = term.history_size() as i64;
    let mut text = String::new();
    let mut cells = Vec::new();
    for row in rows {
        for (column, cell) in term.grid()[Line((row - history) as i32)]
            .into_iter()
            .enumerate()
        {
            if cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }
            text.push(cell.c);
            cells.extend(std::iter::repeat_n((row, column), cell.c.len_utf8()));
        }
    }
    let length = text.trim_end().len();
    text.truncate(length);
    cells.truncate(length);
    (text, cells)
}

// Trigger.m -performActionWithCapturedStrings:…stop: each action; true stops this line's triggers.
fn perform(
    trigger: &mut Compiled,
    term: &mut Term<Listener>,
    extras: &mut Extras,
    listener: &Listener,
    groups: &[&str],
    (start, end): ((i64, usize), (i64, usize)),
    rows: Range<i64>,
) -> bool {
    let parameter = triggers::expand(&trigger.parameter, groups);
    let last_column = term.columns() - 1;
    let popup = matches!(
        trigger.action,
        TriggerAction::Alert | TriggerAction::Notification
    ) && trigger.popup_allowed(Instant::now());
    match trigger.action {
        // HighlightTrigger.m: the matched cells take the colours, so they go when the text does.
        TriggerAction::Highlight | TriggerAction::HighlightLine => {
            let (text, background) = triggers::colors(&trigger.parameter);
            let span = if trigger.action == TriggerAction::Highlight {
                (start, end)
            } else {
                ((rows.start, 0), (rows.end - 1, last_column))
            };
            for_each_cell(term, span, |cell| {
                if let Some(text) = text {
                    cell.fg = Color::Spec(text);
                }
                if let Some(background) = background {
                    cell.bg = Color::Spec(background);
                }
            });
        }
        TriggerAction::Bell => listener.send_event(Event::Bell),
        TriggerAction::SendText => listener.send_event(Event::PtyWrite(parameter)),
        TriggerAction::SetTitle => listener.send_event(Event::Title(parameter)),
        TriggerAction::Mark => {
            extras.add_mark(start.0);
            extras.mark_reported = true;
        }
        // iTermShellPromptTrigger.m: the match is the prompt and the command follows it.
        TriggerAction::PromptDetected => {
            extras.add_mark(start.0);
            extras.mark_reported = true;
            extras.command_start = Some((end.0, end.1 + 1));
        }
        TriggerAction::ReportDirectory => {
            let directory = if parameter.is_empty() {
                groups[0].to_string()
            } else {
                parameter
            };
            extras.directory = Some(PathBuf::from(directory));
        }
        TriggerAction::Annotate => {
            extras.annotate(start, end);
            if let Some(annotation) = extras.annotations.last_mut() {
                annotation.text = parameter;
                annotation.focus = false;
            }
        }
        TriggerAction::Hyperlink => {
            let link = Hyperlink::new(None::<String>, parameter);
            for_each_cell(term, (start, end), |cell| {
                cell.set_hyperlink(Some(link.clone()))
            });
        }
        TriggerAction::RunCommand => run_command(&parameter),
        TriggerAction::Coprocess => extras.fired.push(Fired::Coprocess(parameter)),
        TriggerAction::Alert if popup => {
            extras.fired.push(Fired::Alert {
                trigger: trigger.index,
                message: parameter,
            });
        }
        TriggerAction::Notification if popup => {
            extras.fired.push(Fired::Notification(parameter));
        }
        TriggerAction::Alert | TriggerAction::Notification => {}
        TriggerAction::Stop => return true,
    }
    false
}

fn for_each_cell(
    term: &mut Term<Listener>,
    (start, end): ((i64, usize), (i64, usize)),
    mut change: impl FnMut(&mut Cell),
) {
    let history = term.history_size() as i64;
    let last_column = term.columns() - 1;
    for line in start.0..=end.0 {
        let first = if line == start.0 { start.1 } else { 0 };
        let last = if line == end.0 { end.1 } else { last_column };
        for column in first..=last {
            change(&mut term.grid_mut()[Line((line - history) as i32)][Column(column)]);
        }
    }
}

// PTYSession.m -triggerSideEffectRunBackgroundCommand:pool:: the user's shell runs it unattended.
pub fn run_command(command: &str) {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    match std::process::Command::new(shell)
        .args(["-c", command])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
        }
        Err(error) => tracing::warn!(%error, "a Run Command trigger could not start"),
    }
}

pub fn trimmed_text_and_byte_columns(term: &Term<Listener>, line: Line) -> (String, Vec<usize>) {
    let mut text = String::new();
    let mut columns = Vec::new();
    for (column, cell) in term.grid()[line].into_iter().enumerate() {
        if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        text.push(cell.c);
        columns.extend(std::iter::repeat_n(column, cell.c.len_utf8()));
    }
    let length = text.trim_end().len();
    text.truncate(length);
    columns.truncate(length);
    (text, columns)
}

fn apply(term: &mut Term<Listener>, extras: &Mutex<Extras>, report: Report) {
    let Ok(mut extras) = extras.lock() else {
        return;
    };
    match report {
        Report::Directory(path) => extras.directory = Some(path),
        Report::RemoteHost { username, hostname } => {
            extras.remote_host = Some((username, hostname))
        }
        Report::Mark => {
            extras.add_mark(absolute_line(term));
            extras.mark_reported = true;
        }
        Report::CommandStart => extras.command_start = Some(cursor_point(term)),
        Report::OutputStart => {
            let line = absolute_line(term);
            if let Some(start) = extras.command_start.take() {
                let end = cursor_point(term);
                extras.command = Some((start, end));
                // iTermShellHistoryController.m: the command at OSC 133;C is the one entered.
                let history = term.history_size() as i64;
                let point = |(line, column): (i64, usize)| {
                    Point::new(Line((line - history) as i32), Column(column))
                };
                if end > start {
                    let text = term.bounds_to_string(point(start), point(end));
                    extras.entered.push(text.replace('\n', ""));
                }
            }
            extras.output_start = Some(line);
        }
        Report::CommandEnd(status) => {
            extras.last_status = status;
            extras.end_output(absolute_line(term), term.grid().cursor.point.column.0)
        }
        Report::Notification(notification) => extras.notifications.push(notification),
        Report::Image(image) => {
            if let Some(placed) = place(term, extras.cell, &image) {
                let lines = (placed.height / extras.cell.1).ceil() as usize;
                extras.images.push(placed);
                for _ in 0..lines {
                    term.linefeed();
                }
                term.carriage_return();
            }
        }
    }
}

fn place(
    term: &Term<Listener>,
    (cell_width, cell_height): (f32, f32),
    inline: &InlineImage,
) -> Option<Placed> {
    let image = Image::from_encoded(Data::new_copy(&inline.data))?;
    let natural = (image.width() as f32, image.height() as f32);
    let area = (
        term.columns() as f32 * cell_width,
        term.screen_lines() as f32 * cell_height,
    );
    let size = |dimension: Dimension, cell: f32, whole: f32| match dimension {
        Dimension::Auto => None,
        Dimension::Cells(cells) => Some(cells as f32 * cell),
        Dimension::Pixels(pixels) => Some(pixels as f32),
        Dimension::Percent(percent) => Some(whole * percent as f32 / 100.0),
    };
    let wanted = (
        size(inline.width, cell_width, area.0),
        size(inline.height, cell_height, area.1),
    );
    let ratio = natural.1 / natural.0.max(1.0);
    let (mut width, mut height) = match (wanted, inline.preserve_aspect_ratio) {
        ((Some(width), None), _) => (width, width * ratio),
        ((None, Some(height)), _) => (height / ratio.max(f32::EPSILON), height),
        ((Some(width), Some(height)), true) => {
            let scale = (width / natural.0).min(height / natural.1);
            (natural.0 * scale, natural.1 * scale)
        }
        ((Some(width), Some(height)), false) => (width, height),
        ((None, None), _) => natural,
    };
    if width > area.0 {
        height *= area.0 / width;
        width = area.0;
    }
    Some(Placed {
        line: absolute_line(term),
        column: term.grid().cursor.point.column.0,
        width,
        height,
        image,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_takes_the_time_of_the_next_stamp_and_old_stamps_fall_off() {
        let at = |seconds| SystemTime::UNIX_EPOCH + Duration::from_secs(seconds);
        let mut extras = Extras::default();
        extras.stamp(2, at(1), 3);
        extras.stamp(5, at(2), 3);
        extras.stamp(2, at(3), 3);
        assert_eq!(extras.stamp_of(0), Some(at(3)));
        assert_eq!(extras.stamp_of(4), Some(at(2)));
        assert_eq!(extras.stamp_of(6), None);
        extras.stamp(7, at(4), 3);
        extras.stamp(9, at(5), 3);
        assert_eq!(extras.stamp_of(2), Some(at(2)));
    }
}
