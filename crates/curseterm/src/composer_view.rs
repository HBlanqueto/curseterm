use std::hash::{DefaultHasher, Hash, Hasher};

use barrier_kit::controls::ScrollView;
use barrier_kit::style::text;
use barrier_kit::symbol::{Symbol, symbol};
use barrier_kit::theme::color;
use barrier_kit_next::canvas::{CanvasContext, canvas};
use barrier_kit_next::tokens::COMPOSER;
use barrier_kit_next::views::text_view::{KeyPress, TextView};
use freya_core::prelude::*;
use freya_engine::prelude::{PaintStyle, RRect, Rect as SkRect, SaveLayerRec, TileMode, blur};
use torin::prelude::{Alignment, Content, Direction, Gaps, Position, Size};
use vow_design::{BG_WINDOW_DARK, BG_WINDOW_LIGHT, CAPTION, DARK, LIGHT, Rgba};

use crate::grid::rgba_paint;
use crate::localize::tr;

pub struct ComposerView {
    pub text: String,
    pub bounds: (f32, f32),
    pub family: String,
    pub size: f32,
    pub line_height: f32,
    pub ink: Rgba,
    pub cursor: Rgba,
    pub opacity: f32,
    pub dark: bool,
    pub on_event: Callback<ComposerEvent, ()>,
}

impl PartialEq for ComposerView {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.bounds == other.bounds
            && self.family == other.family
            && self.size == other.size
            && self.line_height == other.line_height
            && self.ink == other.ink
            && self.cursor == other.cursor
            && self.opacity == other.opacity
            && self.dark == other.dark
    }
}

pub enum ComposerEvent {
    Changed(String),
    Focused(bool),
    Close,
    SendAll,
    SendCommandAt(usize),
}

impl Component for ComposerView {
    fn render(&self) -> impl IntoElement {
        let mut value = use_state(|| self.text.clone());
        let mut seen = use_state(|| self.text.clone());
        if *seen.peek() != self.text {
            seen.set(self.text.clone());
            value.set(self.text.clone());
        }
        // PTYSession.m -composerManagerAppearance: the appearance follows the terminal background.
        let (palette, window) = if self.dark {
            (DARK, BG_WINDOW_DARK)
        } else {
            (LIGHT, BG_WINDOW_LIGHT)
        };
        let tint = Rgba {
            a: COMPOSER.tint_alpha,
            ..window
        };
        // PTYSession.m -composerManager:frameForDesiredHeight:…: pinned to the top, 20 points
        // from each side, never narrower than 217 nor taller than the pane minus 8.
        let width = (self.bounds.0 - 2.0 * COMPOSER.side_margin).max(COMPOSER.min_width);
        let height = COMPOSER
            .height
            .min(self.bounds.1 - COMPOSER.bottom_room)
            .max(COMPOSER.min_height);
        let inner = width - 2.0 * COMPOSER.inset;
        let on_event = self.on_event.clone();
        let (on_change, on_focus, on_key, on_close) = (
            on_event.clone(),
            on_event.clone(),
            on_event.clone(),
            on_event.clone(),
        );
        let mut look = DefaultHasher::new();
        [width, height, tint.r, tint.g, tint.b]
            .map(f32::to_bits)
            .hash(&mut look);
        let background = canvas(look.finish(), move |ctx| draw_material(ctx, tint));
        rect()
            .position(Position::new_absolute().left(COMPOSER.side_margin).top(0.0))
            .width(Size::px(width))
            .height(Size::px(height))
            .opacity(self.opacity)
            .padding(Gaps::new_all(COMPOSER.inset))
            .on_mouse_down(|event: Event<MouseEventData>| event.stop_propagation())
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(
                        rect()
                            .position(Position::new_absolute().left(0.0).top(0.0))
                            .width(Size::px(inner))
                            .height(Size::px(height - 2.0 * COMPOSER.inset))
                            .child(background),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::fill())
                            .content(Content::flex())
                            .padding(Gaps::new(
                                COMPOSER.text_inset,
                                COMPOSER.text_inset,
                                0.0,
                                COMPOSER.text_inset,
                            ))
                            .child(
                                // Freya gives the pointer to the scroll view under the close button,
                                // so the text stops short of it.
                                ScrollView::new()
                                    .width(Size::func(|context| {
                                        Some(
                                            context.parent
                                                - (COMPOSER.close_frame.0 + COMPOSER.close_inset.0
                                                    - COMPOSER.inset
                                                    - COMPOSER.text_inset),
                                        )
                                    }))
                                    .height(Size::flex(1.0))
                                    .child(
                                        TextView::new(
                                            value,
                                            self.family.clone(),
                                            self.size,
                                            self.line_height,
                                            color(self.ink),
                                        )
                                        .cursor(color(self.cursor))
                                        .auto_focus(true)
                                        .on_change(move |text| {
                                            on_change.call(ComposerEvent::Changed(text))
                                        })
                                        .on_focus_change(move |focused| {
                                            on_focus.call(ComposerEvent::Focused(focused))
                                        })
                                        .on_key(
                                            move |press: KeyPress| match composer_key(&press) {
                                                Some(event) => {
                                                    on_key.call(event);
                                                    true
                                                }
                                                None => false,
                                            },
                                        ),
                                    ),
                            )
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .height(Size::px(COMPOSER.accessory_height))
                                    .direction(Direction::Horizontal)
                                    .main_align(Alignment::End)
                                    .cross_align(Alignment::Center)
                                    .maybe_child((inner >= COMPOSER.tip_min_width).then(|| {
                                        text(
                                            tr!("composer-send-tip"),
                                            vow_design::Font {
                                                size: COMPOSER.tip_size as u32,
                                                ..CAPTION
                                            },
                                            palette.ink_muted,
                                        )
                                    })),
                            ),
                    ),
            )
            .child(
                rect()
                    .position(
                        Position::new_absolute()
                            .right(COMPOSER.close_inset.0)
                            .top(COMPOSER.close_inset.1),
                    )
                    .layer(1)
                    .width(Size::px(COMPOSER.close_frame.0))
                    .height(Size::px(COMPOSER.close_frame.1))
                    .main_align(Alignment::Center)
                    .cross_align(Alignment::Center)
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(tr!("composer-close"))
                    .on_press(move |_| on_close.call(ComposerEvent::Close))
                    .child(symbol(Symbol::XMark, COMPOSER.close, palette.ink_muted)),
            )
    }
}

