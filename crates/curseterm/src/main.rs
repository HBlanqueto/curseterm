mod activity;
mod add_trigger;
mod advanced_paste;
mod arrangement;
mod arrangements_pane;
mod auxiliary;
mod clock;
mod command_history;
mod composer;
mod composer_view;
mod config;
mod drop_target;
mod find_mode;
mod global_search;
mod grid;
mod hint_view;
mod hints;
mod indicators;
mod integration;
mod io;
mod itermcolors;
mod jobs;
mod key_bindings;
mod key_bindings_pane;
mod key_report;
mod keys;
mod links;
mod localize;
mod menu;
mod mouse;
mod note_view;
mod osc;
mod pane_view;
mod panes;
mod paste_history;
mod paste_transform;
mod paste_warning;
mod pick;
mod preferences;
mod profile_switching;
mod semantic_history;
mod session;
mod session_note;
mod smart_selection;
mod snippets;
mod snippets_pane;
mod status;
#[cfg(feature = "kit-parcheado")]
mod styled;
mod terminal;
mod theme;
mod triggers;
mod windows;
mod workspace;

use std::path::Path;
use std::process::ExitCode;

use barrier_kit::platform::run_window;

use crate::config::{APP_ID, change_defaults};

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
    let startup = arguments
        .is_empty()
        .then(windows::default_arrangement_at_startup)
        .flatten();
    let _registration = windows::register();
    let arguments: Vec<&str> = match &startup {
        Some(name) => vec!["--arrangement", name],
        None => arguments.iter().map(String::as_str).collect(),
    };
    let (app, options) = windows::describe(&arguments);
    let result = run_window(app, options);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "CurseTerm could not open its window");
            ExitCode::FAILURE
        }
    }
}
