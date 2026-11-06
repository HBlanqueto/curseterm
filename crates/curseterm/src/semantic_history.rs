use serde::{Deserialize, Serialize};

// iTermSemanticHistoryPrefsController.m kSemanticHistory…Action, in its pop-up's order; the editor
// choices are macOS apps and are left out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    #[default]
    DefaultApp,
    Url,
    Command,
    RawCommand,
    Coprocess,
    SendText,
}

impl Action {
    pub const ALL: [Self; 6] = [
        Self::DefaultApp,
        Self::Url,
        Self::Command,
        Self::RawCommand,
        Self::Coprocess,
        Self::SendText,
    ];
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SemanticHistory {
    pub action: Action,
    pub text: String,
}

#[derive(Debug, Default, PartialEq)]
pub struct Target {
    pub path: String,
    pub line: String,
    pub column: String,
    pub prefix: String,
    pub suffix: String,
    pub directory: String,
}

// iTermPathCleaner.m: a trailing :line or :line:column is the position, not the name.
pub fn split_position(word: &str) -> (&str, &str, &str) {
    let mut parts = word.rsplitn(3, ':');
    let last = parts.next().unwrap_or_default();
    let middle = parts.next();
    let first = parts.next();
    let number = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    match (first, middle) {
        (Some(path), Some(line)) if number(line) && number(last) => (path, line, last),
        (_, Some(path)) if number(last) => {
            let path = first.map_or(path, |_| &word[..word.len() - last.len() - 1]);
            (path, last, "")
        }
        _ => (word, "", ""),
    }
}

// iTermSemanticHistoryController.m -numericSubstitutions: \1 path, \2 line, \3 text before,
// \4 text after and \5 working directory.
pub fn substitute(template: &str, target: &Target, escape: impl Fn(&str) -> String) -> String {
    let mut text = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        let value = match (c, chars.peek()) {
            ('\\', Some('1')) => Some(escape(&target.path)),
            ('\\', Some('2')) => Some(target.line.clone()),
            ('\\', Some('3')) => Some(escape(&target.prefix)),
            ('\\', Some('4')) => Some(escape(&target.suffix)),
            ('\\', Some('5')) => Some(escape(&target.directory)),
            _ => None,
        };
        match value {
            Some(value) => {
                chars.next();
                text.push_str(&value);
            }
            None => text.push(c),
        }
    }
    text
}

// NSCharacterSet URLPathAllowedCharacterSet.
pub fn percent_encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@/".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_position_after_a_file_name_is_split_off() {
        assert_eq!(
            split_position("src/main.rs:12:5"),
            ("src/main.rs", "12", "5")
        );
        assert_eq!(split_position("main.rs:12"), ("main.rs", "12", ""));
        assert_eq!(split_position("notes.txt"), ("notes.txt", "", ""));
        assert_eq!(split_position("a:b"), ("a:b", "", ""));
    }

    #[test]
    fn substitutions_fill_the_template_and_escape_what_they_insert() {
        let target = Target {
            path: "/tmp/a b.rs".into(),
            line: "7".into(),
            directory: "/tmp".into(),
            ..Target::default()
        };
        assert_eq!(
            substitute(r"vim +\2 \1 # in \5", &target, |text| text
                .replace(' ', "\\ ")),
            r"vim +7 /tmp/a\ b.rs # in /tmp"
        );
        assert_eq!(percent_encoded("/tmp/a b.rs"), "/tmp/a%20b.rs");
    }
}
