use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use alacritty_terminal::event::Event as TermEvent;
use alacritty_terminal::grid::Scroll;
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use barrier_kit::app::use_menu_action;
use barrier_kit::platform::{Timers, WindowControls};
use barrier_kit::style::Palette;
use barrier_kit::theme::color;
use barrier_kit::titlebar::Titlebar;
use barrier_kit_next::app_menu::use_menu_enabled;
use barrier_kit_next::canvas::canvas;
use barrier_kit_next::views::tab_bar::TabBar;
use freya_clipboard::copypasta::ClipboardProvider;
use freya_core::prelude::*;
use torin::prelude::{Direction, Gaps, Size};
use vow_design::SPACE_1;

use crate::config::{APP_ID, Defaults, Scheme};
use crate::grid::{self, Fonts};
use crate::keys::{KeyAction, encode};
use crate::localize::tr;
use crate::menu;
use crate::session::{GridSize, Listener, Session, SessionId};
use crate::windows;

const MULTI_CLICK: Duration = Duration::from_millis(400);
const INITIAL_GRID: (u16, u16) = (80, 24);

struct Tab {
    session: Rc<Session>,
    title: String,
}

#[derive(Default)]
struct Tabs {
    tabs: Vec<Tab>,
    active: usize,
    next_id: SessionId,
    grid: Option<GridSize>,
    area: Option<(f32, f32)>,
}

impl Tabs {
    fn active(&self) -> Option<&Rc<Session>> {
        self.tabs.get(self.active).map(|tab| &tab.session)
    }

