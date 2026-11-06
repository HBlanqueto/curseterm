use alacritty_terminal::term::TermMode;
use freya_core::prelude::{Code, Key, Modifiers, NamedKey};
use serde::{Deserialize, Serialize};

use crate::keys::{KeyAction, encode, modifier_param};

// PTYSession.m iTermKeyMappingMode, in the order of Terminal State's radio group; Raw reports
// macOS key codes and has no meaning here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyMode {
    #[default]
    Standard,
    ModifyOtherKeys1,
    ModifyOtherKeys2,
    CsiU,
}

impl KeyMode {
    pub const ALL: [Self; 4] = [
        Self::Standard,
        Self::ModifyOtherKeys1,
        Self::ModifyOtherKeys2,
        Self::CsiU,
    ];

    // PTYSession.m -screenSendModifiersDidChange: CSI > 4 ; n m.
    pub fn from_modify_other_keys(level: u8) -> Self {
        match level {
            1 => Self::ModifyOtherKeys1,
            2 => Self::ModifyOtherKeys2,
            _ => Self::Standard,
        }
    }
}

// The kitty keyboard flags in the order of Terminal State's checkboxes.
pub const KITTY_FLAGS: [TermMode; 5] = [
    TermMode::DISAMBIGUATE_ESC_CODES,
    TermMode::REPORT_EVENT_TYPES,
    TermMode::REPORT_ALTERNATE_KEYS,
    TermMode::REPORT_ALL_KEYS_AS_ESC,
    TermMode::REPORT_ASSOCIATED_TEXT,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Press,
    Repeat,
    Release,
}

pub struct Press<'a> {
    pub key: &'a Key,
    pub code: Code,
    pub modifiers: Modifiers,
    pub phase: Phase,
}

// PTYSession.m -updateKeyMapper: kitty flags win over the session's key mapping mode.
pub fn report(press: &Press, mode: TermMode, key_mode: KeyMode) -> KeyAction {
    // ADR 0003: Super belongs to the app's shortcuts and never reaches the shell.
    if press.modifiers.contains(Modifiers::META) {
        return KeyAction::Ignore;
    }
    let standard = || encode(press.key, press.modifiers, mode);
    if mode.intersects(TermMode::KITTY_KEYBOARD_PROTOCOL) {
        if matches!(standard(), KeyAction::Copy | KeyAction::Paste) {
            return standard();
        }
        return match kitty(press, mode) {
            Some(bytes) => KeyAction::Write(bytes),
            None if press.phase == Phase::Release => KeyAction::Ignore,
            None => standard(),
        };
    }
    if press.phase == Phase::Release {
        return KeyAction::Ignore;
    }
    let reported = match key_mode {
        KeyMode::Standard => None,
        KeyMode::ModifyOtherKeys1 => modify_other_keys_1(press.key, press.modifiers),
        KeyMode::ModifyOtherKeys2 => modify_other_keys(press.key, press.modifiers),
        KeyMode::CsiU => csi_u(press.key, press.modifiers),
    };
    match reported {
        Some(bytes) if !matches!(standard(), KeyAction::Copy | KeyAction::Paste) => {
            KeyAction::Write(bytes)
        }
        _ => standard(),
    }
}

fn held(modifiers: Modifiers) -> (bool, bool, bool) {
    (
        modifiers.contains(Modifiers::CONTROL),
        modifiers.contains(Modifiers::ALT),
        modifiers.contains(Modifiers::SHIFT),
    )
}

// NSEvent -charactersIgnoringModifiers keeps Shift, but the key arrives unshifted with Control.
fn shifted(c: char, shift: bool) -> char {
    if shift { c.to_ascii_uppercase() } else { c }
}

fn single(key: &Key) -> Option<char> {
    let Key::Character(text) = key else {
        return None;
    };
    let mut chars = text.chars();
    let c = chars.next()?;
    chars.next().is_none().then_some(c)
}

fn c0_code(named: NamedKey) -> Option<u32> {
    match named {
        NamedKey::Enter => Some(13),
        NamedKey::Escape => Some(27),
        NamedKey::Backspace => Some(127),
        NamedKey::Tab => Some(9),
        _ => None,
    }
}

