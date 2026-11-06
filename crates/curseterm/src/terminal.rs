use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use alacritty_terminal::event::{Event as TermEvent, WindowSize};
use alacritty_terminal::grid::Scroll;
use alacritty_terminal::index::Direction as SearchDirection;
use barrier_kit::app::use_menu_action;
use barrier_kit::app_menu::MenuHandle;
use barrier_kit::controls::{Button, ButtonStyle, SearchField, TextField};
use barrier_kit::platform::{Timers, WakeHandle, WindowControls};
use barrier_kit::style::{Palette, text};
use barrier_kit::symbol::Symbol;
use barrier_kit::theme::color;
use barrier_kit::titlebar::Titlebar;
use barrier_kit::views::feedback::{Alert, sheet};
use barrier_kit::views::table::Table;
use barrier_kit_next::app_menu::{use_menu_checked, use_menu_enabled, use_submenu_items};
use barrier_kit_next::tokens::{PASTE_HISTORY_SIZE, RENAME_WIDTH};
use barrier_kit_next::views::split_panes::SplitPanes;
use barrier_kit_next::views::tab_bar::{Badge, TabBar};
use freya_clipboard::copypasta::ClipboardProvider;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Content, Direction, Size};

use vow_design::{CAPTION, DURATION_FAST_MS, DURATION_SLOW_MS, HEADLINE, Rgba, SPACE_1, SPACE_2};

use crate::config::{
    APP_ID, AltKey, ClosePrompt, Defaults, Profile, SessionEnd, WorkingDirectory, change_defaults,
};
use crate::grid::Fonts;
use crate::keys::{COPY_KEYS, CopyCommand, KeyAction, copy_command, encode};
use crate::links;
use crate::localize::tr;
use crate::menu;
use crate::pane_view::TerminalPane;
use crate::panes::{Axis, Pane, Toward};
use crate::paste_history;
use crate::pick;
use crate::session::{
    self, GridSize, Launch, Listener, MarkJump, Session, SessionId, TerminalState,
};
use crate::windows;
use crate::workspace::{Broadcast, Closed, Tab, Workspace};

const INITIAL_GRID: (u16, u16) = (80, 24);
const QUICK_EXIT: Duration = Duration::from_secs(1);
const DIVIDER_STEP: f32 = 0.05;
const UNDO_CLOSE: Duration = Duration::from_secs(5);
const SLOW_PASTE_CHUNK: usize = 16;
const SLOW_PASTE_PAUSE: Duration = Duration::from_millis(30);

type Sessions = HashMap<SessionId, Rc<Session>>;
type ClipboardState = State<Option<Box<dyn ClipboardProvider>>>;
type Events = Rc<(Sender<(SessionId, TermEvent)>, WakeHandle)>;
type FontCache = Rc<RefCell<HashMap<(String, i32), Rc<Fonts>>>>;

// Menu actions and wake-ups run outside Freya's context (BK-017): they use handles taken in render.
#[derive(Clone)]
struct Handles {
    workspace: State<Workspace>,
    sessions: State<Sessions>,
    next_id: State<SessionId>,
    defaults: Memo<Defaults>,
    text_zoom: State<i32>,
    fonts: FontCache,
    controls: WindowControls,
    timers: Timers,
    alive: Rc<Cell<bool>>,
    dark: Rc<Cell<bool>>,
    clipboard: ClipboardState,
    broadcast: State<Broadcast>,
    find: Find,
    closing: State<Option<Closing>>,
    events: Rc<OnceCell<Events>>,
    buried: State<Option<Buried>>,
    pastes: State<Vec<String>>,
    picker: PickerState,
    sheet: State<Option<Sheet>>,
    draft: State<String>,
    held: State<Modifiers>,
    transparency: State<bool>,
}

#[derive(Clone, Copy, PartialEq)]
enum Sheet {
    Pick(Picker),
    RenameTab(SessionId),
    RenameWindow,
    SaveSelection(SessionId),
    Log(SessionId),
    Coprocess(SessionId),
    CopyModeKeys,
}

struct Buried {
    tab: Tab,
    sessions: Vec<Rc<Session>>,
    titles: Vec<(SessionId, String)>,
    at: Instant,
}

#[derive(Clone, Copy, PartialEq)]
enum Picker {
    PasteHistory,
    OpenQuickly,
    Profiles,
}

#[derive(Clone)]
enum Choice {
    Paste(String),
    Focus(SessionId),
    Profile(String),
}

#[derive(Clone, Copy)]
struct PickerState {
    query: State<String>,
    selected: State<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum Closing {
    Session(SessionId),
    TabOf(SessionId),
    Window,
}

#[derive(Clone, Copy)]
struct Find {
    open: State<bool>,
    query: State<String>,
    focused: State<bool>,
    position: State<Option<(usize, usize)>>,
}

impl Find {
    fn show_position(self, session: &Session, query: &str) {
        let mut position = self.position;
        position.set(session.find_position(query));
    }

    fn close(self) {
        let (mut open, mut focused) = (self.open, self.focused);
        open.set(false);
        focused.set(false);
    }
}

impl Handles {
    // BK-032: Timers callbacks outlive their window, so a closed window's ones must do nothing.
    fn after(&self, delay: Duration, callback: impl Fn() + 'static) {
        let alive = self.alive.clone();
        self.timers.after(delay, move || {
            if alive.get() {
                callback();
            }
        });
    }

    fn active(&self) -> Option<Rc<Session>> {
        let id = self.workspace.peek().active_session()?;
        self.sessions.peek().get(&id).cloned()
    }

    fn receivers(&self) -> Vec<Rc<Session>> {
        let ids = self.workspace.peek().receivers(*self.broadcast.peek());
        let sessions = self.sessions.peek();
        ids.iter()
            .filter_map(|id| sessions.get(id).cloned())
            .collect()
    }

    fn find(&self, direction: SearchDirection) {
        let query = self.find.query.peek().clone();
        if let Some(session) = self.active().filter(|_| !query.is_empty()) {
            session.find(&query, direction);
            self.find.show_position(&session, &query);
        }
    }

    fn fonts_for(&self, profile: &Profile) -> Rc<Fonts> {
        let size = profile.text_size(*self.text_zoom.peek());
        self.fonts
            .borrow_mut()
            .entry((profile.font_family.clone(), size as i32))
            .or_insert_with(|| Rc::new(Fonts::new(&profile.font_family, size)))
            .clone()
    }

    fn launch(&self, profile: &str, previous: Option<PathBuf>) -> Launch {
        let profile = self.defaults.peek().profile(profile);
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let directory = match profile.working_directory {
            WorkingDirectory::Home => home,
            WorkingDirectory::Previous => previous.or(home),
            WorkingDirectory::Fixed(path) => Some(path),
        };
        Launch {
            profile: profile.name,
            command: profile.command,
            directory,
        }
    }

