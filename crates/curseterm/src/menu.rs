use barrier_kit::app::{Action, Shortcut, ShortcutKey};
use barrier_kit::app_menu::{AppMenu, Menu};
use barrier_kit::menu::{CheckState, MenuItem};
use barrier_kit_next::app_menu::submenu;
use freya_core::prelude::Modifiers;

use crate::config::{APP_NAME, MarkAlert};
use crate::localize::tr;
use crate::session::TerminalState;
use crate::workspace::Broadcast;

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
pub const NEW_WINDOW_PROFILE: Action =
    Action::new("new-window-profile", || tr!("new-window-profile"))
        .shortcut(super_with(Modifiers::ALT.union(Modifiers::SHIFT), 'n'));
pub const NEW_TAB: Action = Action::new("new-tab", || tr!("new-tab")).shortcut(super_key('t'));
pub const NEW_TAB_PROFILE: Action = Action::new("new-tab-profile", || tr!("new-tab-profile"))
    .shortcut(super_with(Modifiers::ALT.union(Modifiers::SHIFT), 't'));
pub const DUPLICATE_TAB: Action = Action::new("duplicate-tab", || tr!("duplicate-tab"));
pub const DUPLICATE_WINDOW: Action = Action::new("duplicate-window", || tr!("duplicate-window"));
pub const DUPLICATE_SESSION: Action = Action::new("duplicate-session", || tr!("duplicate-session"));
pub const SELECT_CURRENT_COMMAND: Action =
    Action::new("select-current-command", || tr!("select-current-command"));
pub const ARRANGE_EVENLY: &str = "arrange-evenly";
pub const SPLIT_VERTICALLY: Action =
    Action::new("split-vertically", || tr!("split-vertically")).shortcut(super_key('d'));
pub const SPLIT_HORIZONTALLY: Action =
    Action::new("split-horizontally", || tr!("split-horizontally"))
        .shortcut(super_with(Modifiers::SHIFT, 'd'));
pub const SAVE_SELECTION: Action =
    Action::new("save-selection", || tr!("save-selection")).shortcut(super_key('s'));
pub const SAVE_CONTENTS: Action = Action::new("save-contents", || tr!("save-contents"));
pub const OPEN_SELECTION: Action = Action::new("open-selection", || tr!("open-selection"));
pub const CLOSE: Action = Action::new("close", || tr!("close")).shortcut(super_key('w'));
pub const CLOSE_WINDOW: Action =
    Action::new("close-window", || tr!("close-window")).shortcut(super_with(Modifiers::SHIFT, 'w'));
pub const UNDO_CLOSE: Action =
    Action::new("undo-close", || tr!("undo-close")).shortcut(super_with(Modifiers::SHIFT, 't'));
pub const PASTE_HISTORY: Action = Action::new("paste-history", || tr!("paste-history"))
    .shortcut(super_with(Modifiers::SHIFT, 'h'));
pub const RECENT_DIRECTORIES: Action =
    Action::new("recent-directories", || tr!("recent-directories"))
        .shortcut(super_with(Modifiers::ALT, '/'));
pub const CLOSE_PANES: Action =
    Action::new("close-panes", || tr!("close-panes")).shortcut(super_with(Modifiers::ALT, 'w'));

pub const COPY: Action = Action::new("copy", || tr!("copy")).shortcut(super_key('c'));
pub const COPY_STYLED: Action =
    Action::new("copy-styled", || tr!("copy-styled")).shortcut(super_with(Modifiers::ALT, 'c'));
pub const PASTE: Action = Action::new("paste", || tr!("paste")).shortcut(super_key('v'));
pub const CLEAR_BUFFER: Action =
    Action::new("clear-buffer", || tr!("clear-buffer")).shortcut(super_key('k'));
pub const CLEAR_TO_SELECTION: Action =
    Action::new("clear-to-selection", || tr!("clear-to-selection")).shortcut(super_key('l'));
