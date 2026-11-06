use std::fs;

#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub pid: u32,
    pub name: String,
}

pub fn of_session(leader: u32) -> Vec<Job> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut jobs: Vec<Job> = entries
        .flatten()
        .filter_map(|entry| fs::read_to_string(entry.path().join("stat")).ok())
        .filter_map(|stat| parse_stat(&stat))
        .filter(|(pid, _, session)| *session == leader && *pid != leader)
        .map(|(pid, name, _)| Job { pid, name })
        .collect();
    jobs.sort_by_key(|job| job.pid);
    jobs
}

fn parse_stat(stat: &str) -> Option<(u32, String, u32)> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    let pid = stat[..open].trim().parse().ok()?;
    let name = stat[open + 1..close].to_string();
    let session = stat[close + 1..].split_whitespace().nth(3)?.parse().ok()?;
    Some((pid, name, session))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_with_spaces_and_parentheses_keeps_the_session_field() {
        let stat = "4242 (my (odd) job) S 4200 4242 4200 34816 4242 4194304";
        assert_eq!(
            parse_stat(stat),
            Some((4242, "my (odd) job".to_string(), 4200))
        );
    }
}
