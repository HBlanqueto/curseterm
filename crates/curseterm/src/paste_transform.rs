use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::session::shell_escaped;

const LNEXT: char = '\u{16}';
const BASE64_LINE: usize = 77;

// iTermPasteSpecialViewController.m: the Unicode dashes, double and single quotes.
const DASHES: &[(char, char)] = &[
    ('\u{2010}', '\u{2015}'),
    ('\u{207b}', '\u{207b}'),
    ('\u{208b}', '\u{208b}'),
    ('\u{2212}', '\u{2212}'),
    ('\u{2e3a}', '\u{2e3b}'),
    ('\u{fe58}', '\u{fe58}'),
    ('\u{fe63}', '\u{fe63}'),
    ('\u{ff0d}', '\u{ff0d}'),
    ('\u{00ad}', '\u{00ad}'),
];
const DOUBLE_QUOTES: &[(char, char)] = &[
    ('\u{201c}', '\u{201f}'),
    ('\u{301d}', '\u{301f}'),
    ('\u{ff02}', '\u{ff02}'),
];
const SINGLE_QUOTES: &[(char, char)] = &[('\u{2018}', '\u{201b}'), ('\u{ff07}', '\u{ff07}')];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Tabs {
    #[default]
    Keep,
    EscapeWithCtrlV,
    Spaces(usize),
}

// The Transforms of iTerm2's Advanced Paste, with its defaults for a normal paste.
#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub remove_newlines: bool,
    pub convert_newlines: bool,
    pub unicode_punctuation: bool,
    pub remove_controls: bool,
    pub tabs: Tabs,
    pub escape_shell: bool,
    pub regex: String,
    pub substitution: String,
    pub base64: bool,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            remove_newlines: false,
            convert_newlines: true,
            unicode_punctuation: false,
            remove_controls: true,
            tabs: Tabs::Keep,
            escape_shell: false,
            regex: String::new(),
            substitution: String::new(),
            base64: false,
        }
    }
}

// iTermPasteHelper.m +sanitizePasteEvent:encoding:, in its order.
pub fn apply(text: &str, transform: &Transform) -> String {
    let mut text = if transform.remove_newlines {
        text.replace(['\r', '\n'], "")
    } else if transform.convert_newlines {
        text.replace("\r\n", "\r").replace('\n', "\r")
    } else {
        text.to_string()
    };
    if transform.unicode_punctuation {
        text = text
            .chars()
            .map(|c| {
                if within(c, DOUBLE_QUOTES) {
                    '"'
                } else if within(c, SINGLE_QUOTES) {
                    '\''
                } else if within(c, DASHES) {
                    '-'
                } else {
                    c
                }
            })
            .collect();
    }
    if transform.remove_controls {
        // Tab, line feed, form feed and carriage return stay.
        text.retain(|c| !(c < ' ' && !matches!(c, '\t' | '\n' | '\x0c' | '\r')));
    }
    text = match transform.tabs {
        Tabs::Keep => text,
        Tabs::Spaces(count) => text.replace('\t', &" ".repeat(count)),
        Tabs::EscapeWithCtrlV => text.replace('\t', &format!("{LNEXT}\t")),
    };
    if transform.escape_shell {
        text = if transform.tabs == Tabs::EscapeWithCtrlV {
            text.split('\t')
                .map(shell_escaped)
                .collect::<Vec<_>>()
                .join("\t")
        } else {
            shell_escaped(&text)
        };
    }
    if !transform.regex.is_empty()
        && let Ok(regex) = regex::Regex::new(&transform.regex)
    {
        text = regex
            .replace_all(&text, vim_escapes(&transform.substitution).as_str())
            .into_owned();
    }
    if transform.base64 {
        let encoded = STANDARD.encode(text.as_bytes());
        text = encoded
            .as_bytes()
            .chunks(BASE64_LINE)
            .map(|line| format!("{}\r", String::from_utf8_lossy(line)))
            .collect();
    }
    text
}

fn within(c: char, ranges: &[(char, char)]) -> bool {
    ranges.iter().any(|(low, high)| (*low..=*high).contains(&c))
}

// NSString+iTerm.m -stringByExpandingVimSpecialCharacters: \n, \r, \t and \\.
fn vim_escapes(substitution: &str) -> String {
    let mut expanded = String::new();
    let mut chars = substitution.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            expanded.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => expanded.push('\n'),
            Some('r') => expanded.push('\r'),
            Some('t') => expanded.push('\t'),
            Some('\\') => expanded.push('\\'),
            Some(other) => {
                expanded.push('\\');
                expanded.push(other);
            }
            None => expanded.push('\\'),
        }
    }
    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_turn_newlines_into_returns_and_drop_unsafe_control_codes() {
        let text = "uno\r\ndos\n\x07tres\tcuatro\x1b";
        assert_eq!(apply(text, &Transform::default()), "uno\rdos\rtres\tcuatro");
    }

    #[test]
    fn transforms_run_in_iterm2s_order() {
        let transform = Transform {
            unicode_punctuation: true,
            tabs: Tabs::Spaces(2),
            escape_shell: true,
            ..Transform::default()
        };
        assert_eq!(apply("“a b”\t—", &transform), "\\\"a\\ b\\\"\\ \\ -");
        let ctrl_v = Transform {
            tabs: Tabs::EscapeWithCtrlV,
            escape_shell: true,
            ..Transform::default()
        };
        assert_eq!(apply("a b\tc", &ctrl_v), "a\\ b\u{16}\tc");
        let removed = Transform {
            remove_newlines: true,
            regex: "(\\w+)@".into(),
            substitution: "<$1>\\t".into(),
            ..Transform::default()
        };
        assert_eq!(apply("yo@\nexample", &removed), "<yo>\texample");
        let base64 = Transform {
            base64: true,
            ..Transform::default()
        };
        assert_eq!(apply("hola", &base64), "aG9sYQ==\r");
    }
}
