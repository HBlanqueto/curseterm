//! One process per window (BK-001), registered by pid under `$XDG_RUNTIME_DIR/curseterm`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use rustix::process::{Pid, Signal, kill_process};

use crate::config::APP_ID;

fn runtime_dir() -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(runtime).join("curseterm"))
}

fn registry() -> Option<PathBuf> {
    Some(runtime_dir()?.join("windows"))
}

fn is_curseterm(pid: u32) -> bool {
    let exe = std::env::current_exe();
    fs::read_link(format!("/proc/{pid}/exe")).is_ok_and(|path| exe.is_ok_and(|exe| path == exe))
}

// BK-025: without the GlobalShortcuts portal a desktop shortcut runs `curseterm --hotkey`.
pub fn toggle_hotkey() -> Option<Registration> {
    let Some(file) = runtime_dir().map(|dir| dir.join("hotkey")) else {
        return Some(Registration(None));
    };
    let open = fs::read_to_string(&file)
        .ok()
        .and_then(|pid| pid.trim().parse::<u32>().ok())
        .filter(|pid| is_curseterm(*pid))
        .and_then(|pid| Pid::from_raw(pid as i32));
    if let Some(window) = open {
        let _ = kill_process(window, Signal::TERM);
        return None;
    }
    let written = file
        .parent()
        .is_some_and(|dir| fs::create_dir_all(dir).is_ok())
        && fs::write(&file, std::process::id().to_string()).is_ok();
    Some(Registration(written.then_some(file)))
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
    let Some(dir) = registry() else {
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
        match Pid::from_raw(pid as i32) {
            Some(target) if pid != own && is_curseterm(pid) => {
                let _ = kill_process(target, Signal::TERM);
            }
            _ if pid != own => {
                let _ = fs::remove_file(entry.path());
            }
            _ => {}
        }
    }
}

// BK-012: the kit puts the one saved size per app_id over WindowOptions::size.
pub fn forget_saved_size() {
    let _ = barrier_kit::app::save_restoration(&format!("{APP_ID}.window"), &toml::Table::new());
}