pub const CLEAR_TO_LAST_MARK: Action =
    Action::new("clear-to-last-mark", || tr!("clear-to-last-mark"))
        .shortcut(super_with(Modifiers::ALT, 'l'));
pub const CLEAR_REPLAY: Action =
    Action::new("clear-replay", || tr!("clear-replay")).shortcut(super_with(Modifiers::ALT, 'k'));
pub const CLEAR_SCROLLBACK: Action = Action::new("clear-scrollback", || tr!("clear-scrollback"))
    .shortcut(super_with(Modifiers::SHIFT, 'k'));
pub const RESTART_SESSION: Action = Action::new("restart-session", || tr!("restart-session"));
pub const TRIGGERS_MENU: &str = "triggers-menu";
pub const TRIGGERS_ENABLED: Action = Action::new("triggers-enabled", || tr!("triggers-enabled"));
pub const EDIT_TRIGGERS: Action = Action::new("edit-triggers", || tr!("edit-triggers"));
pub const RESET: Action = Action::new("reset", || tr!("reset")).shortcut(super_key('r'));
pub const PASTE_SPECIAL_MENU: &str = "paste-special-menu";
pub const WARN_MULTI_LINE_PASTE: &str = "warn-multi-line-paste";
pub const LIMIT_PASTE_WARNING: &str = "limit-paste-warning";
pub const WARN_ONE_LINE_PASTE: &str = "warn-one-line-paste";
pub const PASTE_SLOWLY: Action = Action::new("paste-slowly", || tr!("paste-slowly"));
pub const ADVANCED_PASTE: Action = Action::new("advanced-paste", || tr!("advanced-paste-menu"))
    .shortcut(super_with(Modifiers::ALT, 'v'));
pub const PASTE_ESCAPED: Action = Action::new("paste-escaped", || tr!("paste-escaped"));
pub const PASTE_SELECTION: Action = Action::new("paste-selection", || tr!("paste-selection"));
pub const LOG_MENU: &str = "log-menu";
pub const LOG_START: Action = Action::new("log-start", || tr!("log-start"));
pub const LOG_STOP: Action = Action::new("log-stop", || tr!("log-stop"));
pub const SELECT_ALL: Action =
    Action::new("select-all", || tr!("select-all")).shortcut(super_key('a'));
pub const SELECT_OUTPUT: Action = Action::new("select-output", || tr!("select-output"))
    .shortcut(super_with(Modifiers::SHIFT, 'a'));
pub const COPY_MODE: Action =
    Action::new("copy-mode", || tr!("copy-mode")).shortcut(super_with(Modifiers::SHIFT, 'c'));

pub const FIND_MENU: &str = "find-menu";
pub const FIND: Action = Action::new("find", || tr!("find")).shortcut(super_key('f'));
pub const FIND_NEXT: Action =
    Action::new("find-next", || tr!("find-next")).shortcut(super_key('g'));
pub const FIND_PREVIOUS: Action = Action::new("find-previous", || tr!("find-previous"))
    .shortcut(super_with(Modifiers::SHIFT, 'g'));
pub const FILTER: Action = Action::new("filter", || tr!("filter"))
    .shortcut(super_with(Modifiers::ALT.union(Modifiers::SHIFT), 'f'));
pub const FIND_URLS: Action =
    Action::new("find-urls", || tr!("find-urls")).shortcut(super_with(Modifiers::ALT, 'u'));
pub const SMART_SELECT_ALL: Action = Action::new("smart-select-all", || tr!("smart-select-all"))
    .shortcut(super_with(Modifiers::ALT.union(Modifiers::SHIFT), 'u'));
pub const PICK_RESULT: Action = Action::new("pick-result", || tr!("pick-result"))
    .shortcut(super_with(Modifiers::ALT.union(Modifiers::SHIFT), 'o'));
pub const USE_SELECTION_FOR_FIND: Action =
    Action::new("use-selection-for-find", || tr!("use-selection-for-find"))
        .shortcut(super_key('e'));
pub const CLEAR_FIND: Action =
    Action::new("clear-find", || tr!("clear-find")).shortcut(super_with(Modifiers::ALT, 'e'));
