mod auxiliary;
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
mod preferences;
mod session;
mod terminal;
mod windows;
mod workspace;

use std::path::Path;
use std::process::ExitCode;

use barrier_kit::platform::{WindowOptions, run_window};
use freya_core::integration::AppComponent;
use vow_design::WINDOW_MIN_SIZE;

use crate::config::{APP_ID, Defaults, change_defaults};

fn profile_names() -> Vec<String> {
    let defaults: Defaults = barrier_kit::app::load_defaults(APP_ID);
    defaults
        .profiles()
        .into_iter()
        .map(|profile| profile.name)
        .collect()
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

const TERMINAL_SIZE: (u32, u32) = (720, 460);

fn main() -> ExitCode {
    barrier_kit::init_tracing();
    barrier_kit::i18n::use_app_language(APP_ID);
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if let [flag, path] = arguments.as_slice()
        && flag == "--import-colors"
    {
        return import_colors(Path::new(path));
    }
    let _registration = windows::register();
    let result = match arguments.first().map(String::as_str) {
        Some("--about") => auxiliary::run_about(),
        Some("--settings") => preferences::run(),
        _ => run_window(
            AppComponent::new(terminal::Terminal {
                profile: match arguments.as_slice() {
                    [flag, name] if flag == "--profile" => Some(name.clone()),
                    _ => None,
                },
            }),
            WindowOptions::new("CurseTerm", "curseterm")
                .app_id(APP_ID)
                .size(TERMINAL_SIZE)
                .min_size(Some(WINDOW_MIN_SIZE))
                .menu(menu::terminal_menu(&profile_names())),
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "CurseTerm could not open its window");
            ExitCode::FAILURE
        }
    }
}
