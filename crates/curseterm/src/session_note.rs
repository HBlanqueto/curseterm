use std::hash::{DefaultHasher, Hash, Hasher};

use crate::tokens::{SESSION_NOTE, SHEET_MATERIAL_BLUR};
use barrier_kit::canvas::{CanvasContext, canvas};
use barrier_kit::style::text;
use barrier_kit::symbol::{Symbol, symbol};
use barrier_kit::theme::color;
use barrier_kit::views::text_view::{KeyPress, TextView};
use freya_core::prelude::*;
use freya_engine::prelude::{
    BlurStyle, MaskFilter, PaintStyle, RRect, Rect as SkRect, SaveLayerRec, TileMode, blur,
};
use torin::prelude::{Alignment, Content, Direction, Gaps, Position, Size};
use vow_design::{BG_WINDOW_DARK, BG_WINDOW_LIGHT, CAPTION, DARK, LIGHT, Rgba};

use crate::grid::rgba_paint;
use crate::localize::tr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SessionNote {
    pub text: String,
    pub frame: Option<Frame>,
    pub collapsed: bool,
    pub visible: bool,
}

impl SessionNote {
    // PTYSession.swift -textViewEditSessionNote: the menu item shows or hides it; showing it
    // expands it (SessionView.m -showSessionNoteWithModel:).
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.collapsed = false;
        }
    }
}

pub enum NoteChange {
    Text(String),
    Focused(bool),
    Moved(Frame),
    Collapsed(bool),
}

// SessionView.m -ensureSessionNoteViewWithModel: a new note sits in the top-right corner.
fn placed(frame: Option<Frame>, bounds: (f32, f32)) -> Frame {
    let mut frame = frame.unwrap_or(Frame {
        x: (bounds.0 - SESSION_NOTE.width - SESSION_NOTE.margin).max(SESSION_NOTE.margin),
        y: SESSION_NOTE.margin,
        width: SESSION_NOTE.width,
        height: SESSION_NOTE.height,
    });
    frame.x = frame.x.min(bounds.0 - frame.width).max(0.0);
    frame.y = frame.y.min(bounds.1 - SESSION_NOTE.title_height).max(0.0);
    frame
}

#[derive(Clone, Copy, PartialEq)]
struct Drag {
    from: (f64, f64),
    start: Frame,
    left: bool,
    right: bool,
    top: bool,
    bottom: bool,
}

impl Drag {
    fn moving(&self) -> bool {
        !(self.left || self.right || self.top || self.bottom)
    }

    // SessionNoteView.swift -mouseDragged: the title bar moves it and the edges resize it.
    fn apply(&self, to: (f64, f64)) -> Frame {
        let (dx, dy) = ((to.0 - self.from.0) as f32, (to.1 - self.from.1) as f32);
        let mut frame = self.start;
        if self.moving() {
            frame.x += dx;
            frame.y += dy;
            return frame;
        }
        if self.right {
            frame.width = (self.start.width + dx).max(SESSION_NOTE.min_width);
        } else if self.left {
            frame.width = (self.start.width - dx).max(SESSION_NOTE.min_width);
            frame.x = self.start.x + self.start.width - frame.width;
        }
        if self.bottom {
            frame.height = (self.start.height + dy).max(SESSION_NOTE.min_height);
        } else if self.top {
            frame.height = (self.start.height - dy).max(SESSION_NOTE.min_height);
            frame.y = self.start.y + self.start.height - frame.height;
        }
        frame
    }
}

pub struct SessionNoteView {
    pub note: SessionNote,
    pub bounds: (f32, f32),
    pub family: String,
    pub size: f32,
    pub line_height: f32,
    pub ink: Rgba,
    pub dark: bool,
    pub on_event: Callback<NoteChange, ()>,
}

impl PartialEq for SessionNoteView {
    fn eq(&self, other: &Self) -> bool {
        self.note == other.note
            && self.bounds == other.bounds
            && self.family == other.family
            && self.size == other.size
            && self.line_height == other.line_height
            && self.ink == other.ink
            && self.dark == other.dark
    }
}

