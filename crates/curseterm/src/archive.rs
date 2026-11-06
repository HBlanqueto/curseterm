use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::arrangement::Arrangement;
use crate::clock;

pub const EXTENSION: &str = "cursetermarchive";
// ArchivesMenuBuilder.swift maxRecents.
pub const RECENTS: usize = 10;

// NSFileManager+iTerm.m -it_defaultArchiveDirectory, under the XDG data directory.
pub fn default_directory() -> Option<PathBuf> {
    Some(
        vow_appearance::xdg::data_home()?
            .join("curseterm")
            .join("archives"),
    )
}

// PTYSession.swift -saveArchive: “date time - name”, safe as a file name and never overwriting
// another archive.
pub fn path_for(directory: &Path, name: &str, at: SystemTime) -> PathBuf {
    let stamp = clock::local_date_time(at)
        .replace(' ', "_")
        .replace(':', ".");
    let mut base: String = format!("{stamp} - {name}")
        .chars()
        .map(|c| {
            if c == '/' || c == ':' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    while base.len() > 230 {
        base.pop();
    }
    let mut path = directory.join(format!("{base}.{EXTENSION}"));
    let mut suffix = 2;
    while path.exists() {
        path = directory.join(format!("{base} ({suffix}).{EXTENSION}"));
        suffix += 1;
    }
    path
}

pub fn save(path: &Path, archive: &Arrangement) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string(archive).map_err(std::io::Error::other)?;
    std::fs::write(path, text)
}

pub fn load(path: &Path) -> Option<Arrangement> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

// ArchivesMenuBuilder.swift -didAdd(path:): the newest last, at most ten.
pub fn remember(recents: &mut Vec<PathBuf>, path: PathBuf) {
    recents.retain(|kept| *kept != path);
    recents.push(path);
    let extra = recents.len().saturating_sub(RECENTS);
    recents.drain(..extra);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_archive_name_is_a_safe_file_name_that_never_overwrites_another() {
        let directory =
            std::env::temp_dir().join(format!("curseterm-archive-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let at = SystemTime::UNIX_EPOCH;
        let first = path_for(&directory, "a/b:c", at);
        assert!(!first.file_name().unwrap().to_string_lossy().contains('/'));
        std::fs::write(&first, "").unwrap();
        let second = path_for(&directory, "a/b:c", at);
        assert!(
            second
                .to_string_lossy()
                .ends_with(&format!(" (2).{EXTENSION}"))
        );
        std::fs::remove_dir_all(&directory).unwrap();
        let mut recents: Vec<PathBuf> = (0..12)
            .map(|index| PathBuf::from(index.to_string()))
            .collect();
        remember(&mut recents, PathBuf::from("3"));
        assert_eq!(recents.len(), RECENTS);
        assert_eq!(recents.last(), Some(&PathBuf::from("3")));
    }
}
