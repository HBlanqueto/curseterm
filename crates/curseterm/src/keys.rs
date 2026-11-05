use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vi_mode::ViMotion;
use freya_core::prelude::{Key, Modifiers, NamedKey};

#[derive(Debug, PartialEq, Eq)]
pub enum KeyAction {
    Write(Vec<u8>),
    Copy,
    Paste,
    PageUp,
    PageDown,
    Ignore,
}

pub fn encode(key: &Key, modifiers: Modifiers, mode: TermMode) -> KeyAction {
    // ADR 0003: Super belongs to the app's shortcuts and never reaches the shell.
    if modifiers.contains(Modifiers::META) {
        return KeyAction::Ignore;
    }
    let ctrl = modifiers.contains(Modifiers::CONTROL);
    let shift = modifiers.contains(Modifiers::SHIFT);
    let alt = modifiers.contains(Modifiers::ALT);
    match key {
        Key::Character(text) if ctrl && shift && text.eq_ignore_ascii_case("c") => KeyAction::Copy,
        Key::Character(text) if ctrl && shift && text.eq_ignore_ascii_case("v") => KeyAction::Paste,
        Key::Character(text) => {
            let mut bytes = match text.chars().next().filter(|_| ctrl) {
                Some(c) => control_code(c).map_or_else(|| text.as_bytes().to_vec(), |b| vec![b]),
                None => text.as_bytes().to_vec(),
            };
            if alt {
                bytes.insert(0, 0x1b);
            }
            KeyAction::Write(bytes)
        }
        Key::Named(NamedKey::PageUp) if shift => KeyAction::PageUp,
        Key::Named(NamedKey::PageDown) if shift => KeyAction::PageDown,
        Key::Named(named) => named_key(*named, modifier_param(modifiers), mode, alt, ctrl, shift),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyCommand {
    Move(ViMotion),
    Top,
    Bottom,
    PageUp,
    PageDown,
    Select(SelectionType),
    Copy,
    Exit,
}

pub const COPY_KEYS: [(&str, CopyCommand); 30] = {
    use CopyCommand::*;
    use ViMotion::*;
    [
        ("h", Move(Left)),
        ("←", Move(Left)),
        ("j", Move(Down)),
        ("↓", Move(Down)),
        ("k", Move(Up)),
        ("↑", Move(Up)),
        ("l", Move(Right)),
        ("→", Move(Right)),
        ("w", Move(SemanticRight)),
        ("b", Move(SemanticLeft)),
        ("e", Move(SemanticRightEnd)),
        ("W", Move(WordRight)),
        ("B", Move(WordLeft)),
        ("E", Move(WordRightEnd)),
        ("0", Move(First)),
        ("^", Move(FirstOccupied)),
        ("$", Move(Last)),
        ("H", Move(High)),
        ("M", Move(Middle)),
        ("L", Move(Low)),
        ("g", Top),
        ("G", Bottom),
        ("Ctrl+B", PageUp),
        ("Ctrl+F", PageDown),
        ("v", Select(SelectionType::Simple)),
        ("Space", Select(SelectionType::Simple)),
        ("V", Select(SelectionType::Lines)),
        ("y", Copy),
        ("Escape", Exit),
        ("q", Exit),
    ]
};

pub fn copy_command(key: &Key, modifiers: Modifiers) -> Option<CopyCommand> {
    let name = match key {
        Key::Named(NamedKey::Escape) => "Escape".to_string(),
        Key::Named(NamedKey::ArrowLeft) => "←".to_string(),
        Key::Named(NamedKey::ArrowDown) => "↓".to_string(),
        Key::Named(NamedKey::ArrowUp) => "↑".to_string(),
        Key::Named(NamedKey::ArrowRight) => "→".to_string(),
        Key::Character(text) if modifiers.contains(Modifiers::CONTROL) => {
            format!("Ctrl+{}", text.to_uppercase())
        }
        Key::Character(text) if text == " " => "Space".to_string(),
        Key::Character(text) => text.to_string(),
        _ => return None,
    };
    COPY_KEYS
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, command)| *command)
}

fn modifier_param(modifiers: Modifiers) -> u8 {
    1 + u8::from(modifiers.contains(Modifiers::SHIFT))
        + 2 * u8::from(modifiers.contains(Modifiers::ALT))
        + 4 * u8::from(modifiers.contains(Modifiers::CONTROL))
}

fn control_code(c: char) -> Option<u8> {
    match c.to_ascii_lowercase() {
        c @ 'a'..='z' => Some(c as u8 & 0x1f),
        '@' | ' ' | '2' => Some(0x00),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '-' | '7' => Some(0x1f),
        '?' | '8' => Some(0x7f),
        _ => None,
    }
}