pub const FIND_GLOBALLY: Action = Action::new("find-globally", || tr!("find-globally"))
    .shortcut(super_with(Modifiers::SHIFT, 'f'));
pub const MARKS_MENU: &str = "marks-menu";
pub const SET_MARK: Action =
    Action::new("set-mark", || tr!("set-mark")).shortcut(super_with(Modifiers::SHIFT, 'm'));
pub const JUMP_TO_MARK: Action =
    Action::new("jump-to-mark", || tr!("jump-to-mark")).shortcut(super_with(Modifiers::SHIFT, 'j'));
pub const NEXT_MARK: &str = "next-mark";
pub const ADD_ANNOTATION: Action = Action::new("add-annotation", || tr!("add-annotation"))
    .shortcut(super_with(Modifiers::ALT, 'm'));
pub const ANNOTATE_SELECTION: Action =
    Action::new("annotate-selection", || tr!("annotate-selection"));
pub const NEXT_ANNOTATION: &str = "next-annotation";
pub const PREVIOUS_ANNOTATION: &str = "previous-annotation";
pub const SET_NAMED_MARK: Action = Action::new("set-named-mark", || tr!("set-named-mark"))
    .shortcut(super_with(Modifiers::ALT.union(Modifiers::SHIFT), 'm'));
pub const NAMED_MARKS_MENU: &str = "named-marks-menu";
pub const ALERTS_MENU: &str = "alerts-menu";
pub const ALERT_ON_NEXT_MARK: &str = "alert-on-next-mark";
pub const ALERT_OFFSCREEN_MARKS: &str = "alert-offscreen-marks";
pub const MARK_ALERT_MODAL: &str = "mark-alert-modal";
pub const MARK_ALERT_NOTIFICATION: &str = "mark-alert-notification";
pub const MARK_ALERT_SOUND: &str = "mark-alert-sound";
pub const NAMED_MARK_PREFIX: &str = "named-mark:";
pub const SHOW_COMPOSER: &str = "show-composer";
pub const SHOW_ANNOTATIONS: Action =
    Action::new("show-annotations", || tr!("show-annotations")).shortcut(super_key('\\'));
pub const PREVIOUS_MARK: &str = "previous-mark";

pub const BROADCAST_MENU: &str = "broadcast-menu";
pub const BROADCAST_OFF: &str = "broadcast-off";
pub const BROADCAST_ALL_TABS: &str = "broadcast-all-tabs";
pub const BROADCAST_CURRENT_TAB: &str = "broadcast-current-tab";

pub const SHOW_TAB_BAR: Action = Action::new("show-tab-bar", || tr!("show-tab-bar"));
pub const FULL_SCREEN: Action =
    Action::new("full-screen", || tr!("full-screen")).shortcut(super_with(Modifiers::CONTROL, 'f'));
pub const SHOW_TIMESTAMPS: Action = Action::new("show-timestamps", || tr!("show-timestamps"))
    .shortcut(super_with(Modifiers::SHIFT, 'e'));
pub const CURSOR_GUIDE: Action =
    Action::new("cursor-guide", || tr!("cursor-guide")).shortcut(super_with(Modifiers::ALT, ';'));
pub const EDIT_SESSION: Action =
    Action::new("edit-session", || tr!("edit-session")).shortcut(super_key('i'));
pub const TERMINAL_STATE_MENU: &str = "terminal-state-menu";
pub const HELP: Action =
    Action::new("help", || tr!("help")).shortcut(super_with(Modifiers::SHIFT, '?'));
pub const RUN_COPROCESS: Action = Action::new("run-coprocess", || tr!("run-coprocess"));
pub const STOP_COPROCESS: Action = Action::new("stop-coprocess", || tr!("stop-coprocess"));
pub const RESET_CHARSET: Action = Action::new("reset-charset", || tr!("reset-charset"));
pub const USE_TRANSPARENCY: Action =
    Action::new("use-transparency", || tr!("use-transparency")).shortcut(super_key('u'));
