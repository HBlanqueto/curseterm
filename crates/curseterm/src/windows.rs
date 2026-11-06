//! One process per window (BK-001), registered by pid under `$XDG_RUNTIME_DIR/curseterm`.

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use barrier_kit::platform::{WindowControls, WindowOptions};
use barrier_kit_next::tokens::TAB_BAR;
use freya_core::integration::AppComponent;
use freya_core::prelude::{use_drop, use_hook};
use rustix::process::{Pid, Signal, kill_process};
use vow_design::{HAIRLINE, SPACE_2, TITLEBAR_CLIENT_HEIGHT, WINDOW_MIN_SIZE};

use crate::config::{APP_ID, APP_NAME, Defaults};
use crate::grid::Fonts;
use crate::{auxiliary, menu, preferences, terminal};

thread_local! {
    static OPEN: RefCell<Vec<(u64, WindowControls)>> = const { RefCell::new(Vec::new()) };
    static NEXT: Cell<u64> = const { Cell::new(0) };
}

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

pub fn describe(arguments: &[&str]) -> (AppComponent, WindowOptions) {
    forget_saved_size();
    match arguments {
        ["--about", ..] => auxiliary::about(),
        ["--settings", profile @ ..] => {
            preferences::window(profile.first().map(|name| name.to_string()))
        }
        ["--profile", name, ..] => terminal_window(Some(name.to_string()), None),
        ["--arrangement", name, ..] => terminal_window(None, Some(name.to_string())),
        _ => terminal_window(None, None),
    }
}

fn terminal_window(
    profile: Option<String>,
    arrangement: Option<String>,
) -> (AppComponent, WindowOptions) {
    let size = terminal_size(profile.as_deref());
    let options = WindowOptions::new(APP_NAME, "curseterm")
        .app_id(APP_ID)
        .size(size)
        .min_size(Some(WINDOW_MIN_SIZE))
        .menu(menu::terminal_menu(&profile_names()));
    (
        AppComponent::new(terminal::Terminal {
            profile,
            arrangement,
        }),
        options,
    )
}

fn profile_names() -> Vec<String> {
    let defaults: Defaults = barrier_kit::app::load_defaults(APP_ID);
    defaults
        .profiles()
        .into_iter()
        .map(|profile| profile.name)
        .collect()
}

fn terminal_size(profile: Option<&str>) -> (u32, u32) {
    let defaults: Defaults = barrier_kit::app::load_defaults(APP_ID);
    let name = profile.map_or_else(|| defaults.default_profile_name(), str::to_string);
    let profile = defaults.profile(&name);
    let fonts = Fonts::new(&profile.font_family, profile.text_size(0));
    let tab_bar = if defaults.show_tab_bar {
        TAB_BAR.height + HAIRLINE
    } else {
        0.0
    };
    let inset = 2.0 * SPACE_2;
    let width = f32::from(profile.columns) * fonts.cell_width + inset;
    let height = f32::from(profile.rows) * fonts.cell_height
        + inset
        + TITLEBAR_CLIENT_HEIGHT as f32
        + tab_bar;
    (width.ceil() as u32, height.ceil() as u32)
}

pub fn use_registered_window() {
    let token = use_hook(|| {
        let token = NEXT.with(|next| next.replace(next.get() + 1));
        OPEN.with(|open| open.borrow_mut().push((token, WindowControls::get())));
        token
    });
    use_drop(move || OPEN.with(|open| open.borrow_mut().retain(|(other, _)| *other != token)));
}

pub fn quit() {
    quit_others();
    let windows: Vec<WindowControls> = OPEN.with(|open| {
        open.borrow()
            .iter()
            .map(|(_, controls)| controls.clone())
            .collect()
    });
    for controls in windows {
        controls.close();
    }
}

pub fn open(arguments: &[&str]) {
    // BK-001: without patch 0008 each window is its own process.
    #[cfg(feature = "kit-parcheado")]
    if let Some(controls) =
        OPEN.with(|open| open.borrow().first().map(|(_, controls)| controls.clone()))
    {
        let (app, options) = describe(arguments);
        controls.open_window(app, options);
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        tracing::warn!("cannot find the CurseTerm executable");
        return;
    };
    if let Err(error) = Command::new(exe).args(arguments).spawn() {
        tracing::warn!(%error, "could not open a CurseTerm window");
    }
}

fn quit_others() {
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