// iTermModifyOtherKeysMapper.m -stringForCodePoint:modifiers: CSI 27 ; modifiers ; code ~.
fn modify_other_keys(key: &Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    let (ctrl, alt, shift) = held(modifiers);
    let code = match key {
        Key::Named(NamedKey::Tab) if shift && !ctrl && !alt => return None,
        Key::Named(named) => c0_code(*named)?,
        _ => {
            let c = single(key)?;
            // Only Control, Alt and a shifted space skip the text path (-keyMapperStringForPreCocoaEvent:).
            if !(ctrl || alt || (shift && c == ' ')) {
                return None;
            }
            u32::from(shifted(c, shift))
        }
    };
    if !(ctrl || alt || shift) {
        return None;
    }
    Some(format!("\x1b[27;{};{code}~", modifier_param(modifiers)).into_bytes())
}

// iTermModifyOtherKeysMapper1.m -shouldModifyOtherKeysForEvent:modifiedEvent: only the keys
// xterm leaves ambiguous at level 1.
fn modify_other_keys_1(key: &Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    let (ctrl, alt, shift) = held(modifiers);
    let modify = match key {
        Key::Named(NamedKey::Tab) => ctrl && !alt && !shift,
        Key::Named(NamedKey::Escape) => (ctrl && alt) || (!ctrl && alt && shift),
        Key::Named(NamedKey::Enter) => ctrl || shift,
        _ => match single(key) {
            Some(digit @ '0'..='9') if ctrl && shift => !matches!(digit, '2' | '6'),
            Some(digit @ '0'..='9') if ctrl => matches!(digit, '1' | '9' | '0'),
            Some(symbol) if ctrl && ",.;=-\\?|{}_+~!@#$%^&*()".contains(symbol) => {
                let kept = if shift { "@^{}[]\\-_`'/" } else { "{}[]\\`'" };
                !kept.contains(symbol)
            }
            _ => false,
        },
    };
    if modify {
        modify_other_keys(key, modifiers)
    } else {
        None
    }
}

fn csi_u_with(code: u32, modifiers: Modifiers) -> Vec<u8> {
    match modifier_param(modifiers) {
        1 => format!("\x1b[{code}u").into_bytes(),
        param => format!("\x1b[{code};{param}u").into_bytes(),
    }
}

// iTermTermkeyKeyMapper.m -termkeySequenceForCodePoint:modifiers:keyCode:optionModifiedData:
// (libtermkey's CSI u).
fn csi_u(key: &Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    let (ctrl, alt, shift) = held(modifiers);
    let space = single(key) == Some(' ');
    let c0 = match key {
        Key::Named(named) => c0_code(*named),
        _ if space => Some(32),
        _ => None,
    };
    if let Some(code) = c0 {
        if !(ctrl || shift) {
            return None;
        }
        if space && ctrl && !alt && !shift {
            return Some(vec![0]);
        }
        if code == 9 && shift {
            let rest = modifier_param(modifiers.difference(Modifiers::SHIFT));
            return Some(match rest {
                1 => b"\x1b[Z".to_vec(),
                rest => format!("\x1b[1;{rest}Z").into_bytes(),
            });
        }
        return Some(csi_u_with(code, modifiers));
    }
    let c = single(key)?;
    if !ctrl {
        return None;
    }
    if !alt && !shift && c.is_ascii_lowercase() && !matches!(c, 'i' | 'j' | 'm') {
        return None;
    }
    Some(csi_u_with(
        u32::from(shifted(c, shift)),
        modifiers.difference(Modifiers::SHIFT),
    ))
}

