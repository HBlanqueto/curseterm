use freya_core::prelude::{Key, Modifiers, NamedKey};
use serde::{Deserialize, Serialize};

use crate::localize::tr;

// iTermEditKeyActionWindowController.m's action pop-up, in its groups and order, for the actions
// CurseTerm has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingAction {
    #[default]
    Ignore,
    SelectMenuItem,
    NewWindowWithProfile,
    NewTabWithProfile,
    DuplicateTab,
    SplitHorizontallyWithProfile,
    SplitVerticallyWithProfile,
    NextTab,
    PreviousTab,
    MoveTabLeft,
    MoveTabRight,
    NextPane,
    PreviousPane,
    PaneAbove,
    PaneBelow,
    PaneLeft,
    PaneRight,
    ScrollToEnd,
    ScrollToTop,
    ScrollLineDown,
    ScrollLineUp,
    ScrollPageDown,
    ScrollPageUp,
    SendCtrlH,
    SendCtrlQuestion,
    SendEscapeSequence,
    SendHexCode,
    SendText,
    SendSnippet,
    RunCoprocess,
    InstantReplay,
    AlertOnNextMark,
    FindAgainDown,
    FindAgainUp,
    CopyMode,
    ToggleFullscreen,
    ToggleMouseReporting,
}

// What the edit sheet asks for under the action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parameter {
    None,
    Text,
    Profile,
    Snippet,
    MenuItem,
}

impl BindingAction {
    // The pop-up's groups; a long kit pop-up does not scroll (BK-040).
    pub const GROUPS: [&'static [Self]; 12] = [
        &[Self::Ignore, Self::SelectMenuItem],
        &[
            Self::NewWindowWithProfile,
            Self::NewTabWithProfile,
            Self::DuplicateTab,
        ],
        &[
            Self::SplitHorizontallyWithProfile,
            Self::SplitVerticallyWithProfile,
        ],
        &[Self::NextTab, Self::PreviousTab],
        &[Self::MoveTabLeft, Self::MoveTabRight],
        &[
            Self::NextPane,
            Self::PreviousPane,
            Self::PaneAbove,
            Self::PaneBelow,
            Self::PaneLeft,
            Self::PaneRight,
        ],
        &[
            Self::ScrollToEnd,
            Self::ScrollToTop,
            Self::ScrollLineDown,
            Self::ScrollLineUp,
            Self::ScrollPageDown,
            Self::ScrollPageUp,
        ],
        &[
            Self::SendCtrlH,
            Self::SendCtrlQuestion,
            Self::SendEscapeSequence,
            Self::SendHexCode,
            Self::SendText,
            Self::SendSnippet,
        ],
        &[
            Self::RunCoprocess,
            Self::InstantReplay,
            Self::AlertOnNextMark,
        ],
        &[Self::FindAgainDown, Self::FindAgainUp],
        &[Self::CopyMode],
        &[Self::ToggleFullscreen, Self::ToggleMouseReporting],
    ];

    pub fn group(self) -> usize {
        Self::GROUPS
            .iter()
            .position(|group| group.contains(&self))
            .unwrap_or_default()
    }

    pub fn parameter(self) -> Parameter {
        match self {
            Self::SelectMenuItem => Parameter::MenuItem,
            Self::NewWindowWithProfile
            | Self::NewTabWithProfile
            | Self::SplitHorizontallyWithProfile
            | Self::SplitVerticallyWithProfile => Parameter::Profile,
            Self::SendEscapeSequence | Self::SendHexCode | Self::SendText | Self::RunCoprocess => {
                Parameter::Text
            }
            Self::SendSnippet => Parameter::Snippet,
            _ => Parameter::None,
        }
    }

