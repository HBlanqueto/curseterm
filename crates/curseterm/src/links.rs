use std::ops::Range;
use std::process::Command;
use std::sync::LazyLock;
use std::thread;

use regex::Regex;

static URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:https?|ftp|file)://[^\s<>"'`]+|mailto:[^\s<>"'`]+"#)
        .expect("the URL pattern is valid")
});

pub fn url_span(text: &str, columns: &[usize], column: usize) -> Option<Range<usize>> {
    URL.find_iter(text)
        .find(|found| columns[found.start()] <= column && column <= columns[found.end() - 1])
        .map(|found| {
            let kept = found.as_str().trim_end_matches(['.', ',', ';', ':', ')']);
            found.start()..found.start() + kept.len()
        })
}

pub fn url_at(text: &str, columns: &[usize], column: usize) -> Option<String> {
    url_span(text, columns, column).map(|span| text[span].to_string())
}

pub fn word_span(text: &str, columns: &[usize], column: usize) -> Option<Range<usize>> {
    let mut start = 0;
    for word in text.split(char::is_whitespace) {
        let end = start + word.len();
        let covers = !word.is_empty() && columns[start] <= column && column <= columns[end - 1];
        if covers {
            let trimmed = word.trim_end_matches(['.', ',', ';', ')', '\'', '"']);
            let kept = trimmed.trim_start_matches(['(', '\'', '"']);
            let first = start + trimmed.len() - kept.len();
            return (!kept.is_empty()).then_some(first..first + kept.len());
        }
        start = end + text[end..].chars().next().map_or(0, char::len_utf8);
    }
    None
}

pub fn open(target: &str) {
    match Command::new("xdg-open").arg(target).spawn() {
        Ok(mut child) => {
            thread::spawn(move || child.wait());
        }
        Err(error) => tracing::warn!(%error, "could not run xdg-open"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_url_under_the_column_is_found_without_trailing_punctuation() {
        let text = "ver https://example.org/a, y mailto:yo@example.org.";
        let columns: Vec<usize> = (0..text.len()).collect();
        assert_eq!(
            url_at(text, &columns, 10),
            Some("https://example.org/a".to_string())
        );
        assert_eq!(
            url_at(text, &columns, 32),
            Some("mailto:yo@example.org".to_string())
        );
        assert_eq!(url_at(text, &columns, 1), None);
    }

    #[test]
    fn a_smart_selection_spans_the_url_or_the_quoted_word() {
        let text = "ver (https://a.org/x). y 'src/b.rs:9'";
        let columns: Vec<usize> = (0..text.len()).collect();
        assert_eq!(
            url_span(text, &columns, 8).map(|span| &text[span]),
            Some("https://a.org/x")
        );
        assert_eq!(
            word_span(text, &columns, 30).map(|span| &text[span]),
            Some("src/b.rs:9")
        );
    }
}
