use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Scroll;
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use barrier_kit_next::canvas::canvas;
use barrier_kit_next::tokens::TERMINAL;
use freya_core::prelude::*;
use torin::prelude::Size;
use vow_design::SPACE_2;

use crate::config::Scheme;
use crate::grid::{self, Fonts};
use crate::links;
use crate::mouse::{self, Pointer};
use crate::session::{self, Session, SessionId};

pub struct TerminalPane {
    pub session: Rc<Session>,
    pub fonts: Rc<Fonts>,
    pub scheme: Rc<Scheme>,
    pub focused: bool,
    pub revision: u64,
    pub broadcast: bool,
    pub find: Option<Rc<str>>,
    pub on_focus: Callback<SessionId, ()>,
    pub held: State<Modifiers>,
    pub badge: Option<Rc<str>>,
    pub opacity: f32,
    pub preedit: Option<Rc<str>>,
    pub replay: Option<usize>,
    pub blink_off: bool,
}

impl PartialEq for TerminalPane {
    fn eq(&self, other: &Self) -> bool {
        self.session.id == other.session.id
            && Rc::ptr_eq(&self.fonts, &other.fonts)
            && *self.scheme == *other.scheme
            && self.focused == other.focused
            && self.revision == other.revision
            && self.broadcast == other.broadcast
            && self.find == other.find
            && self.badge == other.badge
            && self.opacity == other.opacity
            && self.preedit == other.preedit
            && self.replay == other.replay
            && self.blink_off == other.blink_off
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
        let on_focus = self.on_focus.clone();
        let held = self.held;
        let primary = Primary::get();
        let mut pressed = use_state(|| None::<MouseButton>);
        let mut reported_cell = use_state(|| None::<(usize, usize)>);
        let badge = self.badge.clone();
        let opacity = self.opacity;
        let preedit = self.preedit.clone();
        let replay = self.replay;
        let blink_off = self.blink_off;

        let mut look = DefaultHasher::new();
        (
            session.id,
            self.revision,
            *repaint.read(),
            focused,
            broadcast,
            &find,
            &badge,
            opacity.to_bits(),
            &preedit,
            (replay, blink_off),
            &scheme.name,
            fonts.cell_width.to_bits(),
        )
            .hash(&mut look);
        let painter = {
            let session = session.clone();
            let fonts = fonts.clone();
            let scheme = scheme.clone();
            canvas(look.finish(), move |ctx| {
                let term = session.term.lock();
                let extras = session.extras();
                let matches = find
                    .as_deref()
                    .map(|query| session::visible_matches(&term, query))
                    .unwrap_or_default();
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
                        .map(|snapshot| snapshot.rows.as_slice()),
                    blink_off,
                };
                grid::draw(ctx, &term, &fonts, &scheme, focused, &overlay);
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
                            session.fit(
                                area.width() - 2.0 * SPACE_2,
                                area.height() - 2.0 * SPACE_2,
                                fonts.cell_width,
                                fonts.cell_height,
                            );
                        }
                    })
                    .on_pointer_enter(|_| Cursor::set(CursorIcon::Text))
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
                            if held.peek().contains(Modifiers::META)
                                && let Some(link) = session.link_at(point)
                            {
                                links::open(&link);
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
    }
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