fn named_key(
    named: NamedKey,
    param: u8,
    mode: TermMode,
    alt: bool,
    ctrl: bool,
    shift: bool,
) -> KeyAction {
    let app_cursor = mode.contains(TermMode::APP_CURSOR);
    let with_alt = |bytes: &[u8]| {
        let mut out = if alt { vec![0x1b] } else { Vec::new() };
        out.extend_from_slice(bytes);
        KeyAction::Write(out)
    };
    let text = match named {
        NamedKey::Enter => return with_alt(b"\r"),
        NamedKey::Backspace if ctrl => return with_alt(&[0x08]),
        NamedKey::Backspace => return with_alt(&[0x7f]),
        NamedKey::Tab if shift => "\x1b[Z".to_string(),
        NamedKey::Tab => return with_alt(b"\t"),
        NamedKey::Escape => return with_alt(&[0x1b]),
        NamedKey::ArrowUp => cursor_key('A', param, app_cursor),
        NamedKey::ArrowDown => cursor_key('B', param, app_cursor),
        NamedKey::ArrowRight => cursor_key('C', param, app_cursor),
        NamedKey::ArrowLeft => cursor_key('D', param, app_cursor),
        NamedKey::Home => cursor_key('H', param, app_cursor),
        NamedKey::End => cursor_key('F', param, app_cursor),
        NamedKey::Insert => tilde_key(2, param),
        NamedKey::Delete => tilde_key(3, param),
        NamedKey::PageUp => tilde_key(5, param),
        NamedKey::PageDown => tilde_key(6, param),
        NamedKey::F1 => function_key('P', param),
        NamedKey::F2 => function_key('Q', param),
        NamedKey::F3 => function_key('R', param),
        NamedKey::F4 => function_key('S', param),
        NamedKey::F5 => tilde_key(15, param),
        NamedKey::F6 => tilde_key(17, param),
        NamedKey::F7 => tilde_key(18, param),
        NamedKey::F8 => tilde_key(19, param),
        NamedKey::F9 => tilde_key(20, param),
        NamedKey::F10 => tilde_key(21, param),
        NamedKey::F11 => tilde_key(23, param),
        NamedKey::F12 => tilde_key(24, param),
        _ => return KeyAction::Ignore,
    };
    KeyAction::Write(text.into_bytes())
}

fn cursor_key(final_byte: char, param: u8, app_cursor: bool) -> String {
    match (param, app_cursor) {
        (1, true) => format!("\x1bO{final_byte}"),
        (1, false) => format!("\x1b[{final_byte}"),
        _ => format!("\x1b[1;{param}{final_byte}"),
    }
}

fn tilde_key(number: u8, param: u8) -> String {
    if param == 1 {
        format!("\x1b[{number}~")
    } else {
        format!("\x1b[{number};{param}~")
    }
}

fn function_key(final_byte: char, param: u8) -> String {
    if param == 1 {
        format!("\x1bO{final_byte}")
    } else {
        format!("\x1b[1;{param}{final_byte}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(key: Key, modifiers: Modifiers, mode: TermMode) -> Vec<u8> {
        match encode(&key, modifiers, mode) {
            KeyAction::Write(bytes) => bytes,
            other => panic!("expected bytes, got {other:?}"),
        }
    }

    fn character(text: &str) -> Key {
        Key::Character(text.into())
    }

    #[test]
    fn super_never_reaches_the_shell() {
        let modifiers = Modifiers::META | Modifiers::CONTROL;
        assert_eq!(
            encode(&character("c"), modifiers, TermMode::empty()),
            KeyAction::Ignore
        );
    }

    #[test]
    fn ctrl_letters_become_control_codes_and_alt_prefixes_escape() {
        let none = TermMode::empty();
        assert_eq!(write(character("c"), Modifiers::CONTROL, none), vec![0x03]);
        assert_eq!(write(character("["), Modifiers::CONTROL, none), vec![0x1b]);
        assert_eq!(
            write(character("b"), Modifiers::ALT, none),
            b"\x1bb".to_vec()
        );
        assert_eq!(
            write(character("ñ"), Modifiers::empty(), none),
            "ñ".as_bytes()
        );
    }

    #[test]
    fn ctrl_shift_c_and_v_copy_and_paste() {
        let modifiers = Modifiers::CONTROL | Modifiers::SHIFT;
        let none = TermMode::empty();
        assert_eq!(encode(&character("C"), modifiers, none), KeyAction::Copy);
        assert_eq!(encode(&character("V"), modifiers, none), KeyAction::Paste);
    }

    #[test]
    fn arrows_follow_the_application_cursor_mode_and_carry_modifiers() {
        let up = Key::Named(NamedKey::ArrowUp);
        assert_eq!(
            write(up.clone(), Modifiers::empty(), TermMode::empty()),
            b"\x1b[A"
        );
        assert_eq!(
            write(up.clone(), Modifiers::empty(), TermMode::APP_CURSOR),
            b"\x1bOA"
        );
        assert_eq!(
            write(up, Modifiers::CONTROL, TermMode::APP_CURSOR),
            b"\x1b[1;5A"
        );
    }

    #[test]
    fn function_and_editing_keys_use_xterm_sequences() {
        let none = TermMode::empty();
        assert_eq!(
            write(Key::Named(NamedKey::F1), Modifiers::empty(), none),
            b"\x1bOP"
        );
        assert_eq!(
            write(Key::Named(NamedKey::F5), Modifiers::SHIFT, none),
            b"\x1b[15;2~"
        );
        assert_eq!(
            write(Key::Named(NamedKey::Delete), Modifiers::empty(), none),
            b"\x1b[3~"
        );
        assert_eq!(
            write(Key::Named(NamedKey::Tab), Modifiers::SHIFT, none),
            b"\x1b[Z"
        );
    }

    #[test]
    fn copy_mode_reads_vi_keys_with_control_for_pages_and_ignores_the_rest() {
        let command = |text, modifiers| copy_command(&character(text), modifiers);
        assert_eq!(command("G", Modifiers::SHIFT), Some(CopyCommand::Bottom));
        assert_eq!(
            command("b", Modifiers::empty()),
            Some(CopyCommand::Move(ViMotion::SemanticLeft))
        );
        assert_eq!(command("b", Modifiers::CONTROL), Some(CopyCommand::PageUp));
        assert_eq!(command("x", Modifiers::empty()), None);
    }
}