pub const OPEN_QUICKLY: Action =
    Action::new("open-quickly", || tr!("open-quickly")).shortcut(super_with(Modifiers::SHIFT, 'o'));
pub const OPEN_PROFILES: Action =
    Action::new("open-profiles", || tr!("open-profiles")).shortcut(super_key('o'));
pub const TOOLBELT: Action =
    Action::new("toolbelt", || tr!("toolbelt")).shortcut(super_with(Modifiers::SHIFT, 'b'));
pub const INSTANT_REPLAY: Action = Action::new("instant-replay", || tr!("instant-replay"))
    .shortcut(super_with(Modifiers::ALT, 'b'));
pub const FIND_CURSOR: Action =
    Action::new("find-cursor", || tr!("find-cursor")).shortcut(super_key('/'));
pub const MAXIMIZE_PANE: &str = "maximize-pane";
pub const TEXT_BIGGER: Action =
    Action::new("text-bigger", || tr!("text-bigger")).shortcut(super_key('='));
pub const TEXT_NORMAL: Action =
    Action::new("text-normal", || tr!("text-normal")).shortcut(super_key('0'));
pub const TEXT_SMALLER: Action =
    Action::new("text-smaller", || tr!("text-smaller")).shortcut(super_key('-'));

pub const EDIT_TAB_TITLE: Action = Action::new("edit-tab-title", || tr!("edit-tab-title"));
pub const EDIT_WINDOW_TITLE: Action = Action::new("edit-window-title", || tr!("edit-window-title"));
pub const COPY_MODE_KEYS: Action = Action::new("copy-mode-keys", || tr!("copy-mode-keys"));
pub const MINIMIZE: Action = Action::new("minimize", || tr!("minimize")).shortcut(super_key('m'));
pub const ZOOM: &str = "zoom";
pub const TAB_MENU: &str = "tab-menu";
// BK-016: with Shift the kit matches the produced character, so ⇧] is declared as `}` and ⇧0 as `)`.
pub const NEXT_TAB: &str = "next-tab";
pub const PREVIOUS_TAB: &str = "previous-tab";
pub const MOVE_TAB_LEFT: Action = Action::new("move-tab-left", || tr!("move-tab-left"));
pub const MOVE_TAB_RIGHT: Action = Action::new("move-tab-right", || tr!("move-tab-right"));
pub const SELECT_TAB: &str = "select-tab";
pub const SELECT_TAB_PREFIX: &str = "select-tab:";
pub const ARRANGEMENTS_MENU: &str = "arrangements-menu";
pub const RESTORE_ARRANGEMENT: &str = "restore-arrangement";
pub const RESTORE_ARRANGEMENT_TABS: &str = "restore-arrangement-tabs";
pub const ARRANGEMENT_PREFIX: &str = "arrangement:";
pub const ARRANGEMENT_TABS_PREFIX: &str = "arrangement-tabs:";
pub const SAVE_ARRANGEMENT: Action = Action::new("save-arrangement", || tr!("save-arrangement"))
    .shortcut(super_with(Modifiers::ALT.union(Modifiers::SHIFT), 's'));
pub const PROFILE_PREFIX: &str = "profile:";
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

pub fn terminal_state_entry(state: TerminalState) -> (&'static str, &'static str) {
    match state {
        TerminalState::AlternateScreen => ("state-alternate-screen", tr!("state-alternate-screen")),
        TerminalState::FocusReporting => ("state-focus-reporting", tr!("state-focus-reporting")),
        TerminalState::MouseReporting => ("state-mouse-reporting", tr!("state-mouse-reporting")),
        TerminalState::BracketedPaste => ("state-bracketed-paste", tr!("state-bracketed-paste")),
        TerminalState::ApplicationCursor => {
            ("state-application-cursor", tr!("state-application-cursor"))
        }
        TerminalState::ApplicationKeypad => {
            ("state-application-keypad", tr!("state-application-keypad"))
        }
    }
}

