use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};

use alacritty_terminal::event::{Event as TermEvent, WindowSize};
use alacritty_terminal::grid::Scroll;
use barrier_kit::app::use_menu_action;
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

use crate::config::{APP_ID, Defaults, Scheme};
use crate::grid::Fonts;
use crate::keys::{KeyAction, encode};
use crate::localize::tr;
use crate::menu;
use crate::pane_view::TerminalPane;
use crate::panes::{Axis, Pane, Toward};
use crate::session::{GridSize, Listener, Session, SessionId};
use crate::windows;
use crate::workspace::{Closed, Workspace};

const INITIAL_GRID: (u16, u16) = (80, 24);
const DIVIDER_STEP: f32 = 0.05;
const TEXT_SIZE_RANGE: (i32, i32) = (6, 72);

type Sessions = HashMap<SessionId, Rc<Session>>;
type ClipboardState = State<Option<Box<dyn ClipboardProvider>>>;
type Events = Rc<(Sender<(SessionId, TermEvent)>, WakeHandle)>;

// Menu actions and wake-ups run outside Freya's context (BK-017): they use handles taken in render.
#[derive(Clone)]
struct Handles {
    workspace: State<Workspace>,
    sessions: State<Sessions>,
    next_id: State<SessionId>,
    show_tab_bar: State<bool>,
    text_zoom: State<i32>,
    controls: WindowControls,
    clipboard: ClipboardState,
}

impl Handles {
    fn active(&self) -> Option<Rc<Session>> {
        let id = self.workspace.peek().active_session()?;
        self.sessions.peek().get(&id).cloned()
    }
}

pub struct Terminal;

impl App for Terminal {
    fn render(&self) -> impl IntoElement {
        let defaults = barrier_kit::app::use_defaults::<Defaults>(APP_ID);
        let dark = barrier_kit::theme::resolved().scheme.is_dark();
        let scheme: &'static Scheme = defaults.read().scheme(dark);
        let workspace = use_state(Workspace::default);
        let sessions = use_state(Sessions::new);
        let next_id = use_state(|| 0);
        let show_tab_bar = use_state(|| false);
        let text_zoom = use_state(|| 0i32);
        let mut revision = use_state(|| 0u64);
        let fonts = use_memo(move || {
            let defaults = defaults.read();
            let (smallest, largest) = TEXT_SIZE_RANGE;
            let size = (defaults.font_size() as i32 + *text_zoom.read()).clamp(smallest, largest);
            Rc::new(Fonts::new(defaults.font_family(), size as f32))
        });
        let handles = use_hook(move || Handles {
            workspace,
            sessions,
            next_id,
            show_tab_bar,
            text_zoom,
            controls: WindowControls::get(),
            clipboard: consume_root_context(),
        });
        use_side_effect(move || {
            let fonts = fonts.read().clone();
            for session in sessions.peek().values() {
                session.refit(fonts.cell_width, fonts.cell_height);
            }
            *revision.write() += 1;
        });
        let events = use_hook({
            let handles = handles.clone();
            move || listen(handles, revision)
        });
        use_hook({
            let (handles, events) = (handles.clone(), events.clone());
            move || new_tab(&handles, &events, None)
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
        let tab_bar = (state.tabs.len() > 1 || *show_tab_bar.read()).then(|| {
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
                    new_tab(&on_add, &add_events, None)
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
                fonts: fonts.read().clone(),
                scheme,
                active: tab.active,
                window_focused,
                revision: *revision.read(),
                workspace,
                tab: state.active,
            };
            pane_tree(&root, Vec::new(), &painting)
        });
        drop(state);
        #[cfg(feature = "kit-parcheado")]
        use_window_title(&title);

        let clipboard = handles.clipboard;
        let keyboard = handles.clone();
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .direction(Direction::Vertical)
            .content(Content::flex())
            .background(color(palette.window_background))
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                let Some(session) = &keyboard.active() else {
                    return;
                };
                let data = event.data();
                match encode(&data.key, data.modifiers, session.mode()) {
                    KeyAction::Write(bytes) => {
                        session.clear_selection();
                        session.write(bytes);
                    }
                    KeyAction::Copy => copy(session, clipboard),
                    KeyAction::Paste => paste(session, clipboard),
                    KeyAction::PageUp => session.scroll(Scroll::PageUp),
                    KeyAction::PageDown => session.scroll(Scroll::PageDown),
                    KeyAction::Ignore => return,
                }
                *revision.write() += 1;
            })
            .child(Titlebar::new().title(title))
            .maybe_child(tab_bar)
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
    fonts: Rc<Fonts>,
    scheme: &'static Scheme,
    active: SessionId,
    window_focused: bool,
    revision: u64,
    workspace: State<Workspace>,
    tab: usize,
}

