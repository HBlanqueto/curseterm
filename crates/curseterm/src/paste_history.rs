const LIMIT: usize = 20;

pub fn remember(history: &mut Vec<String>, text: String) {
    if text.is_empty() {
        return;
    }
    history.retain(|entry| *entry != text);
    history.insert(0, text);
    history.truncate(LIMIT);
}

pub fn matching(history: &[String], query: &str) -> Vec<String> {
    let query = query.to_lowercase();
    history
        .iter()
        .filter(|entry| entry.to_lowercase().contains(&query))
        .cloned()
        .collect()
}

pub fn one_line(text: &str) -> String {
    text.split(['\r', '\n'])
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ⏎ ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repeated_text_moves_to_the_front_and_the_oldest_falls_off() {
        let mut history = Vec::new();
        for index in 0..=LIMIT {
            remember(&mut history, index.to_string());
        }
        remember(&mut history, "5".to_string());
        assert_eq!(history.len(), LIMIT);
        assert_eq!(history[0], "5");
        assert_eq!(history.iter().filter(|entry| *entry == "5").count(), 1);
        assert!(!history.contains(&"0".to_string()));
    }

    #[test]
    fn the_filter_ignores_case_and_lines_join_with_a_return_sign() {
        let history = vec!["git Push".to_string(), "ls".to_string()];
        assert_eq!(matching(&history, "push"), vec!["git Push".to_string()]);
        assert_eq!(one_line("a\r\nb\n"), "a ⏎ b");
    }
}