// kitty keyboard protocol: CSI key[:shifted] ; modifiers[:event] [; text] u, or the legacy
// final byte for the keys that have one.
fn kitty(press: &Press, mode: TermMode) -> Option<Vec<u8>> {
    let all_keys = mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC);
    let events = mode.contains(TermMode::REPORT_EVENT_TYPES);
    if press.phase != Phase::Press && !events {
        return None;
    }
    let (ctrl, alt, shift) = held(press.modifiers);
    let param = modifier_param(press.modifiers);
    let event = match press.phase {
        Phase::Press => None,
        Phase::Repeat => Some(2),
        Phase::Release => Some(3),
    };
    let modifiers_field = match event {
        Some(event) => format!("{param}:{event}"),
        None if param == 1 => String::new(),
        None => param.to_string(),
    };
    let (number, final_byte) = match press.key {
        Key::Named(NamedKey::Shift | NamedKey::Control | NamedKey::Alt) if !all_keys => {
            return None;
        }
        Key::Named(named) => functional(*named, press.code)?,
        _ => {
            let c = single(press.key)?;
            let plain = !ctrl && !alt;
            if plain && !all_keys && press.phase != Phase::Release {
                return None;
            }
            let base = base_key(press.code).unwrap_or_else(|| c.to_ascii_lowercase());
            let mut number = u32::from(base).to_string();
            let alternate = shifted(c, shift);
            if shift && mode.contains(TermMode::REPORT_ALTERNATE_KEYS) && alternate != base {
                number = format!("{number}:{}", u32::from(alternate));
            }
            let mut sequence = format!("\x1b[{number}");
            let text = (all_keys && mode.contains(TermMode::REPORT_ASSOCIATED_TEXT) && plain)
                .then(|| u32::from(c).to_string());
            match (modifiers_field.is_empty(), text) {
                (true, None) => {}
                (_, Some(text)) => {
                    let field = if modifiers_field.is_empty() {
                        "1".to_string()
                    } else {
                        modifiers_field.clone()
                    };
                    sequence.push_str(&format!(";{field};{text}"));
                }
                (false, None) => sequence.push_str(&format!(";{modifiers_field}")),
            }
            sequence.push('u');
            return Some(sequence.into_bytes());
        }
    };
    let legacy_c0 = matches!(final_byte, 'u') && matches!(number, 13 | 9 | 127);
    if legacy_c0 && !all_keys && (param == 1 || press.phase == Phase::Release) {
        // Enter, Tab and Backspace keep their bytes, and their releases go unreported.
        return None;
    }
    if final_byte != 'u' && param == 1 && event.is_none() && !all_keys {
        return None;
    }
    let lead = match (final_byte, number) {
        ('u' | '~', number) => number.to_string(),
        (_, _) if modifiers_field.is_empty() => String::new(),
        _ => "1".to_string(),
    };
    let field = if modifiers_field.is_empty() {
        String::new()
    } else {
        format!(";{modifiers_field}")
    };
    Some(format!("\x1b[{lead}{field}{final_byte}").into_bytes())
}

// The kitty protocol's functional key numbers and final bytes.
fn functional(named: NamedKey, code: Code) -> Option<(u32, char)> {
    Some(match named {
        NamedKey::Escape => (27, 'u'),
        NamedKey::Enter => (13, 'u'),
        NamedKey::Tab => (9, 'u'),
        NamedKey::Backspace => (127, 'u'),
        NamedKey::Insert => (2, '~'),
        NamedKey::Delete => (3, '~'),
        NamedKey::ArrowLeft => (1, 'D'),
        NamedKey::ArrowRight => (1, 'C'),
        NamedKey::ArrowUp => (1, 'A'),
        NamedKey::ArrowDown => (1, 'B'),
        NamedKey::PageUp => (5, '~'),
        NamedKey::PageDown => (6, '~'),
        NamedKey::Home => (1, 'H'),
        NamedKey::End => (1, 'F'),
        NamedKey::F1 => (1, 'P'),
        NamedKey::F2 => (1, 'Q'),
        NamedKey::F3 => (13, '~'),
        NamedKey::F4 => (1, 'S'),
        NamedKey::F5 => (15, '~'),
        NamedKey::F6 => (17, '~'),
        NamedKey::F7 => (18, '~'),
        NamedKey::F8 => (19, '~'),
        NamedKey::F9 => (20, '~'),
        NamedKey::F10 => (21, '~'),
        NamedKey::F11 => (23, '~'),
        NamedKey::F12 => (24, '~'),
        NamedKey::Shift if code == Code::ShiftRight => (57447, 'u'),
        NamedKey::Shift => (57441, 'u'),
        NamedKey::Control if code == Code::ControlRight => (57448, 'u'),
        NamedKey::Control => (57442, 'u'),
        NamedKey::Alt if code == Code::AltRight => (57449, 'u'),
        NamedKey::Alt => (57443, 'u'),
        _ => return None,
    })
}

// The unshifted key of a US layout, the protocol's base key for a shifted character.
fn base_key(code: Code) -> Option<char> {
    let digits = [
        (Code::Digit0, '0'),
        (Code::Digit1, '1'),
        (Code::Digit2, '2'),
        (Code::Digit3, '3'),
        (Code::Digit4, '4'),
        (Code::Digit5, '5'),
        (Code::Digit6, '6'),
        (Code::Digit7, '7'),
        (Code::Digit8, '8'),
        (Code::Digit9, '9'),
        (Code::Minus, '-'),
        (Code::Equal, '='),
        (Code::BracketLeft, '['),
        (Code::BracketRight, ']'),
        (Code::Backslash, '\\'),
        (Code::Semicolon, ';'),
        (Code::Quote, '\''),
        (Code::Backquote, '`'),
        (Code::Comma, ','),
        (Code::Period, '.'),
        (Code::Slash, '/'),
        (Code::Space, ' '),
    ];
    if let Some((_, c)) = digits.iter().find(|(known, _)| *known == code) {
        return Some(*c);
    }
    let name = format!("{code:?}");
    let letter = name.strip_prefix("Key")?;
    let mut chars = letter.chars();
    let c = chars.next()?;
    chars.next().is_none().then(|| c.to_ascii_lowercase())
}

