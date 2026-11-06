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
    pub badge: f32,
    pub badge_gap: f32,
}

pub const TAB_BAR: TabBarMetrics = TabBarMetrics {
    badge: 12.0,
    badge_gap: 4.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingPaneMetrics {
    pub title_height: f32,
    pub outline: f32,
    pub resize_band: f32,
    pub shadow_opacity: f32,
    pub shadow_blur: f32,
    pub shadow_y: f32,
    pub initial_fraction: f32,
    pub min_cells: f32,
}

// iTermFloatingPaneView.swift outlineWidth, resizeBandWidth and its layer shadow;
// SessionView.m iTermGetSessionViewTitleHeight (statusBarHeight 21 + 1); floating-panes-design.md
// §5: a new float takes 80 % of the tab, and none is smaller than 2×2 cells.
pub const FLOATING_PANE: FloatingPaneMetrics = FloatingPaneMetrics {
    title_height: 22.0,
    outline: 1.0,
    resize_band: 6.0,
    shadow_opacity: 0.4,
    shadow_blur: 8.0,
    shadow_y: 3.0,
    initial_fraction: 0.8,
    min_cells: 2.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabGroupMetrics {
    pub name_pad: f32,
    pub colored_area: f32,
    pub outline: f32,
    pub chevron: f32,
    pub colors: [Rgba; 7],
}

// PSMYosemiteTabStyle.m kPSMSquaredGroup… and PseudoTerminal.m -nextTabGroupColor: AppKit's
// system blue, green, orange, purple, red, #50afbc and pink in the light appearance.
pub const TAB_GROUP: TabGroupMetrics = TabGroupMetrics {
    name_pad: 10.0,
    colored_area: 6.0,
    outline: 1.0,
    chevron: 14.0,
    colors: [
        Rgba::rgb8(0, 122, 255),
        Rgba::rgb8(52, 199, 89),
        Rgba::rgb8(255, 149, 0),
        Rgba::rgb8(175, 82, 222),
        Rgba::rgb8(255, 59, 48),
        Rgba::rgb8(0x50, 0xaf, 0xbc),
        Rgba::rgb8(255, 45, 85),
    ],
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitPanesMetrics {
    pub divider: f32,
    pub pane_min: f32,
    pub light_divider: Rgba,
    pub dark_divider: Rgba,
    pub minimal_prominence: f32,
}

// PseudoTerminal.m -terminalWindowDecorationControlColor: AppKit's light and dark grey, and
// iTermAdvancedSettingsModel minimalSplitPaneDividerProminence.
pub const SPLIT_PANES: SplitPanesMetrics = SplitPanesMetrics {
    light_divider: Rgba::rgb8(170, 170, 170),
    dark_divider: Rgba::rgb8(85, 85, 85),
    minimal_prominence: 0.15,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComposerMetrics {
    pub side_margin: f32,
    pub height: f32,
    pub min_height: f32,
    pub min_width: f32,
    pub bottom_room: f32,
    pub inset: f32,
    pub radius: f32,
    pub border: Rgba,
    pub tint_alpha: f32,
    pub text_inset: f32,
    pub accessory_height: f32,
    pub close: f32,
    pub close_frame: (f32, f32),
    pub close_inset: (f32, f32),
    pub tip_size: f32,
    pub tip_min_width: f32,
    pub fade_ms: u64,
}

// NSVisualEffectView's .sheet material within the window, behind the Composer and Session Note.
pub const SHEET_MATERIAL_BLUR: f32 = 20.0;

pub const COMPOSER: ComposerMetrics = ComposerMetrics {
    side_margin: 20.0,
    height: 135.0,
    min_height: 62.0,
    min_width: 217.0,
    bottom_room: 8.0,
    inset: 9.0,
    radius: 6.0,
    border: Rgba::rgba8(128, 128, 128, 1.0),
    tint_alpha: 0.85,
    text_inset: 5.0,
    accessory_height: 19.0,
    close: 16.0,
    close_frame: (24.0, 23.0),
    close_inset: (8.0, 9.0),
    tip_size: 11.0,
    tip_min_width: 180.0,
    fade_ms: 125,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShortcutLabelMetrics {
    pub height: f32,
    pub padding_x: f32,
    pub gap: f32,
    pub font_size: f32,
    pub stroke: Rgba,
    pub shadow: Shadow,
    pub typed_alpha: f32,
    pub enter_ms: u64,
    pub exit_ms: u64,
    pub enter_scales: [(f32, f32); 3],
    pub pop_scale: f32,
    pub dissolve_scale: f32,
}

pub const SHORTCUT_LABEL: ShortcutLabelMetrics = ShortcutLabelMetrics {
    height: 16.0,
    padding_x: 2.0,
    gap: 2.0,
    font_size: 11.0,
    stroke: Rgba::rgba8(255, 255, 255, 0.95),
    shadow: Shadow {
        color: Rgba::rgba8(0, 0, 0, 0.4),
        offset_y: 0.0,
        blur: 2.0,
    },
    typed_alpha: 0.5,
    enter_ms: 200,
    exit_ms: 200,
    enter_scales: [(0.0, 0.25), (0.3, 1.3), (1.0, 1.0)],
    pop_scale: 2.0,
    dissolve_scale: 0.1,
};

// iTermIndicatorsHelper.m: 26-point squares with a sixth-of-a-side radius, the symbol 4 points
// smaller, 4 apart, at least 5 from the top, drawn at half opacity.
pub const INDICATOR: IndicatorMetrics = IndicatorMetrics {
    size: 26.0,
    symbol_inset: 4.0,
    gap: 4.0,
    top: 5.0,
    alpha: 0.5,
    dark: IndicatorColors {
        fill: Rgba::rgba8(0, 0, 0, 1.0),
        stroke: Rgba::rgba8(170, 170, 170, 1.0),
        ink: Rgba::rgba8(255, 255, 255, 1.0),
    },
    light: IndicatorColors {
        fill: Rgba::rgba8(255, 255, 255, 1.0),
        stroke: Rgba::rgba8(85, 85, 85, 1.0),
        ink: Rgba::rgba8(0, 0, 0, 1.0),
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IndicatorMetrics {
    pub size: f32,
    pub symbol_inset: f32,
    pub gap: f32,
    pub top: f32,
    pub alpha: f32,
    pub dark: IndicatorColors,
    pub light: IndicatorColors,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IndicatorColors {
    pub fill: Rgba,
    pub stroke: Rgba,
    pub ink: Rgba,
}

// PSMTabBarControl's classic styles and iTerm2's compact window (PSMYosemiteTabStyle.m,
// PSMTabBarControl.m, iTermRootTerminalView.m, iTermAdvancedSettingsModel.m).
pub const TAB_STRIP: TabStripMetrics = TabStripMetrics {
    min_tab_width: 75.0,
    add_width: 24.0,
    close: 16.0,
    close_x: 6.0,
    label_inset: 26.0,
    compact_inset: 75.0,
    compact_lights_x: 6.0,
    minimal_height: 38.0,
    title_font_size: 13.0,
    title_alpha: 0.65,
    inactive_close_alpha: 0.5,
    shadow: [
        Rgba::rgba8(0, 0, 0, 70.0 / 255.0),
        Rgba::rgba8(0, 0, 0, 47.0 / 255.0),
        Rgba::rgba8(0, 0, 0, 32.0 / 255.0),
        Rgba::rgba8(0, 0, 0, 16.0 / 255.0),
    ],
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabStripMetrics {
    pub min_tab_width: f32,
    pub add_width: f32,
    pub close: f32,
    pub close_x: f32,
    pub label_inset: f32,
    pub compact_inset: f32,
    pub compact_lights_x: f32,
    pub minimal_height: f32,
    pub title_font_size: f32,
    pub title_alpha: f32,
    pub inactive_close_alpha: f32,
    pub shadow: [Rgba; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabStyle {
    pub height: f32,
    pub font_size: f32,
    pub backing: Rgba,
    pub selected: Rgba,
    pub selected_hover: Rgba,
    pub unselected: Rgba,
    pub unselected_hover: Rgba,
    pub text_selected: Rgba,
    pub text_unselected: Rgba,
    pub text_selected_inactive: Rgba,
    pub text_unselected_inactive: Rgba,
    pub separator: Rgba,
    pub separator_inactive: Rgba,
    pub top_line: Rgba,
    pub bottom_line: Rgba,
    pub shadow: bool,
    pub outline: Rgba,
}

const fn white(level: f32, alpha: f32) -> Rgba {
    Rgba::rgba8(
        (level * 255.0) as u8,
        (level * 255.0) as u8,
        (level * 255.0) as u8,
        alpha,
    )
}

// A titled NSWindow on macOS 11 to 15: a 28-point title bar with the bold 13-point title, in
// NSColor.windowBackgroundColor (236 light, 50 dark), and iTermRootTerminalView.m's 1-point
// division view (HSB brightness 0.70 or 0.86 light, 0.10 or 0.07 dark) when no tab bar is below.
pub const TITLE_BAR: TitleBarMetrics = TitleBarMetrics {
    height: 28.0,
    font_size: 13.0,
    font_weight: 700,
    light: TitleBarColors {
        background: white(236.0 / 255.0, 1.0),
        title: white(0.0, 0.85),
        title_inactive: white(0.0, 0.25),
        division: white(0.70, 1.0),
        division_inactive: white(0.86, 1.0),
    },
    dark: TitleBarColors {
        background: white(50.0 / 255.0, 1.0),
        title: white(1.0, 0.85),
        title_inactive: white(1.0, 0.25),
        division: white(0.10, 1.0),
        division_inactive: white(0.07, 1.0),
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TitleBarMetrics {
    pub height: f32,
    pub font_size: f32,
    pub font_weight: u16,
    pub light: TitleBarColors,
    pub dark: TitleBarColors,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TitleBarColors {
    pub background: Rgba,
    pub title: Rgba,
    pub title_inactive: Rgba,
    pub division: Rgba,
    pub division_inactive: Rgba,
}

// PSMYosemiteTabStyle.m: the selected tab is clear over the title bar's background.
pub const TAB_STYLE_LIGHT: TabStyle = TabStyle {
    height: 24.0,
    font_size: 11.0,
    backing: TITLE_BAR.light.background,
    selected: Rgba::TRANSPARENT,
    selected_hover: Rgba::TRANSPARENT,
    unselected: white(0.0, 0.07),
    unselected_hover: white(0.0, 0.1165),
    text_selected: white(0.0, 1.0),
    text_unselected: white(0.0, 1.0),
    text_selected_inactive: white(189.0 / 255.0, 1.0),
    text_unselected_inactive: white(164.0 / 255.0, 1.0),
    separator: white(0.0, 0.07),
    separator_inactive: white(0.0, 0.07),
    top_line: Rgba::TRANSPARENT,
    bottom_line: white(230.0 / 255.0, 1.0),
    shadow: true,
    outline: Rgba::TRANSPARENT,
};

// PSMDarkTabStyle.m; the bottom row blends its black-to-0.75 gradient over the 0.1 line.
pub const TAB_STYLE_DARK: TabStyle = TabStyle {
    backing: TITLE_BAR.dark.background,
    unselected: white(0.0, 0.5),
    unselected_hover: white(0.0, 0.2),
    text_selected: white(1.0, 1.0),
    text_unselected: white(0.65, 1.0),
    text_selected_inactive: white(0.45, 1.0),
    text_unselected_inactive: white(0.37, 1.0),
    separator: white(1.0, 0.18),
    separator_inactive: white(1.0, 0.15),
    top_line: white(0.0, 1.0),
    bottom_line: white(0.0, 0.875),
    ..TAB_STYLE_LIGHT
};

// PSMLightHighContrastTabStyle.m
pub const TAB_STYLE_LIGHT_HIGH_CONTRAST: TabStyle = TabStyle {
    font_size: 12.0,
    selected: TITLE_BAR.light.background,
    selected_hover: TITLE_BAR.light.background,
    unselected: white(180.0 / 255.0, 1.0),
    unselected_hover: white(180.0 / 255.0 - 0.1, 1.0),
    text_selected_inactive: white(0.0, 1.0),
    text_unselected_inactive: white(0.0, 1.0),
    top_line: white(140.0 / 255.0, 1.0),
    bottom_line: white(140.0 / 255.0, 1.0),
    ..TAB_STYLE_LIGHT
};

// PSMDarkHighContrastTabStyle.m with highVisibility on, its default.
pub const TAB_STYLE_DARK_HIGH_CONTRAST: TabStyle = TabStyle {
    font_size: 12.0,
    selected: white(0.80, 1.0),
    selected_hover: white(0.85, 1.0),
    unselected: white(0.03, 1.0),
    unselected_hover: white(0.03, 1.0),
    text_selected: white(0.0, 1.0),
    text_unselected: white(1.0, 1.0),
    text_selected_inactive: white(0.0, 1.0),
    text_unselected_inactive: white(1.0, 1.0),
    ..TAB_STYLE_DARK
};

// PSMMinimalTabStyle.m and iTermAdvancedSettingsModel.m minimalTabStyle* defaults.
pub const MINIMAL_TABS: MinimalTabMetrics = MinimalTabMetrics {
    background_difference: 0.05,
    hover_difference: 0.1,
    outline_alpha: 0.2,
    text_dark: 0.95,
    text_light: 0.05,
    unselected_text_alpha: 0.5,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimalTabMetrics {
    pub background_difference: f32,
    pub hover_difference: f32,
    pub outline_alpha: f32,
    pub text_dark: f32,
    pub text_light: f32,
    pub unselected_text_alpha: f32,
}

// iTermPasteSpecialViewController.xib: four spaces per tab by default.
pub const ADVANCED_PASTE: AdvancedPasteMetrics = AdvancedPasteMetrics {
    width: 560.0,
    height: 560.0,
    preview_height: 120.0,
    preview_chars: 4000,
    spaces: 4,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdvancedPasteMetrics {
    pub width: f32,
    pub height: f32,
    pub preview_height: f32,
    pub preview_chars: usize,
    pub spaces: usize,
}

pub const PASTE_QUESTION_WIDTH: f32 = 480.0;

// SessionNoteView.swift and SessionView.m -ensureSessionNoteViewWithModel:: 280 × 180, 20 from
// the top-right corner, a 28-point title bar, radius 8 and a black shadow at 0.3.
pub const SESSION_NOTE: SessionNoteMetrics = SessionNoteMetrics {
    width: 280.0,
    height: 180.0,
    margin: 20.0,
    title_height: 28.0,
    min_width: 150.0,
    min_height: 80.0,
    drag_area: 5.0,
    radius: 8.0,
    tint_alpha: 0.85,
    shadow: Shadow {
        color: Rgba::rgba8(0, 0, 0, 0.3),
        offset_y: 2.0,
        blur: 6.0,
    },
    dark_border: Rgba::rgba8(128, 128, 128, 0.5),
    inset: 8.0,
    button: 20.0,
    button_inset: 8.0,
    chevron: 12.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionNoteMetrics {
    pub width: f32,
    pub height: f32,
    pub margin: f32,
    pub title_height: f32,
    pub min_width: f32,
    pub min_height: f32,
    pub drag_area: f32,
    pub radius: f32,
    pub tint_alpha: f32,
    pub shadow: Shadow,
    pub dark_border: Rgba,
    pub inset: f32,
    pub button: f32,
    pub button_inset: f32,
    pub chevron: f32,
}

// SplitSelectionView.m and iTermAdvancedSettingsModel.m splitPane*Color: moving a split pane.
pub const MOVE_PANE: MovePaneMetrics = MovePaneMetrics {
    opacity: 0.9,
    source_fill: Rgba::rgba8(0, 128, 0, 1.0),
    target_fill: Rgba::rgba8(128, 0, 0, 1.0),
    border: Rgba::rgba8(255, 255, 255, 1.0),
    inner_border: Rgba::rgba8(0, 0, 0, 1.0),
    message: Rgba::rgba8(255, 255, 255, 1.0),
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovePaneMetrics {
    pub opacity: f32,
    pub source_fill: Rgba,
    pub target_fill: Rgba,
    pub border: Rgba,
    pub inner_border: Rgba,
    pub message: Rgba,
}

// iTermEditKeyActionWindowController.xib: the Edit Key Action window is 442 points wide.
pub const KEY_BINDING_EDITOR: KeyBindingEditorMetrics = KeyBindingEditorMetrics {
    width: 442.0,
    table_height: 240.0,
    steps_height: 120.0,
    menu_matches: 12,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyBindingEditorMetrics {
    pub width: f32,
    pub table_height: f32,
    pub steps_height: f32,
    pub menu_matches: usize,
}

// iTermEditSnippetWindowController.xib: the Edit Snippet window and its Value box.
pub const SNIPPET_EDITOR: SnippetEditorMetrics = SnippetEditorMetrics {
    width: 637.0,
    value_height: 131.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnippetEditorMetrics {
    pub width: f32,
    pub value_height: f32,
}

// iTermGlobalSearch.xib and iTermGlobalSearchEngineCursor.m: the panel, the snippet's 20
// characters before the match and 256 after it, and the match's yellow at 0.35.
pub const GLOBAL_SEARCH: GlobalSearchMetrics = GlobalSearchMetrics {
    width: 741.0,
    height: 567.0,
    prefix_chars: 20,
    suffix_chars: 256,
    match_background: Rgba {
        r: 1.0,
        g: 1.0,
        b: 0.0,
        a: 0.35,
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalSearchMetrics {
    pub width: f32,
    pub height: f32,
    pub prefix_chars: usize,
    pub suffix_chars: usize,
    pub match_background: Rgba,
}

// HighlightTrigger.m colour menu: AppKit's named colours, in its order.
pub const HIGHLIGHT_COLORS: [Rgba; 14] = [
    Rgba::rgb8(0, 0, 0),
    Rgba::rgb8(85, 85, 85),
    Rgba::rgb8(170, 170, 170),
    Rgba::rgb8(255, 255, 255),
    Rgba::rgb8(128, 128, 128),
    Rgba::rgb8(255, 0, 0),
    Rgba::rgb8(0, 255, 0),
    Rgba::rgb8(0, 0, 255),
    Rgba::rgb8(0, 255, 255),
    Rgba::rgb8(255, 255, 0),
    Rgba::rgb8(255, 0, 255),
    Rgba::rgb8(255, 128, 0),
    Rgba::rgb8(128, 0, 128),
    Rgba::rgb8(153, 102, 51),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PasswordManagerMetrics {
    pub width: f32,
    pub table_height: f32,
    pub password_field: f32,
    pub generated_length: usize,
}

// iTermPasswordManager.xib: an 860 × 360 window around an 820 × 261 table;
// iTermPasswordManagerWindowController.m +randomPassword makes 16 characters.
pub const PASSWORD_MANAGER: PasswordManagerMetrics = PasswordManagerMetrics {
    width: 820.0,
    table_height: 261.0,
    password_field: 300.0,
    generated_length: 16,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriggerEditorMetrics {
    pub action_width: f32,
    pub parameter_width: f32,
}

// TriggerController.m's table: the Action and Parameters columns keep their width.
pub const TRIGGER_EDITOR: TriggerEditorMetrics = TriggerEditorMetrics {
    action_width: 200.0,
    parameter_width: 220.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrangementPreviewMetrics {
    pub height: f32,
    pub padding: f32,
    pub tab_height: f32,
    pub backdrop: Rgba,
    pub screen: Rgba,
    pub window: Rgba,
    pub frame: Rgba,
}

// ArrangementPreviewView.m: light gray behind, the screen in (0.270, 0.545, 0.811), black frames.
pub const ARRANGEMENT_PREVIEW: ArrangementPreviewMetrics = ArrangementPreviewMetrics {
    height: 220.0,
    padding: 16.0,
    tab_height: 18.0,
    backdrop: Rgba::rgb8(170, 170, 170),
    screen: Rgba::rgb8(69, 139, 207),
    window: Rgba::rgb8(255, 255, 255),
    frame: Rgba::rgb8(0, 0, 0),
};
