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
}
