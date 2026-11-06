use freya_core::prelude::State;

use crate::advanced;

thread_local! {
    static SHARED: State<Vec<String>> = State::create_global(Vec::new());
}

pub fn shared() -> State<Vec<String>> {
    SHARED.with(|history| *history)
}

pub fn remember(history: &mut Vec<String>, text: String) {
    if text.is_empty() {
        return;
    }
    history.retain(|entry| *entry != text);
    history.insert(0, text);
    history.truncate(advanced::current().paste_history_limit());
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
        for index in 0..=advanced::current().paste_history_limit() {
            remember(&mut history, index.to_string());
        }
        remember(&mut history, "5".to_string());
        assert_eq!(history.len(), advanced::current().paste_history_limit());
        assert_eq!(history[0], "5");
        assert_eq!(history.iter().filter(|entry| *entry == "5").count(), 1);
        assert!(!history.contains(&"0".to_string()));
    }

    #[test]
    fn lines_join_with_a_return_sign() {
        assert_eq!(one_line("a\r\nb\n"), "a ⏎ b");
    }
}