fn pane_tree(node: &Pane, path: Vec<usize>, painting: &Painting) -> Element {
    match node {
        Pane::Leaf(id) => match painting.sessions.get(id) {
            Some(session) => {
                let mut workspace = painting.workspace;
                TerminalPane {
                    session: session.clone(),
                    fonts: painting.fonts.clone(),
                    scheme: painting.scheme,
                    focused: painting.window_focused && *id == painting.active,
                    revision: painting.revision,
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
    use_menu_enabled(menu::CLOSE_PANES.id, split);
    use_menu_enabled(menu::MAXIMIZE_PANE, split);
    use_menu_checked(menu::MAXIMIZE_PANE, zoomed);
    use_menu_checked(menu::SHOW_TAB_BAR.id, *handles.show_tab_bar.read());
    use_menu_enabled(menu::NEXT_TAB, tabs > 1);
    use_menu_enabled(menu::PREVIOUS_TAB, tabs > 1);
    use_menu_enabled(menu::MOVE_TAB_LEFT.id, active > 0);
    use_menu_enabled(menu::MOVE_TAB_RIGHT.id, active + 1 < tabs);
    use_menu_enabled(menu::SPLIT_PANE, split);
    use_submenu_items(menu::SELECT_TAB, menu::select_tab_items(&titles, active));
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

fn paste(session: &Session, clipboard: ClipboardState) {
    match clipboard_text(clipboard) {
        Some(text) => session.paste(&text),
        None => tracing::warn!("could not read the clipboard"),
    }
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

fn spawn(
    handles: &Handles,
    events: &Events,
    directory: Option<std::path::PathBuf>,
) -> Option<SessionId> {
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
    match Session::spawn(id, size, directory, listener) {
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

fn new_tab(handles: &Handles, events: &Events, directory: Option<std::path::PathBuf>) {
    if let Some(id) = spawn(handles, events, directory) {
        let mut workspace = handles.workspace;
        workspace.write().open_tab(id);
    }
}

fn split(handles: &Handles, events: &Events, axis: Axis) {
    if let Some(id) = spawn(handles, events, None) {
        let mut workspace = handles.workspace;
        workspace.write().split(id, axis);
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
    match id {
        _ if id == menu::ABOUT.id => windows::open(Some("--about")),
        _ if id == menu::SETTINGS.id => windows::open(Some("--settings")),
        _ if id == menu::QUIT.id => {
            windows::quit_others();
            handles.controls.close();
        }
        _ if id == menu::NEW_WINDOW.id => windows::open(None),
        _ if id == menu::NEW_TAB.id => new_tab(handles, events, None),
        _ if id == menu::DUPLICATE_TAB.id => {
            let directory = active
                .as_ref()
                .and_then(|session| session.working_directory());
            new_tab(handles, events, directory);
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
        _ if id == menu::PASTE.id => {
            if let Some(session) = &active {
                paste(session, handles.clipboard);
            }
        }
        _ if id == menu::SELECT_ALL.id => {
            if let Some(session) = &active {
                session.select_all();
            }
        }
        _ if id == menu::SHOW_TAB_BAR.id => {
            let mut show = handles.show_tab_bar;
            let shown = *show.peek();
            show.set(!shown);
        }
        _ if id == menu::FULL_SCREEN.id => handles.controls.toggle_fullscreen(),
        menu::MAXIMIZE_PANE => workspace.write().toggle_zoom(),
        _ if id == menu::TEXT_BIGGER.id => *handles.text_zoom.clone().write() += 1,
        _ if id == menu::TEXT_SMALLER.id => *handles.text_zoom.clone().write() -= 1,
        _ if id == menu::TEXT_NORMAL.id => handles.text_zoom.clone().set(0),
        _ if id == menu::MINIMIZE.id => handles.controls.minimize(),
        _ if id == menu::ZOOM.id => handles.controls.toggle_maximize(),
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

#[cfg(feature = "kit-parcheado")]
fn use_window_title(title: &str) {
    let mut published = use_state(String::new);
    if *published.peek() != title {
        published.set(title.to_string());
        WindowControls::get().set_title(title);
    }
}
