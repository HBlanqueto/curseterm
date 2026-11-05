use barrier_kit::app::{Action, Shortcut, ShortcutKey};
use barrier_kit::app_menu::{AppMenu, Menu};
use barrier_kit::menu::{CheckState, MenuItem};
use barrier_kit_next::app_menu::submenu;
use freya_core::prelude::Modifiers;

use crate::localize::tr;

const fn super_key(c: char) -> Shortcut {
    Shortcut::new(Modifiers::META, ShortcutKey::Char(c))
}

const fn super_with(modifiers: Modifiers, c: char) -> Shortcut {
    Shortcut::new(Modifiers::META.union(modifiers), ShortcutKey::Char(c))
}

pub const ABOUT: Action = Action::new("about", || tr!("about"));
pub const SETTINGS: Action = Action::new("settings", || tr!("settings")).shortcut(super_key(','));
pub const QUIT: Action = Action::new("quit", || tr!("quit")).shortcut(super_key('q'));

pub const NEW_WINDOW: Action =
    Action::new("new-window", || tr!("new-window")).shortcut(super_key('n'));
pub const NEW_TAB: Action = Action::new("new-tab", || tr!("new-tab")).shortcut(super_key('t'));
pub const DUPLICATE_TAB: Action = Action::new("duplicate-tab", || tr!("duplicate-tab"));
pub const SPLIT_VERTICALLY: Action =
    Action::new("split-vertically", || tr!("split-vertically")).shortcut(super_key('d'));
pub const SPLIT_HORIZONTALLY: Action =
    Action::new("split-horizontally", || tr!("split-horizontally"))
        .shortcut(super_with(Modifiers::SHIFT, 'd'));
pub const CLOSE: Action = Action::new("close", || tr!("close")).shortcut(super_key('w'));
pub const CLOSE_WINDOW: Action =
    Action::new("close-window", || tr!("close-window")).shortcut(super_with(Modifiers::SHIFT, 'w'));
pub const CLOSE_PANES: Action =
    Action::new("close-panes", || tr!("close-panes")).shortcut(super_with(Modifiers::ALT, 'w'));

pub const COPY: Action = Action::new("copy", || tr!("copy")).shortcut(super_key('c'));
pub const PASTE: Action = Action::new("paste", || tr!("paste")).shortcut(super_key('v'));
pub const SELECT_ALL: Action =
    Action::new("select-all", || tr!("select-all")).shortcut(super_key('a'));

pub const SHOW_TAB_BAR: Action = Action::new("show-tab-bar", || tr!("show-tab-bar"));
pub const FULL_SCREEN: Action =
    Action::new("full-screen", || tr!("full-screen")).shortcut(super_with(Modifiers::CONTROL, 'f'));
pub const MAXIMIZE_PANE: &str = "maximize-pane";
pub const TEXT_BIGGER: Action =
    Action::new("text-bigger", || tr!("text-bigger")).shortcut(super_key('='));
pub const TEXT_NORMAL: Action =
    Action::new("text-normal", || tr!("text-normal")).shortcut(super_key('0'));
pub const TEXT_SMALLER: Action =
    Action::new("text-smaller", || tr!("text-smaller")).shortcut(super_key('-'));

pub const MINIMIZE: Action = Action::new("minimize", || tr!("minimize")).shortcut(super_key('m'));
pub const ZOOM: Action = Action::new("zoom", || tr!("zoom"));
// BK-016: with Shift the kit matches the produced character, so ⇧] is declared as `}`.
pub const NEXT_TAB: &str = "next-tab";
pub const PREVIOUS_TAB: &str = "previous-tab";
pub const MOVE_TAB_LEFT: Action = Action::new("move-tab-left", || tr!("move-tab-left"));
pub const MOVE_TAB_RIGHT: Action = Action::new("move-tab-right", || tr!("move-tab-right"));
pub const SELECT_TAB: &str = "select-tab";
pub const SELECT_TAB_PREFIX: &str = "select-tab:";
pub const SPLIT_PANE: &str = "split-pane";
pub const NEXT_PANE: Action =
    Action::new("next-pane", || tr!("next-pane")).shortcut(super_key(']'));