    fn current_profile(&self) -> String {
        self.active().map_or_else(
            || self.defaults.peek().default_profile_name(),
            |session| session.profile.clone(),
        )
    }
}

pub struct Terminal {
    pub profile: Option<String>,
}

impl App for Terminal {
    fn render(&self) -> impl IntoElement {
        windows::use_registered_window();
        let defaults = barrier_kit::app::use_defaults::<Defaults>(APP_ID);
        let dark = barrier_kit::theme::resolved().scheme.is_dark();
        let workspace = use_state(Workspace::default);
        let sessions = use_state(Sessions::new);
        let next_id = use_state(|| 0);
        let text_zoom = use_state(|| 0i32);
        let mut revision = use_state(|| 0u64);
        let broadcast = use_state(Broadcast::default);
        let closing = use_state(|| None::<Closing>);
        let buried = use_state(|| None::<Buried>);
        let pastes = paste_history::shared();
        let sheet = use_state(|| None::<Sheet>);
        let mut held = use_state(Modifiers::empty);
        let transparency = use_state(|| true);
        let draft = use_state(String::new);
        let picker = PickerState {
            query: use_state(String::new),
            selected: use_state(|| 0),
        };
        let find = Find {
            open: use_state(|| false),
            query: use_state(String::new),
            focused: use_state(|| false),
            position: use_state(|| None),
        };
        let handles = use_hook(move || Handles {
            workspace,
            sessions,
            next_id,
            defaults,
            text_zoom,
            fonts: FontCache::default(),
            controls: WindowControls::get(),
            timers: Timers::get(),
            alive: Rc::new(Cell::new(true)),
            dark: Rc::new(Cell::new(dark)),
            clipboard: consume_root_context(),
            broadcast,
            find,
            closing,
            events: Rc::default(),
            buried,
            pastes,
            picker,
            sheet,
            draft,
            held,
            transparency,
        });
        handles.dark.set(dark);
        use_drop({
            let alive = handles.alive.clone();
            move || alive.set(false)
        });
        use_side_effect({
            let handles = handles.clone();
            move || {
                let defaults = defaults.read().clone();
                text_zoom.read();
                for session in sessions.peek().values() {
                    let profile = defaults.profile(&session.profile);
                    let fonts = handles.fonts_for(&profile);
                    session.refit(fonts.cell_width, fonts.cell_height);
                    session.apply_profile(&profile);
                }
                *revision.write() += 1;
            }
        });
        use_side_effect({
            let handles = handles.clone();
            move || {
                let query = find.query.read().clone();
                if let Some(session) = handles.active().filter(|_| !query.is_empty()) {
                    session.clear_selection();
                    session.find(&query, SearchDirection::Left);
                    *revision.write() += 1;
                    let typed = query.clone();
                    handles.after(Duration::from_millis(DURATION_FAST_MS), move || {
                        if *find.query.peek() == typed {
                            find.show_position(&session, &typed);
                        }
                    });
                }
            }
        });
        let events = use_hook({
            let handles = handles.clone();
            move || {
                let events = listen(handles.clone(), revision);
                let _ = handles.events.set(events.clone());
                events
            }
        });
        use_hook({
            let (handles, events) = (handles.clone(), events.clone());
            let first = self.profile.clone();
            move || {
                let profile =
                    first.unwrap_or_else(|| handles.defaults.peek().default_profile_name());
                new_tab(&handles, &events, &profile, None)
            }
        });
        let add_events = events.clone();
        use_menu_action({
            let handles = handles.clone();
            move |id| {
                on_action(id, &handles, &events, revision);
                let mut revision = revision;
                *revision.write() += 1;
            }
        });
        use_live_menu(&handles);

        let window_focused = handles.controls.status.read().activated;
        use_focus_report(&handles, window_focused);
        let palette = Palette::current();
        let state = workspace.read();
        let title = state.window_title().to_string();
        let show_tab_bar = defaults.read().show_tab_bar;
        let tab_bar = (state.tabs.len() > 1 || show_tab_bar).then(|| {
            let (on_close, on_add) = (handles.clone(), handles.clone());
            // BK-024: the kit has no bell symbol; SpeakerWave stands in for iTerm2's bell.
            let bell = Badge {
                symbol: Symbol::SpeakerWave,
                label: tr!("tab-bell"),
            };
            state
                .tab_titles()
                .into_iter()
                .zip(state.rang_tabs())
                .fold(
                    TabBar::new(tr!("tabs"), tr!("close-tab")),
                    |bar, (title, rang)| {
                        if rang {
                            bar.badged_tab(title, bell)
                        } else {
                            bar.tab(title)
                        }
                    },
                )
                .selected(state.active)
                .on_select(move |index| {
                    let mut workspace = workspace;
                    workspace.write().select_tab(index);
                })
                .on_close(move |index: usize| {
                    let active = on_close
                        .workspace
                        .peek()
                        .tabs
                        .get(index)
                        .map(|tab| tab.active);
                    if let Some(id) = active {
                        request_close(&on_close, Closing::TabOf(id));
                    }
                })
                .on_add(tr!("new-tab-button"), move |_| {
                    let profile = on_add.defaults.peek().default_profile_name();
                    new_tab(&on_add, &add_events, &profile, None)
                })
        });
        let transparent = transparency_supported() && *transparency.read();
        let body = state.active_tab().map(|tab| {
            let root = if tab.zoomed {
                Pane::Leaf(tab.active)
            } else {
                tab.root.clone()
            };
            let painting = Painting {
                sessions: sessions.read().clone(),
                defaults: defaults.read().clone(),
                handles: handles.clone(),
                dark,
                active: tab.active,
                window_focused,
                revision: *revision.read(),
                workspace,
                tab: state.active,
                find: (*find.open.read() && !find.query.read().is_empty())
                    .then(|| Rc::from(find.query.read().as_str())),
                receivers: match *broadcast.read() {
                    Broadcast::Off => Vec::new(),
                    mode => state.receivers(mode),
                },
                transparent,
                preedit: ime_preedit(&handles),
            };
            pane_tree(&root, Vec::new(), &painting)
        });
        drop(state);
        #[cfg(feature = "kit-parcheado")]
        use_background_alpha(if transparent { 0.0 } else { 1.0 });
        #[cfg(feature = "kit-parcheado")]
        place_ime_cursor(&handles);
        #[cfg(feature = "kit-parcheado")]
        use_window_title(&title);

        let keyboard = handles.clone();
        let find_bar = (*find.open.read()).then(|| find_bar(&handles, revision));
        let alert = (*closing.read()).map(|pending| close_alert(&handles, pending));
        let sheet_view = (*sheet.read()).map(|open| match open {
            Sheet::Pick(picker) => pick_sheet(&handles, picker, revision),
            Sheet::CopyModeKeys => copy_mode_keys_sheet(&handles),
            _ => field_sheet(&handles, open),
        });
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .direction(Direction::Vertical)
            .content(Content::flex())
            .background(color(if transparent {
                Rgba {
                    a: 0.0,
                    ..palette.window_background
                }
            } else {
                palette.window_background
            }))
            .on_global_key_up(move |event: Event<KeyboardEventData>| {
                let data = event.data();
                held.set_if_modified(data.modifiers.difference(key_modifier(&data.key)));
            })
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                let data = event.data();
                held.set_if_modified(data.modifiers.union(key_modifier(&data.key)));
                if keyboard.closing.peek().is_some() {
                    return;
                }
                if keyboard.sheet.peek().is_some() {
                    if data.key == Key::Named(NamedKey::Escape) {
                        let mut sheet = keyboard.sheet;
                        sheet.set(None);
                    }
                    return;
                }
                if *find.open.peek() && *find.focused.peek() {
                    if data.key == Key::Named(NamedKey::Escape) {
                        find.close();
                    }
                    return;
                }
                let Some(session) = &keyboard.active() else {
                    return;
                };
                if session.copy_mode() {
                    if let Some(command) = copy_command(&data.key, data.modifiers) {
                        if command == CopyCommand::Copy {
                            copy(&keyboard, session);
                        }
                        session.copy_mode_step(command);
                        *revision.write() += 1;
                    }
                    return;
                }
                let alt_key = keyboard.defaults.peek().profile(&session.profile).alt_key;
                let modifiers = match (&data.key, alt_key) {
                    (Key::Character(_), AltKey::Normal) => {
                        data.modifiers.difference(Modifiers::ALT)
                    }
                    _ => data.modifiers,
                };
                match encode(&data.key, modifiers, session.mode()) {
                    KeyAction::Write(bytes) => {
                        for receiver in keyboard.receivers() {
                            receiver.clear_selection();
                            receiver.write(bytes.clone());
                        }
                    }
                    KeyAction::Copy => copy(&keyboard, session),
                    KeyAction::Paste => paste(&keyboard),
                    KeyAction::PageUp => session.scroll(Scroll::PageUp),
                    KeyAction::PageDown => session.scroll(Scroll::PageDown),
                    KeyAction::Ignore => return,
                }
                *revision.write() += 1;
            })
            .child(Titlebar::new().title(title))
            .maybe_child(tab_bar)
            .maybe_child(find_bar)
            .maybe_child(alert)
            .maybe_child(sheet_view)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .maybe_child(body),
            )
    }
}

