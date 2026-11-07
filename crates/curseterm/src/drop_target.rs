use crate::tokens::MOVE_PANE;
use barrier_kit::canvas::canvas;
use freya_core::prelude::*;
use freya_engine::prelude::{FontMgr, FontStyle, PaintStyle, Rect as SkRect};
use torin::prelude::{Position, Size};
use vow_design::HEADLINE;

use crate::grid::{font, rgba_paint, typeface};
use crate::localize::tr;
use crate::panes::Axis;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Half {
    Left,
    Right,
    Top,
    Bottom,
}

impl Half {
    // MovePaneController.m -dropInSession:half:atPoint:: the half nearest the pointer.
    pub fn at(x: f32, y: f32, width: f32, height: f32) -> Self {
        let distances = [
            (x, Self::Left),
            (width - x, Self::Right),
            (y, Self::Top),
            (height - y, Self::Bottom),
        ];
        distances
            .into_iter()
            .min_by(|left, right| {
                (left.0 / scale(left.1, width, height))
                    .total_cmp(&(right.0 / scale(right.1, width, height)))
            })
            .map_or(Self::Right, |(_, half)| half)
    }

    pub fn placement(self) -> (Axis, bool) {
        match self {
            Self::Left => (Axis::Row, true),
            Self::Right => (Axis::Row, false),
            Self::Top => (Axis::Column, true),
            Self::Bottom => (Axis::Column, false),
        }
    }
}

fn scale(half: Half, width: f32, height: f32) -> f32 {
    match half {
        Half::Left | Half::Right => width.max(1.0),
        Half::Top | Half::Bottom => height.max(1.0),
    }
}

#[derive(PartialEq)]
pub struct DropTarget {
    pub source: bool,
    pub on_drop: Callback<Option<Half>, ()>,
}

// SplitSelectionView.m: the source pane green with its message, and under the pointer the half
// the moved session would take, dark red, each with a white and a black frame, all at 0.9.
impl Component for DropTarget {
    fn render(&self) -> impl IntoElement {
        let mut size = use_state(|| (0.0f32, 0.0f32));
        let mut half = use_state(|| None::<Half>);
        let source = self.source;
        let on_drop = self.on_drop.clone();
        let shown = (*half.read()).filter(|_| !source);
        let (width, height) = *size.read();
        let area = match (source, shown) {
            (true, _) => Some((0.0, 0.0, width, height)),
            (false, Some(Half::Left)) => Some((0.0, 0.0, width / 2.0, height)),
            (false, Some(Half::Right)) => Some((width / 2.0, 0.0, width / 2.0, height)),
            (false, Some(Half::Top)) => Some((0.0, 0.0, width, height / 2.0)),
            (false, Some(Half::Bottom)) => Some((0.0, height / 2.0, width, height / 2.0)),
            (false, None) => None,
        };
        let fill = if source {
            MOVE_PANE.source_fill
        } else {
            MOVE_PANE.target_fill
        };
        let look = area.map_or(0, |(x, y, w, h)| {
            [x, y, w, h].iter().fold(u64::from(source), |hash, value| {
                hash.wrapping_mul(31)
                    .wrapping_add(u64::from(value.to_bits()))
            })
        });
        let message = if source {
            tr!("move-pane-source")
        } else {
            tr!("move-pane-target")
        };
        let face = use_hook(|| {
            font(
                typeface(&FontMgr::default(), HEADLINE.family, FontStyle::bold()),
                HEADLINE.size as f32,
            )
        });
        rect()
            .position(Position::new_absolute().left(0.0).top(0.0))
            .width(Size::percent(100.0))
            .height(Size::percent(100.0))
            .layer(2)
            .opacity(MOVE_PANE.opacity)
            .on_sized(move |event: Event<SizedEventData>| {
                size.set_if_modified((event.area.width(), event.area.height()));
            })
            .on_pointer_move(move |event: Event<PointerEventData>| {
                let at = event.element_location();
                let (width, height) = *size.peek();
                half.set_if_modified(Some(Half::at(at.x as f32, at.y as f32, width, height)));
            })
            .on_mouse_down(move |event: Event<MouseEventData>| {
                event.stop_propagation();
                on_drop.call(if source { None } else { *half.peek() });
            })
            .child(
                rect()
                    .position(Position::new_absolute().left(0.0).top(0.0))
                    .width(Size::percent(100.0))
                    .height(Size::percent(100.0))
                    .child(canvas(look, move |ctx| {
                        let Some((x, y, w, h)) = area else {
                            return;
                        };
                        let outer = SkRect::new(x, y, x + w, y + h);
                        ctx.canvas.draw_rect(outer, &rgba_paint(fill));
                        for (inset, ink) in [(0.5, MOVE_PANE.border), (1.5, MOVE_PANE.inner_border)]
                        {
                            let mut frame = rgba_paint(ink);
                            frame.set_style(PaintStyle::Stroke);
                            frame.set_stroke_width(1.0);
                            ctx.canvas
                                .draw_rect(outer.with_inset((inset, inset)), &frame);
                        }
                        // Freya repaints the focused terminal's canvas over sibling text, so the
                        // message is drawn here.
                        let (text_width, _) = face.measure_str(message, None);
                        let (_, metrics) = face.metrics();
                        let baseline = y + (h - metrics.descent - metrics.ascent) / 2.0;
                        ctx.canvas.draw_str(
                            message,
                            (x + (w - text_width) / 2.0, baseline),
                            &face,
                            &rgba_paint(MOVE_PANE.message),
                        );
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_edge_relative_to_the_pane_picks_the_half() {
        assert_eq!(Half::at(10.0, 200.0, 800.0, 400.0), Half::Left);
        assert_eq!(Half::at(790.0, 200.0, 800.0, 400.0), Half::Right);
        assert_eq!(Half::at(400.0, 20.0, 800.0, 400.0), Half::Top);
        assert_eq!(Half::at(400.0, 390.0, 800.0, 400.0), Half::Bottom);
    }
}
