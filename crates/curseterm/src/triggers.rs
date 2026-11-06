use std::time::{Duration, Instant};

use alacritty_terminal::vte::ansi::Rgb;
use barrier_kit_next::tokens::HIGHLIGHT_COLORS;
use regex::Regex;
use vow_design::Rgba;

use crate::advanced;
use crate::config::{Trigger, TriggerAction};

// PTYTriggerEvaluator.m kMinimumPartialLineTriggerCheckInterval.
pub const PARTIAL_LINE_INTERVAL: Duration = Duration::from_millis(500);
// iTermAddTriggerViewController defaultBackgroundColor: a new highlight paints red behind the text.
const DEFAULT_HIGHLIGHT_SWATCH: usize = 5;

pub struct Compiled {
    pub index: usize,
    pub regex: Regex,
    pub action: TriggerAction,
    pub parameter: String,
    pub instant: bool,
    pub disabled: bool,
    pub last_line: Option<i64>,
    pub last_popup: Option<Instant>,
}

pub fn compile(triggers: &[Trigger]) -> Vec<Compiled> {
    triggers
        .iter()
        .enumerate()
        .filter(|(_, trigger)| trigger.enabled)
        .filter_map(|(index, trigger)| {
            Some(Compiled {
                index,
                regex: Regex::new(&trigger.regex).ok()?,
                action: trigger.action,
                parameter: trigger.parameter.clone(),
                instant: trigger.instant,
                disabled: false,
                last_line: None,
                last_popup: None,
            })
        })
        .collect()
}

impl Compiled {
    // Trigger.m -tryString:…partialLine:: an instant trigger fires once per line, except the
    // highlights, which keep up with the line as it grows.
    pub fn should_try(&mut self, line: i64, partial: bool) -> bool {
        if self.disabled || (partial && !self.instant) {
            return false;
        }
        let repeats = matches!(
            self.action,
            TriggerAction::Highlight | TriggerAction::HighlightLine
        );
        if self.instant && !repeats && self.last_line == Some(line) {
            if !partial {
                self.last_line = None;
            }
            return false;
        }
        true
    }

    // iTermAdvancedSettingsModel alertTriggerRateLimit and userNotificationTriggerRateLimit.
    pub fn popup_allowed(&mut self, now: Instant) -> bool {
        let advanced = advanced::current();
        let limit = match self.action {
            TriggerAction::Alert => advanced.alert_rate_limit(),
            _ => advanced.notification_rate_limit(),
        };
        let allowed = self
            .last_popup
            .is_none_or(|last| now.duration_since(last) >= limit);
        if allowed {
            self.last_popup = Some(now);
        }
        allowed
    }
}

// Trigger.m -promisedValueOfInterpolatedString:withBackreferencesReplacedWithValues: \0 to \9,
// then \a \b \e \n \r \t \\ and \xNN.
pub fn expand(parameter: &str, groups: &[&str]) -> String {
    let mut expanded = String::new();
    let mut chars = parameter.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            expanded.push(c);
            continue;
        }
        match chars.next() {
            Some(digit @ '0'..='9') => {
                let index = digit as usize - '0' as usize;
                expanded.push_str(groups.get(index).copied().unwrap_or_default());
            }
            Some('a') => expanded.push('\x07'),
            Some('b') => expanded.push('\x08'),
            Some('e') => expanded.push('\x1b'),
            Some('n') => expanded.push('\n'),
            Some('r') => expanded.push('\r'),
            Some('t') => expanded.push('\t'),
            Some('\\') => expanded.push('\\'),
            Some('x') => {
                let hex: String = chars.by_ref().take(2).collect();
                match u8::from_str_radix(&hex, 16) {
                    Ok(byte) => expanded.push(char::from(byte)),
                    Err(_) => {
                        expanded.push_str("\\x");
                        expanded.push_str(&hex);
                    }
                }
            }
            Some(other) => {
                expanded.push('\\');
                expanded.push(other);
            }
            None => expanded.push('\\'),
        }
    }
    expanded
}

// HighlightTrigger.m -stringForTextColor:backgroundColor:: "{#rrggbb,#rrggbb}", either side
// empty to keep the cell's own colour.
pub fn colors(parameter: &str) -> (Option<Rgb>, Option<Rgb>) {
    let Some(inner) = parameter
        .trim()
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    else {
        return (None, Some(rgb(HIGHLIGHT_COLORS[DEFAULT_HIGHLIGHT_SWATCH])));
    };
    let (text, background) = inner.split_once(',').unwrap_or((inner, ""));
    (hex(text), hex(background))
}

pub fn colors_parameter(text: Option<Rgb>, background: Option<Rgb>) -> String {
    let side = |color: Option<Rgb>| {
        color.map_or(String::new(), |Rgb { r, g, b }| {
            format!("#{r:02x}{g:02x}{b:02x}")
        })
    };
    format!("{{{},{}}}", side(text), side(background))
}

pub fn rgb(color: Rgba) -> Rgb {
    let channel = |value: f32| (value * 255.0).round() as u8;
    Rgb {
        r: channel(color.r),
        g: channel(color.g),
        b: channel(color.b),
    }
}

fn hex(side: &str) -> Option<Rgb> {
    let digits = side.trim().strip_prefix('#')?;
    let channel = |at: usize| u8::from_str_radix(digits.get(at..at + 2)?, 16).ok();
    Some(Rgb {
        r: channel(0)?,
        g: channel(2)?,
        b: channel(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backreferences_and_escapes_expand_like_iterm2() {
        assert_eq!(
            expand(r"got \1 of \0\n\x41\\\z", &["error 42", "42"]),
            "got 42 of error 42\nA\\\\z"
        );
        assert_eq!(expand(r"\3", &["only"]), "");
    }

    #[test]
    fn highlight_colours_round_trip_and_an_empty_side_keeps_the_cell() {
        let text = Rgb {
            r: 255,
            g: 255,
            b: 0,
        };
        let parameter = colors_parameter(Some(text), None);
        assert_eq!(parameter, "{#ffff00,}");
        assert_eq!(colors(&parameter), (Some(text), None));
        assert_eq!(colors(""), (None, Some(Rgb { r: 255, g: 0, b: 0 })));
    }

    #[test]
    fn an_instant_trigger_fires_once_per_line_until_the_line_ends() {
        let triggers = [Trigger {
            regex: "x".into(),
            action: TriggerAction::Bell,
            instant: true,
            ..Trigger::default()
        }];
        let mut compiled = compile(&triggers).remove(0);
        assert!(compiled.should_try(3, true));
        compiled.last_line = Some(3);
        assert!(!compiled.should_try(3, true));
        assert!(!compiled.should_try(3, false));
        assert!(compiled.should_try(4, false));
    }
}
