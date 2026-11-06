use barrier_kit_next::canvas::CanvasContext;
use barrier_kit_next::tokens::INDICATOR;
use freya_engine::prelude::{PaintStyle, Path, RRect, Rect as SkRect};
use vow_design::SYMBOL;

use crate::grid::rgba_paint;

// iTermIndicatorsHelper.m +sequentialIndicatorIdentifiers: the first one sits rightmost.
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub enum Indicator {
    Maximized,
    Broadcast,
    Coprocess,
    Alert,
    CopyMode,
}

impl Indicator {
    // BK-024: the SF Symbols of iTerm2's map, drawn on the kit's symbol grid.
    fn path(self) -> &'static str {
        match self {
            Self::Maximized => "M3 3l4 4M3 3h3.5M3 3v3.5M13 13L9 9M13 13H9.5M13 13V9.5",
            Self::Broadcast => {
                "M8.8 8a.8.8 0 1 1-1.6 0a.8.8 0 1 1 1.6 0M5.6 5.4a3.6 3.6 0 0 0 0 5.2M10.4 5.4a3.6 3.6 0 0 1 0 5.2M3.4 3.2a6.6 6.6 0 0 0 0 9.6M12.6 3.2a6.6 6.6 0 0 1 0 9.6"
            }
            Self::Coprocess => {
                "M2.5 3h6v4.5h-6zM7.5 8.5h6V13h-6zM10.5 3.5h1a2 2 0 0 1 2 2v1M5.5 12.5h-1a2 2 0 0 1-2-2v-1"
            }
            Self::Alert => {
                "M1.5 8C3 5.2 5.3 3.8 8 3.8s5 1.4 6.5 4.2c-1.5 2.8-3.8 4.2-6.5 4.2S3 10.8 1.5 8M10 8a2 2 0 1 1-4 0a2 2 0 1 1 4 0"
            }
            Self::CopyMode => "M5.5 4V2.5h7.5v9h-1.5M3 4.5h7.5v9H3z",
        }
    }
}

// iTermIndicatorsHelper.m -iterateAllTopRightIndicatorsInFrame:block: and
// +imageWithSFSymbol:large:size:darkBackground:backgroundless:
pub fn draw(ctx: &mut CanvasContext, shown: &[Indicator], dark: bool) {
    let colors = if dark {
        INDICATOR.dark
    } else {
        INDICATOR.light
    };
    let size = INDICATOR.size;
    let mut right = ctx.width;
    for indicator in shown {
        right -= size + INDICATOR.gap;
        let frame = SkRect::from_xywh(right, INDICATOR.top, size, size);
        let shape = RRect::new_rect_xy(frame.with_inset((0.5, 0.5)), size / 6.0, size / 6.0);
        let mut fill = rgba_paint(colors.fill);
        fill.set_alpha_f(INDICATOR.alpha);
        ctx.canvas.draw_rrect(shape, &fill);
        let mut stroke = rgba_paint(colors.stroke);
        stroke.set_alpha_f(INDICATOR.alpha);
        stroke.set_style(PaintStyle::Stroke);
        stroke.set_stroke_width(1.0);
        ctx.canvas.draw_rrect(shape, &stroke);
        let Some(path) = Path::from_svg(indicator.path()) else {
            continue;
        };
        let box_size = size - INDICATOR.symbol_inset;
        let scale = box_size / SYMBOL.grid;
        let mut ink = rgba_paint(colors.ink);
        ink.set_alpha_f(INDICATOR.alpha);
        ink.set_style(PaintStyle::Stroke);
        ink.set_stroke_width(SYMBOL.stroke);
        ctx.canvas.save();
        ctx.canvas.translate((
            frame.left + INDICATOR.symbol_inset / 2.0,
            frame.top + INDICATOR.symbol_inset / 2.0,
        ));
        ctx.canvas.scale((scale, scale));
        ctx.canvas.draw_path(&path, &ink);
        ctx.canvas.restore();
    }
}