struct Painting {
    sessions: Sessions,
    defaults: Defaults,
    handles: Handles,
    dark: bool,
    active: SessionId,
    window_focused: bool,
    revision: u64,
    workspace: State<Workspace>,
    tab: usize,
    receivers: Vec<SessionId>,
    find: Option<Rc<str>>,
    transparent: bool,
    preedit: Option<Rc<str>>,
}

fn pane_tree(node: &Pane, path: Vec<usize>, painting: &Painting) -> Element {
    match node {
        Pane::Leaf(id) => match painting.sessions.get(id) {
            Some(session) => {
                let mut workspace = painting.workspace;
                let profile = painting.defaults.profile(&session.profile);
                TerminalPane {
                    session: session.clone(),
                    fonts: painting.handles.fonts_for(&profile),
                    scheme: Rc::new(painting.defaults.scheme(&profile, painting.dark)),
                    focused: painting.window_focused && *id == painting.active,
                    revision: painting.revision,
                    broadcast: painting.receivers.contains(id),
                    find: painting.find.clone().filter(|_| *id == painting.active),
                    on_focus: Callback::new(move |id| workspace.write().focus(id)),
                    held: painting.handles.held,
                    preedit: painting.preedit.clone().filter(|_| *id == painting.active),
                    opacity: if painting.transparent {
                        profile.opacity()
                    } else {
                        1.0
                    },
                    badge: (!profile.badge.is_empty()).then(|| {
                        let title = painting.workspace.peek().title(*id).to_string();
                        Rc::from(profile.badge.replace(r"\(session.name)", &title))
                    }),
                }
                .into_element()
            }
            None => rect().into_element(),
        },
        Pane::Split {
            axis,
            children,
            ratios,
        } => {
            let direction = match axis {
                Axis::Row => Direction::Horizontal,
                Axis::Column => Direction::Vertical,
            };
            let (mut workspace, tab, at) = (painting.workspace, painting.tab, path.clone());
            let split = SplitPanes::new(tr!("panes"), direction)
                .ratios(ratios.clone())
                .on_ratios(move |next: Vec<f32>| {
                    let mut state = workspace.write();
                    if let Some(slot) = state
                        .tabs
                        .get_mut(tab)
                        .and_then(|tab| tab.root.ratios_at(&at))
                    {
                        *slot = next;
                    }
                });
            children
                .iter()
                .enumerate()
                .fold(split, |split, (index, child)| {
                    let mut path = path.clone();
                    path.push(index);
                    split.pane(pane_tree(child, path, painting))
                })
                .into_element()
        }
    }
}

fn use_live_menu(handles: &Handles) {
    let state = handles.workspace.read();
    let tabs = state.tabs.len();
    let split = state.active_tab().is_some_and(|tab| tab.root.is_split());
    let zoomed = state.active_tab().is_some_and(|tab| tab.zoomed);
    let titles = state.tab_titles();
    let active = state.active;
    drop(state);
    let selection = handles
        .active()
        .is_some_and(|session| session.selected_text().is_some());
    use_menu_enabled(menu::COPY.id, selection);
    use_menu_enabled(menu::USE_SELECTION_FOR_FIND.id, selection);
    use_menu_enabled(menu::OPEN_SELECTION.id, selection);
    use_menu_enabled(menu::COPY_STYLED.id, selection);
    use_menu_enabled(menu::PASTE_SELECTION.id, selection);
    let logging = handles.active().is_some_and(|session| session.logging());
    use_menu_enabled(menu::LOG_START.id, !logging);
    use_menu_enabled(menu::LOG_STOP.id, logging);
    let coprocess = handles
        .active()
        .is_some_and(|session| session.coprocess_running());
    use_menu_enabled(menu::STOP_COPROCESS.id, coprocess);
    use_menu_enabled(menu::SAVE_SELECTION.id, selection);
    let output = handles.active().is_some_and(|session| session.has_output());
    use_menu_enabled(menu::SELECT_OUTPUT.id, output);
    let copy_mode = handles.active().is_some_and(|session| session.copy_mode());
    use_menu_checked(menu::COPY_MODE.id, copy_mode);
    let timestamps = handles
        .active()
        .is_some_and(|session| session.extras().show_timestamps);
    use_menu_checked(menu::SHOW_TIMESTAMPS.id, timestamps);
    for state in TerminalState::ALL {
        let on = handles
            .active()
            .is_some_and(|session| session.has_state(state));
        use_menu_checked(menu::terminal_state_entry(state).0, on);
    }
    use_menu_enabled(menu::USE_TRANSPARENCY.id, transparency_supported());
    use_menu_checked(menu::USE_TRANSPARENCY.id, *handles.transparency.read());
    let marks = handles.active().is_some_and(|session| session.has_marks());
    let triggers = handles
        .active()
        .is_some_and(|session| session.extras().triggers_enabled);
    use_menu_checked(menu::TRIGGERS_ENABLED.id, triggers);
    use_menu_enabled(menu::JUMP_TO_MARK.id, marks);
    use_menu_enabled(menu::NEXT_MARK, marks);
    use_menu_enabled(menu::PREVIOUS_MARK, marks);
    use_submenu_items(
        menu::BROADCAST_MENU,
        menu::broadcast_items(*handles.broadcast.read()),
    );
    let scrollback = handles
        .active()
        .is_some_and(|session| session.has_scrollback());
    use_menu_enabled(menu::CLEAR_SCROLLBACK.id, scrollback);
    use_menu_enabled(menu::CLOSE_PANES.id, split);
    use_menu_enabled(menu::UNDO_CLOSE.id, handles.buried.read().is_some());
    use_menu_enabled(menu::PASTE_HISTORY.id, !handles.pastes.read().is_empty());
    use_menu_enabled(menu::MAXIMIZE_PANE, split);
    use_menu_checked(menu::MAXIMIZE_PANE, zoomed);
    use_menu_checked(menu::SHOW_TAB_BAR.id, handles.defaults.read().show_tab_bar);
    use_menu_enabled(menu::NEXT_TAB, tabs > 1);
    use_menu_enabled(menu::PREVIOUS_TAB, tabs > 1);
    use_menu_enabled(menu::MOVE_TAB_LEFT.id, active > 0);
    use_menu_enabled(menu::MOVE_TAB_RIGHT.id, active + 1 < tabs);
    use_menu_enabled(menu::SPLIT_PANE, split);
    use_submenu_items(menu::SELECT_TAB, menu::select_tab_items(&titles, active));
    let profiles = handles.defaults.read().profiles();
    use_profiles_menu(profiles.into_iter().map(|profile| profile.name).collect());
}

fn use_profiles_menu(names: Vec<String>) {
    let handle = use_hook(try_consume_root_context::<MenuHandle>);
    let mut published = use_state(|| None::<Vec<String>>);
    if published.peek().as_ref() == Some(&names) {
        return;
    }
    published.set(Some(names.clone()));
    let Some(handle) = handle else {
        return;
    };
    #[cfg(feature = "kit-parcheado")]
    handle.set_menu_items(tr!("menu-profiles"), menu::profile_items(&names));
    // BK-010: without patch 0004 only the entries the window opened with can change.
    #[cfg(not(feature = "kit-parcheado"))]
    for (index, name) in names.iter().enumerate() {
        handle.update(format!("{}{index}", menu::PROFILE_PREFIX), |item| {
            if let barrier_kit::menu::MenuItem::Action { label, .. } = item {
                *label = name.clone();
            }
        });
    }
}

