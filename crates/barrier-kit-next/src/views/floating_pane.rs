//! A pane that floats over the tiled ones, like iTermFloatingPaneView: a title bar to move it,
//! a band round the outline to resize it, and a shadow.

use barrier_kit::style::{Palette, text};
use barrier_kit::theme::color;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Content, Direction, Position, Size};
use vow_design::{CAPTION, Rgba};

use crate::tokens::FLOATING_PANE;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PaneFrame {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

// Which sides a drag moves: the title bar moves all four, an edge or corner of the band its own.
#[derive(Clone, Copy, PartialEq)]
struct Sides {
    left: bool,
    top: bool,
    right: bool,
    bottom: bool,
}

const MOVE: Sides = Sides {
    left: true,
    top: true,
    right: true,
    bottom: true,
};

impl Sides {
    fn apply(self, frame: PaneFrame, dx: f32, dy: f32) -> PaneFrame {
        if self == MOVE {
            return PaneFrame {
                x: frame.x + dx,
                y: frame.y + dy,
                ..frame
            };
        }
        let mut moved = frame;
        if self.left {
            moved.x += dx;
            moved.width -= dx;
        }
        if self.right {
            moved.width += dx;
        }
        if self.top {
            moved.y += dy;
            moved.height -= dy;
        }
        if self.bottom {
            moved.height += dy;
        }
        moved
    }

    fn cursor(self) -> CursorIcon {
        match (self.left || self.right, self.top || self.bottom) {
            (true, false) => CursorIcon::ColResize,
            (false, true) => CursorIcon::RowResize,
            _ if self.left == self.top => CursorIcon::NwseResize,
            _ => CursorIcon::NeswResize,
        }
    }
}

#[derive(PartialEq)]
pub struct FloatingPane {
    pub frame: PaneFrame,
    pub title: String,
    pub active: bool,
    pub accent: Rgba,
    pub title_bar: Rgba,
    pub title_ink: Rgba,
    pub content: Element,
    pub on_raise: Callback<(), ()>,
    pub on_frame: Callback<PaneFrame, ()>,
}

impl Component for FloatingPane {
    fn render(&self) -> impl IntoElement {
        let drag = use_state(|| None::<(Sides, (f64, f64))>);
        let palette = Palette::current();
        let frame = self.frame;
        let band = FLOATING_PANE.resize_band;
        // floating-panes-design.md §4.3: the outline is the focus cue.
        let outline = if self.active {
            self.accent
        } else {
            palette.separator
        };
        let grab = move |sides: Sides| {
            let mut drag = drag;
            move |event: Event<MouseEventData>| {
                event.stop_propagation();
                drag.set(Some((sides, event.data().global_location.to_tuple())));
            }
        };
        let handle = |sides: Sides, position: Position, width: Size, height: Size| {
            rect()
                .position(position)
                .width(width)
                .height(height)
                .layer(1)
                .on_pointer_enter(move |_| Cursor::set(sides.cursor()))
                .on_pointer_leave(move |_| {
                    if drag.peek().is_none() {
                        Cursor::set(CursorIcon::default());
                    }
                })
                .on_mouse_down(grab(sides))
                .into_element()
        };
        let side = |left, top, right, bottom| Sides {
            left,
            top,
            right,
            bottom,
        };
        let edge = Size::px(band);
        let handles = vec![
            handle(
                side(true, false, false, false),
                Position::new_absolute().left(0.0).top(band),
                edge.clone(),
                Size::fill(),
            ),
            handle(
                side(false, false, true, false),
                Position::new_absolute().right(0.0).top(band),
                edge.clone(),
                Size::fill(),
            ),
            handle(
                side(false, true, false, false),
                Position::new_absolute().top(0.0).left(band),
                Size::fill(),
                edge.clone(),
            ),
            handle(
                side(false, false, false, true),
                Position::new_absolute().bottom(0.0).left(band),
                Size::fill(),
                edge.clone(),
            ),
            handle(
                side(true, true, false, false),
                Position::new_absolute().left(0.0).top(0.0),
                edge.clone(),
                edge.clone(),
            ),
            handle(
                side(false, true, true, false),
                Position::new_absolute().right(0.0).top(0.0),
                edge.clone(),
                edge.clone(),
            ),
            handle(
                side(true, false, false, true),
                Position::new_absolute().left(0.0).bottom(0.0),
                edge.clone(),
                edge.clone(),
            ),
            handle(
                side(false, false, true, true),
                Position::new_absolute().right(0.0).bottom(0.0),
                edge.clone(),
                edge,
            ),
        ];
        let on_raise = self.on_raise.clone();
        let on_frame = self.on_frame.clone();
        let title_bar = rect()
            .width(Size::fill())
            .height(Size::px(FLOATING_PANE.title_height))
            .background(color(self.title_bar))
            .main_align(Alignment::Center)
            .cross_align(Alignment::Center)
            .on_pointer_enter(move |_| Cursor::set(CursorIcon::Grab))
            .on_pointer_leave(move |_| {
                if drag.peek().is_none() {
                    Cursor::set(CursorIcon::default());
                }
            })
            .on_mouse_down(grab(MOVE))
            .child(text(self.title.clone(), CAPTION, self.title_ink));
        let shadow = Rgba {
            a: FLOATING_PANE.shadow_opacity,
            ..Rgba::rgb8(0, 0, 0)
        };
        rect()
            .position(
                Position::new_absolute()
                    .left(frame.x - band)
                    .top(frame.y - band),
            )
            .width(Size::px(frame.width + 2.0 * band))
            .height(Size::px(frame.height + 2.0 * band))
            .layer(Layer::Overlay)
            .on_mouse_down(move |_| on_raise.call(()))
            .on_global_pointer_move(move |event: Event<PointerEventData>| {
                let mut drag = drag;
                let Some((sides, (x, y))) = *drag.peek() else {
                    return;
                };
                let (to_x, to_y) = event.data().global_location().to_tuple();
                drag.set(Some((sides, (to_x, to_y))));
                on_frame.call(sides.apply(frame, (to_x - x) as f32, (to_y - y) as f32));
            })
            .on_global_pointer_press(move |_| {
                let mut drag = drag;
                if drag.peek().is_some() {
                    drag.set(None);
                    Cursor::set(CursorIcon::default());
                }
            })
            .child(
                rect()
                    .position(Position::new_absolute().left(band).top(band))
                    .width(Size::px(frame.width))
                    .height(Size::px(frame.height))
                    .direction(Direction::Vertical)
                    .content(Content::flex())
                    .background(color(palette.window_background))
                    .border(
                        Border::new()
                            .width(FLOATING_PANE.outline)
                            .alignment(BorderAlignment::Outer)
                            .fill(color(outline)),
                    )
                    .shadow(
                        Shadow::new()
                            .y(FLOATING_PANE.shadow_y)
                            .blur(FLOATING_PANE.shadow_blur)
                            .color(color(shadow)),
                    )
                    .child(title_bar)
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::flex(1.0))
                            .child(self.content.clone()),
                    ),
            )
            .children(handles)
    }
}
