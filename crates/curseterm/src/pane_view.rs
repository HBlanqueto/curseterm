use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use barrier_kit_next::canvas::canvas;
use barrier_kit_next::tokens::{ANNOTATION, COMPOSER, SHORTCUT_LABEL, TERMINAL};
use freya_core::prelude::*;
use torin::prelude::Size;
use vow_design::SPACE_2;

use crate::composer::Composer;
use crate::composer_view::{ComposerEvent, ComposerView};
use crate::config::Scheme;
use crate::grid::{self, Fonts};
use crate::hint_view::{self, HintLabel};
use crate::hints::Hints;
use crate::indicators::{self, Indicator};
use crate::io::{Annotation, AnnotationId};
use crate::mouse::{self, Pointer};
use crate::note_view::{NoteEvent, NoteView};
use crate::session::{self, Session, SessionId};
use crate::session_note::{NoteChange, SessionNote, SessionNoteView};

pub struct TerminalPane {
    pub session: Rc<Session>,
    pub fonts: Rc<Fonts>,
    pub scheme: Rc<Scheme>,
    pub focused: bool,
    pub revision: u64,
    pub broadcast: bool,
    pub maximized: bool,
    pub find: Option<Rc<str>>,
    pub filter: Option<Rc<str>>,
    pub on_focus: Callback<SessionId, ()>,
    pub on_hover: Callback<SessionId, ()>,
    pub on_select: Callback<String, ()>,
    pub held: State<Modifiers>,
    pub badge: Option<Rc<str>>,
    pub opacity: f32,
    pub preedit: Option<Rc<str>>,
    pub replay: Option<usize>,
    pub blink_off: bool,
    pub annotations: Vec<Annotation>,
    pub dark: bool,
    pub on_note: Callback<(AnnotationId, NoteEvent), ()>,
    pub composer: Composer,
    pub hints: Option<Hints>,
    pub on_composer: Callback<ComposerEvent, ()>,
    pub note: SessionNote,
    pub on_session_note: Callback<NoteChange, ()>,
}