fn copy(handles: &Handles, session: &Session) {
    if let Some(text) = session.selected_text() {
        remember_paste(handles, text.clone());
        set_clipboard(handles.clipboard, text);
    }
}

// BK-035: without patch 0011 the clipboard only takes plain text.
#[cfg(not(feature = "kit-parcheado"))]
fn copy_styled(handles: &Handles, session: &Session) {
    copy(handles, session);
}

#[cfg(feature = "kit-parcheado")]
fn copy_styled(handles: &Handles, session: &Session) {
    let defaults = handles.defaults.peek();
    let profile = defaults.profile(&session.profile);
    let scheme = defaults.scheme(&profile, handles.dark.get());
    let (Some(text), Some(lines)) = (
        session.selected_text(),
        crate::styled::selection(session, &scheme),
    ) else {
        return;
    };
    let html = crate::styled::html(&lines, scheme.background, &profile.font_family);
    drop(defaults);
    remember_paste(handles, text.clone());
    handles.controls.copy_rich(text, html);
}

fn remember_paste(handles: &Handles, text: String) {
    let mut pastes = handles.pastes;
    paste_history::remember(&mut pastes.write(), text);
}

fn set_clipboard(mut clipboard: ClipboardState, text: String) {
    let copied = clipboard
        .write()
        .as_mut()
        .map(|provider| provider.set_contents(text));
    if !matches!(copied, Some(Ok(()))) {
        tracing::warn!("could not copy to the clipboard");
    }
}

fn clipboard_text(mut clipboard: ClipboardState) -> Option<String> {
    clipboard.write().as_mut()?.get_contents().ok()
}

fn paste(handles: &Handles) {
    match clipboard_text(handles.clipboard) {
        Some(text) => paste_text(handles, text),
        None => tracing::warn!("could not read the clipboard"),
    }
}

fn paste_text(handles: &Handles, text: String) {
    for session in handles.receivers() {
        session.paste(&text);
    }
    remember_paste(handles, text);
}

fn pick_entries(handles: &Handles, picker: Picker) -> Vec<(String, Choice)> {
    match picker {
        Picker::PasteHistory => handles
            .pastes
            .read()
            .iter()
            .map(|text| (paste_history::one_line(text), Choice::Paste(text.clone())))
            .collect(),
        Picker::OpenQuickly => {
            let workspace = handles.workspace.read();
            let sessions = handles.sessions.read();
            workspace
                .tabs
                .iter()
                .flat_map(|tab| tab.root.leaves())
                .filter_map(|id| {
                    let profile = &sessions.get(&id)?.profile;
                    let label = format!("{} — {profile}", workspace.title(id));
                    Some((label, Choice::Focus(id)))
                })
                .collect()
        }
        Picker::Profiles => handles
            .defaults
            .read()
            .profiles()
            .into_iter()
            .map(|profile| (profile.name.clone(), Choice::Profile(profile.name)))
            .collect(),
    }
}

fn pick_sheet(handles: &Handles, picker: Picker, mut revision: State<u64>) -> Element {
    let (heading, column) = match picker {
        Picker::PasteHistory => (tr!("paste-history"), tr!("paste-history-text")),
        Picker::OpenQuickly => (tr!("open-quickly"), tr!("open-quickly-session")),
        Picker::Profiles => (tr!("open-profiles"), tr!("open-profiles-name")),
    };
    let history = handles.picker;
    let entries = pick::matching(pick_entries(handles, picker), &history.query.read());
    let selected = (*history.selected.read()).min(entries.len().saturating_sub(1));
    let rows = entries
        .iter()
        .map(|(label, _)| vec![label.clone()])
        .collect();
    let (chosen, navigate) = (handles.clone(), entries.clone());
    let palette = Palette::current();
    let (width, height) = PASTE_HISTORY_SIZE;
    sheet(
        rect()
            .width(Size::px(width as f32))
            .height(Size::px(height as f32))
            .direction(Direction::Vertical)
            .content(Content::flex())
            .spacing(SPACE_2)
            .child(text(heading, HEADLINE, palette.label))
            .child(
                SearchField::new(history.query, tr!("paste-history-filter"))
                    .auto_focus(true)
                    .on_navigate(move |key| {
                        let mut selected = history.selected;
                        match key {
                            NamedKey::ArrowDown => selected
                                .set((*selected.peek() + 1).min(navigate.len().saturating_sub(1))),
                            NamedKey::ArrowUp => selected.set(selected.peek().saturating_sub(1)),
                            _ => {
                                if let Some((_, choice)) = navigate.get(*selected.peek()).cloned() {
                                    choose_later(&chosen, choice);
                                }
                            }
                        }
                        *revision.write() += 1;
                    }),
            )
            .child(
                rect().width(Size::fill()).height(Size::flex(1.0)).child(
                    Table::new(heading, vec![column])
                        .rows(rows)
                        .selected((!entries.is_empty()).then_some(selected))
                        .on_select(move |index: usize| {
                            let mut selected = history.selected;
                            selected.set(index);
                        }),
                ),
            ),
    )
    .into_element()
}

// BK-027: Enter also reaches the global handler, so the sheet closes one loop later.
fn choose_later(handles: &Handles, choice: Choice) {
    let later = handles.clone();
    handles.after(Duration::ZERO, move || {
        let mut sheet = later.sheet;
        sheet.set(None);
        match &choice {
            Choice::Paste(text) => paste_text(&later, text.clone()),
            Choice::Focus(id) => later.workspace.clone().write().focus(*id),
            Choice::Profile(name) => {
                if let Some(events) = later.events.get() {
                    new_tab(&later, events, name, None);
                }
            }
        }
    });
}

fn open_picker(handles: &Handles, picker: Picker) {
    let state = handles.picker;
    let (mut query, mut selected) = (state.query, state.selected);
    query.set(String::new());
    selected.set(0);
    open_sheet(handles, Sheet::Pick(picker));
}

fn open_sheet(handles: &Handles, open: Sheet) {
    let mut sheet = handles.sheet;
    sheet.set(Some(open));
}

fn field_sheet(handles: &Handles, target: Sheet) -> Element {
    let (heading, placeholder, confirm_label) = match target {
        Sheet::RenameTab(_) => (
            tr!("edit-tab-title"),
            tr!("title-automatic"),
            tr!("rename-confirm"),
        ),
        Sheet::SaveSelection(_) => (tr!("save-selection"), tr!("save-path"), tr!("save-confirm")),
        Sheet::Log(_) => (tr!("log-start"), tr!("save-path"), tr!("log-confirm")),
        Sheet::Coprocess(_) => (
            tr!("run-coprocess"),
            tr!("coprocess-command"),
            tr!("coprocess-confirm"),
        ),
        _ => (
            tr!("edit-window-title"),
            tr!("title-automatic"),
            tr!("rename-confirm"),
        ),
    };
    let (enter, confirm) = (handles.clone(), handles.clone());
    let mut shown = handles.sheet;
    sheet(
        rect()
            .width(Size::px(RENAME_WIDTH))
            .direction(Direction::Vertical)
            .spacing(SPACE_2)
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                if event.key == Key::Named(NamedKey::Enter) {
                    confirm_later(&enter, target);
                }
            })
            .child(text(heading, HEADLINE, Palette::current().label))
            .child(field(handles.draft, placeholder))
            .child(
                rect()
                    .width(Size::fill())
                    .direction(Direction::Horizontal)
                    .main_align(Alignment::End)
                    .spacing(SPACE_2)
                    .child(Button::new(tr!("close-cancel")).on_press(move |_| shown.set(None)))
                    .child(
                        Button::new(confirm_label)
                            .style(ButtonStyle::Primary)
                            .on_press(move |_| confirm_later(&confirm, target)),
                    ),
            ),
    )
    .into_element()
}

