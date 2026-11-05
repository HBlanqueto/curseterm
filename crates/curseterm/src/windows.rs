//! BarrierKit opens one window per process (BK-001): each CurseTerm window is
//! its own process, registered by pid under `$XDG_RUNTIME_DIR/curseterm/windows`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use rustix::process::{Pid, Signal, kill_process};

use crate::config::APP_ID;

fn registry() -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(runtime).join("curseterm").join("windows"))
}

pub struct Registration(Option<PathBuf>);

pub fn register() -> Registration {
    let entry = registry().and_then(|dir| {
        fs::create_dir_all(&dir).ok()?;
        let entry = dir.join(std::process::id().to_string());
        fs::write(&entry, b"").ok()?;
        Some(entry)
    });
    Registration(entry)
}

impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(entry) = &self.0 {
            let _ = fs::remove_file(entry);
        }
    }
}

pub fn open(arguments: &[&str]) {
    let Ok(exe) = std::env::current_exe() else {
        tracing::warn!("cannot find the CurseTerm executable");
        return;
    };
    if let Err(error) = Command::new(exe).args(arguments).spawn() {
        tracing::warn!(%error, "could not open a CurseTerm window");
    }
}

pub fn quit_others() {
    let (Some(dir), Ok(exe)) = (registry(), std::env::current_exe()) else {
        return;
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };
    let own = std::process::id();
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let same_program = fs::read_link(format!("/proc/{pid}/exe")).is_ok_and(|path| path == exe);
        match Pid::from_raw(pid as i32) {
            Some(target) if pid != own && same_program => {
                let _ = kill_process(target, Signal::TERM);
            }
            _ if pid != own => {
                let _ = fs::remove_file(entry.path());
            }
            _ => {}
        }
    }
}

/// The kit restores every window of an app from one `<app_id>.window` state
/// (BK-012); an auxiliary window puts the terminal's saved size back on exit.
pub struct KeepTerminalSize(toml::Table);

impl KeepTerminalSize {
    pub fn new() -> Self {
        Self(barrier_kit::app::load_restoration(&format!(
            "{APP_ID}.window"
        )))
    }
}

impl Drop for KeepTerminalSize {
    fn drop(&mut self) {
        let _ = barrier_kit::app::save_restoration(&format!("{APP_ID}.window"), &self.0);
    }
}
