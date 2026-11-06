use std::time::Instant;

use crate::session::SessionId;

// iTermAdvancedSettingsModel.m findUrlsRegex.
pub const URL_PATTERN: &str = r"https?://([a-z0-9A-Z]+(:[a-zA-Z0-9]+)?@)?[a-z0-9A-Z\-]+(\.[a-z0-9A-Z\-]+)*((:[0-9]+)?)(/[a-zA-Z0-9;:/\.\-_+%~?&@=#\(\)]*)?";

// SmartSelectionRules.plist, the rules of normal precision or more joined as
// PTYSession.m -regularExpressonForNonLowPrecisionSmartSelectionRulesCombined does. The engine has
// no look-behind, POSIX letter classes nor Unicode word boundaries: `\p{L}` and `\p{N}` stand for
// the classes, the boundaries are ASCII ones, and the HTTP URL rule keeps its branch without the
// look-behind.
const SMART_RULES: [&str; 10] = [
    r"([a-zA-Z0-9_]+::)+[a-zA-Z0-9_]+",
    r"\~?/?([\p{L}\p{N}._-]+/+)+[\p{L}\p{N}._-]+/?",
    r#"@?"(?:[^"\\]|\\.)*""#,
    r"([\p{L}\p{N}._]+\.)+[\p{L}\p{N}._]+",
    r"(?-u:\b)mailto:([a-z0-9A-Z_]+@)?([a-zA-Z0-9\-]+\.)*[a-zA-Z0-9\-]+(?-u:\b)",
    r"@selector\([^)]+\)",
    r"(?-u:\b)[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,4}(?-u:\b)",
    r"https?://([a-z0-9A-Z]+(:[a-zA-Z0-9]+)?@)?([a-z0-9A-Z][-a-z0-9A-Z]*\.)*[A-Za-z][-A-Za-z]*((:[0-9]+)?)(/[a-zA-Z0-9;/.\-_+%?&@=#~(]*)?",
    r"(?-u:\b)ssh:([a-z0-9A-Z_]+@)?([a-zA-Z0-9\-]+\.)*[a-zA-Z0-9\-]+(?-u:\b)",
    r"(?-u:\b)telnet:([a-z0-9A-Z_]+@)?([a-zA-Z0-9\-]+\.)*[a-zA-Z0-9\-]+(?-u:\b)",
];

pub fn smart_pattern() -> String {
    SMART_RULES.map(|rule| format!("(?:{rule})")).join("|")
}

#[derive(Clone, Copy, PartialEq)]
pub enum HintAction {
    Open,
    Copy,
}

pub type Found = ((i64, usize), (i64, usize), String);

#[derive(Clone, PartialEq)]
pub struct Hint {
    pub start: (i64, usize),
    pub end: (i64, usize),
    pub key: String,
    pub text: String,
}

#[derive(Clone, PartialEq)]
pub struct Hints {
    pub session: SessionId,
    pub items: Vec<Hint>,
    pub action: HintAction,
    pub prefix: String,
    pub shown: Instant,
    pub narrowed: Option<Instant>,
    pub ended: Option<(Instant, Option<usize>)>,
}

pub enum Typed {
    Chosen(usize),
    Narrowed,
    Unhandled,
}

impl Hints {
    pub fn new(session: SessionId, found: Vec<Found>, action: HintAction) -> Self {
        let keys = keys(found.len());
        Self {
            session,
            items: found
                .into_iter()
                .zip(keys)
                .map(|((start, end, text), key)| Hint {
                    start,
                    end,
                    key,
                    text,
                })
                .collect(),
            action,
            prefix: String::new(),
            shown: Instant::now(),
            narrowed: None,
            ended: None,
        }
    }

    pub fn end(&mut self, chosen: Option<usize>) {
        self.ended.get_or_insert((Instant::now(), chosen));
    }

    // iTermCopyModeHandler.m -validKeyEquivalentForEvent: prefix plus key, then the key alone,
    // then a prefix of some shortcut; anything else goes on to the terminal.
    pub fn type_key(&mut self, typed: &str) -> Typed {
        let candidate = format!("{}{typed}", self.prefix);
        let exact = |wanted: &str| {
            self.items
                .iter()
                .position(|hint| hint.key.eq_ignore_ascii_case(wanted))
        };
        if let Some(index) = exact(&candidate).or_else(|| exact(typed)) {
            self.prefix.clear();
            return Typed::Chosen(index);
        }
        let narrows = self.items.iter().any(|hint| {
            hint.key
                .to_lowercase()
                .starts_with(&candidate.to_lowercase())
        });
        if narrows {
            self.prefix = candidate;
            self.narrowed = Some(Instant::now());
            return Typed::Narrowed;
        }
        Typed::Unhandled
    }
}

// PTYTextView+ARC.m: 1 to 9 first, then letters, as many as 26 to the power covers the count.
fn keys(count: usize) -> Vec<String> {
    let mut digits = 1;
    let mut cardinality = 26;
    while cardinality < count {
        digits += 1;
        cardinality *= 26;
    }
    (1..=count)
        .map(|index| {
            if index < 10 {
                return index.to_string();
            }
            let mut rest = index - 10;
            let mut key = String::new();
            while key.len() < digits {
                key.insert(0, char::from(b'A' + (rest % 26) as u8));
                rest /= 26;
            }
            key
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::term::search::RegexSearch;

    use super::*;

    #[test]
    fn the_url_and_smart_selection_expressions_compile_in_the_terminal_search() {
        assert!(RegexSearch::new(URL_PATTERN).is_ok());
        assert!(RegexSearch::new(&smart_pattern()).is_ok());
    }

    #[test]
    fn shortcuts_are_digits_then_letters_and_a_prefix_waits_for_the_next_key() {
        assert_eq!(
            keys(11),
            ["1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B"]
        );
        let found = (0..40)
            .map(|line| ((line, 0), (line, 5), format!("u{line}")))
            .collect();
        let mut hints = Hints::new(1, found, HintAction::Open);
        assert_eq!(hints.items[9].key, "AA");
        assert!(matches!(hints.type_key("b"), Typed::Narrowed));
        assert!(matches!(hints.type_key("a"), Typed::Chosen(35)));
        assert!(matches!(hints.type_key("7"), Typed::Chosen(6)));
        assert!(matches!(hints.type_key("z"), Typed::Unhandled));
    }
}
