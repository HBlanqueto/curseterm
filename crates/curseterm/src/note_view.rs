use std::hash::{DefaultHasher, Hash, Hasher};

use barrier_kit::theme::color;
use barrier_kit_next::canvas::{CanvasContext, canvas};
use barrier_kit_next::tokens::{ANNOTATION, AnnotationColors};
use barrier_kit_next::views::text_view::TextView;
use freya_core::prelude::*;
use freya_engine::prelude::{
    BlurStyle, MaskFilter, PaintStyle, Path, PathBuilder, Point as SkPoint, Rect as SkRect,
};
use torin::prelude::{Gaps, Position, Size};
use vow_design::{Rgba, SYMBOL};

use crate::grid::rgba_paint;
use crate::localize::tr;

const TRASH: &str = "M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.7 9h5.6l.7-9M7 7v4M9 7v4";

pub struct NoteView {
    pub text: String,
    pub anchor: (f32, f32),
    pub bounds: (f32, f32),
    pub dark: bool,
    pub opacity: f32,
    pub highlight: f32,
    pub focus: bool,
    pub on_event: Callback<NoteEvent, ()>,
    pub key: DiffKey,
}

impl KeyExt for NoteView {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

pub enum NoteEvent {
    Changed(String),
    Focused(bool),
    Remove,
}

impl PartialEq for NoteView {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.anchor == other.anchor
            && self.bounds == other.bounds
            && self.dark == other.dark
            && self.opacity == other.opacity
            && self.highlight == other.highlight
            && self.focus == other.focus
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Frame {
    width: f32,
    height: f32,
    tip_x: f32,
    tip_on_top: bool,
}

impl Component for NoteView {
    fn render(&self) -> impl IntoElement {
        let value = use_state(|| self.text.clone());
        let mut height = use_state(|| ANNOTATION.min_height);
        let colors = if self.dark {
            ANNOTATION.dark
        } else {
            ANNOTATION.light
        };
        let margins = ANNOTATION.margins;
        let width = ANNOTATION.text_width + margins.left + margins.right + ANNOTATION.inset;
        let frame = place(self.anchor, self.bounds, width, *height.read());
        let top = if frame.tip_on_top {
            self.anchor.1
        } else {
            self.anchor.1 - frame.height
        };
        let below_tip = if frame.tip_on_top {
            ANNOTATION.pointer_length
        } else {
            0.0
        };
        let above_tip = ANNOTATION.pointer_length - below_tip;
        let bubble = bubble_frame(&frame);
        let button = SkRect::from_xywh(
            bubble.right.ceil() - ANNOTATION.button,
            bubble.top.floor()
                + (ANNOTATION.min_height
                    - ANNOTATION.pointer_length
                    - ANNOTATION.inset
                    - ANNOTATION.button)
                    / 2.0,
            ANNOTATION.button,
            ANNOTATION.button,
        );
        let background = colors.background.mix(ANNOTATION.highlight, self.highlight);
        let mut look = DefaultHasher::new();
        (
            [frame.width, frame.height, frame.tip_x, self.highlight].map(f32::to_bits),
            frame.tip_on_top,
            self.dark,
        )
            .hash(&mut look);
        let painter = canvas(look.finish(), move |ctx| {
            draw(ctx, &frame, colors, background, button)
        });
        let on_event = self.on_event.clone();
        let (on_change, on_focus_change) = (on_event.clone(), on_event.clone());
        rect()
            .position(
                Position::new_absolute()
                    .left(self.anchor.0 - frame.tip_x)
                    .top(top),
            )
            .width(Size::px(width))
            .height(Size::px(frame.height))
            .opacity(self.opacity)
            .on_mouse_down(|event: Event<MouseEventData>| event.stop_propagation())
            .child(
                rect()
                    .position(Position::new_absolute().left(0.0).top(0.0))
                    .width(Size::px(width))
                    .height(Size::px(frame.height))
                    .child(painter),
            )
            .child(
                rect()
                    .on_sized(move |event: Event<SizedEventData>| {
                        height.set_if_modified(event.area.height().max(ANNOTATION.min_height));
                    })
                    .padding(Gaps::new(
                        below_tip.floor() + margins.top,
                        ANNOTATION.inset + margins.right,
                        ANNOTATION.inset + margins.bottom + ANNOTATION.bottom_padding + above_tip,
                        margins.left + ANNOTATION.line_padding,
                    ))
                    .child(
                        TextView::new(value, ANNOTATION.font, color(colors.ink))
                            .width(Size::px(
                                ANNOTATION.text_width - 2.0 * ANNOTATION.line_padding,
                            ))
                            .auto_focus(self.focus)
                            .on_change(move |text| on_change.call(NoteEvent::Changed(text)))
                            .on_focus_change(move |focused| {
                                on_focus_change.call(NoteEvent::Focused(focused))
                            }),
                    ),
            )
            .child(
                rect()
                    .position(Position::new_absolute().left(button.left).top(button.top))
                    .width(Size::px(button.width()))
                    .height(Size::px(button.height()))
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(tr!("delete-annotation"))
                    .on_pointer_enter(|_| Cursor::set(CursorIcon::default()))
                    .on_press(move |_| on_event.call(NoteEvent::Remove)),
            )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

// PTYNoteViewController.m -setAnchor: the tip sits pointer_inset from the left edge when it fits,
// and the bubble flips above the anchor when it would leave the view.
fn place(anchor: (f32, f32), bounds: (f32, f32), width: f32, height: f32) -> Frame {
    let ideal = anchor.0 - ANNOTATION.pointer_inset;
    let tip_x = if ideal < 0.0 {
        anchor.0
    } else if ideal + width > bounds.0 {
        anchor.0 - (bounds.0 - width)
    } else {
        ANNOTATION.pointer_inset
    };
    Frame {
        width,
        height,
        tip_x: tip_x.clamp(0.0, width),
        tip_on_top: anchor.1 + height <= bounds.1,
    }
}

// PTYNoteView.m -bubbleFrameInRect:inset:pointerLength:tipOnEdge:
fn bubble_frame(frame: &Frame) -> SkRect {
    let mut bubble = SkRect::new(
        0.5,
        0.5,
        frame.width - ANNOTATION.inset - 0.5,
        frame.height - ANNOTATION.inset - 0.5,
    );
    if frame.tip_on_top {
        bubble.top += ANNOTATION.pointer_length;
    } else {
        bubble.bottom -= ANNOTATION.pointer_length;
    }
    bubble
}

// PTYNoteView.m -roundedRectangleWithPointerInRect:: each corner is a cubic with both control
// points on the corner, and the pointer base slides away from the corners.
fn bubble_path(frame: &Frame) -> Path {
    let bubble = bubble_frame(frame);
    let radius = ANNOTATION.radius;
    let half = ANNOTATION.pointer_base / 2.0;
    let center = frame
        .tip_x
        .clamp(bubble.left + radius + half, bubble.right - radius - half);
    let mut path = PathBuilder::new();
    path.move_to((bubble.left, bubble.bottom - radius));
    path.line_to((bubble.left, bubble.top + radius));
    corner(
        &mut path,
        (bubble.left, bubble.top),
        (bubble.left + radius, bubble.top),
    );
    if frame.tip_on_top {
        path.line_to((center - half, bubble.top));
        path.line_to((frame.tip_x, 0.0));
        path.line_to((center + half, bubble.top));
    }
    path.line_to((bubble.right - radius, bubble.top));
    corner(
        &mut path,
        (bubble.right, bubble.top),
        (bubble.right, bubble.top + radius),
    );
    path.line_to((bubble.right, bubble.bottom - radius));
    corner(
        &mut path,
        (bubble.right, bubble.bottom),
        (bubble.right - radius, bubble.bottom),
    );
    if !frame.tip_on_top {
        path.line_to((center + half, bubble.bottom));
        path.line_to((frame.tip_x, frame.height - 1.0));
        path.line_to((center - half, bubble.bottom));
    }
    path.line_to((bubble.left + radius, bubble.bottom));
    corner(
        &mut path,
        (bubble.left, bubble.bottom),
        (bubble.left, bubble.bottom - radius),
    );
    path.close();
    path.detach()
}

fn corner(path: &mut PathBuilder, control: (f32, f32), end: (f32, f32)) {
    path.cubic_to(control, control, end);
}

fn draw(
    ctx: &mut CanvasContext,
    frame: &Frame,
    colors: AnnotationColors,
    background: Rgba,
    button: SkRect,
) {
    let path = bubble_path(frame);
    let shadow = ANNOTATION.shadow;
    let mut blur = rgba_paint(shadow.color);
    blur.set_mask_filter(MaskFilter::blur(
        BlurStyle::Normal,
        shadow.blur / 2.0,
        false,
    ));
    ctx.canvas.save();
    ctx.canvas.translate((0.0, shadow.offset_y));
    ctx.canvas.draw_path(&path, &blur);
    ctx.canvas.restore();
    ctx.canvas.draw_path(&path, &rgba_paint(background));
    let mut border = rgba_paint(colors.border);
    border.set_style(PaintStyle::Stroke);
    border.set_stroke_width(1.0);
    ctx.canvas.draw_path(&path, &border);
    draw_trash(ctx, button, colors.ink);
}

// BK-024: the kit has no trash symbol, so it is drawn with the kit's symbol grid and stroke.
fn draw_trash(ctx: &mut CanvasContext, area: SkRect, ink: Rgba) {
    let Some(path) = Path::from_svg(TRASH) else {
        return;
    };
    let scale = area.width() / SYMBOL.grid;
    let mut stroke = rgba_paint(ink);
    stroke.set_style(PaintStyle::Stroke);
    stroke.set_stroke_width(SYMBOL.stroke);
    ctx.canvas.save();
    ctx.canvas.translate(SkPoint::new(area.left, area.top));
    ctx.canvas.scale((scale, scale));
    ctx.canvas.draw_path(&path, &stroke);
    ctx.canvas.restore();
}
