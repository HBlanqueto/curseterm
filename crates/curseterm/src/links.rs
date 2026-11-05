use std::process::Command;
use std::sync::LazyLock;
use std::thread;

use regex::Regex;

static URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:https?|ftp|file)://[^\s<>"'`]+|mailto:[^\s<>"'`]+"#)
        .expect("the URL pattern is valid")
});

pub fn url_at(text: &str, columns: &[usize], column: usize) -> Option<String> {
    URL.find_iter(text)
        .find(|found| columns[found.start()] <= column && column <= columns[found.end() - 1])
        .map(|found| {
            found
                .as_str()
                .trim_end_matches(['.', ',', ';', ':', ')'])
                .to_string()
        })
}

pub fn word_at(text: &str, columns: &[usize], column: usize) -> Option<String> {
    let mut start = 0;
    for word in text.split(char::is_whitespace) {
        let end = start + word.len();
        let covers = !word.is_empty() && columns[start] <= column && column <= columns[end - 1];
        if covers {
            let word = word.trim_end_matches(['.', ',', ';', ')', '\'', '"']);
            let word = word.trim_start_matches(['(', '\'', '"']);
            let word = match word.split_once(':') {
                Some((path, line)) if line.starts_with(|c: char| c.is_ascii_digit()) => path,
                _ => word,
            };
            return (!word.is_empty()).then(|| word.to_string());
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
    fn the_word_under_the_column_loses_its_line_number_and_quotes() {
        let text = "error en src/main.rs:42:7, ver 'notas.txt'";
        let columns: Vec<usize> = (0..text.len()).collect();
        assert_eq!(word_at(text, &columns, 12), Some("src/main.rs".to_string()));
        assert_eq!(word_at(text, &columns, 36), Some("notas.txt".to_string()));
        assert_eq!(word_at(text, &columns, 6), Some("en".to_string()));
        assert_eq!(word_at(text, &columns, 5), None);
        assert_eq!(
            word_at("file2", &[0, 1, 2, 3, 4], 4),
            Some("file2".to_string())
        );
    }
}