    pub fn group_name(group: usize) -> &'static str {
        [
            tr!("binding-group-general"),
            tr!("binding-group-new-tab-or-window"),
            tr!("binding-group-split"),
            tr!("binding-group-navigate-tabs"),
            tr!("binding-group-reorder-tabs"),
            tr!("binding-group-navigate-panes"),
            tr!("binding-group-scroll"),
            tr!("binding-group-send-keystrokes"),
            tr!("binding-group-miscellaneous"),
            tr!("binding-group-search"),
            tr!("binding-group-copy-mode"),
            tr!("binding-group-toggles"),
        ][group]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Ignore => tr!("binding-ignore"),
            Self::SelectMenuItem => tr!("binding-select-menu-item"),
            Self::NewWindowWithProfile => tr!("binding-new-window-with-profile"),
            Self::NewTabWithProfile => tr!("binding-new-tab-with-profile"),
            Self::DuplicateTab => tr!("binding-duplicate-tab"),
            Self::SplitHorizontallyWithProfile => tr!("binding-split-horizontally-with-profile"),
            Self::SplitVerticallyWithProfile => tr!("binding-split-vertically-with-profile"),
            Self::NextTab => tr!("binding-next-tab"),
            Self::PreviousTab => tr!("binding-previous-tab"),
            Self::MoveTabLeft => tr!("binding-move-tab-left"),
            Self::MoveTabRight => tr!("binding-move-tab-right"),
            Self::NextPane => tr!("binding-next-pane"),
            Self::PreviousPane => tr!("binding-previous-pane"),
            Self::PaneAbove => tr!("binding-pane-above"),
            Self::PaneBelow => tr!("binding-pane-below"),
            Self::PaneLeft => tr!("binding-pane-left"),
            Self::PaneRight => tr!("binding-pane-right"),
            Self::ScrollToEnd => tr!("binding-scroll-to-end"),
            Self::ScrollToTop => tr!("binding-scroll-to-top"),
            Self::ScrollLineDown => tr!("binding-scroll-line-down"),
            Self::ScrollLineUp => tr!("binding-scroll-line-up"),
            Self::ScrollPageDown => tr!("binding-scroll-page-down"),
            Self::ScrollPageUp => tr!("binding-scroll-page-up"),
            Self::SendCtrlH => tr!("binding-send-ctrl-h"),
            Self::SendCtrlQuestion => tr!("binding-send-ctrl-question"),
            Self::SendEscapeSequence => tr!("binding-send-escape-sequence"),
            Self::SendHexCode => tr!("binding-send-hex-code"),
            Self::SendText => tr!("binding-send-text"),
            Self::SendSnippet => tr!("binding-send-snippet"),
            Self::RunCoprocess => tr!("binding-run-coprocess"),
            Self::InstantReplay => tr!("binding-instant-replay"),
            Self::AlertOnNextMark => tr!("binding-alert-on-next-mark"),
            Self::FindAgainDown => tr!("binding-find-again-down"),
            Self::FindAgainUp => tr!("binding-find-again-up"),
            Self::CopyMode => tr!("binding-copy-mode"),
            Self::ToggleFullscreen => tr!("binding-toggle-fullscreen"),
            Self::ToggleMouseReporting => tr!("binding-toggle-mouse-reporting"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Binding {
    pub combo: String,
    pub action: BindingAction,
    pub parameter: String,
}

// iTermActionsModel.m iTermAction: a key action under a title, run from Edit › Actions.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NamedAction {
    pub title: String,
    pub action: BindingAction,
    pub parameter: String,
}

impl NamedAction {
    pub fn binding(&self) -> Binding {
        Binding {
            combo: self.title.clone(),
            action: self.action,
            parameter: self.parameter.clone(),
        }
    }

    pub fn from_binding(binding: Binding) -> Self {
        Self {
            title: binding.combo,
            action: binding.action,
            parameter: binding.parameter,
        }
    }
}

// iTermKeyMappings.m: the profile's mappings come before the global ones.
pub fn find<'a>(
    profile: &'a [Binding],
    global: &'a [Binding],
    key: &Key,
    modifiers: Modifiers,
) -> Option<&'a Binding> {
    let combo = combo(key, modifiers)?;
    profile
        .iter()
        .chain(global)
        .find(|binding| binding.combo == combo)
}

