//! Tengen values that move to `tengen/components.json` under the same name.

use vow_design::Font;

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
    font_sizes: (6, 72),
};

pub const SETTINGS_WINDOW_SIZE: (u32, u32) = (980, 600);
pub const PASTE_HISTORY_SIZE: (u32, u32) = (480, 320);
pub const RENAME_WIDTH: f32 = 360.0;
pub const ABOUT_WINDOW_SIZE: (u32, u32) = (320, 280);
