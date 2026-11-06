//! PTY reading and writing threads (ADR 0005).

use std::borrow::Cow;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Read, Write};
use std::ops::{Range, RangeInclusive};
use std::path::PathBuf;
use std::process::ChildStdin;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Term;
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::tty::{EventedReadWrite, Pty};
use alacritty_terminal::vte::ansi::{Handler, Processor};
use freya_engine::prelude::{Data, Image};
use regex::Regex;
use rustix::event::{PollFd, PollFlags, Timespec, poll};

use crate::config::{Trigger, TriggerAction};
use crate::integration::{self, Dimension, InlineImage, Report};
use crate::osc::{Piece, Splitter};
use crate::session::Listener;

const READ_BUFFER: usize = 1 << 16;

pub struct Placed {
    pub line: i64,
    pub column: usize,
    pub width: f32,
    pub height: f32,
    pub image: Image,
}

pub struct Highlight {
    pub line: i64,
    pub columns: RangeInclusive<usize>,
}

struct Compiled {
    regex: Regex,
    action: TriggerAction,
    text: String,
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
    pub marks: Vec<i64>,
    pub output: Option<[i64; 2]>,
    output_start: Option<i64>,
    pub images: Vec<Placed>,
    pub highlights: Vec<Highlight>,
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
    triggers: Vec<Trigger>,
    compiled: Vec<Compiled>,
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
        self.compiled = triggers
            .iter()
            .filter_map(|trigger| {
                Some(Compiled {
                    regex: Regex::new(&trigger.regex).ok()?,
                    action: trigger.action,
                    text: trigger.parameter.replace("\\n", "\r"),
                })
            })
            .collect();
    }

    pub fn forget_everything(&mut self) {
        self.marks.clear();
        self.output = None;
        self.output_start = None;
        self.stamps.clear();
        self.images.clear();
        self.highlights.clear();
        self.annotations.clear();
    }

    fn anchored_lines(&mut self) -> impl Iterator<Item = &mut i64> {
        let images = self.images.iter_mut().map(|image| &mut image.line);
        let highlights = self
            .highlights
            .iter_mut()
            .map(|highlight| &mut highlight.line);
        let output = self.output.iter_mut().flatten();
        let annotations = self
            .annotations
            .iter_mut()
            .flat_map(|annotation| [&mut annotation.start.0, &mut annotation.end.0]);
        self.marks
            .iter_mut()
            .chain(output)
            .chain(self.output_start.iter_mut())
            .chain(images)
            .chain(highlights)
            .chain(annotations)
            .chain(self.stamps.iter_mut().map(|(line, _)| line))
    }

    pub fn forget_lines(&mut self, lines: Range<i64>) {
        let kept = |line: i64| !lines.contains(&line);
        self.marks.retain(|mark| kept(*mark));
        self.output = self.output.filter(|[start, _]| kept(*start));
        self.output_start = self.output_start.filter(|start| kept(*start));
        self.stamps.retain(|(stamped, _)| kept(*stamped));
        self.images.retain(|image| kept(image.line));
        self.highlights.retain(|highlight| kept(highlight.line));
        self.annotations
            .retain(|annotation| kept(annotation.start.0) && kept(annotation.end.0));
        let removed = lines.end - lines.start;
        self.anchored_lines()
            .filter(|anchored| **anchored >= lines.end)
            .for_each(|anchored| *anchored -= removed);
    }
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
    let mut buffer = vec![0; READ_BUFFER];
    let mut scanned = 0;
    loop {
        let deadline = processor.sync_timeout().sync_timeout();
        if !wait(&reader, PollFlags::IN, deadline) {
            if deadline.is_some_and(|deadline| deadline <= Instant::now()) {
                processor.stop_sync(&mut *term.lock());
                listener.send_event(Event::Wakeup);
            }
            continue;
        }
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
        {
            let mut term = term.lock();
            for piece in splitter.feed(&buffer[..read]) {
                match piece {
                    Piece::Bytes(bytes) => processor.advance(&mut *term, &bytes),
                    Piece::Osc(payload) => {
                        if let Some(report) = integration::parse(&payload) {
                            apply(&mut term, &extras, report);
                        }
                    }
                }
            }
            run_triggers(&term, &extras, &mut scanned, &listener);
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

fn run_triggers(
    term: &Term<Listener>,
    extras: &Mutex<Extras>,
    scanned: &mut i64,
    listener: &Listener,
) {
    let cursor = absolute_line(term);
    let from = (*scanned).clamp(0, cursor);
    *scanned = cursor;
    let Ok(mut extras) = extras.lock() else {
        return;
    };
    if !extras.triggers_enabled || extras.compiled.is_empty() {
        return;
    }
    let history = term.history_size() as i64;
    let Extras {
        compiled,
        highlights,
        ..
    } = &mut *extras;
    for row in from..cursor {
        let (text, columns) = trimmed_text_and_byte_columns(term, Line((row - history) as i32));
        for trigger in compiled.iter() {
            for found in trigger
                .regex
                .find_iter(&text)
                .filter(|found| !found.is_empty())
            {
                match trigger.action {
                    TriggerAction::Highlight => highlights.push(Highlight {
                        line: row,
                        columns: columns[found.start()]..=columns[found.end() - 1],
                    }),
                    TriggerAction::Bell => listener.send_event(Event::Bell),
                    TriggerAction::SendText => {
                        listener.send_event(Event::PtyWrite(trigger.text.clone()))
                    }
                }
            }
        }
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
        Report::Mark => extras.add_mark(absolute_line(term)),
        Report::OutputStart => extras.output_start = Some(absolute_line(term)),
        Report::CommandEnd => {
            extras.end_output(absolute_line(term), term.grid().cursor.point.column.0)
        }
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