    fn index_of(&self, id: SessionId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.session.id == id)
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Click {
    at: Instant,
    point: Point,
    count: u8,
}

type ClipboardState = State<Option<Box<dyn ClipboardProvider>>>;

// Menu actions and wake-ups run outside Freya's context (BK-017): they use handles taken in render.
#[derive(Clone)]
struct Handles {
    tabs: State<Tabs>,
    controls: WindowControls,
    clipboard: ClipboardState,
}

pub struct Terminal;

impl App for Terminal {
    fn render(&self) -> impl IntoElement {
        let defaults = barrier_kit::app::use_defaults::<Defaults>(APP_ID);
        let dark = barrier_kit::theme::resolved().scheme.is_dark();
        let scheme: &'static Scheme = defaults.read().scheme(dark);
        let fonts = use_memo(move || {
            let defaults = defaults.read();
            Rc::new(Fonts::new(
                defaults.font_family(),
                defaults.font_size() as f32,
            ))
        });
        let mut tabs = use_state(Tabs::default);
        let mut revision = use_state(|| 0u64);
        let mut dragging = use_state(|| false);
        let mut last_click = use_state(|| None::<Click>);
        let handles = use_hook(move || Handles {
            tabs,
            controls: WindowControls::get(),
            clipboard: consume_root_context(),
        });
        use_side_effect(move || {
            let fonts = fonts.read().clone();
            let area = tabs.peek().area;
            if let Some((width, height)) = area {
                resize(tabs, &fonts, width, height);
            }
            *revision.write() += 1;
        });
        let events = use_hook({
            let handles = handles.clone();
            move || listen(handles, revision)
        });

        use_hook({
            let events = events.clone();
            move || open_tab(tabs, &events)
        });
        use_menu_action({
            let handles = handles.clone();
            move |id| {
                on_action(id, &handles, &events);
                let mut revision = revision;
                *revision.write() += 1;
            }
        });
        use_menu_enabled(
            menu::COPY.id,
            tabs.read()
                .active()
                .is_some_and(|session| session.selected_text().is_some()),
        );

        let focused = handles.controls.status.read().activated;
        let clipboard = handles.clipboard;
        let palette = Palette::current();
        let tabs_now = tabs.read();
        let title = tabs_now
            .tabs
            .get(tabs_now.active)
            .map_or_else(|| "CurseTerm".to_string(), |tab| tab.title.clone());
        let tab_bar = (tabs_now.tabs.len() > 1).then(|| {
            tabs_now
                .tabs
                .iter()
                .fold(TabBar::new(tr!("tabs"), tr!("close-tab")), |bar, tab| {
                    bar.tab(tab.title.clone())
                })
                .selected(tabs_now.active)
                .on_select(move |index| {
                    tabs.write().active = index;
                })
                .on_close({
                    let handles = handles.clone();
                    move |index| close_tab(&handles, index)
                })
        });
        let active = tabs_now.active().cloned();
        drop(tabs_now);
        #[cfg(feature = "kit-parcheado")]
        use_window_title(&title);

        let cells = fonts.read().clone();
        let painter = active.clone().map(|session| {
            let fonts = cells.clone();
            let mut look = DefaultHasher::new();
            (
                *revision.read(),
                focused,
                &scheme.name,
                fonts.cell_width.to_bits(),
            )
                .hash(&mut look);
            canvas(look.finish(), move |ctx| {
                grid::draw(ctx, &session.term.lock(), &fonts, scheme, focused);
            })
        });
        let point_at = {
            let fonts = cells.clone();
            let active = active.clone();
            move |location: (f64, f64)| -> Option<(Point, Side)> {
                let session = active.as_ref()?;
                let column = (location.0 as f32 / fonts.cell_width).max(0.0);
                let line = (location.1 as f32 / fonts.cell_height).max(0.0) as i32;
                let size = session.size();
                let point = Point::new(
                    Line(line.min(i32::from(size.lines) - 1) - session.display_offset() as i32),
                    Column((column as usize).min(usize::from(size.columns).saturating_sub(1))),
                );
                let side = if column.fract() < 0.5 {
                    Side::Left
                } else {
                    Side::Right
                };
                Some((point, side))
            }
        };

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .direction(Direction::Vertical)
            .background(color(palette.window_background))
            .on_global_key_down({
                let active = active.clone();
                move |event: Event<KeyboardEventData>| {
                    let Some(session) = &active else {
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
                }
            })
            .child(Titlebar::new().title(title))
            .maybe_child(tab_bar)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .padding(Gaps::new_all(SPACE_1))
                    .background(rgb(scheme.background))
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::fill())
                            .on_sized({
                                let fonts = cells.clone();
                                move |event: Event<SizedEventData>| {
                                    let area = event.data().area;
                                    resize(tabs, &fonts, area.width(), area.height());
                                }
                            })
                            .on_pointer_enter(|_| Cursor::set(CursorIcon::Text))
                            .on_pointer_leave(|_| Cursor::set(CursorIcon::default()))
                            .on_mouse_down({
                                let active = active.clone();
                                let point_at = point_at.clone();
                                move |event: Event<MouseEventData>| {
                                    let data = event.data();
                                    if data.button != Some(MouseButton::Left) {
                                        return;
                                    }
                                    let (Some(session), Some((point, side))) =
                                        (&active, point_at(data.element_location.to_tuple()))
                                    else {
                                        return;
                                    };
                                    let click = next_click(*last_click.read(), point);
                                    last_click.set(Some(click));
                                    let kind = match click.count {
                                        1 => SelectionType::Simple,
                                        2 => SelectionType::Semantic,
                                        _ => SelectionType::Lines,
                                    };
                                    session.start_selection(kind, point, side);
                                    dragging.set(true);
                                    *revision.write() += 1;
                                }
                            })
                            .on_mouse_move({
                                let active = active.clone();
                                move |event: Event<MouseEventData>| {
                                    if !*dragging.read() {
                                        return;
                                    }
                                    if let (Some(session), Some((point, side))) = (
                                        &active,
                                        point_at(event.data().element_location.to_tuple()),
                                    ) {
                                        session.extend_selection(point, side);
                                        *revision.write() += 1;
                                    }
                                }
                            })
                            .on_global_pointer_press(move |_| dragging.set_if_modified(false))
                            .on_wheel({
                                let active = active.clone();
                                let fonts = cells.clone();
                                move |event: Event<WheelEventData>| {
                                    let Some(session) = &active else {
                                        return;
                                    };
                                    let lines = (-event.data().delta_y as f32 / fonts.cell_height)
                                        .round()
                                        as i32;
                                    wheel(session, lines);
                                    *revision.write() += 1;
                                }
                            })
                            .maybe_child(painter),
                    ),
            )
    }
}

fn rgb(rgb: alacritty_terminal::vte::ansi::Rgb) -> Color {
    Color::from_rgb(rgb.r, rgb.g, rgb.b)
}