fn keyed(id: &'static str, label: &str, shortcut: &str) -> MenuItem {
    MenuItem::action(id, label).with_shortcut(shortcut)
}

pub fn terminal_menu(profiles: &[String]) -> AppMenu {
    AppMenu::new(vec![
        app_menu(),
        Menu::new(
            tr!("menu-shell"),
            vec![
                NEW_WINDOW.item(true),
                NEW_WINDOW_PROFILE.item(true),
                MenuItem::Separator,
                NEW_TAB.item(true),
                NEW_TAB_PROFILE.item(true),
                MenuItem::Separator,
                DUPLICATE_TAB.item(true),
                DUPLICATE_WINDOW.item(true),
                MenuItem::Separator,
                SPLIT_HORIZONTALLY.item(true),
                SPLIT_VERTICALLY.item(true),
                MenuItem::Separator,
                SAVE_CONTENTS.item(true),
                SAVE_SELECTION.item(false),
                MenuItem::Separator,
                CLOSE.item(true),
                CLOSE_WINDOW.item(true),
                CLOSE_PANES.item(false),
                UNDO_CLOSE.item(false),
                MenuItem::Separator,
                submenu(
                    BROADCAST_MENU,
                    tr!("broadcast-menu"),
                    broadcast_items(Broadcast::Off),
                ),
            ],
        ),
        Menu::new(
            tr!("menu-edit"),
            vec![
                COPY.item(false),
                COPY_STYLED.item(false),
                COPY_MODE.item_with(true, CheckState::Checkbox(false)),
                PASTE.item(true),
                submenu(
                    PASTE_SPECIAL_MENU,
                    tr!("paste-special-menu"),
                    vec![
                        ADVANCED_PASTE.item(true),
                        PASTE_SLOWLY.item(true),
                        PASTE_ESCAPED.item(true),
                        PASTE_SELECTION.item(false),
                        MenuItem::Separator,
                        MenuItem::action(WARN_MULTI_LINE_PASTE, tr!("warn-multi-line-paste"))
                            .with_check(CheckState::Checkbox(true)),
                        MenuItem::action(LIMIT_PASTE_WARNING, tr!("limit-paste-warning"))
                            .with_check(CheckState::Checkbox(true)),
                        MenuItem::action(WARN_ONE_LINE_PASTE, tr!("warn-one-line-paste"))
                            .with_check(CheckState::Checkbox(true)),
                    ],
                ),
                MenuItem::Separator,
                OPEN_SELECTION.item(false),
                MenuItem::Separator,
                SELECT_ALL.item(true),
                SELECT_OUTPUT.item(false),
                SELECT_CURRENT_COMMAND.item(false),
                MenuItem::Separator,
                submenu(
                    FIND_MENU,
                    tr!("find-menu"),
                    vec![
                        FIND.item(true),
                        FIND_NEXT.item(true),
                        FIND_PREVIOUS.item(true),
                        USE_SELECTION_FOR_FIND.item(false),
                        CLEAR_FIND.item(false),
                        FIND_GLOBALLY.item(true),
                        MenuItem::Separator,
                        FIND_URLS.item(true),
                        SMART_SELECT_ALL.item(true),
                        PICK_RESULT.item(false),
                        MenuItem::Separator,
                        FILTER.item(true),
                    ],
                ),
                submenu(
                    MARKS_MENU,
                    tr!("marks-menu"),
                    vec![
                        SET_MARK.item(true),
                        JUMP_TO_MARK.item(false),
                        keyed(NEXT_MARK, tr!("next-mark"), "Super+Shift+Down").with_enabled(false),
                        keyed(PREVIOUS_MARK, tr!("previous-mark"), "Super+Shift+Up")
                            .with_enabled(false),
                        ADD_ANNOTATION.item(true),
                        ANNOTATE_SELECTION.item(false),
                        keyed(
                            NEXT_ANNOTATION,
                            tr!("next-annotation"),
                            "Super+Alt+Shift+Down",
                        ),
                        keyed(
                            PREVIOUS_ANNOTATION,
                            tr!("previous-annotation"),
                            "Super+Alt+Shift+Up",
                        ),
                        submenu(ALERTS_MENU, tr!("alerts-menu"), Vec::new()),
                    ],
                ),
                SET_NAMED_MARK.item(true),
                submenu(NAMED_MARKS_MENU, tr!("named-marks-menu"), Vec::new()),
                MenuItem::Separator,
                CLEAR_BUFFER.item(true),
                CLEAR_REPLAY.item(true),
                CLEAR_SCROLLBACK.item(false),
                CLEAR_TO_SELECTION.item(false),
                CLEAR_TO_LAST_MARK.item(false),
            ],
        ),
        Menu::new(
            tr!("menu-view"),
            vec![
                SHOW_TAB_BAR.item_with(true, CheckState::Checkbox(false)),
                FULL_SCREEN.item(true),
                USE_TRANSPARENCY.item_with(false, CheckState::Checkbox(true)),
                MenuItem::Separator,
                FIND_CURSOR.item(true),
                SHOW_ANNOTATIONS.item_with(true, CheckState::Checkbox(false)),
                keyed(SHOW_COMPOSER, tr!("show-composer"), "Super+Shift+>"),
                OPEN_QUICKLY.item(true),
                INSTANT_REPLAY.item(true),
                TOOLBELT.item_with(true, CheckState::Checkbox(false)),
                SHOW_TIMESTAMPS.item_with(true, CheckState::Checkbox(false)),
                CURSOR_GUIDE.item_with(true, CheckState::Checkbox(false)),
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
            tr!("menu-session"),
            vec![
                EDIT_SESSION.item(true),
                DUPLICATE_SESSION.item(true),
                MenuItem::Separator,
                RUN_COPROCESS.item(true),
                STOP_COPROCESS.item(false),
                MenuItem::Separator,
                RESTART_SESSION.item(true),
                MenuItem::Separator,
                RECENT_DIRECTORIES.item(false),
                PASTE_HISTORY.item(false),
                MenuItem::Separator,
                submenu(
                    LOG_MENU,
                    tr!("log-menu"),
                    vec![LOG_START.item(true), LOG_STOP.item(false)],
                ),
                MenuItem::Separator,
                RESET.item(true),
                RESET_CHARSET.item(true),
                submenu(
                    TERMINAL_STATE_MENU,
                    tr!("terminal-state-menu"),
                    TerminalState::ALL
                        .map(|state| {
                            let (id, label) = terminal_state_entry(state);
                            MenuItem::action(id, label).with_check(CheckState::Checkbox(false))
                        })
                        .to_vec(),
                ),
                MenuItem::Separator,
                submenu(
                    TRIGGERS_MENU,
                    tr!("triggers-menu"),
                    vec![
                        TRIGGERS_ENABLED.item_with(true, CheckState::Checkbox(true)),
                        EDIT_TRIGGERS.item(true),
                    ],
                ),
            ],
        ),
        Menu::new(tr!("menu-profiles"), profile_items(profiles)),
        Menu::new(
            tr!("menu-window"),
            vec![
                MINIMIZE.item(true),
                keyed(ZOOM, tr!("zoom"), "Super+Shift+)"),
                MenuItem::Separator,
                MenuItem::action(ARRANGE_EVENLY, tr!("arrange-evenly")).with_enabled(false),
                MenuItem::Separator,
                EDIT_WINDOW_TITLE.item(true),
                EDIT_TAB_TITLE.item(true),
                MenuItem::Separator,
                submenu(
                    TAB_MENU,
                    tr!("tab-menu"),
                    vec![
                        keyed(NEXT_TAB, tr!("next-tab"), "Super+Shift+}").with_enabled(false),
                        keyed(PREVIOUS_TAB, tr!("previous-tab"), "Super+Shift+{")
                            .with_enabled(false),
                        MenuItem::Separator,
                        MOVE_TAB_LEFT.item(false),
                        MOVE_TAB_RIGHT.item(false),
                        MenuItem::Separator,
                        submenu(SELECT_TAB, tr!("select-tab"), Vec::new()),
                    ],
                ),
                submenu(SPLIT_PANE, tr!("split-pane"), split_pane_items()),
                MenuItem::Separator,
                submenu(
                    ARRANGEMENTS_MENU,
                    tr!("arrangements-menu"),
                    vec![
                        SAVE_ARRANGEMENT.item(true),
                        MenuItem::Separator,
                        submenu(RESTORE_ARRANGEMENT, tr!("restore-arrangement"), Vec::new()),
                        submenu(
                            RESTORE_ARRANGEMENT_TABS,
                            tr!("restore-arrangement-tabs"),
                            Vec::new(),
                        ),
                    ],
                ),
            ],
        ),
        Menu::new(
            tr!("menu-help"),
            vec![HELP.item(true), COPY_MODE_KEYS.item(true)],
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

pub fn broadcast_items(current: Broadcast) -> Vec<MenuItem> {
    [
        (BROADCAST_OFF, tr!("broadcast-off"), Broadcast::Off, None),
        (
            BROADCAST_ALL_TABS,
            tr!("broadcast-all-tabs"),
            Broadcast::AllTabs,
            Some("Super+Alt+i"),
        ),
        (
            BROADCAST_CURRENT_TAB,
            tr!("broadcast-current-tab"),
            Broadcast::CurrentTab,
            Some("Super+Shift+Alt+I"),
        ),
    ]
    .into_iter()
    .map(|(id, label, mode, shortcut)| {
        let item = MenuItem::action(id, label).with_check(CheckState::Radio(mode == current));
        match shortcut {
            Some(shortcut) => item.with_shortcut(shortcut),
            None => item,
        }
    })
    .collect()
}

// MainMenu.xib Marks and Annotations › Alerts.
pub fn alert_items(next_mark: bool, offscreen: bool, action: MarkAlert) -> Vec<MenuItem> {
    let radio = |id: &'static str, label: &str, mode: MarkAlert| {
        MenuItem::action(id, label).with_check(CheckState::Radio(mode == action))
    };
    vec![
        keyed(ALERT_ON_NEXT_MARK, tr!("alert-on-next-mark"), "Super+Alt+a")
            .with_check(CheckState::Checkbox(next_mark)),
        MenuItem::action(ALERT_OFFSCREEN_MARKS, tr!("alert-offscreen-marks"))
            .with_check(CheckState::Checkbox(offscreen)),
        MenuItem::Separator,
        radio(
            MARK_ALERT_MODAL,
            tr!("mark-alert-modal"),
            MarkAlert::ModalAlert,
        ),
        radio(
            MARK_ALERT_NOTIFICATION,
            tr!("mark-alert-notification"),
            MarkAlert::PostNotification,
        ),
        radio(
            MARK_ALERT_SOUND,
            tr!("mark-alert-sound"),
            MarkAlert::PlaySound,
        ),
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

pub fn indexed_items(prefix: &str, names: &[String]) -> Vec<MenuItem> {
    names
        .iter()
        .enumerate()
        .map(|(index, name)| MenuItem::action(format!("{prefix}{index}"), name.clone()))
        .collect()
}

pub fn profile_items(profiles: &[String]) -> Vec<MenuItem> {
    let open = [OPEN_PROFILES.item(true), MenuItem::Separator];
    let listed = profiles.iter().enumerate().map(|(index, name)| {
        let item = MenuItem::action(format!("{PROFILE_PREFIX}{index}"), name.clone());
        match index {
            0..9 => item.with_shortcut(format!("Ctrl+Super+{}", index + 1)),
            _ => item,
        }
    });
    open.into_iter().chain(listed).collect()
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
        // The shell merges a menu titled like the app into its own app menu.
        APP_NAME,
        vec![
            ABOUT.item(true),
            MenuItem::Separator,
            SETTINGS.item(true),
            MenuItem::Separator,
            QUIT.item(true),
        ],
    )
}
