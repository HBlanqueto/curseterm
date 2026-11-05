mod auxiliary;
mod config;
mod grid;
mod keys;
mod localize;
mod menu;
mod pane_view;
mod panes;
mod session;
mod terminal;
mod windows;
mod workspace;

use std::process::ExitCode;

use barrier_kit::platform::{WindowOptions, run_window};
use freya_core::integration::AppComponent;
use vow_design::WINDOW_MIN_SIZE;

use crate::config::APP_ID;

const TERMINAL_SIZE: (u32, u32) = (720, 460);

fn main() -> ExitCode {
    barrier_kit::init_tracing();
    barrier_kit::i18n::use_app_language(APP_ID);
    let _registration = windows::register();
    let result = match std::env::args().nth(1).as_deref() {
        Some("--about") => auxiliary::run_about(),
        Some("--settings") => auxiliary::run_settings(),
        _ => run_window(
            AppComponent::new(terminal::Terminal),
            WindowOptions::new("CurseTerm", "curseterm")
                .app_id(APP_ID)
                .size(TERMINAL_SIZE)
                .min_size(Some(WINDOW_MIN_SIZE))
                .menu(menu::terminal_menu()),
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