#[cfg(feature = "kit-parcheado")]
fn field(draft: State<String>, placeholder: &'static str) -> TextField {
    TextField::new(draft, placeholder).auto_focus(true)
}

// BK-029: without patch 0006 the field only takes the focus when clicked.
#[cfg(not(feature = "kit-parcheado"))]
fn field(draft: State<String>, placeholder: &'static str) -> TextField {
    TextField::new(draft, placeholder)
}

// BK-027: Enter also reaches the global handler, so the sheet closes one loop later.
fn confirm_later(handles: &Handles, target: Sheet) {
    let later = handles.clone();
    handles.after(Duration::ZERO, move || {
        let typed = later.draft.peek().clone();
        let mut workspace = later.workspace;
        match target {
            Sheet::RenameTab(of) => workspace.write().rename_tab(of, &typed),
            Sheet::SaveSelection(of) => save_selection(&later, of, &typed),
            Sheet::Log(of) => start_log(&later, of, &typed),
            Sheet::Coprocess(of) => start_coprocess(&later, of, &typed),
            _ => workspace.write().rename_window(&typed),
        }
        let mut sheet = later.sheet;
        sheet.set(None);
    });
}

fn save_selection(handles: &Handles, of: SessionId, path: &str) {
    let text = handles
        .sessions
        .peek()
        .get(&of)
        .and_then(|session| session.selected_text());
    let path = from_home(path);
    if let Some(text) = text
        && let Err(error) = std::fs::write(&path, text)
    {
        tracing::warn!(%error, path = %path.display(), "could not save the selection");
    }
}

fn from_home(typed: &str) -> PathBuf {
    let typed = typed.strip_prefix("~/").unwrap_or(typed);
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join(typed),
        None => PathBuf::from(typed),
    }
}

fn start_log(handles: &Handles, of: SessionId, path: &str) {
    let path = from_home(path);
    if let Some(session) = handles.sessions.peek().get(&of)
        && let Err(error) = session.start_log(&path)
    {
        tracing::warn!(%error, path = %path.display(), "could not start the session log");
    }
}

fn start_coprocess(handles: &Handles, of: SessionId, command: &str) {
    if command.trim().is_empty() {
        return;
    }
    if let Some(session) = handles.sessions.peek().get(&of)
        && let Err(error) = session.start_coprocess(command)
    {
        tracing::warn!(%error, command, "could not start the coprocess");
    }
}

fn paste_slowly(handles: &Handles, text: String) {
    for session in handles.receivers() {
        let bytes = session.pasted_bytes(&text);
        for (index, chunk) in bytes.chunks(SLOW_PASTE_CHUNK).enumerate() {
            let (session, chunk) = (session.clone(), chunk.to_vec());
            handles.after(SLOW_PASTE_PAUSE * index as u32, move || {
                session.write(chunk.clone())
            });
        }
    }
    remember_paste(handles, text);
}

fn copy_mode_keys_sheet(handles: &Handles) -> Element {
    let rows = COPY_KEYS
        .iter()
        .map(|(key, command)| vec![key.to_string(), copy_command_label(*command).to_string()])
        .collect();
    let mut shown = handles.sheet;
    let (width, height) = PASTE_HISTORY_SIZE;
    sheet(
        rect()
            .width(Size::px(width as f32))
            .height(Size::px(height as f32))
            .direction(Direction::Vertical)
            .content(Content::flex())
            .spacing(SPACE_2)
            .child(text(
                tr!("copy-mode-keys"),
                HEADLINE,
                Palette::current().label,
            ))
            .child(
                rect().width(Size::fill()).height(Size::flex(1.0)).child(
                    Table::new(
                        tr!("copy-mode-keys"),
                        vec![tr!("copy-mode-key"), tr!("copy-mode-action")],
                    )
                    .rows(rows),
                ),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .direction(Direction::Horizontal)
                    .main_align(Alignment::End)
                    .child(
                        Button::new(tr!("done"))
                            .style(ButtonStyle::Primary)
                            .on_press(move |_| shown.set(None)),
                    ),
            ),
    )
    .into_element()
}

fn copy_command_label(command: CopyCommand) -> &'static str {
    use alacritty_terminal::vi_mode::ViMotion::*;
    match command {
        CopyCommand::Move(Left) => tr!("copy-left"),
        CopyCommand::Move(Down) => tr!("copy-down"),
        CopyCommand::Move(Up) => tr!("copy-up"),
        CopyCommand::Move(Right) => tr!("copy-right"),
        CopyCommand::Move(SemanticRight) => tr!("copy-next-word"),
        CopyCommand::Move(SemanticLeft) => tr!("copy-previous-word"),
        CopyCommand::Move(SemanticRightEnd) => tr!("copy-word-end"),
        CopyCommand::Move(WordRight) => tr!("copy-next-big-word"),
        CopyCommand::Move(WordLeft) => tr!("copy-previous-big-word"),
        CopyCommand::Move(WordRightEnd) => tr!("copy-big-word-end"),
        CopyCommand::Move(First) => tr!("copy-line-start"),
        CopyCommand::Move(FirstOccupied) => tr!("copy-first-character"),
        CopyCommand::Move(Last) => tr!("copy-line-end"),
        CopyCommand::Move(High) => tr!("copy-screen-top"),
        CopyCommand::Move(Middle) => tr!("copy-screen-middle"),
        CopyCommand::Move(Low) => tr!("copy-screen-bottom"),
        CopyCommand::Move(_) => tr!("copy-move"),
        CopyCommand::Top => tr!("copy-buffer-start"),
        CopyCommand::Bottom => tr!("copy-buffer-end"),
        CopyCommand::PageUp => tr!("copy-page-up"),
        CopyCommand::PageDown => tr!("copy-page-down"),
        CopyCommand::Select(alacritty_terminal::selection::SelectionType::Lines) => {
            tr!("copy-select-lines")
        }
        CopyCommand::Select(_) => tr!("copy-select-characters"),
        CopyCommand::Copy => tr!("copy-and-leave"),
        CopyCommand::Exit => tr!("copy-leave"),
    }
}

fn find_bar(handles: &Handles, mut revision: State<u64>) -> Element {
    let find = handles.find;
    let navigate = handles.clone();
    rect()
        .background(color(Palette::current().window_background))
        .width(Size::fill())
        .direction(Direction::Horizontal)
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .padding(SPACE_1)
        .spacing(SPACE_1)
        .child(
            SearchField::new(find.query, tr!("find-placeholder"))
                .width(Size::flex(1.0))
                .auto_focus(true)
                .on_focus_change(move |focused| {
                    let mut state = find.focused;
                    state.set(focused);
                })
                .on_navigate(move |key| {
                    let direction = match key {
                        NamedKey::ArrowDown => SearchDirection::Right,
                        _ => SearchDirection::Left,
                    };
                    navigate.find(direction);
                    *revision.write() += 1;
                }),
        )
        .maybe_child((!find.query.read().is_empty()).then(|| {
            let position = match *find.position.read() {
                Some((current, total)) => {
                    let (current, total) = (current as u64, total as u64);
                    tr!("find-count", current = current, total = total)
                }
                None => tr!("find-none").to_string(),
            };
            text(position, CAPTION, Palette::current().secondary_label)
        }))
        .child(Button::new(tr!("find-close")).on_press(move |_| find.close()))
        .into_element()
}

