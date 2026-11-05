pub fn matching<T>(entries: Vec<(String, T)>, query: &str) -> Vec<(String, T)> {
    let query = query.to_lowercase();
    entries
        .into_iter()
        .filter(|(label, _)| label.to_lowercase().contains(&query))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filter_ignores_case_and_keeps_the_order() {
        let entries = vec![
            ("git Push".to_string(), 1),
            ("ls".to_string(), 2),
            ("push -f".to_string(), 3),
        ];
        let labels: Vec<i32> = matching(entries, "PUSH")
            .into_iter()
            .map(|(_, id)| id)
            .collect();
        assert_eq!(labels, vec![1, 3]);
    }
}
