use freya_core::prelude::State;

// iTermShellHistoryController.m keeps every command; a session's window needs only the recent ones.
const LIMIT: usize = 1000;

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub command: String,
    pub uses: u32,
    pub last: u64,
}

thread_local! {
    static SHARED: State<Vec<Entry>> = State::create_global(Vec::new());
}

pub fn shared() -> State<Vec<Entry>> {
    SHARED.with(|history| *history)
}

// iTermShellHistoryController.m -addCommand:onHost:inDirectory:withMark:
pub fn remember(history: &mut Vec<Entry>, command: &str) {
    let command = command.trim();
    if command.is_empty() {
        return;
    }
    let last = history
        .iter()
        .map(|entry| entry.last)
        .max()
        .unwrap_or_default()
        + 1;
    match history.iter_mut().find(|entry| entry.command == command) {
        Some(entry) => {
            entry.uses += 1;
            entry.last = last;
        }
        None => history.push(Entry {
            command: command.to_string(),
            uses: 1,
            last,
        }),
    }
    if history.len() > LIMIT {
        history.sort_by_key(|entry| std::cmp::Reverse(entry.last));
        history.truncate(LIMIT);
    }
}

// CommandHistoryPopup.m -commandsForHost:partialCommand:expand: the commands that start with what
// was typed, the most used first and then the most recent.
pub fn candidates(history: &[Entry], prefix: &str) -> Vec<Entry> {
    let mut found: Vec<Entry> = history
        .iter()
        .filter(|entry| entry.command.starts_with(prefix) && entry.command != prefix)
        .cloned()
        .collect();
    found.sort_by(|left, right| right.uses.cmp(&left.uses).then(right.last.cmp(&left.last)));
    found
}

// Autocomplete.m: the words on the screen that continue the one before the cursor, nearest first.
pub fn words(lines_from_cursor: &[String], prefix: &str) -> Vec<String> {
    let mut found = Vec::new();
    if prefix.is_empty() {
        return found;
    }
    for line in lines_from_cursor {
        for word in line.split(|c: char| c.is_whitespace() || "\"'`()[]{}<>,;|&".contains(c)) {
            if word.len() > prefix.len()
                && word.starts_with(prefix)
                && !found.iter().any(|known| known == word)
            {
                found.push(word.to_string());
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_most_used_command_comes_first_and_only_those_with_the_prefix() {
        let mut history = Vec::new();
        for command in [
            "git status",
            "git log",
            "git status",
            "ls",
            "git log --oneline",
        ] {
            remember(&mut history, command);
        }
        let found: Vec<String> = candidates(&history, "git")
            .into_iter()
            .map(|entry| entry.command)
            .collect();
        assert_eq!(found, ["git status", "git log --oneline", "git log"]);
        assert!(candidates(&history, "ls").is_empty());
        let lines = [
            "cargo build --release".to_string(),
            "carbon cargo".to_string(),
        ];
        assert_eq!(words(&lines, "car"), ["cargo", "carbon"]);
    }
}
