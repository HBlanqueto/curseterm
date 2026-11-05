mod auxiliary;
mod clock;
mod config;
mod grid;
mod integration;
mod io;
mod itermcolors;
mod keys;
mod localize;
mod menu;
mod osc;
mod pane_view;
mod panes;
mod paste_history;
mod preferences;
mod session;
mod terminal;
mod windows;
mod workspace;

use std::path::Path;
use std::process::ExitCode;

use barrier_kit::platform::{WindowOptions, run_window};
use barrier_kit_next::tokens::TAB_BAR;
use freya_core::integration::AppComponent;
use vow_design::{HAIRLINE, SPACE_2, TITLEBAR_CLIENT_HEIGHT, WINDOW_MIN_SIZE};

use crate::config::{APP_ID, APP_NAME, Defaults, change_defaults};
use crate::grid::Fonts;

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

fn import_colors(path: &Path) -> ExitCode {
    match itermcolors::import(path) {
        Ok(scheme) => {
            println!("{}", scheme.name);
            change_defaults(|defaults| {
                defaults.schemes.retain(|known| known.name != scheme.name);
                defaults.schemes.push(scheme);
            });
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("curseterm: {}: {error}", path.display());
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    barrier_kit::init_tracing();
    barrier_kit::i18n::use_app_language(APP_ID);
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if let [flag, path] = arguments.as_slice()
        && flag == "--import-colors"
    {
        return import_colors(Path::new(path));
    }
    let _hotkey = match arguments.first().map(String::as_str) {
        Some("--hotkey") => match windows::toggle_hotkey() {
            Some(registration) => Some(registration),
            None => return ExitCode::SUCCESS,
        },
        _ => None,
    };
    let _registration = windows::register();
    windows::forget_saved_size();
    let profile = match arguments.as_slice() {
        [flag, name] if flag == "--profile" => Some(name.clone()),
        _ => None,
    };
    let result = match arguments.first().map(String::as_str) {
        Some("--about") => auxiliary::run_about(),
        Some("--settings") => preferences::run(),
        _ => {
            let size = terminal_size(profile.as_deref());
            run_window(
                AppComponent::new(terminal::Terminal { profile }),
                WindowOptions::new(APP_NAME, "curseterm")
                    .app_id(APP_ID)
                    .size(size)
                    .min_size(Some(WINDOW_MIN_SIZE))
                    .menu(menu::terminal_menu(&profile_names())),
            )
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "CurseTerm could not open its window");
            ExitCode::FAILURE
        }
    }
}