// Watches the PTY output for xterm's CSI > 4 ; n m, which alacritty_terminal ignores.
#[derive(Default)]
pub struct ModifyOtherKeysWatch {
    held: Vec<u8>,
}

impl ModifyOtherKeysWatch {
    const PREFIX: &'static [u8] = b"\x1b[>4";

    pub fn feed(&mut self, bytes: &[u8]) -> Option<u8> {
        let mut found = None;
        for &byte in bytes {
            self.held.push(byte);
            let prefix = Self::PREFIX.len().min(self.held.len());
            if self.held[..prefix] != Self::PREFIX[..prefix] {
                self.held.clear();
                if byte == 0x1b {
                    self.held.push(byte);
                }
                continue;
            }
            if self.held.len() <= Self::PREFIX.len() {
                continue;
            }
            match byte {
                b'm' => {
                    found = Some(level(&self.held[Self::PREFIX.len()..self.held.len() - 1]));
                    self.held.clear();
                }
                b';' | b'0'..=b'9' if self.held.len() < 12 => {}
                _ => self.held.clear(),
            }
        }
        found
    }
}

fn level(parameters: &[u8]) -> u8 {
    parameters
        .strip_prefix(b";")
        .and_then(|digits| std::str::from_utf8(digits).ok())
        .and_then(|digits| digits.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: Key, code: Code, modifiers: Modifiers) -> Vec<u8> {
        write(key, code, modifiers, Phase::Press, TermMode::empty())
    }

    fn write(key: Key, code: Code, modifiers: Modifiers, phase: Phase, mode: TermMode) -> Vec<u8> {
        let press = Press {
            key: &key,
            code,
            modifiers,
            phase,
        };
        match report(&press, mode, KeyMode::Standard) {
            KeyAction::Write(bytes) => bytes,
            _ => Vec::new(),
        }
    }

    fn character(text: &str) -> Key {
        Key::Character(text.into())
    }

    fn mapped(key_mode: KeyMode, key: Key, modifiers: Modifiers) -> Vec<u8> {
        let press = Press {
            key: &key,
            code: Code::Unidentified,
            modifiers,
            phase: Phase::Press,
        };
        match report(&press, TermMode::empty(), key_mode) {
            KeyAction::Write(bytes) => bytes,
            _ => Vec::new(),
        }
    }

    #[test]
    fn modify_other_keys_two_reports_control_and_alt_and_one_only_ambiguous_keys() {
        let two = KeyMode::ModifyOtherKeys2;
        assert_eq!(
            mapped(two, character("a"), Modifiers::CONTROL),
            b"\x1b[27;5;97~"
        );
        assert_eq!(
            mapped(two, character("x"), Modifiers::ALT),
            b"\x1b[27;3;120~"
        );
        assert_eq!(mapped(two, character("A"), Modifiers::SHIFT), b"A");
        assert_eq!(
            mapped(two, character("a"), Modifiers::CONTROL | Modifiers::SHIFT),
            b"\x1b[27;6;65~"
        );
        assert_eq!(
            mapped(two, Key::Named(NamedKey::Enter), Modifiers::SHIFT),
            b"\x1b[27;2;13~"
        );
        let one = KeyMode::ModifyOtherKeys1;
        assert_eq!(mapped(one, character("a"), Modifiers::CONTROL), [1]);
        assert_eq!(
            mapped(one, character("1"), Modifiers::CONTROL),
            b"\x1b[27;5;49~"
        );
        assert_eq!(mapped(one, character("3"), Modifiers::CONTROL), [0x1b]);
        assert_eq!(
            mapped(one, Key::Named(NamedKey::Tab), Modifiers::CONTROL),
            b"\x1b[27;5;9~"
        );
    }

    #[test]
    fn csi_u_keeps_unambiguous_control_codes_and_reports_the_rest() {
        let csi = KeyMode::CsiU;
        assert_eq!(mapped(csi, character("a"), Modifiers::CONTROL), [1]);
        assert_eq!(
            mapped(csi, character("i"), Modifiers::CONTROL),
            b"\x1b[105;5u"
        );
        assert_eq!(
            mapped(csi, character("a"), Modifiers::CONTROL | Modifiers::SHIFT),
            b"\x1b[65;5u"
        );
        assert_eq!(
            mapped(csi, Key::Named(NamedKey::Enter), Modifiers::SHIFT),
            b"\x1b[13;2u"
        );
        assert_eq!(
            mapped(csi, Key::Named(NamedKey::Tab), Modifiers::SHIFT),
            b"\x1b[Z"
        );
        assert_eq!(mapped(csi, character(" "), Modifiers::CONTROL), [0]);
        assert_eq!(mapped(csi, character("x"), Modifiers::ALT), b"\x1bx");
    }

    #[test]
    fn kitty_flags_follow_the_protocol() {
        let disambiguate = TermMode::DISAMBIGUATE_ESC_CODES;
        let k = |key, code, modifiers, phase, mode| write(key, code, modifiers, phase, mode);
        assert_eq!(press(character("a"), Code::KeyA, Modifiers::empty()), b"a");
        let escape = Key::Named(NamedKey::Escape);
        assert_eq!(
            k(
                escape.clone(),
                Code::Escape,
                Modifiers::empty(),
                Phase::Press,
                disambiguate
            ),
            b"\x1b[27u"
        );
        assert_eq!(
            k(
                character("a"),
                Code::KeyA,
                Modifiers::empty(),
                Phase::Press,
                disambiguate
            ),
            b"a"
        );
        assert_eq!(
            k(
                character("a"),
                Code::KeyA,
                Modifiers::CONTROL,
                Phase::Press,
                disambiguate
            ),
            b"\x1b[97;5u"
        );
        let enter = Key::Named(NamedKey::Enter);
        assert_eq!(
            k(
                enter.clone(),
                Code::Enter,
                Modifiers::empty(),
                Phase::Press,
                disambiguate
            ),
            b"\r"
        );
        assert_eq!(
            k(
                enter,
                Code::Enter,
                Modifiers::SHIFT,
                Phase::Press,
                disambiguate
            ),
            b"\x1b[13;2u"
        );
        let up = Key::Named(NamedKey::ArrowUp);
        assert_eq!(
            k(
                up.clone(),
                Code::ArrowUp,
                Modifiers::CONTROL,
                Phase::Press,
                disambiguate
            ),
            b"\x1b[1;5A"
        );
        let events = disambiguate | TermMode::REPORT_EVENT_TYPES;
        assert_eq!(
            k(
                character("a"),
                Code::KeyA,
                Modifiers::empty(),
                Phase::Release,
                events
            ),
            b"\x1b[97;1:3u"
        );
        assert_eq!(
            k(up, Code::ArrowUp, Modifiers::empty(), Phase::Repeat, events),
            b"\x1b[1;1:2A"
        );
        let alternate = disambiguate | TermMode::REPORT_ALTERNATE_KEYS;
        assert_eq!(
            k(
                character("!"),
                Code::Digit1,
                Modifiers::CONTROL | Modifiers::SHIFT,
                Phase::Press,
                alternate
            ),
            b"\x1b[49:33;6u"
        );
        assert_eq!(
            k(
                character("a"),
                Code::KeyA,
                Modifiers::CONTROL | Modifiers::SHIFT,
                Phase::Press,
                alternate
            ),
            b"\x1b[97:65;6u"
        );
        let all = TermMode::REPORT_ALL_KEYS_AS_ESC | TermMode::REPORT_ASSOCIATED_TEXT;
        assert_eq!(
            k(
                character("a"),
                Code::KeyA,
                Modifiers::empty(),
                Phase::Press,
                all
            ),
            b"\x1b[97;1;97u"
        );
        assert_eq!(
            k(
                Key::Named(NamedKey::Shift),
                Code::ShiftLeft,
                Modifiers::SHIFT,
                Phase::Press,
                TermMode::REPORT_ALL_KEYS_AS_ESC
            ),
            b"\x1b[57441;2u"
        );
    }

    #[test]
    fn the_watch_finds_modify_other_keys_even_split_across_reads() {
        let mut watch = ModifyOtherKeysWatch::default();
        assert_eq!(watch.feed(b"hola\x1b[>4;"), None);
        assert_eq!(watch.feed(b"2m resto"), Some(2));
        assert_eq!(watch.feed(b"\x1b[>4m"), Some(0));
        assert_eq!(watch.feed(b"\x1b[>4;1m\x1b[1m"), Some(1));
        assert_eq!(watch.feed(b"\x1b[>5;1m"), None);
    }
}