pub const PREVIOUS_PANE: Action =
    Action::new("previous-pane", || tr!("previous-pane")).shortcut(super_key('['));
pub const PANE_UP: &str = "pane-up";
pub const PANE_DOWN: &str = "pane-down";
pub const PANE_LEFT: &str = "pane-left";
pub const PANE_RIGHT: &str = "pane-right";
pub const DIVIDER_UP: &str = "divider-up";
pub const DIVIDER_DOWN: &str = "divider-down";
pub const DIVIDER_LEFT: &str = "divider-left";
pub const DIVIDER_RIGHT: &str = "divider-right";

// The shell merges a menu titled like the app into its own app menu.
const APP_MENU_TITLE: &str = "CurseTerm";

fn keyed(id: &'static str, label: &str, shortcut: &str) -> MenuItem {
    MenuItem::action(id, label).with_shortcut(shortcut)
}

pub fn terminal_menu() -> AppMenu {
    AppMenu::new(vec![
        app_menu(),
        Menu::new(
            tr!("menu-shell"),
            vec![
                NEW_WINDOW.item(true),
                NEW_TAB.item(true),
                DUPLICATE_TAB.item(true),
                MenuItem::Separator,
                SPLIT_VERTICALLY.item(true),
                SPLIT_HORIZONTALLY.item(true),
                MenuItem::Separator,
                CLOSE.item(true),
                CLOSE_WINDOW.item(true),
                CLOSE_PANES.item(false),
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
        Menu::new(
            tr!("menu-view"),
            vec![
                SHOW_TAB_BAR.item_with(true, CheckState::Checkbox(false)),
                FULL_SCREEN.item(true),
                MenuItem::Separator,
                keyed(MAXIMIZE_PANE, tr!("maximize-pane"), "Super+Shift+Return")
                    .with_enabled(false)
                    .with_check(CheckState::Checkbox(false)),
                MenuItem::Separator,
                TEXT_BIGGER.item(true),
                TEXT_NORMAL.item(true),
                TEXT_SMALLER.item(true),
            ],
        ),
        Menu::new(
            tr!("menu-window"),
            vec![
                MINIMIZE.item(true),
                ZOOM.item(true),
                MenuItem::Separator,
                keyed(NEXT_TAB, tr!("next-tab"), "Super+Shift+}").with_enabled(false),
                keyed(PREVIOUS_TAB, tr!("previous-tab"), "Super+Shift+{").with_enabled(false),
                MOVE_TAB_LEFT.item(false),
                MOVE_TAB_RIGHT.item(false),
                submenu(SELECT_TAB, tr!("select-tab"), Vec::new()),
                MenuItem::Separator,
                submenu(SPLIT_PANE, tr!("split-pane"), split_pane_items()),
            ],
        ),
    ])
}

fn split_pane_items() -> Vec<MenuItem> {
    vec![
        keyed(PANE_UP, tr!("pane-up"), "Super+Alt+Up"),
        keyed(PANE_DOWN, tr!("pane-down"), "Super+Alt+Down"),
        keyed(PANE_LEFT, tr!("pane-left"), "Super+Alt+Left"),
        keyed(PANE_RIGHT, tr!("pane-right"), "Super+Alt+Right"),
        MenuItem::Separator,
        NEXT_PANE.item(true),
        PREVIOUS_PANE.item(true),
        MenuItem::Separator,
        keyed(DIVIDER_UP, tr!("divider-up"), "Ctrl+Super+Up"),
        keyed(DIVIDER_DOWN, tr!("divider-down"), "Ctrl+Super+Down"),
        keyed(DIVIDER_LEFT, tr!("divider-left"), "Ctrl+Super+Left"),
        keyed(DIVIDER_RIGHT, tr!("divider-right"), "Ctrl+Super+Right"),
    ]
}

pub fn select_tab_items(titles: &[String], active: usize) -> Vec<MenuItem> {
    titles
        .iter()
        .enumerate()
        .map(|(index, title)| {
            let item = MenuItem::action(format!("{SELECT_TAB_PREFIX}{index}"), title.clone())
                .with_check(CheckState::Radio(index == active));
            match index {
                0..9 => item.with_shortcut(format!("Super+{}", index + 1)),
                _ => item,
            }
        })
        .collect()
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
