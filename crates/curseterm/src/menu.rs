use barrier_kit::app::{Action, Shortcut, ShortcutKey};
use barrier_kit::app_menu::{AppMenu, Menu};
use barrier_kit::menu::MenuItem;
use freya_core::prelude::Modifiers;

use crate::localize::tr;

const fn super_key(c: char) -> Shortcut {
    Shortcut::new(Modifiers::META, ShortcutKey::Char(c))
}

pub const ABOUT: Action = Action::new("about", || tr!("about"));
pub const SETTINGS: Action = Action::new("settings", || tr!("settings")).shortcut(super_key(','));
pub const QUIT: Action = Action::new("quit", || tr!("quit")).shortcut(super_key('q'));
pub const NEW_WINDOW: Action =
    Action::new("new-window", || tr!("new-window")).shortcut(super_key('n'));
pub const NEW_TAB: Action = Action::new("new-tab", || tr!("new-tab")).shortcut(super_key('t'));
pub const CLOSE: Action = Action::new("close", || tr!("close")).shortcut(super_key('w'));
pub const COPY: Action = Action::new("copy", || tr!("copy")).shortcut(super_key('c'));
pub const PASTE: Action = Action::new("paste", || tr!("paste")).shortcut(super_key('v'));
pub const SELECT_ALL: Action =
    Action::new("select-all", || tr!("select-all")).shortcut(super_key('a'));

// The shell merges a menu titled like the app into its own app menu.
const APP_MENU_TITLE: &str = "CurseTerm";

pub fn terminal_menu() -> AppMenu {
    AppMenu::new(vec![
        app_menu(),
        Menu::new(
            tr!("menu-shell"),
            vec![
                NEW_WINDOW.item(true),
                NEW_TAB.item(true),
                MenuItem::Separator,
                CLOSE.item(true),
            ],
        ),
        Menu::new(
            tr!("menu-edit"),
            vec![
                COPY.item(false),
                PASTE.item(true),
                MenuItem::Separator,
                SELECT_ALL.item(true),
            ],
        ),
    ])
}

pub fn auxiliary_menu() -> AppMenu {
    AppMenu::new(vec![
        app_menu(),
        Menu::new(
            tr!("menu-shell"),
            vec![NEW_WINDOW.item(true), MenuItem::Separator, CLOSE.item(true)],
        ),
    ])
}

fn app_menu() -> Menu {
    Menu::new(
        APP_MENU_TITLE,
        vec![
            ABOUT.item(true),
            MenuItem::Separator,
            SETTINGS.item(true),
            MenuItem::Separator,
            QUIT.item(true),
        ],
    )
}