fn listen(handles: Handles, revision: State<u64>) -> Events {
    let (sender, receiver) = channel();
    let receiver: RefCell<Receiver<(SessionId, TermEvent)>> = RefCell::new(receiver);
    let timers = handles.timers.clone();
    let wake = timers.wakeup(move || {
        if !handles.alive.get() {
            return;
        }
        let pending: Vec<_> = receiver.borrow().try_iter().collect();
        if pending.is_empty() {
            return;
        }
        for (id, event) in pending {
            on_term_event(&handles, id, event, revision);
        }
        let mut revision = revision;
        *revision.write() += 1;
    });
    Rc::new((sender, wake))
}

fn on_term_event(handles: &Handles, id: SessionId, event: TermEvent, revision: State<u64>) {
    let Some(session) = handles.sessions.peek().get(&id).cloned() else {
        return;
    };
    let mut workspace = handles.workspace;
    match event {
        TermEvent::Title(title) => workspace.write().set_title(id, Some(title)),
        TermEvent::ResetTitle => workspace.write().set_title(id, None),
        TermEvent::PtyWrite(text) => session.write(text.into_bytes()),
        TermEvent::ClipboardStore(_, text) => set_clipboard(handles.clipboard, text),
        TermEvent::ClipboardLoad(_, format) => {
            if let Some(text) = clipboard_text(handles.clipboard) {
                session.write(format(&text).into_bytes());
            }
        }
        TermEvent::TextAreaSizeRequest(format) => {
            let size = session.size();
            session.write(
                format(WindowSize {
                    num_lines: size.lines,
                    num_cols: size.columns,
                    cell_width: size.cell_width as u16,
                    cell_height: size.cell_height as u16,
                })
                .into_bytes(),
            );
        }
        TermEvent::Bell => {
            let profile = handles.defaults.peek().profile(&session.profile);
            if profile.bell_icon_in_tabs {
                workspace.write().ring(id);
            }
            if profile.flash_visual_bell {
                session.extras().bell = Some(Instant::now());
                handles.after(Duration::from_millis(DURATION_FAST_MS), move || {
                    let mut revision = revision;
                    *revision.write() += 1;
                });
            }
        }
        TermEvent::ChildExit(_) | TermEvent::Exit => {
            match handles
                .defaults
                .peek()
                .profile(&session.profile)
                .when_session_ends
            {
                SessionEnd::Close => close_session(handles, id),
                SessionEnd::Restart if session.started.elapsed() < QUICK_EXIT => {
                    session.show_ended(tr!("session-ended-quickly"));
                }
                SessionEnd::Restart => {
                    if let Some(events) = handles.events.get() {
                        restart(handles, events, &session);
                    }
                }
                SessionEnd::KeepOpen => session.show_ended(tr!("session-ended")),
            }
        }
        _ => {}
    }
}

fn spawn(handles: &Handles, events: &Events, launch: Launch) -> Option<SessionId> {
    let (columns, lines) = INITIAL_GRID;
    let size = handles.active().map_or(
        GridSize {
            columns,
            lines,
            cell_width: 1.0,
            cell_height: 1.0,
        },
        |session| session.size(),
    );
    let mut next_id = handles.next_id;
    let id = *next_id.peek();
    *next_id.write() += 1;
    let listener = Listener::new(id, events.0.clone(), events.1.clone());
    let profile = handles.defaults.peek().profile(&launch.profile);
    match Session::spawn(id, size, launch, listener) {
        Ok(session) => {
            session.apply_profile(&profile);
            let mut sessions = handles.sessions;
            sessions.write().insert(id, Rc::new(session));
            Some(id)
        }
        Err(error) => {
            tracing::error!(%error, "could not start the shell");
            None
        }
    }
}

fn new_tab(handles: &Handles, events: &Events, profile: &str, directory: Option<PathBuf>) {
    let previous = handles
        .active()
        .and_then(|session| session.working_directory());
    let mut launch = handles.launch(profile, previous);
    if directory.is_some() {
        launch.directory = directory;
    }
    if let Some(id) = spawn(handles, events, launch) {
        let mut workspace = handles.workspace;
        workspace.write().open_tab(id);
    }
}

fn split(handles: &Handles, events: &Events, axis: Axis) {
    let previous = handles
        .active()
        .and_then(|session| session.working_directory());
    let launch = handles.launch(&handles.current_profile(), previous);
    if let Some(id) = spawn(handles, events, launch) {
        let mut workspace = handles.workspace;
        workspace.write().split(id, axis);
    }
}

fn restart(handles: &Handles, events: &Events, session: &Session) {
    let mut launch = handles.launch(&session.profile, None);
    launch.directory = session.working_directory().or(launch.directory);
    if let Some(id) = spawn(handles, events, launch) {
        let mut workspace = handles.workspace;
        workspace.write().replace(session.id, id);
        let mut sessions = handles.sessions;
        sessions.write().remove(&session.id);
    }
}

fn request_close(handles: &Handles, closing: Closing) {
    let workspace = handles.workspace.peek();
    let ids = match closing {
        Closing::Session(id) => vec![id],
        Closing::TabOf(id) => workspace
            .tab_of(id)
            .map(|index| workspace.tabs[index].root.leaves())
            .unwrap_or_default(),
        Closing::Window => workspace.receivers(Broadcast::AllTabs),
    };
    drop(workspace);
    let defaults = handles.defaults.peek();
    let ask = ids
        .iter()
        .filter_map(|id| handles.sessions.peek().get(id).cloned())
        .any(
            |session| match defaults.profile(&session.profile).prompt_before_closing {
                ClosePrompt::Always => true,
                ClosePrompt::Never => false,
                ClosePrompt::IfJobs => session.has_jobs(),
            },
        );
    drop(defaults);
    if ask {
        let mut pending = handles.closing;
        pending.set(Some(closing));
    } else {
        close_now(handles, closing);
    }
}

fn close_now(handles: &Handles, closing: Closing) {
    let workspace = handles.workspace.peek();
    let index = match closing {
        Closing::Session(id) | Closing::TabOf(id) => workspace.tab_of(id),
        Closing::Window => None,
    };
    let tab = index.map(|index| workspace.tabs[index].clone());
    drop(workspace);
    match (closing, tab, index) {
        (Closing::Session(id), Some(tab), _) => {
            bury(
                handles,
                if tab.root.is_split() {
                    Tab::single(id)
                } else {
                    tab
                },
            );
            close_session(handles, id);
        }
        (Closing::TabOf(_), Some(tab), Some(index)) => {
            bury(handles, tab);
            close_tab(handles, index);
        }
        (Closing::Window, ..) => handles.controls.close(),
        _ => {}
    }
}

fn bury(handles: &Handles, tab: Tab) {
    let ids = tab.root.leaves();
    let sessions = ids
        .iter()
        .filter_map(|id| handles.sessions.peek().get(id).cloned())
        .collect();
    let workspace = handles.workspace.peek();
    let titles = ids
        .iter()
        .map(|id| (*id, workspace.title(*id).to_string()))
        .collect();
    drop(workspace);
    let at = Instant::now();
    let mut buried = handles.buried;
    buried.set(Some(Buried {
        tab,
        sessions,
        titles,
        at,
    }));
    handles.after(UNDO_CLOSE, move || {
        if buried.peek().as_ref().is_some_and(|buried| buried.at == at) {
            let mut buried = buried;
            buried.set(None);
        }
    });
}

fn undo_close(handles: &Handles) {
    let mut buried = handles.buried;
    let Some(buried) = buried.write().take() else {
        return;
    };
    let mut sessions = handles.sessions;
    for session in buried.sessions {
        sessions.write().insert(session.id, session);
    }
    let mut workspace = handles.workspace;
    let mut workspace = workspace.write();
    for (id, title) in buried.titles {
        workspace.set_title(id, Some(title));
    }
    workspace.restore(buried.tab);
}

