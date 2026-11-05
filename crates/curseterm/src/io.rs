//! PTY reading and writing threads (ADR 0005).

use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Term;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::tty::{EventedReadWrite, Pty};
use alacritty_terminal::vte::ansi::{Handler, Processor};
use freya_engine::prelude::{Data, Image};
use rustix::event::{PollFd, PollFlags, Timespec, poll};

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

#[derive(Default)]
pub struct Extras {
    pub directory: Option<PathBuf>,
    pub marks: Vec<i64>,
    pub images: Vec<Placed>,
    pub cell: (f32, f32),
}

impl Extras {
    pub fn add_mark(&mut self, line: i64) {
        if let Err(at) = self.marks.binary_search(&line) {
            self.marks.insert(at, line);
        }
    }

    fn anchored_lines(&mut self) -> impl Iterator<Item = &mut i64> {
        let images = self.images.iter_mut().map(|image| &mut image.line);
        self.marks.iter_mut().chain(images)
    }

    pub fn forget_lines_above(&mut self, line: i64) {
        self.marks.retain(|mark| *mark >= line);
        self.images.retain(|image| image.line >= line);
        self.anchored_lines().for_each(|anchored| *anchored -= line);
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

/// Keeps marks and images on their text when a resize rewraps the lines above them.
pub fn reflow(
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
        }
        listener.send_event(Event::Wakeup);
    }
    listener.send_event(Event::Exit);
}

fn apply(term: &mut Term<Listener>, extras: &Mutex<Extras>, report: Report) {
    let Ok(mut extras) = extras.lock() else {
        return;
    };
    match report {
        Report::Directory(path) => extras.directory = Some(path),
        Report::Mark => extras.add_mark(absolute_line(term)),
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