impl Component for SessionNoteView {
    fn render(&self) -> impl IntoElement {
        let mut value = use_state(|| self.note.text.clone());
        let mut seen = use_state(|| self.note.text.clone());
        if *seen.peek() != self.note.text {
            seen.set(self.note.text.clone());
            value.set(self.note.text.clone());
        }
        let mut drag = use_state(|| None::<Drag>);
        let mut dragged = use_state(|| None::<Frame>);
        let stored = placed(self.note.frame, self.bounds);
        let frame = dragged.read().unwrap_or(stored);
        let collapsed = self.note.collapsed;
        let height = if collapsed {
            SESSION_NOTE.title_height
        } else {
            frame.height
        };
        let palette = if self.dark { DARK } else { LIGHT };
        let window = if self.dark {
            BG_WINDOW_DARK
        } else {
            BG_WINDOW_LIGHT
        };
        let tint = Rgba {
            a: SESSION_NOTE.tint_alpha,
            ..window
        };
        let on_event = self.on_event.clone();
        let (on_change, on_focus, on_key, on_collapse, on_moved) = (
            on_event.clone(),
            on_event.clone(),
            on_event.clone(),
            on_event.clone(),
            on_event,
        );
        let start = move |left, right, top, bottom| {
            move |event: Event<MouseEventData>| {
                event.stop_propagation();
                let at = event.global_location;
                drag.set(Some(Drag {
                    from: (at.x, at.y),
                    start: frame,
                    left,
                    right,
                    top,
                    bottom,
                }));
            }
        };
        // SessionNoteView.swift -updateTitle: a collapsed note shows its first line.
        let first_line = self.note.text.lines().next().unwrap_or_default();
        let title = if collapsed && !first_line.is_empty() {
            first_line.to_string()
        } else {
            tr!("session-note-title").to_string()
        };
        let edge = |position: Position, horizontal: bool| {
            let strip = rect().position(position).layer(2);
            if horizontal {
                strip
                    .width(Size::fill())
                    .height(Size::px(SESSION_NOTE.drag_area))
            } else {
                strip
                    .width(Size::px(SESSION_NOTE.drag_area))
                    .height(Size::percent(100.0))
            }
        };
        let edges = (!collapsed).then(|| {
            vec![
                edge(Position::new_absolute().left(0.0).top(0.0), false)
                    .on_mouse_down(start(true, false, false, false))
                    .into_element(),
                edge(Position::new_absolute().right(0.0).top(0.0), false)
                    .on_mouse_down(start(false, true, false, false))
                    .into_element(),
                edge(Position::new_absolute().left(0.0).bottom(0.0), true)
                    .on_mouse_down(start(false, false, false, true))
                    .into_element(),
                edge(Position::new_absolute().left(0.0).top(0.0), true)
                    .on_mouse_down(start(false, false, true, false))
                    .into_element(),
            ]
        });
        rect()
            .position(Position::new_absolute().left(frame.x).top(frame.y))
            .width(Size::px(frame.width))
            .height(Size::px(height))
            .layer(1)
            .direction(Direction::Vertical)
            .content(Content::flex())
            .on_mouse_down(|event: Event<MouseEventData>| event.stop_propagation())
            .child(background(frame.width, height, tint, self.dark))
            .on_global_pointer_move(move |event: Event<PointerEventData>| {
                if let Some(active) = *drag.peek() {
                    let at = event.global_location();
                    dragged.set(Some(active.apply((at.x, at.y))));
                }
            })
            .on_global_pointer_press(move |_| {
                if drag.peek().is_some() {
                    drag.set(None);
                    if let Some(frame) = dragged.peek().as_ref() {
                        on_moved.call(NoteChange::Moved(*frame));
                    }
                    dragged.set(None);
                }
            })
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(SESSION_NOTE.title_height))
                    .padding(Gaps::new(
                        0.0,
                        SESSION_NOTE.button_inset,
                        0.0,
                        SESSION_NOTE.inset,
                    ))
                    .direction(Direction::Horizontal)
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .on_mouse_down(start(false, false, false, false))
                    .child(rect().width(Size::flex(1.0)).child(text(
                        title,
                        CAPTION,
                        palette.ink_muted,
                    )))
                    .child(
                        rect()
                            .width(Size::px(SESSION_NOTE.button))
                            .height(Size::px(SESSION_NOTE.button))
                            .main_align(Alignment::Center)
                            .cross_align(Alignment::Center)
                            .a11y_role(AccessibilityRole::Button)
                            .a11y_alt(if collapsed {
                                tr!("session-note-expand")
                            } else {
                                tr!("session-note-collapse")
                            })
                            .on_mouse_down(|event: Event<MouseEventData>| event.stop_propagation())
                            .on_press(move |_| on_collapse.call(NoteChange::Collapsed(!collapsed)))
                            .child(symbol(
                                if collapsed {
                                    Symbol::ChevronRight
                                } else {
                                    Symbol::ChevronDown
                                },
                                SESSION_NOTE.chevron,
                                palette.ink_muted,
                            )),
                    ),
            )
            .maybe_child((!collapsed).then(|| {
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .padding(Gaps::new_all(SESSION_NOTE.inset))
                    .child(
                        TextView::new(
                            value,
                            self.family.clone(),
                            self.size,
                            self.line_height,
                            color(self.ink),
                        )
                        .height(Size::fill())
                        .auto_focus(true)
                        .on_change(move |text| on_change.call(NoteChange::Text(text)))
                        .on_focus_change(move |focused| on_focus.call(NoteChange::Focused(focused)))
                        .on_key(move |press: KeyPress| {
                            // SessionNoteView.swift SessionNoteTextView -keyDown: ⇧↩ collapses.
                            let shift = press.modifiers
                                & (Modifiers::SHIFT
                                    | Modifiers::ALT
                                    | Modifiers::META
                                    | Modifiers::CONTROL)
                                == Modifiers::SHIFT;
                            let collapse = shift && press.key == Key::Named(NamedKey::Enter);
                            if collapse {
                                on_key.call(NoteChange::Collapsed(true));
                            }
                            collapse
                        }),
                    )
            }))
            .children(edges.unwrap_or_default())
    }
}