// BK-027: the Alert's key also reaches the global handler, so the alert goes one loop later.
fn dismiss_later(handles: &Handles, then: impl Fn() + 'static) {
    let closing = handles.closing;
    handles.after(Duration::ZERO, move || {
        let mut closing = closing;
        closing.set(None);
        then();
    });
}

fn close_alert(handles: &Handles, pending: Closing) -> Alert {
    let (confirm, cancel) = (handles.clone(), handles.clone());
    Alert::new(tr!("close-title"), tr!("close-message"))
        .confirm(tr!("close-confirm"), move |_| {
            let handles = confirm.clone();
            dismiss_later(&confirm, move || close_now(&handles, pending));
        })
        .cancel(tr!("close-cancel"), move |_| dismiss_later(&cancel, || {}))
}

fn close_session(handles: &Handles, id: SessionId) {
    let mut workspace = handles.workspace;
    let closed = workspace.write().close(id);
    let mut sessions = handles.sessions;
    sessions.write().remove(&id);
    if closed == Some(Closed::Window) {
        handles.controls.close();
    }
}

fn close_tab(handles: &Handles, index: usize) {
    let mut workspace = handles.workspace;
    let closed = workspace.write().close_tab(index);
    let mut sessions = handles.sessions;
    for id in closed {
        sessions.write().remove(&id);
    }
    if workspace.peek().tabs.is_empty() {
        handles.controls.close();
    }
}

fn on_action(id: &str, handles: &Handles, events: &Events, revision: State<u64>) {
    let mut workspace = handles.workspace;
    let active = handles.active();
    let toward = match id {
        menu::PANE_UP | menu::DIVIDER_UP => Some(Toward::Up),
        menu::PANE_DOWN | menu::DIVIDER_DOWN => Some(Toward::Down),
        menu::PANE_LEFT | menu::DIVIDER_LEFT => Some(Toward::Left),
        menu::PANE_RIGHT | menu::DIVIDER_RIGHT => Some(Toward::Right),
        _ => None,
    };
    if let Some(index) = id
        .strip_prefix(menu::SELECT_TAB_PREFIX)
        .and_then(|index| index.parse().ok())
    {
        workspace.write().select_tab(index);
        return;
    }
    if let Some(profile) = id
        .strip_prefix(menu::PROFILE_PREFIX)
        .and_then(|index| index.parse::<usize>().ok())
        .and_then(|index| handles.defaults.peek().profiles().get(index).cloned())
    {
        new_tab(handles, events, &profile.name, None);
        return;
    }
    match id {
        _ if id == menu::ABOUT.id => windows::open(&["--about"]),
        _ if id == menu::SETTINGS.id => windows::open(&["--settings"]),
        _ if id == menu::QUIT.id => windows::quit(),
        _ if id == menu::NEW_WINDOW.id => windows::open(&[]),
        _ if id == menu::NEW_TAB.id => {
            let profile = handles.defaults.peek().default_profile_name();
            new_tab(handles, events, &profile, None);
        }
        _ if id == menu::DUPLICATE_TAB.id => {
            let directory = active
                .as_ref()
                .and_then(|session| session.working_directory());
            new_tab(handles, events, &handles.current_profile(), directory);
        }
        _ if id == menu::SPLIT_VERTICALLY.id => split(handles, events, Axis::Row),
        _ if id == menu::SPLIT_HORIZONTALLY.id => split(handles, events, Axis::Column),
        _ if id == menu::CLOSE.id => {
            if let Some(session) = active {
                request_close(handles, Closing::Session(session.id));
            }
        }
        _ if id == menu::CLOSE_WINDOW.id => request_close(handles, Closing::Window),
        _ if id == menu::UNDO_CLOSE.id => undo_close(handles),
        _ if id == menu::PASTE_HISTORY.id => {
            open_picker(handles, Picker::PasteHistory);
        }
        _ if id == menu::OPEN_QUICKLY.id => open_picker(handles, Picker::OpenQuickly),
        _ if id == menu::OPEN_PROFILES.id => open_picker(handles, Picker::Profiles),
        _ if id == menu::EDIT_TAB_TITLE.id => {
            if let Some(session) = active {
                let workspace = handles.workspace.peek();
                let title = workspace
                    .tab_of(session.id)
                    .and_then(|index| workspace.tabs[index].title.clone())
                    .unwrap_or_default();
                drop(workspace);
                handles.draft.clone().set(title);
                open_sheet(handles, Sheet::RenameTab(session.id));
            }
        }
        _ if id == menu::EDIT_WINDOW_TITLE.id => {
            let title = handles.workspace.peek().window_title.clone();
            handles.draft.clone().set(title.unwrap_or_default());
            open_sheet(handles, Sheet::RenameWindow);
        }
        _ if id == menu::COPY_MODE_KEYS.id => open_sheet(handles, Sheet::CopyModeKeys),
        _ if id == menu::SAVE_SELECTION.id => {
            if let Some(session) = active {
                handles.draft.clone().set(String::new());
                open_sheet(handles, Sheet::SaveSelection(session.id));
            }
        }
        _ if id == menu::LOG_START.id => {
            if let Some(session) = active {
                handles.draft.clone().set(String::new());
                open_sheet(handles, Sheet::Log(session.id));
            }
        }
        _ if id == menu::RUN_COPROCESS.id => {
            if let Some(session) = &active {
                handles.draft.clone().set(String::new());
                open_sheet(handles, Sheet::Coprocess(session.id));
            }
        }
        _ if id == menu::STOP_COPROCESS.id => {
            if let Some(session) = &active {
                session.stop_coprocess();
            }
        }
        _ if id == menu::LOG_STOP.id => {
            if let Some(session) = active {
                session.stop_log();
            }
        }
        _ if id == menu::PASTE_SLOWLY.id => {
            if let Some(text) = clipboard_text(handles.clipboard) {
                paste_slowly(handles, text);
            }
        }
        _ if id == menu::PASTE_ESCAPED.id => {
            if let Some(text) = clipboard_text(handles.clipboard) {
                paste_text(handles, session::shell_escaped(&text));
            }
        }
        _ if id == menu::PASTE_SELECTION.id => {
            if let Some(text) = active.and_then(|session| session.selected_text()) {
                paste_text(handles, text);
            }
        }
        _ if id == menu::COPY_STYLED.id => {
            if let Some(session) = &active {
                copy_styled(handles, session);
            }
        }
        _ if id == menu::OPEN_SELECTION.id => {
            if let Some(text) = active.and_then(|session| session.selected_text()) {
                links::open(text.trim());
            }
        }
        _ if id == menu::CLOSE_PANES.id => {
            if let Some(session) = active {
                request_close(handles, Closing::TabOf(session.id));
            }
        }
        _ if id == menu::COPY.id => {
            if let Some(session) = &active {
                copy(handles, session);
            }
        }
        _ if id == menu::PASTE.id => paste(handles),
        _ if id == menu::FIND.id => {
            let mut open = handles.find.open;
            open.set(true);
        }
        _ if id == menu::FIND_NEXT.id => handles.find(SearchDirection::Right),
        _ if id == menu::FIND_PREVIOUS.id => handles.find(SearchDirection::Left),
        _ if id == menu::USE_SELECTION_FOR_FIND.id => {
            if let Some(text) = active.as_ref().and_then(|session| session.selected_text()) {
                let (mut query, mut open) = (handles.find.query, handles.find.open);
                query.set(text);
                open.set(true);
            }
        }
        _ if id == menu::SET_MARK.id => {
            if let Some(session) = &active {
                session.set_mark();
            }
        }
        _ if id == menu::JUMP_TO_MARK.id => jump(active, MarkJump::Latest),
        menu::NEXT_MARK => jump(active, MarkJump::Next),
        menu::PREVIOUS_MARK => jump(active, MarkJump::Previous),
        menu::BROADCAST_OFF => handles.broadcast.clone().set(Broadcast::Off),
        menu::BROADCAST_ALL_TABS => handles.broadcast.clone().set(Broadcast::AllTabs),
        menu::BROADCAST_CURRENT_TAB => handles.broadcast.clone().set(Broadcast::CurrentTab),
        _ if id == menu::SELECT_ALL.id => {
            if let Some(session) = &active {
                session.select_all();
            }
        }
        _ if id == menu::SELECT_OUTPUT.id => {
            if let Some(session) = &active {
                session.select_output();
            }
        }
        _ if id == menu::COPY_MODE.id => {
            if let Some(session) = &active {
                session.toggle_copy_mode();
            }
        }
        _ if id == menu::USE_TRANSPARENCY.id => {
            let mut transparency = handles.transparency;
            let next = !*transparency.peek();
            transparency.set(next);
        }
        _ if id == menu::EDIT_SESSION.id => {
            let profile = handles.current_profile();
            windows::open(&["--settings", &profile]);
        }
        _ if id == menu::HELP.id => links::open(env!("CARGO_PKG_REPOSITORY")),
        _ if let Some(state) = TerminalState::ALL
            .into_iter()
            .find(|state| menu::terminal_state_entry(*state).0 == id) =>
        {
            if let Some(session) = &active {
                session.toggle_state(state);
            }
        }
        _ if id == menu::SHOW_TIMESTAMPS.id => {
            if let Some(session) = &active {
                let mut extras = session.extras();
                extras.show_timestamps = !extras.show_timestamps;
            }
        }
        _ if id == menu::RESET_CHARSET.id => {
            if let Some(session) = &active {
                session.reset_character_set();
            }
        }
        _ if id == menu::FIND_CURSOR.id => {
            if let Some(session) = &active {
                session.extras().cursor_spotlight = Some(Instant::now());
                handles.after(Duration::from_millis(DURATION_SLOW_MS), move || {
                    let mut revision = revision;
                    *revision.write() += 1;
                });
            }
        }
        _ if id == menu::SHOW_TAB_BAR.id => {
            change_defaults(|defaults| defaults.show_tab_bar = !defaults.show_tab_bar);
        }
        _ if id == menu::FULL_SCREEN.id => handles.controls.toggle_fullscreen(),
        menu::MAXIMIZE_PANE => workspace.write().toggle_zoom(),
        _ if id == menu::TEXT_BIGGER.id => *handles.text_zoom.clone().write() += 1,
        _ if id == menu::TEXT_SMALLER.id => *handles.text_zoom.clone().write() -= 1,
        _ if id == menu::TEXT_NORMAL.id => handles.text_zoom.clone().set(0),
        _ if id == menu::MINIMIZE.id => handles.controls.minimize(),
        menu::ZOOM => handles.controls.toggle_maximize(),
        _ if id == menu::NEW_WINDOW_PROFILE.id => {
            windows::open(&["--profile", &handles.current_profile()]);
        }
        _ if id == menu::NEW_TAB_PROFILE.id => {
            new_tab(handles, events, &handles.current_profile(), None);
        }
        _ if id == menu::CLEAR_BUFFER.id => {
            if let Some(session) = &active {
                session.clear_buffer();
            }
        }
        _ if id == menu::CLEAR_SCROLLBACK.id => {
            if let Some(session) = &active {
                session.clear_scrollback();
            }
        }
        _ if id == menu::RESET.id => {
            if let Some(session) = &active {
                session.reset();
            }
        }
        _ if id == menu::TRIGGERS_ENABLED.id => {
            if let Some(session) = &active {
                let mut extras = session.extras();
                extras.triggers_enabled = !extras.triggers_enabled;
            }
        }
        _ if id == menu::EDIT_TRIGGERS.id => windows::open(&["--settings"]),
        _ if id == menu::RESTART_SESSION.id => {
            if let Some(session) = active {
                restart(handles, events, &session);
            }
        }
        menu::NEXT_TAB => workspace.write().step_tab(1),
        menu::PREVIOUS_TAB => workspace.write().step_tab(-1),
        _ if id == menu::MOVE_TAB_LEFT.id => workspace.write().move_tab(-1),
        _ if id == menu::MOVE_TAB_RIGHT.id => workspace.write().move_tab(1),
        _ if id == menu::NEXT_PANE.id => workspace.write().cycle_pane(1),
        _ if id == menu::PREVIOUS_PANE.id => workspace.write().cycle_pane(-1),
        menu::PANE_UP | menu::PANE_DOWN | menu::PANE_LEFT | menu::PANE_RIGHT => {
            if let Some(toward) = toward {
                workspace.write().focus_toward(toward);
            }
        }
        menu::DIVIDER_UP | menu::DIVIDER_DOWN | menu::DIVIDER_LEFT | menu::DIVIDER_RIGHT => {
            if let Some(toward) = toward {
                workspace.write().move_divider(toward, DIVIDER_STEP);
            }
        }
        _ => {}
    }
}

