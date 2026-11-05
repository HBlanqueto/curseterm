//! Tengen values these components need; each one moves to `tengen/tokens.json`
//! or `components.json` under the same name.

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
}

pub const TAB_BAR: TabBarMetrics = TabBarMetrics {
    height: 28.0,
    tab_max_width: 240.0,
    padding: 8.0,
};
