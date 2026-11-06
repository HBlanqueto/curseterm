use crate::config::Profile;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Place {
    pub hostname: String,
    pub username: String,
    pub path: String,
    pub job: String,
    pub command_line: String,
}

#[derive(Debug, Default, PartialEq)]
struct Rule {
    username: Option<String>,
    hostname: Option<String>,
    path: Option<String>,
    job: Option<String>,
    sticky: bool,
}

// iTermRule.m +ruleWithString:: [!][user@][host][:path][&job], or /path[&job].
fn parse(text: &str) -> Rule {
    let (sticky, text) = match text.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (text, job) = match text.split_once('&') {
        Some((text, job)) => (text, Some(job.to_string())),
        None => (text, None),
    };
    let mut rule = Rule {
        job,
        sticky,
        ..Rule::default()
    };
    if text.starts_with('/') {
        rule.path = Some(text.to_string());
        return rule;
    }
    let at = text.find('@');
    let colon = text
        .find(':')
        .filter(|colon| at.is_none_or(|at| *colon > at));
    let non_empty = |part: &str| (!part.is_empty()).then(|| part.to_string());
    if let Some(at) = at {
        rule.username = Some(text[..at].to_string());
        rule.hostname = non_empty(&text[at + 1..colon.unwrap_or(text.len())]);
    }
    if let Some(colon) = colon {
        if rule.hostname.is_none() && at.is_none() {
            rule.hostname = non_empty(&text[..colon]);
        }
        rule.path = Some(text[colon + 1..].to_string());
    } else if at.is_none() {
        rule.hostname = non_empty(text);
    }
    rule
}

fn squash(x: f64) -> f64 {
    x / (x + 1.0)
}

// iTermRule.m -scoreForHostname:username:path:job:commandLine:…: host beats job beats user beats
// path, exact beats a glob, and any part that does not match rules the profile out.
fn score(rule: &Rule, place: &Place) -> f64 {
    const CATCHALL: f64 = 1.0;
    const PATH_EXACT: f64 = 2.0;
    const USER_EXACT: f64 = 4.0;
    const JOB: f64 = 8.0;
    const HOST_PARTIAL: f64 = 16.0;
    const HOST_EXACT: f64 = 32.0;
    let mut total = 0.0;
    if let Some(job) = &rule.job {
        if !glob(&place.job, job, true) && !glob(&place.command_line, job, true) {
            return 0.0;
        }
        total += JOB;
    }
    if let Some(hostname) = &rule.hostname {
        if !hostname.contains('*') && *hostname == place.hostname {
            total += HOST_EXACT;
        } else if hostname == "*" {
            if rule.username.is_none() && rule.path.is_none() && rule.job.is_none() {
                total += CATCHALL;
            }
        } else if glob(&place.hostname, hostname, false) {
            total += HOST_PARTIAL * (1.0 + squash(hostname.len() as f64));
        } else {
            return 0.0;
        }
    }
    match &rule.username {
        Some(username) if *username == place.username => total += USER_EXACT,
        Some(username) if !username.is_empty() => return 0.0,
        _ => {}
    }
    if let Some(path) = &rule.path {
        let augmented = if place.path.ends_with('/') {
            place.path.clone()
        } else {
            format!("{}/", place.path)
        };
        if !path.contains('*') && *path == place.path {
            total += PATH_EXACT;
        } else if glob(&augmented, path, true) || glob(&place.path, path, true) {
            total += squash(path.len() as f64);
        } else if !path.is_empty() {
            return 0.0;
        }
    }
    total
}