// ComposerTextView.swift standardActions: ⇧↩ sends, ⌥⇧↩ sends the command at the cursor and
// Escape dismisses keeping the text.
fn composer_key(press: &KeyPress) -> Option<ComposerEvent> {
    let held = press.modifiers
        & (Modifiers::SHIFT | Modifiers::ALT | Modifiers::META | Modifiers::CONTROL);
    match &press.key {
        Key::Named(NamedKey::Escape) => Some(ComposerEvent::Close),
        Key::Named(NamedKey::Enter) if held == Modifiers::SHIFT => Some(ComposerEvent::SendAll),
        Key::Named(NamedKey::Enter) if held == Modifiers::SHIFT | Modifiers::ALT => {
            Some(ComposerEvent::SendCommandAt(press.cursor_utf16))
        }
        _ => None,
    }
}

// iTermMinimalComposerViewController.xib: a «sheet» NSVisualEffectView with radius 6 and a
// one-point grey border.
fn draw_material(ctx: &mut CanvasContext, tint: Rgba) {
    let bounds = SkRect::new(0.0, 0.0, ctx.width, ctx.height);
    let shape = RRect::new_rect_xy(bounds, COMPOSER.radius, COMPOSER.radius);
    ctx.canvas.save();
    ctx.canvas.clip_rrect(shape, None, Some(true));
    if let Some(filter) = blur((COMPOSER.blur, COMPOSER.blur), TileMode::Clamp, None, None) {
        ctx.canvas
            .save_layer(&SaveLayerRec::default().bounds(&bounds).backdrop(&filter));
        ctx.canvas.restore();
    }
    ctx.canvas.draw_rrect(shape, &rgba_paint(tint));
    ctx.canvas.restore();
    let mut border = rgba_paint(COMPOSER.border);
    border.set_style(PaintStyle::Stroke);
    border.set_stroke_width(1.0);
    ctx.canvas.draw_rrect(
        RRect::new_rect_xy(
            bounds.with_inset((0.5, 0.5)),
            COMPOSER.radius,
            COMPOSER.radius,
        ),
        &border,
    );
}