fn jump(session: Option<Rc<Session>>, jump: MarkJump) {
    if let Some(session) = session {
        session.jump_to_mark(jump);
    }
}

// BK-030: the compositor sends a modifier key before its modifiers, so the key itself counts.
fn key_modifier(key: &Key) -> Modifiers {
    match key {
        Key::Named(NamedKey::Meta) => Modifiers::META,
        Key::Named(NamedKey::Shift) => Modifiers::SHIFT,
        Key::Named(NamedKey::Control) => Modifiers::CONTROL,
        Key::Named(NamedKey::Alt) => Modifiers::ALT,
        _ => Modifiers::empty(),
    }
}

fn use_focus_report(handles: &Handles, focused: bool) {
    let mut reported = use_state(|| focused);
    if *reported.peek() == focused {
        return;
    }
    reported.set(focused);
    if let Some(session) = handles
        .active()
        .filter(|session| session.has_state(TerminalState::FocusReporting))
    {
        let sequence: &[u8] = if focused { b"\x1b[I" } else { b"\x1b[O" };
        session.write(sequence.to_vec());
    }
}

// BK-033: without patch 0009 no input method reaches the window.
#[cfg(feature = "kit-parcheado")]
fn ime_preedit(handles: &Handles) -> Option<Rc<str>> {
    let text = handles.controls.ime_preedit.read();
    (!text.is_empty()).then(|| Rc::from(text.as_str()))
}

#[cfg(not(feature = "kit-parcheado"))]
fn ime_preedit(_: &Handles) -> Option<Rc<str>> {
    None
}

#[cfg(feature = "kit-parcheado")]
fn place_ime_cursor(handles: &Handles) {
    let Some(session) = handles.active() else {
        return;
    };
    let fonts = handles.fonts_for(&handles.defaults.peek().profile(&session.profile));
    let (column, line) = session.cursor_cell();
    let (x, y) = session.origin.get();
    handles.controls.set_ime_cursor_area(
        (x + SPACE_2 + column as f32 * fonts.cell_width) as i32,
        (y + SPACE_2 + line as f32 * fonts.cell_height) as i32,
        fonts.cell_width.ceil() as i32,
        fonts.cell_height.ceil() as i32,
    );
}

// BK-031: without patch 0007 the runner clears the window opaque, so transparency stays off.
fn transparency_supported() -> bool {
    cfg!(feature = "kit-parcheado")
}

#[cfg(feature = "kit-parcheado")]
fn use_background_alpha(alpha: f32) {
    let mut published = use_state(|| 1.0f32);
    if *published.peek() != alpha {
        published.set(alpha);
        WindowControls::get().set_background_alpha(alpha);
    }
}

#[cfg(feature = "kit-parcheado")]
fn use_window_title(title: &str) {
    let mut published = use_state(String::new);
    if *published.peek() != title {
        published.set(title.to_string());
        WindowControls::get().set_title(title);
    }
}