// A key and its modifiers as Ctrl+Alt+Shift+Super+Key; a shifted symbol keeps only its character.
pub fn combo(key: &Key, modifiers: Modifiers) -> Option<String> {
    let (name, shift_counts) = match key {
        Key::Character(text) if text == " " => ("Space".to_string(), true),
        Key::Character(text) => {
            let mut chars = text.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            (c.to_uppercase().to_string(), c.is_alphabetic())
        }
        Key::Named(named) => (named_key(*named)?.to_string(), true),
    };
    let mut parts = Vec::new();
    for (modifier, label) in [
        (Modifiers::CONTROL, "Ctrl"),
        (Modifiers::ALT, "Alt"),
        (Modifiers::SHIFT, "Shift"),
        (Modifiers::META, "Super"),
    ] {
        if modifiers.contains(modifier) && (modifier != Modifiers::SHIFT || shift_counts) {
            parts.push(label.to_string());
        }
    }
    parts.push(name);
    Some(parts.join("+"))
}

fn named_key(named: NamedKey) -> Option<&'static str> {
    Some(match named {
        NamedKey::Enter => "Enter",
        NamedKey::Tab => "Tab",
        NamedKey::Escape => "Escape",
        NamedKey::Backspace => "Backspace",
        NamedKey::Delete => "Delete",
        NamedKey::Insert => "Insert",
        NamedKey::Home => "Home",
        NamedKey::End => "End",
        NamedKey::PageUp => "PageUp",
        NamedKey::PageDown => "PageDown",
        NamedKey::ArrowUp => "Up",
        NamedKey::ArrowDown => "Down",
        NamedKey::ArrowLeft => "Left",
        NamedKey::ArrowRight => "Right",
        NamedKey::F1 => "F1",
        NamedKey::F2 => "F2",
        NamedKey::F3 => "F3",
        NamedKey::F4 => "F4",
        NamedKey::F5 => "F5",
        NamedKey::F6 => "F6",
        NamedKey::F7 => "F7",
        NamedKey::F8 => "F8",
        NamedKey::F9 => "F9",
        NamedKey::F10 => "F10",
        NamedKey::F11 => "F11",
        NamedKey::F12 => "F12",
        _ => return None,
    })
}

// iTermKeystrokeFormatter: the modifier glyphs, then the key's.
pub fn shown(combo: &str) -> String {
    combo
        .split('+')
        .map(|part| match part {
            "Ctrl" => "⌃",
            "Alt" => "⌥",
            "Shift" => "⇧",
            "Super" => "⌘",
            "Enter" => "↩",
            "Tab" => "⇥",
            "Escape" => "⎋",
            "Backspace" => "⌫",
            "Delete" => "⌦",
            "Home" => "↖",
            "End" => "↘",
            "PageUp" => "⇞",
            "PageDown" => "⇟",
            "Up" => "↑",
            "Down" => "↓",
            "Left" => "←",
            "Right" => "→",
            "Space" => "Space",
            other => other,
        })
        .collect()
}

// iTermKeyBindingAction.m KEY_ACTION_HEX_CODE: whitespace-separated hexadecimal bytes.
pub fn hex_bytes(text: &str) -> Vec<u8> {
    text.split_whitespace()
        .filter_map(|token| {
            let digits = token
                .strip_prefix("0x")
                .or_else(|| token.strip_prefix("0X"))
                .unwrap_or(token);
            u8::from_str_radix(digits, 16).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combos_name_the_modifiers_in_order_and_profile_bindings_win() {
        let key = Key::Character("k".into());
        assert_eq!(
            combo(&key, Modifiers::CONTROL | Modifiers::ALT).as_deref(),
            Some("Ctrl+Alt+K")
        );
        assert_eq!(
            combo(&Key::Character("!".into()), Modifiers::SHIFT).as_deref(),
            Some("!")
        );
        assert_eq!(
            combo(&Key::Named(NamedKey::ArrowLeft), Modifiers::ALT).as_deref(),
            Some("Alt+Left")
        );
        assert_eq!(shown("Ctrl+Shift+Left"), "⌃⇧←");
        let global = [Binding {
            combo: "Ctrl+Alt+K".into(),
            action: BindingAction::NextTab,
            parameter: String::new(),
        }];
        let profile = [Binding {
            action: BindingAction::Ignore,
            ..global[0].clone()
        }];
        let found = find(&profile, &global, &key, Modifiers::CONTROL | Modifiers::ALT);
        assert_eq!(
            found.map(|binding| binding.action),
            Some(BindingAction::Ignore)
        );
        assert_eq!(hex_bytes("0x1b 5b 0x41 zz"), vec![0x1b, 0x5b, 0x41]);
    }
}