impl PartialEq for TerminalPane {
    fn eq(&self, other: &Self) -> bool {
        self.session.id == other.session.id
            && Rc::ptr_eq(&self.fonts, &other.fonts)
            && *self.scheme == *other.scheme
            && self.focused == other.focused
            && self.revision == other.revision
            && self.broadcast == other.broadcast
            && self.maximized == other.maximized
            && self.find == other.find
            && self.filter == other.filter
            && self.badge == other.badge
            && self.opacity == other.opacity
            && self.preedit == other.preedit
            && self.replay == other.replay
            && self.blink_off == other.blink_off
            && self.annotations == other.annotations
            && self.dark == other.dark
            && self.composer == other.composer
            && self.hints == other.hints
            && self.note == other.note
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Click {
    at: Instant,
    point: Point,
    count: u8,
}

impl Component for TerminalPane {
    fn render(&self) -> impl IntoElement {
        let mut dragging = use_state(|| false);
        let mut last_click = use_state(|| None::<Click>);
        let mut repaint = use_state(|| 0u64);
        let session = self.session.clone();
        let fonts = self.fonts.clone();
        let scheme = self.scheme.clone();
        let focused = self.focused;
        let broadcast = self.broadcast;
        let find = self.find.clone();
        let filter = self.filter.clone();
        let on_focus = self.on_focus.clone();
        let on_hover = self.on_hover.clone();
        let on_select = self.on_select.clone();
        let held = self.held;
        let primary = Primary::get();
        let mut pressed = use_state(|| None::<MouseButton>);
        let mut reported_cell = use_state(|| None::<(usize, usize)>);
        let badge = self.badge.clone();
        let opacity = self.opacity;
        let preedit = self.preedit.clone();
        let replay = self.replay;
        let blink_off = self.blink_off;
        let mut size = use_state(|| (0.0, 0.0));
        let indicators =
            shown_indicators(&session, self.maximized, broadcast, self.filter.is_some());
        let dark = is_dark(scheme.background);
        let notes = notes(
            &session,
            &fonts,
            &self.annotations,
            *size.read(),
            self.dark,
            &self.on_note,
        );

        let mut look = DefaultHasher::new();
        (
            session.id,
            self.revision,
            *repaint.read(),
            focused,
            broadcast,
            (&find, &filter),
            &badge,
            opacity.to_bits(),
            &preedit,
            (
                replay,
                blink_off,
                notes.len(),
                &self
                    .annotations
                    .iter()
                    .map(|note| (note.start, note.end))
                    .collect::<Vec<_>>(),
                self.hints.as_ref().map(|hints| hints.items.len()),
                &indicators,
            ),
            &scheme.name,
            fonts.cell_width.to_bits(),
        )
            .hash(&mut look);
        let hinted: Vec<((i64, usize), (i64, usize))> = self
            .hints
            .iter()
            .flat_map(|hints| hints.items.iter().map(|hint| (hint.start, hint.end)))
            .collect();
        let labels = hint_labels(&session, &fonts, self.hints.as_ref());
        let painter = {
            let session = session.clone();
            let fonts = fonts.clone();
            let scheme = scheme.clone();
            canvas(look.finish(), move |ctx| {
                let filtered = filter.as_deref().map(|query| session.filtered_rows(query));
                let term = session.term.lock();
                let extras = session.extras();
                let history = term.history_size() as i64;
                let point = |(line, column): (i64, usize)| {
                    Point::new(Line((line - history) as i32), Column(column))
                };
                let matches: Vec<_> = find
                    .as_deref()
                    .map(|query| session::visible_matches(&term, query))
                    .unwrap_or_default()
                    .into_iter()
                    .chain(
                        hinted
                            .iter()
                            .map(|(start, end)| point(*start)..=point(*end)),
                    )
                    .collect();
                let overlay = grid::Overlay {
                    extras: &extras,
                    matches: &matches,
                    inset: SPACE_2,
                    broadcast: broadcast.then_some(scheme.ansi[1]),
                    badge: badge.as_deref(),
                    opacity,
                    preedit: preedit.as_deref(),
                    replay: replay
                        .and_then(|index| extras.replay.get(index))
                        .map(|snapshot| snapshot.rows.as_slice())
                        .or(filtered.as_deref())
                        .or(extras.zoomed.as_deref()),
                    blink_off,
                };
                grid::draw(ctx, &term, &fonts, &scheme, focused, &overlay);
                indicators::draw(ctx, &indicators, dark);
            })
        };
        let point_at = {
            let session = session.clone();
            let fonts = fonts.clone();
            move |(x, y): (f64, f64)| {
                point_at(&session, &fonts, x as f32 - SPACE_2, y as f32 - SPACE_2)
            }
        };

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(
                (opacity * 255.0).round() as u8,
                scheme.background.r,
                scheme.background.g,
                scheme.background.b,
            ))
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .on_sized({
                        let session = session.clone();
                        let fonts = fonts.clone();
                        move |event: Event<SizedEventData>| {
                            let area = event.data().area;
                            session.origin.set((area.min_x(), area.min_y()));
                            size.set_if_modified((area.width(), area.height()));
                            session.fit(
                                area.width() - 2.0 * SPACE_2,
                                area.height() - 2.0 * SPACE_2,
                                fonts.cell_width,
                                fonts.cell_height,
                            );
                        }
                    })
                    .on_pointer_enter({
                        let id = session.id;
                        move |_| {
                            Cursor::set(CursorIcon::Text);
                            on_hover.call(id);
                        }
                    })
                    .on_pointer_leave(|_| Cursor::set(CursorIcon::default()))
                    .on_mouse_down({
                        let session = session.clone();
                        let point_at = point_at.clone();
                        let primary = primary.clone();
                        move |event: Event<MouseEventData>| {
                            let data = event.data();
                            let (point, side) = point_at(data.element_location.to_tuple());
                            if let Some(button) = data.button
                                && mouse::reported(session.mode(), *held.peek())
                            {
                                on_focus.call(session.id);
                                pressed.set(Some(button));
                                send(&session, Pointer::Press(button), point, *held.peek());
                                return;
                            }
                            if data.button == Some(MouseButton::Middle) {
                                on_focus.call(session.id);
                                if let Some(text) = primary.text() {
                                    session.paste(&text);
                                }
                                return;
                            }
                            if data.button != Some(MouseButton::Left) {
                                return;
                            }
                            on_focus.call(session.id);
                            if held.peek().contains(Modifiers::META) && session.open_at(point) {
                                return;
                            }
                            let click = next_click(*last_click.read(), point);
                            last_click.set(Some(click));
                            let kind = match click.count {
                                1 => SelectionType::Simple,
                                2 => SelectionType::Semantic,
                                3 => SelectionType::Lines,
                                _ => {
                                    session.smart_select(point);
                                    *repaint.write() += 1;
                                    return;
                                }
                            };
                            session.start_selection(kind, point, side);
                            dragging.set(true);
                            *repaint.write() += 1;
                        }
                    })
                    .on_mouse_up({
                        let session = session.clone();
                        let point_at = point_at.clone();
                        let primary = primary.clone();
                        move |event: Event<MouseEventData>| {
                            let Some(button) = pressed.write().take() else {
                                if let Some(text) = session.selected_text() {
                                    on_select.call(text.clone());
                                    primary.publish(text);
                                }
                                return;
                            };
                            if mouse::reported(session.mode(), *held.peek()) {
                                let (point, _) = point_at(event.data().element_location.to_tuple());
                                send(&session, Pointer::Release(button), point, *held.peek());
                            }
                        }
                    })
                    .on_mouse_move({
                        let session = session.clone();
                        move |event: Event<MouseEventData>| {
                            let (point, side) = point_at(event.data().element_location.to_tuple());
                            if mouse::reported(session.mode(), *held.peek()) {
                                let cell = viewport_cell(&session, point);
                                if *reported_cell.peek() != Some(cell) {
                                    reported_cell.set(Some(cell));
                                    let pointer =
                                        (*pressed.peek()).map_or(Pointer::Motion, Pointer::Drag);
                                    send(&session, pointer, point, *held.peek());
                                }
                                return;
                            }
                            if !*dragging.read() {
                                return;
                            }
                            session.extend_selection(point, side);
                            *repaint.write() += 1;
                        }
                    })
                    .on_global_pointer_press(move |_| dragging.set_if_modified(false))
                    .on_wheel({
                        let fonts = fonts.clone();
                        move |event: Event<WheelEventData>| {
                            let lines =
                                (-event.data().delta_y as f32 / fonts.cell_height).round() as i32;
                            if mouse::reported(session.mode(), *held.peek()) {
                                let pointer = if lines > 0 {
                                    Pointer::WheelUp
                                } else {
                                    Pointer::WheelDown
                                };
                                let at = (*reported_cell.peek()).unwrap_or_default();
                                for _ in 0..lines.unsigned_abs() {
                                    if let Some(bytes) =
                                        mouse::report(pointer, at, *held.peek(), session.mode())
                                    {
                                        session.write(bytes);
                                    }
                                }
                                return;
                            }
                            wheel(&session, lines);
                            *repaint.write() += 1;
                        }
                    })
                    .child(painter),
            )
            .children(notes)
            .children(labels)
            .maybe_child(
                composer_opacity(&self.composer).map(|opacity| ComposerView {
                    text: self.composer.text.clone(),
                    bounds: *size.read(),
                    family: fonts.family.clone(),
                    size: fonts.size,
                    line_height: fonts.cell_height,
                    ink: rgb(scheme.foreground),
                    cursor: rgb(scheme.cursor),
                    opacity,
                    dark: is_dark(scheme.background),
                    on_event: self.on_composer.clone(),
                }),
            )
            .maybe_child(self.note.visible.then(|| SessionNoteView {
                note: self.note.clone(),
                bounds: *size.read(),
                family: fonts.family.clone(),
                size: fonts.size,
                line_height: fonts.cell_height,
                ink: rgb(scheme.foreground),
                dark: is_dark(scheme.background),
                on_event: self.on_session_note.clone(),
            }))
    }
}