// Freya paints the terminal canvas over element backgrounds, so the note draws its own, wider
// than itself by the room its shadow needs.
fn background(width: f32, height: f32, tint: Rgba, dark: bool) -> Element {
    let room = SESSION_NOTE.shadow.blur * 2.0 + SESSION_NOTE.shadow.offset_y;
    let mut look = DefaultHasher::new();
    [
        width,
        height,
        tint.r,
        tint.g,
        tint.b,
        f32::from(u8::from(dark)),
    ]
    .map(f32::to_bits)
    .hash(&mut look);
    rect()
        .position(Position::new_absolute().left(-room).top(-room))
        .width(Size::px(width + 2.0 * room))
        .height(Size::px(height + 2.0 * room))
        .child(canvas(look.finish(), move |ctx| {
            draw_material(ctx, room, tint, dark)
        }))
        .into_element()
}

// SessionNoteView.swift: a «sheet» material with radius 8, a black shadow at 0.3 and, in dark
// mode, a grey border.
fn draw_material(ctx: &mut CanvasContext, room: f32, tint: Rgba, dark: bool) {
    let bounds = SkRect::new(room, room, ctx.width - room, ctx.height - room);
    let shape = RRect::new_rect_xy(bounds, SESSION_NOTE.radius, SESSION_NOTE.radius);
    let shadow = SESSION_NOTE.shadow;
    let mut shade = rgba_paint(shadow.color);
    shade.set_mask_filter(MaskFilter::blur(
        BlurStyle::Normal,
        shadow.blur / 2.0,
        false,
    ));
    ctx.canvas.save();
    ctx.canvas.translate((0.0, shadow.offset_y));
    ctx.canvas.draw_rrect(shape, &shade);
    ctx.canvas.restore();
    // SessionNoteView.swift: a .sheet NSVisualEffectView, blurring what lies behind it.
    ctx.canvas.save();
    ctx.canvas.clip_rrect(shape, None, Some(true));
    if let Some(filter) = blur(
        (SHEET_MATERIAL_BLUR, SHEET_MATERIAL_BLUR),
        TileMode::Clamp,
        None,
        None,
    ) {
        ctx.canvas
            .save_layer(&SaveLayerRec::default().bounds(&bounds).backdrop(&filter));
        ctx.canvas.restore();
    }
    ctx.canvas.draw_rrect(shape, &rgba_paint(tint));
    ctx.canvas.restore();
    if dark {
        let mut border = rgba_paint(SESSION_NOTE.dark_border);
        border.set_style(PaintStyle::Stroke);
        border.set_stroke_width(1.0);
        let inset = RRect::new_rect_xy(
            bounds.with_inset((0.5, 0.5)),
            SESSION_NOTE.radius,
            SESSION_NOTE.radius,
        );
        ctx.canvas.draw_rrect(inset, &border);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_resize_down_to_the_minimum_and_the_title_bar_moves() {
        let start = Frame {
            x: 100.0,
            y: 20.0,
            width: 280.0,
            height: 180.0,
        };
        let drag = |left, right, top, bottom| Drag {
            from: (0.0, 0.0),
            start,
            left,
            right,
            top,
            bottom,
        };
        let moved = drag(false, false, false, false).apply((-30.0, 15.0));
        assert_eq!((moved.x, moved.y, moved.width), (70.0, 35.0, 280.0));
        let narrowed = drag(true, false, false, false).apply((500.0, 0.0));
        assert_eq!(narrowed.width, SESSION_NOTE.min_width);
        assert_eq!(narrowed.x + narrowed.width, start.x + start.width);
        let taller = drag(false, false, false, true).apply((0.0, 40.0));
        assert_eq!(taller.height, 220.0);
        let placed = placed(None, (800.0, 500.0));
        assert_eq!((placed.x, placed.y), (500.0, SESSION_NOTE.margin));
    }
}