fn next_click(last: Option<Click>, point: Point) -> Click {
    let now = Instant::now();
    let count = match last {
        Some(last) if last.point == point && now - last.at < MULTI_CLICK => last.count % 3 + 1,
        _ => 1,
    };
    Click {
        at: now,
        point,
        count,
    }
}

fn wheel(session: &Session, lines: i32) {
    if lines == 0 {
        return;
    }
    let mode = session.mode();
    if mode.contains(TermMode::ALT_SCREEN) && mode.contains(TermMode::ALTERNATE_SCROLL) {
        let arrow: &[u8] = if lines > 0 { b"\x1bOA" } else { b"\x1bOB" };
        session.write(arrow.repeat(lines.unsigned_abs() as usize));
    } else {
        session.scroll(Scroll::Delta(lines));
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

fn paste(session: &Session, clipboard: ClipboardState) {
    match clipboard_text(clipboard) {
        Some(text) => session.paste(&text),
        None => tracing::warn!("could not read the clipboard"),
    }
}

fn resize(mut tabs: State<Tabs>, fonts: &Fonts, width: f32, height: f32) {
    let grid = GridSize {
        columns: (width / fonts.cell_width).floor().max(1.0) as u16,
        lines: (height / fonts.cell_height).floor().max(1.0) as u16,
        cell_width: fonts.cell_width,
        cell_height: fonts.cell_height,
    };
    let mut tabs = tabs.write();
    tabs.grid = Some(grid);
    tabs.area = Some((width, height));
    for tab in &tabs.tabs {
        tab.session.resize(grid);
    }
}

type Events = Rc<(
    std::sync::mpsc::Sender<(SessionId, TermEvent)>,
    barrier_kit::platform::WakeHandle,
)>;

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
    let mut tabs = handles.tabs;
    let Some(index) = tabs.peek().index_of(id) else {
        return;
    };
    let session = tabs.peek().tabs[index].session.clone();
    match event {
        TermEvent::Title(title) => tabs.write().tabs[index].title = title,
        TermEvent::ResetTitle => tabs.write().tabs[index].title = "CurseTerm".into(),
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
                format(alacritty_terminal::event::WindowSize {
                    num_lines: size.lines,
                    num_cols: size.columns,
                    cell_width: size.cell_width as u16,
                    cell_height: size.cell_height as u16,
                })
                .into_bytes(),
            );
        }
        TermEvent::ChildExit(_) | TermEvent::Exit => close_tab(handles, index),
        _ => {}
    }
}

fn open_tab(mut tabs: State<Tabs>, events: &Events) {
    let (columns, lines) = INITIAL_GRID;
    let mut state = tabs.write();
    let size = state
        .grid
        .or_else(|| state.active().map(|session| session.size()))
        .unwrap_or(GridSize {
            columns,
            lines,
            cell_width: 1.0,
            cell_height: 1.0,
        });
    let id = state.next_id;
    state.next_id += 1;
    let listener = Listener::new(id, events.0.clone(), events.1.clone());
    match Session::spawn(id, size, listener) {
        Ok(session) => {
            state.tabs.push(Tab {
                session: Rc::new(session),
                title: "CurseTerm".into(),
            });
            state.active = state.tabs.len() - 1;
        }
        Err(error) => tracing::error!(%error, "could not start the shell"),
    }
}

fn close_tab(handles: &Handles, index: usize) {
    let mut tabs = handles.tabs;
    let mut state = tabs.write();
    if index >= state.tabs.len() {
        return;
    }
    state.tabs.remove(index);
    if state.tabs.is_empty() {
        handles.controls.close();
        return;
    }
    state.active = state.active.min(state.tabs.len() - 1);
}

fn on_action(id: &str, handles: &Handles, events: &Events) {
    let tabs = handles.tabs;
    tracing::debug!(id, "menu action");
    let active = tabs.peek().active().cloned();
    match id {
        _ if id == menu::ABOUT.id => windows::open(Some("--about")),
        _ if id == menu::SETTINGS.id => windows::open(Some("--settings")),
        _ if id == menu::QUIT.id => {
            windows::quit_others();
            handles.controls.close();
        }
        _ if id == menu::NEW_WINDOW.id => windows::open(None),
        _ if id == menu::NEW_TAB.id => open_tab(tabs, events),
        _ if id == menu::CLOSE.id => {
            let active_index = tabs.peek().active;
            close_tab(handles, active_index);
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