fn hint_labels(session: &Session, fonts: &Fonts, hints: Option<&Hints>) -> Vec<Element> {
    let Some(hints) = hints else {
        return Vec::new();
    };
    let top = {
        let term = session.term.lock();
        term.history_size() as i64 - term.grid().display_offset() as i64
    };
    let prefix = hints.prefix.to_lowercase();
    hints
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, hint)| {
            let matches_prefix = hint.key.to_lowercase().starts_with(&prefix);
            let motion = match (hints.ended, hints.narrowed) {
                (Some((at, chosen)), _) if chosen == Some(index) => hint_view::popping(at),
                (Some((at, _)), _) => hint_view::dissolving(at),
                (None, Some(at)) if !matches_prefix => hint_view::popping(at),
                _ => hint_view::entering(hints.shown),
            };
            (motion.opacity > 0.0).then(|| {
                let first = hint.start.1.min(hint.end.1) as f32;
                HintLabel {
                    key: hint.key.clone(),
                    typed: if matches_prefix { prefix.len() } else { 0 },
                    target: (
                        first * fonts.cell_width + SPACE_2,
                        (hint.start.0 - top) as f32 * fonts.cell_height + SPACE_2,
                        fonts.cell_height,
                    ),
                    motion,
                    diff_key: DiffKey::None,
                }
                .key(index)
                .into_element()
            })
        })
        .collect()
}

