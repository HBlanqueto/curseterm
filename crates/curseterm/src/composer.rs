use std::ops::Range;
use std::time::Instant;

#[derive(Clone, Default, PartialEq)]
pub struct Composer {
    pub open: bool,
    pub text: String,
    pub closed: Option<Instant>,
}

impl Composer {
    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open = true;
            self.closed = None;
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.closed = Some(Instant::now());
    }

    pub fn send_all(&mut self) -> Option<String> {
        self.open = false;
        self.closed = None;
        (!self.text.is_empty()).then(|| std::mem::take(&mut self.text))
    }

    pub fn take_command(&mut self, cursor_utf16: usize) -> Option<String> {
        let range = command_at(&self.text, cursor_utf16)?;
        let command = self.text[range.clone()].trim_end_matches('\n').to_string();
        self.text.replace_range(range, "");
        Some(command)
    }
}

// ComposerTextView.swift -characterRangeOfCommand(atCharacterIndex:): the paragraph at the
// cursor, joined with its neighbours through trailing-backslash continuations.
fn command_at(text: &str, cursor_utf16: usize) -> Option<Range<usize>> {
    let mut seen = 0;
    let at = text
        .char_indices()
        .find(|(_, c)| {
            seen += c.len_utf16();
            seen > cursor_utf16
        })
        .map_or(text.len(), |(index, _)| index);
    let paragraphs: Vec<Range<usize>> = text
        .split_inclusive('\n')
        .scan(0, |start, line| {
            let range = *start..*start + line.len();
            *start = range.end;
            Some(range)
        })
        .collect();
    let continues =
        |range: &Range<usize>| text[range.clone()].trim_end_matches('\n').ends_with('\\');
    let current = paragraphs
        .iter()
        .position(|range| range.contains(&at))
        .or_else(|| paragraphs.len().checked_sub(1))?;
    let first = (0..current)
        .rev()
        .take_while(|index| continues(&paragraphs[*index]))
        .last()
        .unwrap_or(current);
    let last = (current..paragraphs.len())
        .find(|index| !continues(&paragraphs[*index]))
        .unwrap_or(paragraphs.len() - 1);
    Some(paragraphs[first].start..paragraphs[last].end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_at_the_cursor_is_its_line_joined_through_backslash_continuations() {
        let text = "ls\necho \\\n  ñandú\npwd";
        let mut composer = Composer {
            text: text.into(),
            ..Composer::default()
        };
        let inside_ñandú = "ls\necho \\\n  ñan".encode_utf16().count();
        assert_eq!(
            composer.take_command(inside_ñandú).as_deref(),
            Some("echo \\\n  ñandú")
        );
        assert_eq!(composer.text, "ls\npwd");
        assert_eq!(composer.take_command(99).as_deref(), Some("pwd"));
        assert_eq!(composer.text, "ls\n");
    }
}
