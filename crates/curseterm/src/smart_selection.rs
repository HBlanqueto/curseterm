use std::ops::Range;

use regex::Regex;
use serde::{Deserialize, Serialize};

// SmartSelectionController.m kVeryLowPrecision… and SmartSelectionVeryLowPrecision….
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Precision {
    VeryLow,
    Low,
    #[default]
    Normal,
    High,
    VeryHigh,
}

impl Precision {
    pub const ALL: [Self; 5] = [
        Self::VeryLow,
        Self::Low,
        Self::Normal,
        Self::High,
        Self::VeryHigh,
    ];

    fn weight(self) -> f64 {
        match self {
            Self::VeryLow => 0.00001,
            Self::Low => 0.001,
            Self::Normal => 1.0,
            Self::High => 1000.0,
            Self::VeryHigh => 1_000_000.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rule {
    pub notes: String,
    pub regex: String,
    pub precision: Precision,
}

// SmartSelectionRules.plist. The engine has no look-behind, POSIX letter classes nor Unicode word
// boundaries: `\p{L}` and `\p{N}` stand for the classes, the boundaries are ASCII ones, and the
// HTTP URL rule keeps its branch without the look-behind.
const DEFAULT_RULES: [(&str, &str, Precision); 11] = [
    ("Word bounded by whitespace", r"\S+", Precision::Low),
    (
        "C++ namespace::identifier",
        r"([a-zA-Z0-9_]+::)+[a-zA-Z0-9_]+",
        Precision::Normal,
    ),
    (
        "Paths",
        r"\~?/?([\p{L}\p{N}._-]+/+)+[\p{L}\p{N}._-]+/?",
        Precision::Normal,
    ),
    ("Quoted string", r#"@?"(?:[^"\\]|\\.)*""#, Precision::Normal),
    (
        "Java/Python include paths",
        r"([\p{L}\p{N}._]+\.)+[\p{L}\p{N}._]+",
        Precision::Normal,
    ),
    (
        "mailto URL",
        r"(?-u:\b)mailto:([a-z0-9A-Z_]+@)?([a-zA-Z0-9\-]+\.)*[a-zA-Z0-9\-]+(?-u:\b)",
        Precision::Normal,
    ),
    ("Obj-C selector", r"@selector\([^)]+\)", Precision::High),
    (
        "email address",
        r"(?-u:\b)[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,4}(?-u:\b)",
        Precision::High,
    ),
    (
        "HTTP URL",
        r"https?://([a-z0-9A-Z]+(:[a-zA-Z0-9]+)?@)?([a-z0-9A-Z][-a-z0-9A-Z]*\.)*[A-Za-z][-A-Za-z]*((:[0-9]+)?)(/[a-zA-Z0-9;/.\-_+%?&@=#~(]*)?",
        Precision::VeryHigh,
    ),
    (
        "SSH URL",
        r"(?-u:\b)ssh:([a-z0-9A-Z_]+@)?([a-zA-Z0-9\-]+\.)*[a-zA-Z0-9\-]+(?-u:\b)",
        Precision::VeryHigh,
    ),
    (
        "Telnet URL",
        r"(?-u:\b)telnet:([a-z0-9A-Z_]+@)?([a-zA-Z0-9\-]+\.)*[a-zA-Z0-9\-]+(?-u:\b)",
        Precision::VeryHigh,
    ),
];

pub fn default_rules() -> Vec<Rule> {
    DEFAULT_RULES
        .iter()
        .map(|(notes, regex, precision)| Rule {
            notes: notes.to_string(),
            regex: regex.to_string(),
            precision: *precision,
        })
        .collect()
}

// PTYSession.m -regularExpressonForNonLowPrecisionSmartSelectionRulesCombined.
pub fn combined_pattern(rules: &[Rule]) -> String {
    rules
        .iter()
        .filter(|rule| rule.precision.weight() >= Precision::Normal.weight())
        .map(|rule| format!("(?:{})", rule.regex))
        .collect::<Vec<_>>()
        .join("|")
}

pub fn compile(rules: &[Rule]) -> Vec<(Regex, Precision)> {
    rules
        .iter()
        .filter_map(|rule| Some((Regex::new(&rule.regex).ok()?, rule.precision)))
        .collect()
}

// iTermTextExtractor.m -smartSelectionAt:withRules:…: every match of every rule that covers the
// clicked character scores its precision times its length, and the best one wins.
pub fn best_match(text: &str, target: usize, rules: &[(Regex, Precision)]) -> Option<Range<usize>> {
    let mut best: Option<(f64, Range<usize>)> = None;
    for (regex, precision) in rules {
        let mut start = 0;
        while start <= target {
            let Some(found) = regex.find_at(text, start) else {
                break;
            };
            if found.start() > target {
                break;
            }
            if found.end() > target {
                let score = precision.weight() * found.as_str().chars().count() as f64;
                if best.as_ref().is_none_or(|(kept, _)| score > *kept) {
                    best = Some((score, found.range()));
                }
                start = found.end().max(start + 1);
            } else {
                start = found.start() + 1;
            }
            while !text.is_char_boundary(start) {
                start += 1;
            }
        }
    }
    best.map(|(_, range)| range)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_most_precise_match_under_the_click_wins_over_longer_vaguer_ones() {
        let rules = compile(&default_rules());
        let text = "see https://example.org/a,b and /usr/lib/x.so now";
        let at = |needle: &str| text.find(needle).unwrap() + 2;
        let picked = |target| &text[best_match(text, target, &rules).unwrap()];
        assert_eq!(picked(at("example")), "https://example.org/a");
        assert_eq!(picked(at("lib")), "/usr/lib/x.so");
        assert_eq!(picked(at("now")), "now");
        assert!(best_match("   ", 1, &rules).is_none());
    }
}