pub fn hints_animating(hints: &Hints) -> bool {
    let busy = |since: Instant, ms| !hint_view::settled(since, ms);
    busy(hints.shown, SHORTCUT_LABEL.enter_ms)
        || hints
            .narrowed
            .is_some_and(|at| busy(at, SHORTCUT_LABEL.exit_ms))
        || hints.ended.is_some()
}

fn rgb(color: alacritty_terminal::vte::ansi::Rgb) -> vow_design::Rgba {
    vow_design::Rgba::rgb8(color.r, color.g, color.b)
}

// NSColor+iTerm.m -isDark: perceived brightness under one half.
fn is_dark(color: alacritty_terminal::vte::ansi::Rgb) -> bool {
    let brightness =
        0.30 * f32::from(color.r) + 0.59 * f32::from(color.g) + 0.11 * f32::from(color.b);
    brightness < 0.5 * 255.0
}

fn composer_opacity(composer: &Composer) -> Option<f32> {
    if composer.open {
        return Some(1.0);
    }
    let faded = progress(composer.closed, COMPOSER.fade_ms);
    (faded < 1.0).then_some(1.0 - faded)
}

pub fn composer_animating(composer: &Composer) -> bool {
    !composer.open && progress(composer.closed, COMPOSER.fade_ms) < 1.0
}

fn notes(
    session: &Rc<Session>,
    fonts: &Fonts,
    annotations: &[Annotation],
    bounds: (f32, f32),
    dark: bool,
    on_note: &Callback<(AnnotationId, NoteEvent), ()>,
) -> Vec<Element> {
    let (history, offset, lines) = {
        let term = session.term.lock();
        (
            term.history_size() as i64,
            term.grid().display_offset() as i64,
            term.screen_lines() as i64,
        )
    };
    let top = history - offset;
    annotations
        .iter()
        .filter_map(|annotation| {
            let row = annotation.end.0 - top;
            let opacity = note_opacity(annotation);
            if !(0..lines).contains(&row) || opacity == 0.0 {
                return None;
            }
            let id = annotation.id;
            let on_note = on_note.clone();
            Some(
                NoteView {
                    text: annotation.text.clone(),
                    anchor: (
                        annotation.end.1 as f32 * fonts.cell_width + SPACE_2,
                        (row + 1) as f32 * fonts.cell_height + SPACE_2,
                    ),
                    bounds,
                    dark,
                    opacity,
                    highlight: note_highlight(annotation),
                    focus: annotation.focus,
                    on_event: Callback::new(move |event| on_note.call((id, event))),
                    key: DiffKey::None,
                }
                .key(id)
                .into_element(),
            )
        })
        .collect()
}

fn progress(since: Option<Instant>, duration_ms: u64) -> f32 {
    since.map_or(1.0, |since| {
        (since.elapsed().as_secs_f32() * 1000.0 / duration_ms as f32).min(1.0)
    })
}

pub fn note_animating(annotation: &Annotation) -> bool {
    progress(annotation.toggled, ANNOTATION.fade_ms) < 1.0
        || progress(annotation.highlighted, ANNOTATION.highlight_ms) < 1.0
}

