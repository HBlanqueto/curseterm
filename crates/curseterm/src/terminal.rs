use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};

use alacritty_terminal::event::{Event as TermEvent, WindowSize};
use alacritty_terminal::grid::Scroll;
use alacritty_terminal::index::Direction as SearchDirection;
use barrier_kit::app::use_menu_action;
use barrier_kit::app_menu::MenuHandle;
use barrier_kit::controls::{Button, SearchField};
use barrier_kit::platform::{Timers, WakeHandle, WindowControls};
use barrier_kit::style::Palette;
use barrier_kit::theme::color;
use barrier_kit::titlebar::Titlebar;
use barrier_kit_next::app_menu::{use_menu_checked, use_menu_enabled, use_submenu_items};
use barrier_kit_next::views::split_panes::SplitPanes;
use barrier_kit_next::views::tab_bar::TabBar;
use freya_clipboard::copypasta::ClipboardProvider;
use freya_core::prelude::*;
use torin::prelude::{Content, Direction, Size};

use vow_design::SPACE_1;

use crate::config::{APP_ID, Defaults, Profile, WorkingDirectory, change_defaults};
use crate::grid::Fonts;
use crate::keys::{KeyAction, encode};
use crate::localize::tr;
use crate::menu;
use crate::pane_view::TerminalPane;
use crate::panes::{Axis, Pane, Toward};
use crate::session::{GridSize, Launch, Listener, MarkJump, Session, SessionId};
use crate::windows;
use crate::workspace::{Broadcast, Closed, Workspace};

const INITIAL_GRID: (u16, u16) = (80, 24);
const DIVIDER_STEP: f32 = 0.05;
const TEXT_SIZE_RANGE: (i32, i32) = (6, 72);

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
    clipboard: ClipboardState,
    broadcast: State<Broadcast>,
    find: Find,
}

#[derive(Clone, Copy)]
struct Find {
    open: State<bool>,
    query: State<String>,
    focused: State<bool>,
}

impl Find {
    fn close(self) {
        let (mut open, mut focused) = (self.open, self.focused);
        open.set(false);
        focused.set(false);
    }
}

