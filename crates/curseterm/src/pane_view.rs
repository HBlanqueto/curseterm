use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Scroll;
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use barrier_kit_next::canvas::canvas;
use freya_core::prelude::*;
use torin::prelude::Size;
use vow_design::SPACE_2;

use crate::config::Scheme;
use crate::grid::{self, Fonts};
use crate::session::{Session, SessionId};

const MULTI_CLICK: Duration = Duration::from_millis(400);

pub struct TerminalPane {
    pub session: Rc<Session>,
    pub fonts: Rc<Fonts>,
    pub scheme: Rc<Scheme>,
    pub focused: bool,
    pub revision: u64,
    pub broadcast: bool,
    pub on_focus: Callback<SessionId, ()>,
}

impl PartialEq for TerminalPane {
    fn eq(&self, other: &Self) -> bool {
        self.session.id == other.session.id
            && Rc::ptr_eq(&self.fonts, &other.fonts)
            && *self.scheme == *other.scheme
            && self.focused == other.focused
            && self.revision == other.revision
            && self.broadcast == other.broadcast
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
        let on_focus = self.on_focus.clone();

        let mut look = DefaultHasher::new();
        (
            self.revision,
            *repaint.read(),
            focused,
            broadcast,
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
                let overlay = grid::Overlay {
                    extras: &extras,
                    inset: SPACE_2,
                    broadcast: broadcast.then_some(scheme.ansi[1]),
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
            .background(Color::from_rgb(
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
                        move |event: Event<MouseEventData>| {
                            let data = event.data();
                            if data.button != Some(MouseButton::Left) {
                                return;
                            }
                            on_focus.call(session.id);
                            let (point, side) = point_at(data.element_location.to_tuple());
                            let click = next_click(*last_click.read(), point);
                            last_click.set(Some(click));
                            let kind = match click.count {
                                1 => SelectionType::Simple,
                                2 => SelectionType::Semantic,
                                _ => SelectionType::Lines,
                            };
                            session.start_selection(kind, point, side);
                            dragging.set(true);
                            *repaint.write() += 1;
                        }
                    })
                    .on_mouse_move({
                        let session = session.clone();
                        move |event: Event<MouseEventData>| {
                            if !*dragging.read() {
                                return;
                            }
                            let (point, side) = point_at(event.data().element_location.to_tuple());
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
                            wheel(&session, lines);
                            *repaint.write() += 1;
                        }
                    })
                    .child(painter),
            )
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
        Some(last) if last.point == point && now - last.at < MULTI_CLICK => last.count % 3 + 1,
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