fn note_opacity(annotation: &Annotation) -> f32 {
    let shown = progress(annotation.toggled, ANNOTATION.fade_ms);
    if annotation.hidden {
        1.0 - shown
    } else {
        shown
    }
}

// PTYNoteViewController.m -updateBackgroundColor squares the progress to linger on the highlight.
fn note_highlight(annotation: &Annotation) -> f32 {
    annotation.highlighted.map_or(0.0, |_| {
        1.0 - progress(annotation.highlighted, ANNOTATION.highlight_ms).powi(2)
    })
}

#[derive(Clone)]
struct Primary(#[cfg(feature = "kit-parcheado")] barrier_kit::platform::WindowControls);

// BK-034: without patch 0010 there is no primary selection, so it reads nothing and keeps nothing.
impl Primary {
    fn get() -> Self {
        Self(
            #[cfg(feature = "kit-parcheado")]
            barrier_kit::platform::WindowControls::get(),
        )
    }

    fn publish(&self, text: String) {
        #[cfg(feature = "kit-parcheado")]
        let _ = self.0.primary_selection().set_contents(text);
        #[cfg(not(feature = "kit-parcheado"))]
        drop(text);
    }

    fn text(&self) -> Option<String> {
        #[cfg(feature = "kit-parcheado")]
        return self.0.primary_selection().get_contents().ok();
        #[cfg(not(feature = "kit-parcheado"))]
        None
    }
}

fn viewport_cell(session: &Session, point: Point) -> (usize, usize) {
    let line = point.line.0 + session.display_offset() as i32;
    (point.column.0, line.max(0) as usize)
}

fn send(session: &Session, pointer: Pointer, point: Point, held: Modifiers) {
    if let Some(bytes) = mouse::report(pointer, viewport_cell(session, point), held, session.mode())
    {
        session.write(bytes);
    }
}

fn point_at(session: &Session, fonts: &Fonts, x: f32, y: f32) -> (Point, Side) {
    let column = (x / fonts.cell_width).max(0.0);
    let line = (y / fonts.cell_height).max(0.0) as i32;
    let size = session.size();
    let point = Point::new(
        Line(line.min(i32::from(size.lines) - 1) - session.display_offset() as i32),
        Column((column as usize).min(usize::from(size.columns).saturating_sub(1))),
    );
    let side = if column.fract() < 0.5 {
        Side::Left
    } else {
        Side::Right
    };
    (point, side)
}

fn next_click(last: Option<Click>, point: Point) -> Click {
    let now = Instant::now();
    let count = match last {
        Some(last)
            if last.point == point
                && now - last.at < Duration::from_millis(TERMINAL.multi_click_ms) =>
        {
            last.count % 4 + 1
        }
        _ => 1,
    };
    Click {
        at: now,
        point,
        count,
    }
}

fn wheel(session: &Session, lines: i32) {
    if lines == 0 {
        return;
    }
    let mode = session.mode();
    if mode.contains(TermMode::ALT_SCREEN) && mode.contains(TermMode::ALTERNATE_SCROLL) {
        let arrow: &[u8] = if lines > 0 { b"\x1bOA" } else { b"\x1bOB" };
        session.write(arrow.repeat(lines.unsigned_abs() as usize));
    } else {
        session.scroll(Scroll::Delta(lines));
    }
}

// PTYTextView.m -updateIndicators, for the states CurseTerm has.
fn shown_indicators(
    session: &Session,
    maximized: bool,
    broadcast: bool,
    filtered: bool,
) -> Vec<Indicator> {
    // copy_mode() locks the terminal, which must not happen while the extras are held.
    let copy_mode = session.copy_mode();
    let extras = session.extras();
    [
        (Indicator::Maximized, maximized),
        (Indicator::Broadcast, broadcast),
        (Indicator::Coprocess, extras.coprocess.is_some()),
        (Indicator::Alert, extras.alert_on_next_mark),
        (Indicator::Filter, filtered),
        (Indicator::CopyMode, copy_mode),
    ]
    .into_iter()
    .filter_map(|(indicator, shown)| shown.then_some(indicator))
    .collect()
}
