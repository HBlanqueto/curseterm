//! Tengen values that move to `tengen/components.json` under the same name.

use vow_design::{Font, Rgba};

pub const MONO: Font = Font {
    family: "Geist Mono",
    size: 13,
    line_height: 16,
    weight: 400,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabBarMetrics {
    pub height: f32,
    pub tab_max_width: f32,
    pub padding: f32,
    pub badge: f32,
    pub badge_gap: f32,
}

pub const TAB_BAR: TabBarMetrics = TabBarMetrics {
    height: 28.0,
    tab_max_width: 240.0,
    padding: 8.0,
    badge: 12.0,
    badge_gap: 4.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitPanesMetrics {
    pub divider: f32,
    pub pane_min: f32,
}

pub const SPLIT_PANES: SplitPanesMetrics = SplitPanesMetrics {
    divider: 3.0,
    pane_min: 60.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalMetrics {
    pub broadcast_alpha: f32,
    pub broadcast_stripe: f32,
    pub highlight_alpha: f32,
    pub bell_alpha: f32,
    pub spotlight_alpha: f32,
    pub spotlight_radius: f32,
    pub badge_scale: f32,
    pub badge_alpha: f32,
    pub scrollbar_width: f32,
    pub scrollbar_alpha: f32,
    pub multi_click_ms: u64,
    pub cursor_blink_ms: u64,
    pub cursor_guide_alpha: f32,
    pub font_sizes: (u32, u32),
}

pub const TERMINAL: TerminalMetrics = TerminalMetrics {
    broadcast_alpha: 0.12,
    broadcast_stripe: 6.0,
    highlight_alpha: 0.45,
    bell_alpha: 0.2,
    spotlight_alpha: 0.6,
    spotlight_radius: 48.0,
    badge_scale: 3.0,
    badge_alpha: 0.3,
    scrollbar_width: 4.0,
    scrollbar_alpha: 0.45,
    multi_click_ms: 400,
    cursor_blink_ms: 500,
    cursor_guide_alpha: 0.25,
    font_sizes: (6, 72),
};

pub const SETTINGS_WINDOW_SIZE: (u32, u32) = (980, 600);
pub const TOOLBELT_WIDTH: f32 = 280.0;

pub const PASTE_HISTORY_SIZE: (u32, u32) = (480, 320);
pub const RENAME_WIDTH: f32 = 360.0;
pub const ABOUT_WINDOW_SIZE: (u32, u32) = (320, 280);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnnotationMetrics {
    pub text_width: f32,
    pub min_height: f32,
    pub margins: Gaps4,
    pub inset: f32,
    pub radius: f32,
    pub pointer_base: f32,
    pub pointer_length: f32,
    pub pointer_inset: f32,
    pub button: f32,
    pub bottom_padding: f32,
    pub line_padding: f32,
    pub shadow: Shadow,
    pub underline: Rgba,
    pub dark: AnnotationColors,
    pub light: AnnotationColors,
    pub highlight: Rgba,
    pub highlight_ms: u64,
    pub fade_ms: u64,
    pub frame_ms: u64,
    pub font: Font,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaps4 {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub color: Rgba,
    pub offset_y: f32,
    pub blur: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnnotationColors {
    pub background: Rgba,
    pub border: Rgba,
    pub ink: Rgba,
}

pub const ANNOTATION: AnnotationMetrics = AnnotationMetrics {
    text_width: 250.0,
    min_height: 39.0,
    margins: Gaps4 {
        top: 6.0,
        right: 20.0,
        bottom: 2.0,
        left: 8.0,
    },
    inset: 5.0,
    radius: 5.0,
    pointer_base: 7.0,
    pointer_length: 7.0,
    pointer_inset: 40.0,
    button: 17.0,
    bottom_padding: 3.0,
    line_padding: 5.0,
    shadow: Shadow {
        color: Rgba::rgba8(0, 0, 0, 0.15),
        offset_y: 2.0,
        blur: 8.0,
    },
    underline: Rgba::rgba8(255, 255, 0, 1.0),
    dark: AnnotationColors {
        background: Rgba::rgba8(50, 50, 52, 1.0),
        border: Rgba::rgba8(255, 255, 255, 0.4),
        ink: Rgba::rgba8(255, 255, 255, 1.0),
    },
    light: AnnotationColors {
        background: Rgba::rgba8(250, 250, 250, 1.0),
        border: Rgba::rgba8(0, 0, 0, 0.1),
        ink: Rgba::rgba8(0, 0, 0, 1.0),
    },
    highlight: Rgba::rgba8(217, 235, 255, 1.0),
    highlight_ms: 750,
    fade_ms: 250,
    frame_ms: 33,
    font: Font {
        family: vow_design::TEXT.family,
        size: 12,
        line_height: 15,
        weight: 400,
    },
};