impl Handles {
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
        }
    }

    fn fonts_for(&self, profile: &Profile) -> Rc<Fonts> {
        let (smallest, largest) = TEXT_SIZE_RANGE;
        let size = (profile.font_size as i32 + *self.text_zoom.peek()).clamp(smallest, largest);
        self.fonts
            .borrow_mut()
            .entry((profile.font_family.clone(), size))
            .or_insert_with(|| Rc::new(Fonts::new(&profile.font_family, size as f32)))
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
        let defaults = barrier_kit::app::use_defaults::<Defaults>(APP_ID);
        let dark = barrier_kit::theme::resolved().scheme.is_dark();
        let workspace = use_state(Workspace::default);
        let sessions = use_state(Sessions::new);
        let next_id = use_state(|| 0);
        let text_zoom = use_state(|| 0i32);
        let mut revision = use_state(|| 0u64);
        let broadcast = use_state(Broadcast::default);
        let find = Find {
            open: use_state(|| false),
            query: use_state(String::new),
            focused: use_state(|| false),
        };
        let handles = use_hook(move || Handles {
            workspace,
            sessions,
            next_id,
            defaults,
            text_zoom,
            fonts: FontCache::default(),
            controls: WindowControls::get(),
            clipboard: consume_root_context(),
            broadcast,
            find,
        });
        use_side_effect({
            let handles = handles.clone();
            move || {
                let defaults = defaults.read().clone();
                text_zoom.read();
                for session in sessions.peek().values() {
                    let fonts = handles.fonts_for(&defaults.profile(&session.profile));
                    session.refit(fonts.cell_width, fonts.cell_height);
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
                }
            }
        });
        let events = use_hook({
            let handles = handles.clone();
            move || listen(handles, revision)
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
                on_action(id, &handles, &events);
                let mut revision = revision;
                *revision.write() += 1;
            }
        });
        use_live_menu(&handles);

        let window_focused = handles.controls.status.read().activated;
        let palette = Palette::current();
        let state = workspace.read();
        let title = state
            .active_session()
            .map_or(crate::workspace::DEFAULT_TITLE, |id| state.title(id))
            .to_string();
        let show_tab_bar = defaults.read().show_tab_bar;
        let tab_bar = (state.tabs.len() > 1 || show_tab_bar).then(|| {
            let (on_close, on_add) = (handles.clone(), handles.clone());
            state
                .tab_titles()
                .into_iter()
                .fold(TabBar::new(tr!("tabs"), tr!("close-tab")), TabBar::tab)
                .selected(state.active)
                .on_select(move |index| {
                    let mut workspace = workspace;
                    workspace.write().select_tab(index);
                })
                .on_close(move |index| close_tab(&on_close, index))
                .on_add(tr!("new-tab-button"), move |_| {
                    let profile = on_add.defaults.peek().default_profile_name();
                    new_tab(&on_add, &add_events, &profile, None)
                })
        });
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
                receivers: match *broadcast.read() {
                    Broadcast::Off => Vec::new(),
                    mode => state.receivers(mode),
                },
            };
            pane_tree(&root, Vec::new(), &painting)
        });
        drop(state);
        #[cfg(feature = "kit-parcheado")]
        use_window_title(&title);

        let keyboard = handles.clone();
        let find_bar = (*find.open.read()).then(|| find_bar(&handles, revision));
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .direction(Direction::Vertical)
            .content(Content::flex())
            .background(color(palette.window_background))
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                let data = event.data();
                if *find.open.peek() && *find.focused.peek() {
                    if data.key == Key::Named(NamedKey::Escape) {
                        find.close();
                    }
                    return;
                }
                let Some(session) = &keyboard.active() else {
                    return;
                };
                match encode(&data.key, data.modifiers, session.mode()) {
                    KeyAction::Write(bytes) => {
                        for receiver in keyboard.receivers() {
                            receiver.clear_selection();
                            receiver.write(bytes.clone());
                        }
                    }
                    KeyAction::Copy => copy(session, keyboard.clipboard),
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
                    on_focus: Callback::new(move |id| workspace.write().focus(id)),
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
    let marks = handles.active().is_some_and(|session| session.has_marks());
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

fn copy(session: &Session, clipboard: ClipboardState) {
    if let Some(text) = session.selected_text() {
        set_clipboard(clipboard, text);
    }
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
        Some(text) => handles
            .receivers()
            .iter()
            .for_each(|session| session.paste(&text)),
        None => tracing::warn!("could not read the clipboard"),
    }
}

fn find_bar(handles: &Handles, mut revision: State<u64>) -> Element {
    let find = handles.find;
    let navigate = handles.clone();
    rect()
        .width(Size::fill())
        .direction(Direction::Horizontal)
        .content(Content::flex())
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
        .child(Button::new(tr!("find-close")).on_press(move |_| find.close()))
        .into_element()
}

fn listen(handles: Handles, revision: State<u64>) -> Events {
    let (sender, receiver) = channel();
    let receiver: RefCell<Receiver<(SessionId, TermEvent)>> = RefCell::new(receiver);
    let wake = Timers::get().wakeup(move || {
        let pending: Vec<_> = receiver.borrow().try_iter().collect();
        if pending.is_empty() {
            return;
        }
        for (id, event) in pending {
            on_term_event(&handles, id, event);
        }
        let mut revision = revision;
        *revision.write() += 1;
    });
    Rc::new((sender, wake))
}

fn on_term_event(handles: &Handles, id: SessionId, event: TermEvent) {
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
        TermEvent::ChildExit(_) | TermEvent::Exit => close_session(handles, id),
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
    match Session::spawn(id, size, launch, listener) {
        Ok(session) => {
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

fn on_action(id: &str, handles: &Handles, events: &Events) {
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
        _ if id == menu::QUIT.id => {
            windows::quit_others();
            handles.controls.close();
        }
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
                close_session(handles, session.id);
            }
        }
        _ if id == menu::CLOSE_WINDOW.id => handles.controls.close(),
        _ if id == menu::CLOSE_PANES.id => {
            let index = workspace.peek().active;
            close_tab(handles, index);
        }
        _ if id == menu::COPY.id => {
            if let Some(session) = &active {
                copy(session, handles.clipboard);
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

#[cfg(feature = "kit-parcheado")]
fn use_window_title(title: &str) {
    let mut published = use_state(String::new);
    if *published.peek() != title {
        published.set(title.to_string());
        WindowControls::get().set_title(title);
    }
}
