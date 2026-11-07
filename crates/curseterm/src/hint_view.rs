use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Instant;

use crate::tokens::SHORTCUT_LABEL;
use barrier_kit::canvas::canvas;
use barrier_kit::style::Palette;
use freya_core::prelude::*;
use freya_engine::prelude::{
    BlurStyle, Font, FontMgr, FontStyle, MaskFilter, PaintStyle, RRect, Rect as SkRect,
};
use torin::prelude::{Position, Size};
use vow_design::{Rgba, TEXT};

use crate::grid::{font, rgba_paint, typeface};

#[derive(PartialEq)]
pub struct HintLabel {
    pub key: String,
    pub typed: usize,
    pub target: (f32, f32, f32),
    pub motion: Motion,
    pub diff_key: DiffKey,
}

impl KeyExt for HintLabel {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.diff_key
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Motion {
    pub scale: f32,
    pub opacity: f32,
    pub rotation: f32,
}

impl Component for HintLabel {
    fn render(&self) -> impl IntoElement {
        let face = use_hook(|| {
            font(
                typeface(&FontMgr::default(), TEXT.family, FontStyle::normal()),
                SHORTCUT_LABEL.font_size,
            )
        });
        let (text_width, _) = face.measure_str(&self.key, None);
        let height = SHORTCUT_LABEL.height;
        let width = height.max(text_width + 2.0 * SHORTCUT_LABEL.padding_x);
        let (left, top, row_height) = self.target;
        // ContentNavigationShortcutLayerOuter -topLeft: left of the target, top-aligned when the
        // row is taller than the label and centred otherwise.
        let x = (left - width - SHORTCUT_LABEL.gap).max(SHORTCUT_LABEL.gap);
        let y = if row_height < height {
            top + row_height / 2.0 - height / 2.0
        } else {
            top
        };
        let accent = Palette::current().accent;
        let mut look = DefaultHasher::new();
        (
            &self.key,
            self.typed,
            [accent.r, accent.g, accent.b].map(f32::to_bits),
        )
            .hash(&mut look);
        let (key, typed) = (self.key.clone(), self.typed);
        rect()
            .position(Position::new_absolute().left(x).top(y))
            .layer(1)
            .width(Size::px(width))
            .height(Size::px(height))
            .scale(self.motion.scale)
            .rotate(self.motion.rotation)
            .opacity(self.motion.opacity)
            .child(canvas(look.finish(), move |ctx| {
                draw(ctx, &face, &key, typed, accent)
            }))
    }

    fn render_key(&self) -> DiffKey {
        self.diff_key.clone().or(self.default_key())
    }
}

// ContentNavigationShortcut.swift: an accent capsule with a white outline and a soft shadow; the
// typed part of the key at half white.
fn draw(
    ctx: &mut barrier_kit::canvas::CanvasContext,
    face: &Font,
    key: &str,
    typed: usize,
    accent: Rgba,
) {
    let radius = ctx.height / 2.0;
    let shape = RRect::new_rect_xy(
        SkRect::new(0.5, 0.5, ctx.width - 0.5, ctx.height - 0.5),
        radius,
        radius,
    );
    let shadow = SHORTCUT_LABEL.shadow;
    let mut blur = rgba_paint(shadow.color);
    blur.set_mask_filter(MaskFilter::blur(BlurStyle::Normal, shadow.blur, false));
    ctx.canvas.draw_rrect(shape, &blur);
    ctx.canvas.draw_rrect(shape, &rgba_paint(accent));
    let mut stroke = rgba_paint(SHORTCUT_LABEL.stroke);
    stroke.set_style(PaintStyle::Stroke);
    stroke.set_stroke_width(1.0);
    ctx.canvas.draw_rrect(shape, &stroke);
    let (width, _) = face.measure_str(key, None);
    let (_, metrics) = face.metrics();
    let baseline = (ctx.height - metrics.descent - metrics.ascent) / 2.0;
    let mut x = (ctx.width - width) / 2.0;
    let (done, rest) = key.split_at(typed.min(key.len()));
    for (part, alpha) in [(done, SHORTCUT_LABEL.typed_alpha), (rest, 1.0)] {
        ctx.canvas.draw_str(
            part,
            (x, baseline),
            face,
            &rgba_paint(Rgba::rgba8(255, 255, 255, alpha)),
        );
        x += face.measure_str(part, None).0;
    }
}

fn progress(since: Instant, duration_ms: u64) -> f32 {
    (since.elapsed().as_secs_f32() * 1000.0 / duration_ms as f32).min(1.0)
}

// ContentNavigationShortcut.swift -animateIn: keyframes 0.25, 1.3 and 1 at 0, 0.3 and 1.
pub fn entering(shown: Instant) -> Motion {
    let t = progress(shown, SHORTCUT_LABEL.enter_ms);
    let [first, second, last] = SHORTCUT_LABEL.enter_scales;
    let (from, to) = if t < second.0 {
        (first, second)
    } else {
        (second, last)
    };
    let span = (t - from.0) / (to.0 - from.0);
    Motion {
        scale: from.1 + (to.1 - from.1) * span,
        opacity: 1.0,
        rotation: 0.0,
    }
}

// ContentNavigationShortcut.swift -pop: grows to twice, fades and turns half a circle.
pub fn popping(since: Instant) -> Motion {
    let t = progress(since, SHORTCUT_LABEL.exit_ms);
    Motion {
        scale: 1.0 + (SHORTCUT_LABEL.pop_scale - 1.0) * t,
        opacity: 1.0 - t,
        rotation: 180.0 * t,
    }
}

// ContentNavigationShortcut.swift -dissolve: shrinks to a tenth and fades.
pub fn dissolving(since: Instant) -> Motion {
    let t = progress(since, SHORTCUT_LABEL.exit_ms);
    Motion {
        scale: 1.0 + (SHORTCUT_LABEL.dissolve_scale - 1.0) * t,
        opacity: 1.0 - t,
        rotation: 0.0,
    }
}

pub fn settled(since: Instant, duration_ms: u64) -> bool {
    progress(since, duration_ms) >= 1.0
}
