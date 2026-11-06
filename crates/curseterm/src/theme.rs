use alacritty_terminal::vte::ansi::Rgb;
use barrier_kit_next::tokens::{
    MINIMAL_TABS, TAB_STRIP, TAB_STYLE_DARK, TAB_STYLE_DARK_HIGH_CONTRAST, TAB_STYLE_LIGHT,
    TAB_STYLE_LIGHT_HIGH_CONTRAST, TITLE_BAR, TabStyle,
};
use barrier_kit_next::views::tab_bar::brightness;
use vow_design::Rgba;

use crate::config::Theme;

// ITAddressBookMgr.m: Compact and Minimal turn windows into compact ones.
pub fn compact(theme: Theme) -> bool {
    matches!(theme, Theme::Compact | Theme::Minimal)
}

// iTermTheme.m -tabStyleWithDelegate:effectiveAppearance: Regular and Compact follow the system
// appearance; a compact window's top tabs are not attached to a title bar, so they lose the top
// line (PseudoTerminal.m -updateTabBarStyle).
pub fn tab_style(
    theme: Theme,
    system_dark: bool,
    background: Rgb,
    compact_window: bool,
) -> TabStyle {
    let style = match theme {
        Theme::Regular | Theme::Compact if system_dark => TAB_STYLE_DARK,
        Theme::Regular | Theme::Compact => TAB_STYLE_LIGHT,
        Theme::Light => TAB_STYLE_LIGHT,
        Theme::Dark => TAB_STYLE_DARK,
        Theme::LightHighContrast => TAB_STYLE_LIGHT_HIGH_CONTRAST,
        Theme::DarkHighContrast => TAB_STYLE_DARK_HIGH_CONTRAST,
        Theme::Minimal => return minimal(background, compact_window),
    };
    if theme == Theme::Compact && compact_window {
        return TabStyle {
            top_line: Rgba::TRANSPARENT,
            ..style
        };
    }
    style
}

// PSMMinimalTabStyle.m, for an opaque terminal: the selected tab is the terminal background and
// the rest is 0.05 darker (lighter on a nearly black one).
fn minimal(background: Rgb, tall: bool) -> TabStyle {
    let bar = Rgba::rgba8(background.r, background.g, background.b, 1.0);
    let lightness = brightness(bar);
    let difference = MINIMAL_TABS.background_difference;
    let unselected = shift(
        bar,
        if lightness - difference < 0.0 {
            difference
        } else {
            -difference
        },
    );
    let hover = MINIMAL_TABS.hover_difference;
    let unselected_hover = shift(
        unselected,
        if brightness(unselected) + hover > 1.0 {
            -hover
        } else {
            hover
        },
    );
    let level = if perceived_dark(background) {
        MINIMAL_TABS.text_dark
    } else {
        MINIMAL_TABS.text_light
    };
    let text = gray(level, 1.0);
    let dimmed = gray(level, MINIMAL_TABS.unselected_text_alpha);
    let outline = gray(
        if lightness < 0.5 { 1.0 } else { 0.0 },
        MINIMAL_TABS.outline_alpha,
    );
    TabStyle {
        height: if tall {
            TAB_STRIP.minimal_height
        } else {
            TAB_STYLE_LIGHT.height
        },
        font_size: TAB_STYLE_LIGHT.font_size,
        backing: bar,
        selected: bar,
        selected_hover: bar,
        unselected,
        unselected_hover,
        text_selected: text,
        text_unselected: dimmed,
        text_selected_inactive: text,
        text_unselected_inactive: dimmed,
        separator: Rgba::TRANSPARENT,
        separator_inactive: Rgba::TRANSPARENT,
        top_line: bar,
        bottom_line: Rgba::TRANSPARENT,
        shadow: false,
        outline,
    }
}

// The window chrome above the panes: a compact window has only its strip; a regular one, the
// 28-point title bar and the tabs or the 1-point division view.
pub fn chrome_height(theme: Theme, tab_bar_shown: bool) -> f32 {
    let tabs = if theme == Theme::Minimal {
        TAB_STRIP.minimal_height
    } else {
        TAB_STYLE_LIGHT.height
    };
    match (compact(theme), tab_bar_shown) {
        (true, _) => tabs,
        (false, true) => TITLE_BAR.height + tabs,
        (false, false) => TITLE_BAR.height + 1.0,
    }
}

// NSColor+iTerm.m -perceivedBrightness
fn perceived_dark(color: Rgb) -> bool {
    let channel = |value: u8| f32::from(value) / 255.0;
    0.30 * channel(color.r) + 0.59 * channel(color.g) + 0.11 * channel(color.b) < 0.5
}

fn shift(color: Rgba, amount: f32) -> Rgba {
    let move_by = |channel: f32| (channel + amount).clamp(0.0, 1.0);
    Rgba {
        r: move_by(color.r),
        g: move_by(color.g),
        b: move_by(color.b),
        a: color.a,
    }
}

fn gray(level: f32, alpha: f32) -> Rgba {
    Rgba {
        r: level,
        g: level,
        b: level,
        a: alpha,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_tabs_are_the_terminal_background_and_unselected_ones_darker_unless_nearly_black() {
        let solarized = Rgb { r: 0, g: 43, b: 54 };
        let style = tab_style(Theme::Minimal, false, solarized, true);
        assert_eq!(style.selected, Rgba::rgba8(0, 43, 54, 1.0));
        assert_eq!(style.height, TAB_STRIP.minimal_height);
        assert!(style.unselected.b < style.selected.b);
        let black = tab_style(Theme::Minimal, false, Rgb { r: 0, g: 0, b: 0 }, true);
        assert!(black.unselected.r > black.selected.r);
        let paper = Rgb {
            r: 250,
            g: 250,
            b: 250,
        };
        let style = tab_style(Theme::Minimal, true, paper, false);
        assert!(style.unselected.r < style.selected.r);
        assert_eq!(style.text_selected.r, MINIMAL_TABS.text_light);
    }
}
