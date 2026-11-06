use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use alacritty_terminal::event::{Event as TermEvent, WindowSize};
use alacritty_terminal::grid::Scroll;
use alacritty_terminal::index::Direction as SearchDirection;
use alacritty_terminal::term::TermMode;
use barrier_kit::app::use_menu_action;
use barrier_kit::app_menu::MenuHandle;
use barrier_kit::compact::Stepper;
use barrier_kit::controls::{Button, ButtonStyle, SearchField, Slider, TextField};
use barrier_kit::platform::{DecorationResponse, Timers, WakeHandle, WindowControls};
use barrier_kit::popup_button::PopUpButton;
use barrier_kit::style::{Palette, text};
use barrier_kit::symbol::Symbol;
use barrier_kit::theme::color;
use barrier_kit::titlebar::{window_drag_handler, window_traffic_lights};
use barrier_kit::views::feedback::{Alert, sheet};
use barrier_kit::views::table::Table;
use barrier_kit_next::app_menu::{use_menu_checked, use_menu_enabled, use_submenu_items};
use barrier_kit_next::notifications::{self, Notice};
use barrier_kit_next::tokens::{
    ANNOTATION, PASTE_HISTORY_SIZE, PASTE_QUESTION_WIDTH, RENAME_WIDTH, SHORTCUT_LABEL, TAB_STRIP,
    TERMINAL, TITLE_BAR, TOOLBELT_WIDTH, TabStyle,
};
use barrier_kit_next::views::split_panes::SplitPanes;
use barrier_kit_next::views::tab_bar::{Badge, TabBar};
use freya_clipboard::copypasta::ClipboardProvider;
use freya_core::prelude::*;
use rustix::process::{Pid, Signal, kill_process};
use torin::prelude::{Alignment, Content, Direction, Gaps, Position, Size};

use vow_design::{
    CAPTION, DURATION_FAST_MS, DURATION_SLOW_MS, HEADLINE, Rgba, SPACE_1, SPACE_2, SPACE_3,
    TITLEBAR,
};

use crate::advanced_paste::{AdvancedPaste, advanced_paste_sheet};
use crate::arrangement::{self, Arrangement, TabLayout};
use crate::clock::{local_date_time, local_time};
use crate::composer_view::ComposerEvent;
use crate::config::{
    APP_ID, APP_NAME, AltKey, ClosePrompt, Defaults, MarkAlert, Profile, SessionEnd, Theme,
    WorkingDirectory, change_defaults,
};
use crate::find_mode::FindMode;
use crate::global_search::{GlobalSearch, SessionResults, global_search_panel};
use crate::grid::Fonts;
use crate::hint_view;
use crate::hints::{self, HintAction, Hints, Typed};
use crate::integration::Notification;
use crate::io::AnnotationId;
use crate::keys::{COPY_KEYS, CopyCommand, KeyAction, copy_command, encode};
use crate::links;
use crate::localize::tr;
use crate::menu;
use crate::note_view::NoteEvent;
use crate::pane_view::{self, TerminalPane};
use crate::panes::{Axis, Pane, Toward};
use crate::paste_history;
use crate::paste_transform::Transform;
use crate::paste_warning::{self, Warning};
use crate::pick;
use crate::session::{
    self, GridSize, Launch, Listener, MarkJump, Session, SessionId, TerminalState,
};
use crate::snippets::{self, Snippet};
use crate::status;
use crate::theme;
use crate::windows;
use crate::workspace::{Broadcast, Closed, Tab, Workspace};
use vow_appearance::ColorScheme;

const INITIAL_GRID: (u16, u16) = (80, 24);
const QUICK_EXIT: Duration = Duration::from_secs(1);
const DIVIDER_STEP: f32 = 0.05;
const UNDO_CLOSE: Duration = Duration::from_secs(5);
const SLOW_PASTE_CHUNK: usize = 16;
const SLOW_PASTE_PAUSE: Duration = Duration::from_millis(30);

type Sessions = HashMap<SessionId, Rc<Session>>;
type ClipboardState = State<Option<Box<dyn ClipboardProvider>>>;
type Events = Rc<(Sender<(SessionId, TermEvent)>, WakeHandle)>;
type Reveals = (Sender<(SessionId, Option<String>)>, WakeHandle);
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
    clock_armed: Rc<Cell<bool>>,
    blink_armed: Rc<Cell<bool>>,
    notes_armed: Rc<Cell<bool>>,
    editing: State<bool>,
    hints: State<Option<Hints>>,
    replay: State<Option<(SessionId, usize)>>,
    toolbelt: Toolbelt,
    clipboard: ClipboardState,
    broadcast: State<Broadcast>,
    find: Find,
    filter: Find,
    confirm: State<Option<Confirm>>,
    events: Rc<OnceCell<Events>>,
    reveals: Rc<OnceCell<Reveals>>,
    buried: State<Option<Buried>>,
    pastes: State<Vec<String>>,
    pending_paste: State<Option<(String, bool)>>,
    advanced: AdvancedPaste,
    global: GlobalSearch,
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
    SaveContents(SessionId),
    Log(SessionId),
    Coprocess(SessionId),
    SaveArrangement,
    NamedMark(SessionId),
    CopyModeKeys,
    AdvancedPaste,
    PasteWarning(Warning),
    PasteTabs,
}

struct Buried {
    tab: Tab,
    sessions: Vec<Rc<Session>>,
    titles: Vec<(SessionId, String)>,
    at: Instant,
}

#[derive(Clone, Copy)]
struct Toolbelt {
    shown: State<bool>,
    directories: State<Vec<String>>,
    job: State<Option<u32>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Picker {
    PasteHistory,
    OpenQuickly,
    Profiles,
    RecentDirectories,
}

#[derive(Clone)]
enum Choice {
    Paste(String),
    Type(String),
    Focus(SessionId),
    Profile(String),
}

#[derive(Clone, Copy)]
struct PickerState {
    query: State<String>,
    selected: State<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum Confirm {
    Close(Closing),
    RemoveAnnotation(SessionId, AnnotationId),
    MarkSet(SessionId),
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
    field: AccessibilityId,
    mode: fn(&Defaults) -> FindMode,
    save_mode: fn(&mut Defaults, FindMode),
}

impl Find {
    fn show_position(self, session: &Session, pattern: &str) {
        let mut position = self.position;
        position.set(session.find_position(pattern));
    }