// NSString+iTerm -stringMatchesGlobPattern:caseSensitive:: `*` and `?`.
fn glob(text: &str, pattern: &str, case_sensitive: bool) -> bool {
    let fold = |text: &str| {
        if case_sensitive {
            text.chars().collect::<Vec<_>>()
        } else {
            text.to_lowercase().chars().collect()
        }
    };
    let (text, pattern) = (fold(text), fold(pattern));
    let (mut t, mut p) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        match pattern.get(p) {
            Some('*') => {
                star = Some((p, t));
                p += 1;
            }
            Some(c) if *c == '?' || *c == text[t] => {
                t += 1;
                p += 1;
            }
            _ => match star {
                Some((star_p, star_t)) => {
                    p = star_p + 1;
                    t = star_t + 1;
                    star = Some((star_p, star_t + 1));
                }
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|c| *c == '*')
}

fn profile_score(profile: &Profile, place: &Place) -> (f64, bool) {
    profile
        .bound_hosts
        .iter()
        .map(|text| {
            let rule = parse(text);
            (score(&rule, place), rule.sticky)
        })
        .fold(
            (0.0, false),
            |best, found| if found.0 > best.0 { found } else { best },
        )
}

// iTermAutomaticProfileSwitcher.m: the profiles it switched away from wait in a stack, so leaving
// a host goes back to what was there before.
#[derive(Default)]
pub struct Switcher {
    stack: Vec<String>,
    last: Option<(Place, String)>,
}

impl Switcher {
    // -setHostname:username:path:job:commandLine:…: the profile to switch to, if any.
    pub fn update(&mut self, profiles: &[Profile], current: &str, place: Place) -> Option<String> {
        let seen = (place, current.to_string());
        if self.last.as_ref() == Some(&seen) {
            return None;
        }
        let place = &seen.0;
        let named = |name: &str| profiles.iter().find(|profile| profile.name == name);
        let current_score = named(current).map_or(0.0, |profile| profile_score(profile, place).0);
        let stacked = self.stack.iter().rev().find_map(|name| {
            let score = profile_score(named(name)?, place).0;
            (score > 0.0).then(|| (name.clone(), score))
        });
        let switch = match stacked {
            Some((name, score)) if name != current && score > current_score => {
                while self.stack.last().is_some_and(|top| *top != name) {
                    self.stack.pop();
                }
                self.stack.pop();
                Some(name)
            }
            _ => self.best_unstacked(profiles, current, place),
        };
        self.last = Some((seen.0.clone(), switch.clone().unwrap_or(seen.1)));
        switch
    }

    fn best_unstacked(
        &mut self,
        profiles: &[Profile],
        current: &str,
        place: &Place,
    ) -> Option<String> {
        let best = profiles
            .iter()
            .map(|profile| (profile, profile_score(profile, place)))
            .filter(|(_, (score, _))| *score > 0.0)
            .fold(None::<(&Profile, (f64, bool))>, |best, found| match best {
                Some(kept) if kept.1.0 >= found.1.0 => Some(kept),
                _ => Some(found),
            });
        match best {
            Some((profile, (_, sticky))) if profile.name != current => {
                if sticky {
                    self.stack.clear();
                } else if self.stack.last().map(String::as_str) != Some(current) {
                    self.stack.push(current.to_string());
                }
                if sticky {
                    self.stack.push(profile.name.clone());
                }
                Some(profile.name.clone())
            }
            None if !self.stack.is_empty() => {
                self.stack.truncate(1);
                (self.stack[0] != current).then(|| self.stack[0].clone())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(hostname: &str, path: &str, job: &str) -> Place {
        Place {
            hostname: hostname.into(),
            username: "ana".into(),
            path: path.into(),
            job: job.into(),
            command_line: job.into(),
        }
    }

    #[test]
    fn rules_parse_and_the_more_specific_part_scores_higher() {
        let rule = parse("!ana@*.example.com:/srv/*&vim");
        assert_eq!(rule.username.as_deref(), Some("ana"));
        assert_eq!(rule.hostname.as_deref(), Some("*.example.com"));
        assert_eq!(rule.path.as_deref(), Some("/srv/*"));
        assert_eq!(rule.job.as_deref(), Some("vim"));
        assert!(rule.sticky);
        let here = place("web.example.com", "/srv/app", "vim");
        assert!(score(&parse("web.example.com"), &here) > score(&parse("*.example.com"), &here));
        assert!(score(&parse("&vim"), &here) > score(&parse("/srv/app"), &here));
        assert_eq!(score(&parse("ana@other"), &here), 0.0);
        assert!(glob("/srv/app/", "/srv/*", true));
    }

    #[test]
    fn leaving_the_matching_place_returns_to_the_previous_profile() {
        let profile = |name: &str, hosts: &[&str]| Profile {
            name: name.into(),
            bound_hosts: hosts.iter().map(|host| host.to_string()).collect(),
            ..Profile::default()
        };
        let profiles = [profile("Default", &[]), profile("Vim", &["&vim"])];
        let mut switcher = Switcher::default();
        let editing = place("pc", "/home/ana", "vim");
        assert_eq!(
            switcher.update(&profiles, "Default", editing),
            Some("Vim".to_string())
        );
        let shell = place("pc", "/home/ana", "bash");
        assert_eq!(
            switcher.update(&profiles, "Vim", shell.clone()),
            Some("Default".to_string())
        );
        assert_eq!(switcher.update(&profiles, "Default", shell), None);
    }
}
