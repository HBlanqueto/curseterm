use serde::{Deserialize, Serialize};

// iTermFindViewController.h iTermFindMode, in the order of FindView.xib's menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindMode {
    #[default]
    SmartCase,
    CaseSensitive,
    CaseInsensitive,
    RegexCaseSensitive,
    RegexCaseInsensitive,
}

impl FindMode {
    pub const ALL: [Self; 5] = [
        Self::SmartCase,
        Self::CaseSensitive,
        Self::CaseInsensitive,
        Self::RegexCaseSensitive,
        Self::RegexCaseInsensitive,
    ];

    // The inline flag fixes the case, so the pattern means the same to alacritty's smart-case
    // search and to the regex crate (iTermCoreSearch.m: smart case is sensitive with a capital).
    pub fn pattern(self, query: &str) -> String {
        let sensitive = match self {
            Self::SmartCase => query.chars().any(char::is_uppercase),
            Self::CaseSensitive | Self::RegexCaseSensitive => true,
            Self::CaseInsensitive | Self::RegexCaseInsensitive => false,
        };
        let flag = if sensitive { "(?-i)" } else { "(?i)" };
        match self {
            Self::RegexCaseSensitive | Self::RegexCaseInsensitive => format!("{flag}{query}"),
            _ => format!("{flag}{}", regex::escape(query)),
        }
    }
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::term::search::RegexSearch;

    use super::*;

    #[test]
    fn each_mode_matches_like_iterm2() {
        let matches = |mode: FindMode, query: &str, text: &str| {
            regex::Regex::new(&mode.pattern(query)).is_ok_and(|regex| regex.is_match(text))
        };
        assert!(matches(FindMode::SmartCase, "error", "ERROR here"));
        assert!(!matches(FindMode::SmartCase, "Error", "error here"));
        assert!(!matches(FindMode::CaseSensitive, "error", "ERROR"));
        assert!(matches(FindMode::CaseInsensitive, "Error", "error"));
        assert!(!matches(FindMode::CaseInsensitive, "a.c", "abc"));
        assert!(matches(FindMode::RegexCaseInsensitive, "a.C", "abc"));
        assert!(!matches(FindMode::RegexCaseSensitive, "a.C", "abc"));
        assert!(!matches(FindMode::RegexCaseSensitive, "(", "("));
        for mode in FindMode::ALL {
            assert!(RegexSearch::new(&mode.pattern("a.b")).is_ok());
        }
    }
}