    fn pattern(self, defaults: &Defaults) -> Option<String> {
        let query = self.query.read();
        (!query.is_empty()).then(|| (self.mode)(defaults).pattern(&query))
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

    fn replay_frames(&self, id: SessionId) -> usize {
        self.sessions
            .peek()
            .get(&id)
            .map_or(0, |session| session.extras().replay.len())
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
        let pattern = self.find.pattern(&self.defaults.peek());
        if let Some((session, pattern)) = self.active().zip(pattern) {
            session.find(&pattern, direction);
            self.find.show_position(&session, &pattern);
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
    pub arrangement: Option<String>,
    pub layout: Option<Arrangement>,
}

impl App for Terminal {
    fn render(&self) -> impl IntoElement {
        windows::use_registered_window();
        let defaults = barrier_kit::app::use_defaults::<Defaults>(APP_ID);
        // PseudoTerminal.m -updateWindowAppearance: the fixed themes force Aqua or DarkAqua.
        barrier_kit::theme::use_appearance(move |config| {
            config.color_scheme = match defaults.read().theme {
                Theme::Light | Theme::LightHighContrast => ColorScheme::Light,
                Theme::Dark | Theme::DarkHighContrast => ColorScheme::Dark,
                _ => config.color_scheme,
            }
        });
        let dark = barrier_kit::theme::resolved().scheme.is_dark();
        let workspace = use_state(Workspace::default);
        let sessions = use_state(Sessions::new);
        let next_id = use_state(|| 0);
        let text_zoom = use_state(|| 0i32);
        let mut revision = use_state(|| 0u64);
        let broadcast = use_state(Broadcast::default);
        let confirm = use_state(|| None::<Confirm>);
        let buried = use_state(|| None::<Buried>);
        let pastes = paste_history::shared();
        let pending_paste = use_state(|| None::<(String, bool)>);
        let advanced = AdvancedPaste {
            text: use_state(String::new),
            transform: use_state(Transform::default),
            regex: use_state(String::new),
            substitution: use_state(String::new),
            bracketed: use_state(|| true),
        };
        let global = GlobalSearch {
            open: use_state(|| false),
            query: use_state(String::new),
            focused: use_state(|| false),
            results: use_state(Vec::new),
            selected: use_state(|| None),
        };
        let sheet = use_state(|| None::<Sheet>);
        let mut held = use_state(Modifiers::empty);
        let transparency = use_state(|| true);
        let editing = use_state(|| false);
        let hints = use_state(|| None::<Hints>);
        let replay = use_state(|| None::<(SessionId, usize)>);
        let toolbelt = Toolbelt {
            shown: use_state(|| false),
            directories: use_state(Vec::new),
            job: use_state(|| None),
        };
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
            field: use_hook(AccessibilityId::new_unique),
            mode: |defaults| defaults.find_mode,
            save_mode: |defaults, mode| defaults.find_mode = mode,
        };
        let filter = Find {
            open: use_state(|| false),
            query: use_state(String::new),
            focused: use_state(|| false),
            position: use_state(|| None),
            field: use_hook(AccessibilityId::new_unique),
            mode: |defaults| defaults.filter_mode,
            save_mode: |defaults, mode| defaults.filter_mode = mode,
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
            clock_armed: Rc::new(Cell::new(false)),
            blink_armed: Rc::new(Cell::new(false)),
            notes_armed: Rc::new(Cell::new(false)),
            editing,
            hints,
            replay,
            toolbelt,
            clipboard: consume_root_context(),
            broadcast,
            find,
            filter,
            confirm,
            events: Rc::default(),
            reveals: Rc::default(),
            buried,
            pastes,
            pending_paste,
            advanced,
            global,
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
                let pattern = find.pattern(&defaults.read());
                if let Some((session, pattern)) = handles.active().zip(pattern) {
                    session.clear_selection();
                    session.find(&pattern, SearchDirection::Left);
                    *revision.write() += 1;
                    let later = handles.clone();
                    handles.after(Duration::from_millis(DURATION_FAST_MS), move || {
                        if find.pattern(&later.defaults.peek()).as_ref() == Some(&pattern) {
                            find.show_position(&session, &pattern);
                        }
                    });
                }
            }
        });
        use_side_effect({
            let handles = handles.clone();
            move || search_globally(&handles, &defaults.read())
        });
        let events = use_hook({
            let handles = handles.clone();
            move || {
                let events = listen(handles.clone(), revision);
                let _ = handles.events.set(events.clone());
                let _ = handles.reveals.set(listen_reveals(handles.clone()));
                events
            }
        });
        use_hook({
            let (handles, events) = (handles.clone(), events.clone());
            let first = self.profile.clone();
            let arrangement = self.arrangement.clone();
            let layout = self.layout.clone();
            move || {
                if arrangement.is_some_and(|name| restore_arrangement(&handles, &events, &name)) {
                    return;
                }
                if layout.is_some_and(|layout| open_layouts(&handles, &events, layout.tabs)) {
                    return;
                }
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
        // PTYTextView+ARC.m: the shortcuts take the keys, so the find field gives up focus; it runs
        // here because menu handlers have no Freya context (BK-017).
        if *find.focused.peek()
            && handles
                .hints
                .read()
                .as_ref()
                .is_some_and(|hints| hints.ended.is_none())
        {
            find.field.request_unfocus();
        }

        let window_focused = handles.controls.status.read().activated;
        use_focus_report(&handles, window_focused);
        let drag = window_drag_handler(handles.controls.fixed);
        let theme = defaults.read().theme;
        let chrome = match handles.controls.status.read().decoration {
            DecorationResponse::Server => Chrome::Server,
            DecorationResponse::Client if theme::compact(theme) => Chrome::Compact,
            DecorationResponse::Client => Chrome::Regular,
        };
        let compact = chrome == Chrome::Compact;
        let terminal_background = handles
            .active()
            .map(|session| {
                let profile = defaults.read().profile(&session.profile);
                defaults.read().scheme(&profile, dark).background
            })
            .unwrap_or_default();
        let tab_style = theme::tab_style(theme, dark, terminal_background, compact);
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
                    TabBar::new(tr!("tabs"), tr!("close-tab"), tab_style)
                        .active(window_focused)
                        .leading(if compact {
                            TAB_STRIP.compact_inset
                        } else {
                            0.0
                        })
                        .on_empty_mouse_down({
                            let drag = drag.clone();
                            move |event| drag(event)
                        }),
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
                find: find
                    .pattern(&defaults.read())
                    .filter(|_| *find.open.read())
                    .map(Rc::from),
                filter: handles
                    .filter
                    .pattern(&defaults.read())
                    .filter(|_| *handles.filter.open.read())
                    .map(Rc::from),
                receivers: match *broadcast.read() {
                    Broadcast::Off => Vec::new(),
                    mode => state.receivers(mode),
                },
                transparent,
                preedit: ime_preedit(&handles),
                replay: *replay.read(),
                blink_off: window_focused && cursor_blink_off(&handles, revision),
                maximized: tab.zoomed,
            };
            animate_notes(&handles, revision);
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
        let filter_bar = (*handles.filter.open.read())
            .then(|| filter_bar(handles.filter, &handles.defaults.read()));
        let replay_bar = (*replay.read()).map(|(id, index)| replay_bar(&handles, id, index));
        let alert = (*confirm.read()).map(|pending| confirm_alert(&handles, pending));
        let global_panel = (*handles.global.open.read()).then(|| {
            let (revealing, mode) = (handles.clone(), handles.defaults.read().global_search_mode);
            global_search_panel(
                handles.global,
                mode,
                |mode| change_defaults(|defaults| defaults.global_search_mode = mode),
                move |at| reveal_global_result(&revealing, at),
            )
        });
        let sheet_view = (*sheet.read()).map(|open| match open {
            Sheet::Pick(picker) => pick_sheet(&handles, picker, revision),
            Sheet::CopyModeKeys => copy_mode_keys_sheet(&handles),
            Sheet::PasteWarning(_) | Sheet::PasteTabs => paste_question_sheet(&handles, open),
            Sheet::AdvancedPaste => {
                let (pasting, cancelling) = (handles.clone(), handles.clone());
                advanced_paste_sheet(
                    handles.advanced,
                    move |text, bracketed| paste_advanced(&pasting, text, bracketed),
                    move || {
                        let mut sheet = cancelling.sheet;
                        sheet.set(None);
                    },
                )
            }
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
                if keyboard.confirm.peek().is_some() || *keyboard.editing.peek() {
                    return;
                }
                let replaying = *keyboard.replay.peek();
                if let Some((id, index)) = replaying {
                    let mut replay = keyboard.replay;
                    match data.key {
                        Key::Named(NamedKey::Escape) => replay.set(None),
                        Key::Named(NamedKey::ArrowLeft) => {
                            replay.set(Some((id, index.saturating_sub(1))))
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            let last = keyboard.replay_frames(id).saturating_sub(1);
                            replay.set(Some((id, (index + 1).min(last))));
                        }
                        _ => {}
                    }
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
                let global = keyboard.global;
                if *global.open.peek() && *global.focused.peek() {
                    if data.key == Key::Named(NamedKey::Escape) {
                        global.close();
                    }
                    return;
                }
                let filter = keyboard.filter;
                if *filter.open.peek() && *filter.focused.peek() {
                    if data.key == Key::Named(NamedKey::Escape) {
                        close_filter(filter);
                    }
                    return;
                }
                let Some(session) = &keyboard.active() else {
                    return;
                };
                if navigate_hints(&keyboard, session, &data.key, data.modifiers) {
                    return;
                }
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
            .children(window_chrome(
                chrome,
                dark,
                tab_style,
                title,
                tab_bar,
                window_focused,
                drag,
            ))
            .maybe_child(find_bar)
            .maybe_child(filter_bar)
            .maybe_child(alert)
            .maybe_child(sheet_view)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .direction(Direction::Horizontal)
                    .content(Content::flex())
                    .maybe_child(global_panel)
                    .child(
                        rect()
                            .width(Size::flex(1.0))
                            .height(Size::fill())
                            .maybe_child(body),
                    )
                    .maybe_child(
                        (*toolbelt.shown.read()).then(|| toolbelt_view(&handles, revision)),
                    ),
            )
            .maybe_child(status_bar(&handles, revision))
            .maybe_child(replay_bar)
    }
}

// The window chrome iTerm2 draws: with CSD, a macOS title bar above the tabs (Regular and the
// fixed themes) or one strip with the stoplights and the tabs, or a fake title bar as tall as
// the tabs when there is no tab bar (iTermRootTerminalView.m, Compact and Minimal).
#[derive(Clone, Copy, PartialEq)]
enum Chrome {
    Server,
    Regular,
    Compact,
}

fn window_chrome(
    chrome: Chrome,
    dark: bool,
    style: TabStyle,
    title: String,
    tab_bar: Option<TabBar>,
    active: bool,
    drag: impl Fn(Event<MouseEventData>) + 'static,
) -> Vec<Element> {
    if chrome == Chrome::Server {
        return tab_bar.into_iter().map(Into::into).collect();
    }
    let fixed = WindowControls::get().fixed;
    let lights = |height: f32| {
        rect()
            .position(Position::new_absolute().left(0.0).top(0.0))
            .layer(1)
            .width(Size::px(TAB_STRIP.compact_inset))
            .height(Size::px(height))
            .padding(Gaps::new(0.0, 0.0, 0.0, TAB_STRIP.compact_lights_x))
            .main_align(Alignment::Center)
            .child(window_traffic_lights(active, fixed))
    };
    let title_bar = |height: f32, background: Rgba, ink: Rgba| {
        rect()
            .width(Size::fill())
            .height(Size::px(height))
            .background(color(background))
            .on_mouse_down(drag)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .padding(Gaps::new(
                        0.0,
                        TAB_STRIP.compact_inset,
                        0.0,
                        TAB_STRIP.compact_inset,
                    ))
                    .main_align(Alignment::Center)
                    .cross_align(Alignment::Center)
                    .child(text(
                        title,
                        vow_design::Font {
                            size: TITLE_BAR.font_size as u32,
                            weight: TITLE_BAR.font_weight,
                            ..TITLEBAR.font
                        },
                        ink,
                    )),
            )
            .child(lights(height))
    };
    if chrome == Chrome::Regular {
        let colors = if dark {
            TITLE_BAR.dark
        } else {
            TITLE_BAR.light
        };
        let (ink, division) = if active {
            (colors.title, colors.division)
        } else {
            (colors.title_inactive, colors.division_inactive)
        };
        let below: Element = match tab_bar {
            Some(bar) => bar.into(),
            None => rect()
                .width(Size::fill())
                .height(Size::px(1.0))
                .background(color(division))
                .into(),
        };
        return vec![
            title_bar(TITLE_BAR.height, colors.background, ink).into(),
            below,
        ];
    }
    match tab_bar {
        Some(bar) => vec![
            rect()
                .width(Size::fill())
                .child(bar)
                .child(lights(style.height - 1.0))
                .into(),
        ],
        None => {
            let mut ink = if active {
                style.text_selected
            } else {
                style.text_selected_inactive
            };
            if active {
                ink.a *= TAB_STRIP.title_alpha;
            }
            vec![title_bar(style.height, style.backing, ink).into()]
        }
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
    filter: Option<Rc<str>>,
    transparent: bool,
    preedit: Option<Rc<str>>,
    replay: Option<(SessionId, usize)>,
    blink_off: bool,
    maximized: bool,
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
                    maximized: painting.maximized,
                    find: painting.find.clone().filter(|_| *id == painting.active),
                    filter: painting.filter.clone().filter(|_| *id == painting.active),
                    on_focus: {
                        let mut hints = painting.handles.hints;
                        Callback::new(move |id| {
                            if let Some(shown) = hints.write().as_mut() {
                                shown.end(None);
                            }
                            workspace.write().focus(id)
                        })
                    },
                    on_hover: {
                        let defaults = painting.handles.defaults;
                        Callback::new(move |id| {
                            if defaults.peek().focus_follows_mouse {
                                workspace.write().focus(id);
                            }
                        })
                    },
                    on_select: {
                        let handles = painting.handles.clone();
                        Callback::new(move |text| {
                            if handles.defaults.peek().copy_on_select {
                                copy_text(&handles, text);
                            }
                        })
                    },
                    held: painting.handles.held,
                    preedit: painting.preedit.clone().filter(|_| *id == painting.active),
                    replay: painting
                        .replay
                        .filter(|(replayed, _)| replayed == id)
                        .map(|(_, index)| index),
                    blink_off: painting.blink_off && *id == painting.active,
                    annotations: session.extras().annotations.clone(),
                    dark: painting.dark,
                    composer: session.composer.borrow().clone(),
                    hints: painting
                        .handles
                        .hints
                        .read()
                        .clone()
                        .filter(|hints| hints.session == *id),
                    on_composer: {
                        let handles = painting.handles.clone();
                        let session = session.clone();
                        Callback::new(move |event| composer_event(&handles, &session, event))
                    },
                    on_note: {
                        let handles = painting.handles.clone();
                        let session = session.clone();
                        Callback::new(move |(note, event)| match event {
                            NoteEvent::Changed(text) => session.set_annotation_text(note, text),
                            NoteEvent::Focused(focused) => {
                                let mut editing = handles.editing;
                                editing.set(focused);
                                if focused {
                                    session.annotation_focused(note);
                                }
                            }
                            NoteEvent::Remove => {
                                let mut confirm = handles.confirm;
                                confirm.set(Some(Confirm::RemoveAnnotation(session.id, note)));
                            }
                        })
                    },
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
    use_menu_enabled(
        menu::SELECT_CURRENT_COMMAND.id,
        handles
            .active()
            .is_some_and(|session| session.has_current_command()),
    );
    let copy_mode = handles.active().is_some_and(|session| session.copy_mode());
    use_menu_checked(menu::COPY_MODE.id, copy_mode);
    let timestamps = handles
        .active()
        .is_some_and(|session| session.extras().show_timestamps);
    use_menu_checked(menu::SHOW_TIMESTAMPS.id, timestamps);
    let guide = handles
        .active()
        .is_some_and(|session| session.extras().cursor_guide);
    use_menu_checked(menu::CURSOR_GUIDE.id, guide);
    let annotated = handles
        .active()
        .is_some_and(|session| session.has_annotations());
    use_menu_checked(menu::SHOW_ANNOTATIONS.id, annotated);
    use_menu_enabled(menu::ANNOTATE_SELECTION.id, selection);
    let searching = *handles.find.open.read() && !handles.find.query.read().is_empty();
    use_menu_enabled(menu::PICK_RESULT.id, searching);
    use_menu_enabled(menu::SELECT_MATCHES.id, searching);
    use_menu_enabled(menu::CLEAR_FIND.id, !handles.find.query.read().is_empty());
    use_menu_checked(menu::TOOLBELT.id, *handles.toolbelt.shown.read());
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
    let arrangements: Vec<String> = handles
        .defaults
        .read()
        .arrangements
        .iter()
        .map(|saved| saved.name.clone())
        .collect();
    use_submenu_items(
        menu::RESTORE_ARRANGEMENT,
        menu::indexed_items(menu::ARRANGEMENT_PREFIX, &arrangements),
    );
    use_submenu_items(
        menu::RESTORE_ARRANGEMENT_TABS,
        menu::indexed_items(menu::ARRANGEMENT_TABS_PREFIX, &arrangements),
    );
    let named_marks = handles
        .active()
        .map(|session| session.named_marks())
        .unwrap_or_default();
    use_submenu_items(
        menu::NAMED_MARKS_MENU,
        menu::indexed_items(menu::NAMED_MARK_PREFIX, &named_marks),
    );
    let next_mark = handles
        .active()
        .is_some_and(|session| session.extras().alert_on_next_mark);
    let (offscreen, action) = {
        let defaults = handles.defaults.read();
        (defaults.alert_on_offscreen_marks, defaults.mark_alert)
    };
    use_submenu_items(
        menu::ALERTS_MENU,
        menu::alert_items(next_mark, offscreen, action),
    );
    use_submenu_items(
        menu::SNIPPETS_MENU,
        snippets::menu_items(&handles.defaults.read().snippets),
    );
    use_submenu_items(
        menu::BROADCAST_MENU,
        menu::broadcast_items(*handles.broadcast.read()),
    );
    let scrollback = handles
        .active()
        .is_some_and(|session| session.has_scrollback());
    use_menu_enabled(menu::CLEAR_SCROLLBACK.id, scrollback);
    use_menu_enabled(menu::CLOSE_PANES.id, split);
    use_menu_enabled(menu::ARRANGE_EVENLY, split);
    let warnings = handles.defaults.read().paste_warnings;
    use_menu_checked(menu::WARN_MULTI_LINE_PASTE, warnings.multi_line);
    use_menu_checked(menu::LIMIT_PASTE_WARNING, warnings.only_at_prompt);
    use_menu_enabled(menu::LIMIT_PASTE_WARNING, warnings.multi_line);
    use_menu_checked(menu::WARN_ONE_LINE_PASTE, warnings.one_line_at_prompt);
    use_menu_enabled(menu::UNDO_CLOSE.id, handles.buried.read().is_some());
    use_menu_enabled(menu::PASTE_HISTORY.id, !handles.pastes.read().is_empty());
    let current = handles.active();
    use_menu_enabled(
        menu::CLEAR_TO_SELECTION.id,
        current
            .as_ref()
            .is_some_and(|session| session.selection_start().is_some()),
    );
    use_menu_enabled(
        menu::CLEAR_TO_LAST_MARK.id,
        current.is_some_and(|session| session.last_mark_above_cursor().is_some()),
    );
    use_menu_enabled(
        menu::RECENT_DIRECTORIES.id,
        !handles.toolbelt.directories.read().is_empty(),
    );
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
        copy_text(handles, text);
    }
}

fn copy_text(handles: &Handles, text: String) {
    remember_paste(handles, text.clone());
    set_clipboard(handles.clipboard, text);
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

// iTermPasteHelper.m -pasteString:…: the transformed text goes out as is, bracketed when asked
// and the program wants it. BK-027: the sheet closes one loop later.
fn paste_advanced(handles: &Handles, text: String, bracketed: bool) {
    let later = handles.clone();
    handles.after(Duration::ZERO, move || {
        let mut sheet = later.sheet;
        sheet.set(None);
        for session in later.receivers() {
            let bytes = if bracketed && session.mode().contains(TermMode::BRACKETED_PASTE) {
                session.pasted_bytes(&text)
            } else {
                text.clone().into_bytes()
            };
            session.write(bytes);
        }
        remember_paste(&later, text.clone());
    });
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
    paste_checked(handles, text, false);
}

// PTYSession.m -pasteString:flags: tabs are asked about first unless the program brackets.
fn paste_checked(handles: &Handles, text: String, slowly: bool) {
    let bracketing = handles
        .active()
        .is_some_and(|session| session.mode().contains(TermMode::BRACKETED_PASTE));
    if !bracketing && text.contains('\t') {
        ask_paste(handles, text, slowly, Sheet::PasteTabs);
        return;
    }
    paste_past_tabs(handles, text, slowly);
}

// iTermPasteHelper.m -tryToPasteEvent: every paste, slow ones too, goes past the warning.
fn paste_past_tabs(handles: &Handles, text: String, slowly: bool) {
    let at_prompt = handles.active().is_some_and(|session| session.at_prompt());
    let warnings = handles.defaults.peek().paste_warnings;
    match paste_warning::warning(&text, at_prompt, warnings) {
        Some(warning) => ask_paste(handles, text, slowly, Sheet::PasteWarning(warning)),
        None => deliver_paste(handles, text, slowly),
    }
}

fn deliver_paste(handles: &Handles, text: String, slowly: bool) {
    if slowly {
        paste_slowly(handles, text);
    } else {
        paste_now(handles, text);
    }
}

fn ask_paste(handles: &Handles, text: String, slowly: bool, question: Sheet) {
    if handles.sheet.peek().is_some() {
        return;
    }
    let mut pending = handles.pending_paste;
    pending.set(Some((text, slowly)));
    open_sheet(handles, question);
}

#[derive(Clone, Copy, PartialEq)]
enum PasteAnswer {
    Paste,
    WithoutNewline,
    ConvertTabs,
    Advanced,
    Cancel,
}

// iTermPasteHelper.m -maybeWarnAboutMultiLinePaste: and +pasteWithTabsWarningWithAccessory…:
// iTermWarning's buttons, which the kit's two-button Alert cannot hold (BK-037).
fn paste_question_sheet(handles: &Handles, question: Sheet) -> Element {
    let palette = Palette::current();
    let (heading, message, answers) = match question {
        Sheet::PasteWarning(warning) => {
            let message = match warning {
                Warning::Lines(count) => tr!("paste-lines", count = count),
                Warning::LinesAtPrompt(count) => tr!("paste-lines-at-prompt", count = count),
                Warning::OneLine => tr!("paste-one-line").to_string(),
                Warning::OneLineAtPrompt => tr!("paste-one-line-at-prompt").to_string(),
            };
            let one_line = matches!(warning, Warning::OneLine | Warning::OneLineAtPrompt);
            let mut answers = vec![(tr!("paste-confirm"), PasteAnswer::Paste)];
            if one_line {
                answers.push((tr!("paste-without-newline"), PasteAnswer::WithoutNewline));
            }
            answers.push((tr!("close-cancel"), PasteAnswer::Cancel));
            answers.push((tr!("paste-advanced"), PasteAnswer::Advanced));
            (Some(tr!("paste-warning-title")), message, answers)
        }
        _ => (
            None,
            tr!("paste-tabs-title").to_string(),
            vec![
                (tr!("mark-alert-ok"), PasteAnswer::Paste),
                (tr!("close-cancel"), PasteAnswer::Cancel),
                (tr!("paste-convert-tabs"), PasteAnswer::ConvertTabs),
                (tr!("paste-advanced"), PasteAnswer::Advanced),
            ],
        ),
    };
    let spaces = (question == Sheet::PasteTabs).then(|| {
        rect()
            .direction(Direction::Horizontal)
            .cross_align(Alignment::Center)
            .spacing(SPACE_2)
            .child(text(tr!("paste-spaces"), CAPTION, palette.label))
            .child(
                Stepper::new(tr!("paste-spaces"), 1, 100)
                    .value(i64::from(handles.defaults.read().paste_tab_spaces))
                    .on_change(|spaces: i64| {
                        change_defaults(|defaults| defaults.paste_tab_spaces = spaces as u8)
                    }),
            )
    });
    let buttons = answers
        .into_iter()
        .enumerate()
        .map(|(index, (label, answer))| {
            let answering = handles.clone();
            let button =
                Button::new(label).on_press(move |_| answer_paste(&answering, question, answer));
            if index == 0 {
                button.style(ButtonStyle::Primary).into_element()
            } else {
                button.into_element()
            }
        });
    sheet(
        rect()
            .width(Size::px(PASTE_QUESTION_WIDTH))
            .direction(Direction::Vertical)
            .spacing(SPACE_2)
            .maybe_child(heading.map(|heading| text(heading, HEADLINE, palette.label)))
            .child(text(message, CAPTION, palette.secondary_label))
            .maybe_child(spaces)
            .child(
                rect()
                    .width(Size::fill())
                    .direction(Direction::Horizontal)
                    .main_align(Alignment::End)
                    .spacing(SPACE_2)
                    .children(buttons),
            ),
    )
    .into_element()
}

// BK-027: Enter also reaches the global handler, so the sheet closes one loop later.
fn answer_paste(handles: &Handles, question: Sheet, answer: PasteAnswer) {
    let later = handles.clone();
    handles.after(Duration::ZERO, move || {
        let mut sheet = later.sheet;
        sheet.set(None);
        let mut pending = later.pending_paste;
        let Some((text, slowly)) = pending.write().take() else {
            return;
        };
        let asked_about_tabs = question == Sheet::PasteTabs;
        match answer {
            PasteAnswer::Cancel => {}
            PasteAnswer::Paste if asked_about_tabs => paste_past_tabs(&later, text, slowly),
            PasteAnswer::Paste => deliver_paste(&later, text, slowly),
            PasteAnswer::WithoutNewline => {
                let trimmed = text.trim_end_matches(['\r', '\n']).to_string();
                deliver_paste(&later, trimmed, slowly);
            }
            PasteAnswer::ConvertTabs => {
                let spaces = " ".repeat(usize::from(later.defaults.peek().paste_tab_spaces));
                paste_past_tabs(&later, text.replace('\t', &spaces), slowly);
            }
            PasteAnswer::Advanced => {
                later.advanced.open(text);
                open_sheet(&later, Sheet::AdvancedPaste);
            }
        }
    });
}

fn paste_now(handles: &Handles, text: String) {
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
        Picker::RecentDirectories => handles
            .toolbelt
            .directories
            .read()
            .iter()
            .map(|path| (path.clone(), Choice::Type(session::shell_escaped(path))))
            .collect(),
    }
}

fn pick_sheet(handles: &Handles, picker: Picker, mut revision: State<u64>) -> Element {
    let (heading, column) = match picker {
        Picker::PasteHistory => (tr!("paste-history"), tr!("paste-history-text")),
        Picker::OpenQuickly => (tr!("open-quickly"), tr!("open-quickly-session")),
        Picker::Profiles => (tr!("open-profiles"), tr!("open-profiles-name")),
        Picker::RecentDirectories => (tr!("recent-directories-title"), tr!("toolbelt-path")),
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
            Choice::Type(text) => {
                if let Some(session) = later.active() {
                    session.write(text.clone().into_bytes());
                }
            }
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
        Sheet::SaveContents(_) => (tr!("save-contents"), tr!("save-path"), tr!("save-confirm")),
        Sheet::Log(_) => (tr!("log-start"), tr!("save-path"), tr!("log-confirm")),
        Sheet::SaveArrangement => (
            tr!("save-arrangement"),
            tr!("arrangement-name"),
            tr!("save-confirm"),
        ),
        Sheet::NamedMark(_) => (
            tr!("enter-mark-name"),
            tr!("mark-name"),
            tr!("mark-name-confirm"),
        ),
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
            Sheet::SaveSelection(of) => save(&later, of, &typed, Session::selected_text),
            Sheet::SaveContents(of) => save(&later, of, &typed, |session| Some(session.contents())),
            Sheet::Log(of) => start_log(&later, of, &typed),
            Sheet::Coprocess(of) => start_coprocess(&later, of, &typed),
            Sheet::SaveArrangement => save_arrangement(&later, &typed),
            Sheet::NamedMark(of) => {
                if let Some(session) = later.sessions.peek().get(&of).filter(|_| !typed.is_empty())
                {
                    session.set_named_mark(
                        &typed,
                        tr!("named-mark-annotation", name = typed.as_str()),
                    );
                }
            }
            _ => workspace.write().rename_window(&typed),
        }
        let mut sheet = later.sheet;
        sheet.set(None);
    });
}

fn save(handles: &Handles, of: SessionId, path: &str, text: fn(&Session) -> Option<String>) {
    let text = handles
        .sessions
        .peek()
        .get(&of)
        .and_then(|session| text(session));
    let path = from_home(path);
    if let Some(text) = text
        && let Err(error) = std::fs::write(&path, text)
    {
        tracing::warn!(%error, path = %path.display(), "could not save the text");
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

fn toolbelt_view(handles: &Handles, mut revision: State<u64>) -> Element {
    let palette = Palette::current();
    let toolbelt = handles.toolbelt;
    let jobs = handles
        .active()
        .map(|session| session.jobs())
        .unwrap_or_default();
    let chosen = *toolbelt.job.read();
    let job_rows = jobs
        .iter()
        .map(|job| vec![job.pid.to_string(), job.name.clone()])
        .collect();
    let selected_job = jobs.iter().position(|job| Some(job.pid) == chosen);
    let pids: Vec<u32> = jobs.iter().map(|job| job.pid).collect();
    let pastes = handles.pastes.read().clone();
    let paste_rows = pastes
        .iter()
        .map(|text| vec![paste_history::one_line(text)])
        .collect();
    let directories = toolbelt.directories.read().clone();
    let directory_rows = directories.iter().map(|path| vec![path.clone()]).collect();
    let (paster, typer) = (handles.clone(), handles.clone());
    let heading = |title: &'static str| text(title, CAPTION, palette.secondary_label);
    rect()
        .width(Size::px(TOOLBELT_WIDTH))
        .height(Size::fill())
        .direction(Direction::Vertical)
        .content(Content::flex())
        .padding(SPACE_2)
        .spacing(SPACE_2)
        .background(color(palette.window_background))
        .child(heading(tr!("toolbelt-jobs")))
        .child(
            rect().width(Size::fill()).height(Size::flex(1.0)).child(
                Table::new(
                    tr!("toolbelt-jobs"),
                    vec![tr!("toolbelt-pid"), tr!("toolbelt-command")],
                )
                .rows(job_rows)
                .selected(selected_job)
                .on_select(move |index: usize| {
                    let mut job = toolbelt.job;
                    job.set(pids.get(index).copied());
                }),
            ),
        )
        .child(
            Button::new(tr!("toolbelt-terminate"))
                .disabled(selected_job.is_none())
                .on_press(move |_| {
                    if let Some(pid) = chosen.and_then(|pid| Pid::from_raw(pid as i32)) {
                        let _ = kill_process(pid, Signal::TERM);
                    }
                    *revision.write() += 1;
                }),
        )
        .child(heading(tr!("paste-history")))
        .child(
            rect().width(Size::fill()).height(Size::flex(1.0)).child(
                Table::new(tr!("paste-history"), vec![tr!("paste-history-text")])
                    .rows(paste_rows)
                    .on_select(move |index: usize| {
                        if let Some(text) = pastes.get(index) {
                            paste_text(&paster, text.clone());
                        }
                    }),
            ),
        )
        .child(heading(tr!("toolbelt-directories")))
        .child(
            rect().width(Size::fill()).height(Size::flex(1.0)).child(
                Table::new(tr!("toolbelt-directories"), vec![tr!("toolbelt-path")])
                    .rows(directory_rows)
                    .on_select(move |index: usize| {
                        if let (Some(path), Some(session)) =
                            (directories.get(index), typer.active())
                        {
                            session.write(
                                format!("cd {}\r", session::shell_escaped(path)).into_bytes(),
                            );
                        }
                    }),
            ),
        )
        .into_element()
}

fn repaint_after(handles: &Handles, revision: State<u64>, armed: &Rc<Cell<bool>>, delay: Duration) {
    if !armed.replace(true) {
        let armed = armed.clone();
        handles.after(delay, move || {
            armed.set(false);
            let mut revision = revision;
            *revision.write() += 1;
        });
    }
}

// iTermComposerManager.m: Escape and the close button keep the text for the next time; a send
// empties it and dismisses without the fade.
fn composer_event(handles: &Handles, session: &Rc<Session>, event: ComposerEvent) {
    let mut composer = session.composer.borrow_mut();
    match event {
        ComposerEvent::Changed(text) => composer.text = text,
        ComposerEvent::Focused(focused) => {
            let mut editing = handles.editing;
            editing.set(focused);
            return;
        }
        ComposerEvent::Close => composer.close(),
        ComposerEvent::SendAll => {
            if let Some(command) = composer.send_all() {
                session.send_command(&command);
            }
        }
        ComposerEvent::SendCommandAt(cursor) => {
            if let Some(command) = composer.take_command(cursor) {
                session.send_command(&command);
            }
        }
    }
    let closed = !composer.open;
    drop(composer);
    if closed {
        let editing = handles.editing;
        // BK-027: the closing key still reaches the global handler, so editing ends one loop later.
        handles.after(Duration::ZERO, move || {
            let mut editing = editing;
            editing.set(false);
        });
    }
    let mut workspace = handles.workspace;
    workspace.write();
}

fn show_hints(handles: &Handles, session: &Session, found: Vec<hints::Found>, action: HintAction) {
    if !found.is_empty() {
        let mut hints = handles.hints;
        hints.set(Some(Hints::new(session.id, found, action)));
    }
}

// iTermCopyModeHandler.m iTermShortcutNavigationModeHandler: Command and Control keys pass
// through, Escape leaves, and an unknown key goes on to the terminal.
fn navigate_hints(handles: &Handles, session: &Rc<Session>, key: &Key, held: Modifiers) -> bool {
    let mut state = handles.hints;
    let mut state = state.write();
    let Some(hints) = state
        .as_mut()
        .filter(|hints| hints.session == session.id && hints.ended.is_none())
    else {
        return false;
    };
    if held.intersects(Modifiers::META | Modifiers::CONTROL) {
        return false;
    }
    match key {
        Key::Named(NamedKey::Escape) => {
            hints.end(None);
            true
        }
        Key::Character(typed) => match hints.type_key(typed) {
            // PTYTextView+ARC.m: ⌥ swaps open for copy, and copy for typing into the shell.
            Typed::Chosen(index) => {
                let text = hints.items[index].text.clone();
                let action = hints.action;
                hints.end(Some(index));
                drop(state);
                match (action, held.contains(Modifiers::ALT)) {
                    (HintAction::Open, false) => links::open(&session.open_target(&text)),
                    (HintAction::Open, true) | (HintAction::Copy, false) => {
                        copy_text(handles, text)
                    }
                    (HintAction::Copy, true) => session.write(text.into_bytes()),
                }
                true
            }
            Typed::Narrowed => true,
            Typed::Unhandled => false,
        },
        _ => false,
    }
}

fn animate_notes(handles: &Handles, revision: State<u64>) {
    let mut hints = handles.hints;
    let finished = hints.peek().as_ref().is_some_and(|shown| {
        shown
            .ended
            .is_some_and(|(at, _)| hint_view::settled(at, SHORTCUT_LABEL.exit_ms))
    });
    if finished {
        hints.set(None);
    }
    let animating = handles.sessions.peek().values().any(|session| {
        session
            .extras()
            .annotations
            .iter()
            .any(pane_view::note_animating)
            || pane_view::composer_animating(&session.composer.borrow())
    }) || hints
        .peek()
        .as_ref()
        .is_some_and(pane_view::hints_animating);
    if animating {
        repaint_after(
            handles,
            revision,
            &handles.notes_armed,
            Duration::from_millis(ANNOTATION.frame_ms),
        );
    }
}

fn cursor_blink_off(handles: &Handles, revision: State<u64>) -> bool {
    let Some(session) = handles.active() else {
        return false;
    };
    if !session.term.lock().cursor_style().blinking {
        return false;
    }
    let period = TERMINAL.cursor_blink_ms;
    repaint_after(
        handles,
        revision,
        &handles.blink_armed,
        Duration::from_millis(period),
    );
    (session.started.elapsed().as_millis() / u128::from(period)) % 2 == 1
}

fn status_bar(handles: &Handles, revision: State<u64>) -> Option<Element> {
    let session = handles.active()?;
    let profile = handles.defaults.read().profile(&session.profile);
    if !profile.status_bar {
        return None;
    }
    repaint_after(
        handles,
        revision,
        &handles.clock_armed,
        Duration::from_secs(1),
    );
    let directory = session.working_directory();
    let parts = [
        directory.as_ref().map(|path| path.display().to_string()),
        session.foreground_name(),
        directory.as_deref().and_then(status::git_branch),
        status::host_name(),
        Some(local_time(std::time::SystemTime::now())),
    ];
    let palette = Palette::current();
    Some(
        rect()
            .width(Size::fill())
            .direction(Direction::Horizontal)
            .padding(SPACE_1)
            .spacing(SPACE_3)
            .background(color(palette.window_background))
            .children(
                parts
                    .into_iter()
                    .flatten()
                    .map(|part| text(part, CAPTION, palette.secondary_label).into_element()),
            )
            .into_element(),
    )
}

fn replay_bar(handles: &Handles, id: SessionId, index: usize) -> Element {
    let session = handles.sessions.read().get(&id).cloned();
    let (frames, at) = session.map_or((0, None), |session| {
        let extras = session.extras();
        (
            extras.replay.len(),
            extras.replay.get(index).map(|frame| frame.at),
        )
    });
    let mut replay = handles.replay;
    let palette = Palette::current();
    rect()
        .width(Size::fill())
        .direction(Direction::Horizontal)
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .padding(SPACE_1)
        .spacing(SPACE_2)
        .background(color(palette.window_background))
        .child(text(
            tr!("instant-replay-bar"),
            CAPTION,
            palette.secondary_label,
        ))
        .child(
            rect().width(Size::flex(1.0)).child(
                Slider::stepped(frames.max(2))
                    .value(index)
                    .on_change(move |step: usize| replay.set(Some((id, step)))),
            ),
        )
        .maybe_child(at.map(|at| text(local_time(at), CAPTION, palette.label)))
        .child(Button::new(tr!("done")).on_press(move |_| replay.set(None)))
        .into_element()
}

// PTYTextView+ARC.m -sendSnippet: with Option held the snippet opens in Advanced Paste.
fn send_snippet(handles: &Handles, session: Option<&Session>, snippet: &Snippet) {
    let text = snippet.escaped();
    if handles.held.peek().contains(Modifiers::ALT) {
        handles.advanced.open(text);
        open_sheet(handles, Sheet::AdvancedPaste);
    } else if let Some(session) = session {
        session.write(text.into_bytes());
    }
}

// iTermGlobalSearchWindowController.m -search: every session of the window, tab by tab.
fn search_globally(handles: &Handles, defaults: &Defaults) {
    let global = handles.global;
    let query = global.query.read().clone();
    let open = *global.open.read();
    let (mut results, mut selected) = (global.results, global.selected);
    selected.set(None);
    if !open || query.is_empty() {
        results.set(Vec::new());
        return;
    }
    let pattern = defaults.global_search_mode.pattern(&query);
    let workspace = handles.workspace.peek();
    let sessions = handles.sessions.peek();
    let found = workspace
        .tabs
        .iter()
        .flat_map(|tab| tab.root.leaves())
        .filter_map(|id| {
            let snippets = sessions.get(&id)?.snippets(&pattern);
            (!snippets.is_empty()).then(|| SessionResults {
                session: id,
                name: workspace.title(id).to_string(),
                snippets,
            })
        })
        .collect();
    results.set(found);
}

// iTermGlobalSearchResult.m -revealWithState:completion:
fn reveal_global_result(handles: &Handles, (group, row): (usize, usize)) {
    let global = handles.global;
    let Some((id, snippet)) = global
        .results
        .peek()
        .get(group)
        .and_then(|found| Some((found.session, found.snippets.get(row)?.clone())))
    else {
        return;
    };
    let mut selected = global.selected;
    selected.set(Some((group, row)));
    let mut workspace = handles.workspace;
    workspace.write().focus(id);
    if let Some(session) = handles.sessions.peek().get(&id) {
        session.reveal_match(snippet.start, snippet.end);
    }
}

// FindView.xib: the search field's menu chooses how the query matches.
fn mode_popup(find: Find, defaults: &Defaults) -> PopUpButton {
    let names = [
        tr!("find-mode-smart-case"),
        tr!("find-mode-case-sensitive"),
        tr!("find-mode-case-insensitive"),
        tr!("find-mode-regex-case-sensitive"),
        tr!("find-mode-regex-case-insensitive"),
    ];
    let selected = FindMode::ALL
        .iter()
        .position(|mode| *mode == (find.mode)(defaults))
        .unwrap_or_default();
    names
        .into_iter()
        .fold(PopUpButton::new(tr!("find-mode")), |button, name| {
            button.item(name)
        })
        .selected(selected)
        .on_change(move |position: usize| {
            change_defaults(|defaults| (find.save_mode)(defaults, FindMode::ALL[position]));
        })
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
                .a11y_id(find.field)
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
        .child(mode_popup(find, &handles.defaults.read()))
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

// iTermFindDriver.m: the Filter field; the session shows only the lines that match until it
// closes.
fn filter_bar(filter: Find, defaults: &Defaults) -> Element {
    rect()
        .background(color(Palette::current().window_background))
        .width(Size::fill())
        .direction(Direction::Horizontal)
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .padding(SPACE_1)
        .spacing(SPACE_1)
        .child(
            SearchField::new(filter.query, tr!("filter-placeholder"))
                .a11y_id(filter.field)
                .width(Size::flex(1.0))
                .auto_focus(true)
                .on_focus_change(move |focused| {
                    let mut state = filter.focused;
                    state.set(focused);
                }),
        )
        .child(mode_popup(filter, defaults))
        .child(Button::new(tr!("find-close")).on_press(move |_| close_filter(filter)))
        .into_element()
}

fn close_filter(filter: Find) {
    let mut query = filter.query;
    query.set(String::new());
    filter.close();
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
        TermEvent::Title(title) => {
            workspace.write().set_title(id, Some(title));
            if let Some(directory) = session.working_directory() {
                let mut directories = handles.toolbelt.directories;
                paste_history::remember(&mut directories.write(), directory.display().to_string());
            }
        }
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
            if profile.notify_bell && should_post_notification(handles, id, &profile) {
                let (name, tab) = session_label(&workspace.peek(), id);
                post_for_session(
                    handles,
                    id,
                    tr!("bell-title").to_string(),
                    tr!("bell-notification", name = name, tab = tab),
                );
            }
            if profile.flash_visual_bell {
                session.extras().bell = Some(Instant::now());
                handles.after(Duration::from_millis(DURATION_FAST_MS), move || {
                    let mut revision = revision;
                    *revision.write() += 1;
                });
            }
        }
        TermEvent::Wakeup => {
            let (reported, posted) = {
                let mut extras = session.extras();
                (
                    std::mem::take(&mut extras.mark_reported),
                    std::mem::take(&mut extras.notifications),
                )
            };
            if reported {
                mark_set(handles, &session);
            }
            let profile = handles.defaults.peek().profile(&session.profile);
            for notification in posted {
                post_escape_sequence_notification(handles, id, &profile, notification);
            }
            note_output(handles, &session, &profile);
        }
        TermEvent::ChildExit(_) | TermEvent::Exit => {
            let profile = handles.defaults.peek().profile(&session.profile);
            if profile.notify_session_ended && should_post_notification(handles, id, &profile) {
                let (name, tab) = session_label(&workspace.peek(), id);
                post_for_session(
                    handles,
                    id,
                    tr!("session-ended-title").to_string(),
                    tr!("session-ended-notification", name = name, tab = tab),
                );
            }
            match profile.when_session_ends {
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

fn session_label(workspace: &Workspace, id: SessionId) -> (String, usize) {
    let tab = workspace.tab_of(id).map_or(0, |index| index + 1);
    (workspace.title(id).to_string(), tab)
}

fn post_for_session(handles: &Handles, id: SessionId, summary: String, body: String) {
    let Some((sender, wake)) = handles.reveals.get().cloned() else {
        return;
    };
    let notice = Notice {
        app_name: APP_NAME,
        desktop_entry: APP_ID,
        summary,
        body,
        action: tr!("reveal").to_string(),
    };
    notifications::post(notice, move |token| {
        if sender.send((id, token)).is_ok() {
            wake.wake();
        }
    });
}

// PTYSession.m -notificationsSuppressed
fn notifications_suppressed(handles: &Handles, id: SessionId, profile: &Profile) -> bool {
    profile.suppress_alerts_in_active_session
        && handles.workspace.peek().active_session() == Some(id)
        && handles.controls.status.peek().activated
}

// PTYSession.m -shouldPostUserNotification
fn should_post_notification(handles: &Handles, id: SessionId, profile: &Profile) -> bool {
    if !profile.notifications || notifications_suppressed(handles, id, profile) {
        return false;
    }
    let workspace = handles.workspace.peek();
    workspace.tab_of(id) != Some(workspace.active) || !handles.controls.status.peek().activated
}

// PTYSession.m -screenPostUserNotification:rich:
fn post_escape_sequence_notification(
    handles: &Handles,
    id: SessionId,
    profile: &Profile,
    notification: Notification,
) {
    if !profile.notifications
        || !profile.notify_escape_sequences
        || notifications_suppressed(handles, id, profile)
    {
        return;
    }
    let (summary, body) = if notification.rich {
        let body = match notification.subtitle {
            Some(subtitle) => format!("{subtitle}\n{}", notification.message),
            None => notification.message,
        };
        (notification.title.unwrap_or_default(), body)
    } else {
        let (name, tab) = session_label(&handles.workspace.peek(), id);
        let message = notification.message;
        (
            tr!("escape-notification-title").to_string(),
            tr!(
                "escape-notification",
                name = name,
                tab = tab,
                message = message
            ),
        )
    };
    post_for_session(handles, id, summary, body);
}

// PTYTab.m -setLabelAttributesForActiveTab: and -setLabelAttributesForIdleTab
fn note_output(handles: &Handles, session: &Rc<Session>, profile: &Profile) {
    let id = session.id;
    let seen = {
        let workspace = handles.workspace.peek();
        workspace.tab_of(id) == Some(workspace.active)
    };
    let may_notify = *handles.broadcast.peek() == Broadcast::Off
        && should_post_notification(handles, id, profile);
    let mut activity = session.activity.get();
    if activity.output(Instant::now(), seen, may_notify) && profile.notify_new_output {
        let (name, tab) = session_label(&handles.workspace.peek(), id);
        post_for_session(
            handles,
            id,
            tr!("new-output-title").to_string(),
            tr!("new-output-notification", name = name, tab = tab),
        );
    }
    session.activity.set(activity);
    if !seen && !session.idle_armed.replace(true) {
        watch_idle(handles, session.clone());
    }
}

fn watch_idle(handles: &Handles, session: Rc<Session>) {
    let Some(at) = session.activity.get().idle_at() else {
        session.idle_armed.set(false);
        return;
    };
    let waiting = handles.clone();
    handles.after(at.saturating_duration_since(Instant::now()), move || {
        let now = Instant::now();
        if session.activity.get().idle_at().is_some_and(|at| at > now) {
            watch_idle(&waiting, session.clone());
            return;
        }
        session.idle_armed.set(false);
        let profile = waiting.defaults.peek().profile(&session.profile);
        let may_notify = should_post_notification(&waiting, session.id, &profile);
        let mut activity = session.activity.get();
        if activity.idle(now, may_notify) && profile.notify_idle {
            let (name, tab) = session_label(&waiting.workspace.peek(), session.id);
            post_for_session(
                &waiting,
                session.id,
                tr!("idle-title").to_string(),
                tr!("idle-notification", name = name, tab = tab),
            );
        }
        session.activity.set(activity);
    });
}

fn listen_reveals(handles: Handles) -> Reveals {
    let (sender, receiver) = channel();
    let timers = handles.timers.clone();
    let wake = timers.wakeup(move || {
        if handles.alive.get() {
            for (id, token) in receiver.try_iter() {
                reveal(&handles, id, token);
            }
        }
    });
    (sender, wake)
}

fn reveal(handles: &Handles, id: SessionId, token: Option<String>) {
    let mut workspace = handles.workspace;
    workspace.write().focus(id);
    if let Some(token) = token {
        handles.controls.present(token);
    }
}

// PTYSession.m -screenDidAddMark:alert:completion:
fn mark_set(handles: &Handles, session: &Session) {
    let defaults = handles.defaults.peek();
    let workspace = handles.workspace.peek();
    let offscreen =
        defaults.alert_on_offscreen_marks && workspace.tab_of(session.id) != Some(workspace.active);
    let mut extras = session.extras();
    if !std::mem::take(&mut extras.alert_on_next_mark) && !offscreen {
        return;
    }
    match defaults.mark_alert {
        MarkAlert::ModalAlert => {
            let mut confirm = handles.confirm;
            if confirm.peek().is_none() {
                confirm.set(Some(Confirm::MarkSet(session.id)));
            }
        }
        MarkAlert::PostNotification => {
            let (name, tab) = session_label(&workspace, session.id);
            post_for_session(
                handles,
                session.id,
                tr!("mark-set-title").to_string(),
                tr!("mark-set-notification", name = name, tab = tab),
            );
        }
        // The freedesktop names for iTerm2's success-sound and error-sound.
        MarkAlert::PlaySound => notifications::play_event_sound(
            if extras.last_status.is_none_or(|status| status == 0) {
                "complete"
            } else {
                "dialog-error"
            },
        ),
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
            session.extras().cursor_guide = profile.cursor_guide;
            if !profile.initial_text.is_empty() {
                session.write(format!("{}\r", profile.initial_text).into_bytes());
            }
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

fn save_arrangement(handles: &Handles, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    let tabs: Vec<TabLayout> = handles
        .workspace
        .peek()
        .tabs
        .iter()
        .filter_map(|tab| tab_layout(handles, tab))
        .collect();
    change_defaults(|defaults| {
        defaults.arrangements.retain(|saved| saved.name != name);
        defaults.arrangements.push(Arrangement {
            name: name.to_string(),
            tabs,
        });
    });
}

fn restore_arrangement(handles: &Handles, events: &Events, name: &str) -> bool {
    let Some(saved) = handles
        .defaults
        .peek()
        .arrangements
        .iter()
        .find(|saved| saved.name == name)
        .cloned()
    else {
        return false;
    };
    open_layouts(handles, events, saved.tabs)
}

fn open_layouts(handles: &Handles, events: &Events, tabs: Vec<TabLayout>) -> bool {
    let mut restored = false;
    for tab in tabs {
        restored |= open_layout(handles, events, tab);
    }
    restored
}

fn tab_layout(handles: &Handles, tab: &Tab) -> Option<TabLayout> {
    let sessions = handles.sessions.peek();
    let describe = |id| {
        let session = sessions.get(&id)?;
        Some((session.profile.clone(), session.working_directory()))
    };
    Some(TabLayout {
        title: tab.title.clone(),
        root: arrangement::capture(&tab.root, &describe)?,
    })
}

fn open_layout(handles: &Handles, events: &Events, tab: TabLayout) -> bool {
    let root = arrangement::build(&tab.root, &mut |profile, directory| {
        let mut launch = handles.launch(profile, None);
        if directory.is_some() {
            launch.directory = directory;
        }
        spawn(handles, events, launch)
    });
    let Some(root) = root else {
        return false;
    };
    let active = root.leaves()[0];
    let mut workspace = handles.workspace;
    workspace.write().restore(Tab {
        root,
        active,
        zoomed: false,
        title: tab.title,
    });
    true
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
        let mut pending = handles.confirm;
        pending.set(Some(Confirm::Close(closing)));
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
    let confirm = handles.confirm;
    handles.after(Duration::ZERO, move || {
        let mut confirm = confirm;
        confirm.set(None);
        then();
    });
}

fn confirm_alert(handles: &Handles, pending: Confirm) -> Alert {
    let (confirm, cancel) = (handles.clone(), handles.clone());
    let alert = match pending {
        Confirm::Close(_) => Alert::new(tr!("close-title"), tr!("close-message")),
        Confirm::RemoveAnnotation(..) => Alert::new(
            tr!("remove-annotation-title"),
            tr!("remove-annotation-message"),
        ),
        Confirm::MarkSet(id) => {
            let name = handles.workspace.peek().title(id).to_string();
            Alert::new(
                tr!("mark-alert-title"),
                tr!("mark-alert-message", name = name),
            )
        }
    };
    let (label, dismiss) = match pending {
        Confirm::Close(_) => (tr!("close-confirm"), tr!("close-cancel")),
        Confirm::RemoveAnnotation(..) => (tr!("remove-annotation-confirm"), tr!("close-cancel")),
        Confirm::MarkSet(_) => (tr!("reveal"), tr!("mark-alert-ok")),
    };
    alert
        .confirm(label, move |_| {
            let handles = confirm.clone();
            dismiss_later(&confirm, move || match pending {
                Confirm::Close(closing) => close_now(&handles, closing),
                Confirm::RemoveAnnotation(session, note) => {
                    if let Some(session) = handles.sessions.peek().get(&session) {
                        session.remove_annotation(note);
                    }
                }
                Confirm::MarkSet(id) => reveal(&handles, id, None),
            });
        })
        .cancel(dismiss, move |_| dismiss_later(&cancel, || {}))
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
    if let Some(index) = id
        .strip_prefix(menu::NAMED_MARK_PREFIX)
        .and_then(|index| index.parse().ok())
    {
        if let Some(session) = handles.active() {
            session.jump_to_named_mark(index);
        }
        return;
    }
    let defaults = handles.defaults.peek();
    let arrangement = |prefix| {
        id.strip_prefix(prefix)
            .and_then(|index| index.parse::<usize>().ok())
            .and_then(|index| defaults.arrangements.get(index))
            .map(|arrangement| arrangement.name.clone())
    };
    let (window, tabs) = (
        arrangement(menu::ARRANGEMENT_PREFIX),
        arrangement(menu::ARRANGEMENT_TABS_PREFIX),
    );
    drop(defaults);
    if let Some(name) = window {
        windows::open(&["--arrangement", &name]);
        return;
    }
    if let Some(name) = tabs {
        restore_arrangement(handles, events, &name);
        return;
    }
    if let Some(snippet) = id
        .strip_prefix(snippets::SNIPPET_PREFIX)
        .and_then(|index| index.parse::<usize>().ok())
        .and_then(|index| handles.defaults.peek().snippets.get(index).cloned())
    {
        send_snippet(handles, active.as_deref(), &snippet);
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
        // PseudoTerminal.m -duplicateSession: a new tab with only this session, in its folder.
        _ if id == menu::DUPLICATE_SESSION.id => {
            let directory = active
                .as_ref()
                .and_then(|session| session.working_directory());
            new_tab(handles, events, &handles.current_profile(), directory);
        }
        // PseudoTerminal.m -createDuplicateOfTab: the same panes, profiles and folders.
        _ if id == menu::DUPLICATE_TAB.id => {
            let workspace = handles.workspace.peek();
            let copy = workspace
                .active_tab()
                .and_then(|tab| tab_layout(handles, tab));
            drop(workspace);
            if let Some(copy) = copy {
                open_layout(handles, events, copy);
            }
        }
        _ if id == menu::FILTER.id => {
            let mut open = handles.filter.open;
            open.set(true);
        }
        _ if id == menu::SELECT_CURRENT_COMMAND.id => {
            if let Some(session) = &active {
                session.select_current_command();
            }
        }
        menu::ARRANGE_EVENLY => workspace.write().arrange_evenly(),
        // PseudoTerminal.m -duplicateWindow: the arrangement of every tab, without contents.
        _ if id == menu::DUPLICATE_WINDOW.id => {
            let tabs: Vec<TabLayout> = workspace
                .peek()
                .tabs
                .iter()
                .filter_map(|tab| tab_layout(handles, tab))
                .collect();
            let layout = Arrangement {
                name: String::new(),
                tabs,
            };
            match toml::to_string(&layout) {
                Ok(layout) => windows::open(&["--layout", &layout]),
                Err(error) => tracing::warn!(%error, "could not describe the window"),
            }
        }
        menu::WARN_MULTI_LINE_PASTE => change_defaults(|defaults| {
            defaults.paste_warnings.multi_line = !defaults.paste_warnings.multi_line
        }),
        menu::LIMIT_PASTE_WARNING => change_defaults(|defaults| {
            defaults.paste_warnings.only_at_prompt = !defaults.paste_warnings.only_at_prompt
        }),
        menu::WARN_ONE_LINE_PASTE => change_defaults(|defaults| {
            defaults.paste_warnings.one_line_at_prompt = !defaults.paste_warnings.one_line_at_prompt
        }),
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
        _ if id == menu::RECENT_DIRECTORIES.id => open_picker(handles, Picker::RecentDirectories),
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
        _ if id == menu::SET_NAMED_MARK.id => {
            if let Some(session) = active {
                handles
                    .draft
                    .clone()
                    .set(local_date_time(std::time::SystemTime::now()));
                open_sheet(handles, Sheet::NamedMark(session.id));
            }
        }
        _ if id == menu::SAVE_ARRANGEMENT.id => {
            handles.draft.clone().set(String::new());
            open_sheet(handles, Sheet::SaveArrangement);
        }
        _ if id == menu::SAVE_CONTENTS.id => {
            if let Some(session) = active {
                handles.draft.clone().set(String::new());
                open_sheet(handles, Sheet::SaveContents(session.id));
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
        _ if id == menu::ADVANCED_PASTE.id => {
            if let Some(text) = clipboard_text(handles.clipboard) {
                handles.advanced.open(text);
                open_sheet(handles, Sheet::AdvancedPaste);
            }
        }
        _ if id == menu::PASTE_SLOWLY.id => {
            if let Some(text) = clipboard_text(handles.clipboard) {
                paste_checked(handles, text, true);
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
        _ if id == menu::FIND_GLOBALLY.id => {
            let mut open = handles.global.open;
            open.set(true);
        }
        _ if id == menu::SELECT_MATCHES.id => {
            let pattern = handles.find.pattern(&handles.defaults.peek());
            if let Some((session, pattern)) = active.as_ref().zip(pattern) {
                session.select_matches(&pattern);
            }
        }
        _ if id == menu::CLEAR_FIND.id => {
            let mut query = handles.find.query;
            query.set(String::new());
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
        _ if id == menu::FIND_URLS.id => {
            if let Some(session) = &active {
                let found = session::visible_found(&session.term.lock(), hints::URL_PATTERN);
                show_hints(handles, session, found, HintAction::Open);
            }
        }
        _ if id == menu::SMART_SELECT_ALL.id => {
            if let Some(session) = &active {
                let found = session::visible_found(&session.term.lock(), &hints::smart_pattern());
                show_hints(handles, session, found, HintAction::Copy);
            }
        }
        _ if id == menu::PICK_RESULT.id => {
            let pattern = handles.find.pattern(&handles.defaults.peek());
            if let Some((session, pattern)) = active.as_ref().zip(pattern) {
                let found = session::visible_found(&session.term.lock(), &pattern);
                show_hints(handles, session, found, HintAction::Open);
            }
        }
        _ if id == menu::ADD_ANNOTATION.id => {
            if let Some(session) = &active {
                session.annotate_cursor();
            }
        }
        _ if id == menu::ANNOTATE_SELECTION.id => {
            if let Some(session) = &active {
                session.annotate_selection();
            }
        }
        menu::NEXT_ANNOTATION | menu::PREVIOUS_ANNOTATION => {
            if let Some(session) = &active {
                session.step_annotation(id == menu::NEXT_ANNOTATION);
            }
        }
        menu::SHOW_COMPOSER => {
            if let Some(session) = &active {
                session.composer.borrow_mut().toggle();
                let mut editing = handles.editing;
                editing.set(false);
            }
        }
        _ if id == menu::SHOW_ANNOTATIONS.id => {
            if let Some(session) = &active {
                session.toggle_annotations();
            }
        }
        menu::ALERT_ON_NEXT_MARK => {
            if let Some(session) = &active {
                let mut extras = session.extras();
                extras.alert_on_next_mark = !extras.alert_on_next_mark;
            }
        }
        menu::ALERT_OFFSCREEN_MARKS => change_defaults(|defaults| {
            defaults.alert_on_offscreen_marks = !defaults.alert_on_offscreen_marks
        }),
        menu::MARK_ALERT_MODAL => {
            change_defaults(|defaults| defaults.mark_alert = MarkAlert::ModalAlert)
        }
        menu::MARK_ALERT_NOTIFICATION => {
            change_defaults(|defaults| defaults.mark_alert = MarkAlert::PostNotification)
        }
        menu::MARK_ALERT_SOUND => {
            change_defaults(|defaults| defaults.mark_alert = MarkAlert::PlaySound)
        }
        _ if id == menu::CURSOR_GUIDE.id => {
            if let Some(session) = &active {
                let mut extras = session.extras();
                extras.cursor_guide = !extras.cursor_guide;
            }
        }
        _ if id == menu::RESET_CHARSET.id => {
            if let Some(session) = &active {
                session.reset_character_set();
            }
        }
        _ if id == menu::TOOLBELT.id => {
            let mut shown = handles.toolbelt.shown;
            let next = !*shown.peek();
            shown.set(next);
        }
        _ if id == menu::INSTANT_REPLAY.id => {
            if let Some(session) = &active {
                let frames = session.extras().replay.len();
                if frames > 0 {
                    handles.replay.clone().set(Some((session.id, frames - 1)));
                }
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
        _ if id == menu::CLEAR_TO_SELECTION.id => {
            if let Some(session) = &active
                && let Some(line) = session.selection_start()
            {
                session.clear_from(line);
            }
        }
        _ if id == menu::CLEAR_TO_LAST_MARK.id => {
            if let Some(session) = &active {
                session.clear_to_last_mark();
            }
        }
        _ if id == menu::CLEAR_REPLAY.id => {
            if let Some(session) = &active {
                session.extras().replay.clear();
                handles.replay.clone().set(None);
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
