use barrier_kit::menu::MenuItem;
use serde::{Deserialize, Serialize};

use crate::localize::tr;

pub const SNIPPET_PREFIX: &str = "snippet-";
const DISPLAY_TITLE_CHARS: usize = 30;

// iTermEditSnippetWindowController.xib's Escaping pop-up; new snippets use C-style escaping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Escaping {
    Literal,
    Compatibility,
    #[default]
    CStyle,
}

impl Escaping {
    pub const ALL: [Self; 3] = [Self::Literal, Self::Compatibility, Self::CStyle];
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Snippet {
    pub title: String,
    pub value: String,
    pub escaping: Escaping,
    pub tags: Vec<String>,
}

impl Snippet {
    // iTermSnippetsModel.m -displayTitle
    pub fn display_title(&self) -> String {
        if !self.title.is_empty() {
            return self.title.clone();
        }
        let mut shown: String = self.value.chars().take(DISPLAY_TITLE_CHARS).collect();
        if self.value.chars().count() > DISPLAY_TITLE_CHARS {
            shown.pop();
            shown.push('…');
        }
        shown
    }

    // iTermKeyBindingAction.m +escapedText:mode:
    pub fn escaped(&self) -> String {
        let mut escaped = String::new();
        let mut chars = self.value.chars().peekable();
        while let Some(c) = chars.next() {
            let Some(&next) = chars.peek().filter(|_| c == '\\') else {
                escaped.push(c);
                continue;
            };
            let control = match (self.escaping, next) {
                (Escaping::Literal, _) => None,
                (_, 'n') => Some('\n'),
                (_, 'e') => Some('\x1b'),
                (_, 'a') => Some('\x07'),
                (_, 't') => Some('\t'),
                (Escaping::CStyle, 'b') => Some('\x08'),
                (Escaping::CStyle, 'r') => Some('\r'),
                (Escaping::CStyle, '\\') => Some('\\'),
                (Escaping::CStyle, 'x') => hex_byte(&mut chars),
                _ => None,
            };
            match control {
                Some(control) => {
                    escaped.push(control);
                    if next != 'x' {
                        chars.next();
                    }
                }
                // An escaped backslash never starts an escape of its own.
                None if next == '\\' => {
                    escaped.push_str("\\\\");
                    chars.next();
                }
                None => escaped.push(c),
            }
        }
        escaped
    }
}

// NSString+iTerm.m -stringByReplacingEscapedHexValuesWithChars: \x and two hex digits.
fn hex_byte(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<char> {
    let ahead: String = chars.clone().take(3).collect();
    let digits = ahead.strip_prefix('x').filter(|digits| digits.len() == 2)?;
    let value = u8::from_str_radix(digits, 16).ok()?;
    chars.nth(2);
    Some(char::from(value))
}

// iTermSnippetsMenuController.m -reload: untagged snippets first, then a folder per tag, split
// at `/` and sorted by name.
pub fn menu_items(snippets: &[Snippet]) -> Vec<MenuItem> {
    let hint = MenuItem::action("snippets-hint", tr!("snippets-hint")).with_enabled(false);
    let untagged = snippets
        .iter()
        .enumerate()
        .filter(|(_, snippet)| snippet.tags.is_empty())
        .map(|(index, snippet)| item(index, snippet));
    let mut tree = Folder::default();
    for (index, snippet) in snippets.iter().enumerate() {
        for tag in &snippet.tags {
            tree.add(&tag.split('/').collect::<Vec<_>>(), index);
        }
    }
    let mut items: Vec<MenuItem> = [hint, MenuItem::Separator]
        .into_iter()
        .chain(untagged)
        .collect();
    if !tree.folders.is_empty() {
        items.push(MenuItem::Separator);
        items.extend(tree.items(snippets));
    }
    items
}

fn item(index: usize, snippet: &Snippet) -> MenuItem {
    MenuItem::action(format!("{SNIPPET_PREFIX}{index}"), snippet.display_title())
}

#[derive(Default)]
struct Folder {
    folders: Vec<(String, Folder)>,
    snippets: Vec<usize>,
}

impl Folder {
    fn add(&mut self, path: &[&str], index: usize) {
        let Some((name, rest)) = path.split_first() else {
            self.snippets.push(index);
            return;
        };
        let position = match self.folders.iter().position(|(other, _)| other == name) {
            Some(position) => position,
            None => {
                self.folders.push((name.to_string(), Folder::default()));
                self.folders.len() - 1
            }
        };
        self.folders[position].1.add(rest, index);
    }

    fn items(&self, snippets: &[Snippet]) -> Vec<MenuItem> {
        let mut folders: Vec<&(String, Folder)> = self.folders.iter().collect();
        folders.sort_by(|left, right| left.0.cmp(&right.0));
        folders
            .into_iter()
            .map(|(name, folder)| {
                let mut items = folder.items(snippets);
                if !items.is_empty() && !folder.snippets.is_empty() {
                    items.push(MenuItem::Separator);
                }
                items.extend(
                    folder
                        .snippets
                        .iter()
                        .map(|index| item(*index, &snippets[*index])),
                );
                MenuItem::Submenu {
                    id: None,
                    label: name.clone(),
                    enabled: true,
                    items,
                    icon: None,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(value: &str, escaping: Escaping) -> Snippet {
        Snippet {
            value: value.into(),
            escaping,
            ..Snippet::default()
        }
    }

    #[test]
    fn each_escaping_expands_what_iterm2_does() {
        let text = r"a\nb\rc\x41\\n\q";
        assert_eq!(snippet(text, Escaping::Literal).escaped(), text);
        assert_eq!(
            snippet(text, Escaping::Compatibility).escaped(),
            "a\nb\\rc\\x41\\\\n\\q"
        );
        assert_eq!(snippet(text, Escaping::CStyle).escaped(), "a\nb\rcA\\n\\q");
        assert_eq!(
            snippet(r"\e[1m\x4", Escaping::CStyle).escaped(),
            "\x1b[1m\\x4"
        );
    }

    #[test]
    fn an_untitled_snippet_shows_its_value_cut_to_thirty_characters() {
        let long = snippet(&"x".repeat(40), Escaping::CStyle);
        assert_eq!(long.display_title(), format!("{}…", "x".repeat(29)));
    }
}
