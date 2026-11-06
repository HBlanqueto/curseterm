//! Settings window laid out like iTerm2's, in its own process (BK-001).

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use barrier_kit::app::{use_defaults, use_menu_action};
use barrier_kit::choices::RadioGroup;
use barrier_kit::compact::{IconButton, Stepper};
use barrier_kit::controls::{Button, ScrollView, SegmentedControl, Slider, Switch, TextField};
use barrier_kit::platform::{WindowControls, WindowOptions};
use barrier_kit::popup_button::PopUpButton;
use barrier_kit::style::{Palette, text};
use barrier_kit::symbol::Symbol;
use barrier_kit::theme::color;
use barrier_kit::titlebar::Titlebar;
use barrier_kit::views::form::{Form, Row, Section};
use barrier_kit::views::table::Table;
use barrier_kit::views::tabs::TabView;
use barrier_kit_next::tokens::{SETTINGS_WINDOW_SIZE, TERMINAL};
use freya_core::integration::AppComponent;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Content, Direction, Gaps, Size};
use vow_design::{CAPTION, SIDEBAR, SPACE_1, SPACE_3, WINDOW_MIN_SIZE};

use crate::auxiliary::on_action;
use crate::config::{
    APP_ID, AltKey, ClosePrompt, CursorType, Defaults, Profile, SCROLLBACK_LIMIT, SessionEnd,
    Theme, Trigger, TriggerAction, WorkingDirectory, change_defaults,
};
use crate::itermcolors;
use crate::localize::tr;
use crate::menu;
use crate::snippets_pane::SnippetsPane;

thread_local! {
    static INTERNED: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new());
}

// BK-020: pop-up items take `&'static str`; scheme names are interned once.
fn intern(text: &str) -> &'static str {
    INTERNED.with(|interned| {
        let mut interned = interned.borrow_mut();
        match interned.get(text) {
            Some(known) => known,
            None => {
                let leaked: &'static str = Box::leak(text.to_owned().into_boxed_str());
                interned.insert(leaked);
                leaked
            }
        }
    })
}

pub fn window(profile: Option<String>) -> (AppComponent, WindowOptions) {
    let options = WindowOptions::new(tr!("settings-title"), "curseterm")
        .app_id(APP_ID)
        .size(SETTINGS_WINDOW_SIZE)
        .min_size(Some(WINDOW_MIN_SIZE))
        .menu(menu::auxiliary_menu());
    (AppComponent::new(Settings { profile }), options)
}

struct Settings {
    profile: Option<String>,
}

impl App for Settings {
    fn render(&self) -> impl IntoElement {
        crate::windows::use_registered_window();
        let controls = WindowControls::get();
        use_menu_action(move |id| on_action(id, &controls));
        let defaults = use_defaults::<Defaults>(APP_ID).read().clone();
        let mut tab = use_state(|| 2usize);
        let palette = Palette::current();
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .direction(Direction::Vertical)
            .content(Content::flex())
            .background(color(palette.window_background))
            .child(Titlebar::new().title(tr!("settings-title")))
            .child(
                rect().width(Size::fill()).height(Size::flex(1.0)).child(
                    TabView::new(tr!("settings-title"))
                        .tab(
                            Symbol::SlidersVertical,
                            tr!("pane-general"),
                            switch_pane(
                                tr!("section-selection"),
                                tr!("copy-on-select"),
                                defaults.copy_on_select,
                                |defaults, on| defaults.copy_on_select = on,
                            ),
                        )
                        .tab(
                            Symbol::CircleHalf,
                            tr!("pane-appearance"),
                            appearance_pane(&defaults),
                        )
                        .tab(
                            Symbol::ListBullet,
                            tr!("pane-profiles"),
                            ProfilesPane {
                                defaults: defaults.clone(),
                                opened: self.profile.clone(),
                            },
                        )
                        .tab(
                            Symbol::SquareGrid,
                            tr!("pane-pointer"),
                            switch_pane(
                                tr!("section-pointer-misc"),
                                tr!("focus-follows-mouse"),
                                defaults.focus_follows_mouse,
                                |defaults, on| defaults.focus_follows_mouse = on,
                            ),
                        )
                        .tab(
                            Symbol::Bookmark,
                            tr!("pane-shortcuts"),
                            SnippetsPane {
                                defaults: defaults.clone(),
                            },
                        )
                        .selected(*tab.read())
                        .on_change(move |index| tab.set(index)),
                ),
            )
    }
}

fn switch_pane(
    section: &'static str,
    label: &'static str,
    on: bool,
    apply: fn(&mut Defaults, bool),
) -> Rect {
    rect().padding(Gaps::new_all(SPACE_3)).child(
        Form::new().section(
            Section::titled(section).row(
                Row::new(label).control(
                    Switch::new(label)
                        .on(on)
                        .on_toggle(move |on: bool| change_defaults(|defaults| apply(defaults, on))),
                ),
            ),
        ),
    )
}

// PreferencePanel.xib Appearance › General › Theme and Tabs.
fn appearance_pane(defaults: &Defaults) -> Rect {
    let names = [
        tr!("theme-regular"),
        tr!("theme-minimal"),
        tr!("theme-compact"),
        tr!("theme-light"),
        tr!("theme-dark"),
        tr!("theme-light-high-contrast"),
        tr!("theme-dark-high-contrast"),
    ];
    let selected = Theme::ALL
        .iter()
        .position(|theme| *theme == defaults.theme)
        .unwrap_or_default();
    let theme = names
        .iter()
        .fold(PopUpButton::new(tr!("theme")), |button, name| {
            button.item(name)
        })
        .selected(selected)
        .on_change(|position: usize| {
            change_defaults(|defaults| defaults.theme = Theme::ALL[position])
        });
    let show_tab_bar = defaults.show_tab_bar;
    rect().padding(Gaps::new_all(SPACE_3)).child(
        Form::new()
            .section(
                Section::titled(tr!("section-general")).row(Row::new(tr!("theme")).control(theme)),
            )
            .section(
                Section::titled(tr!("section-tabs")).row(
                    Row::new(tr!("show-tab-bar-always")).control(
                        Switch::new(tr!("show-tab-bar-always"))
                            .on(show_tab_bar)
                            .on_toggle(|on: bool| {
                                change_defaults(|defaults| defaults.show_tab_bar = on)
                            }),
                    ),
                ),
            ),
    )
}

#[derive(PartialEq)]
struct ProfilesPane {
    defaults: Defaults,
    opened: Option<String>,
}

impl Component for ProfilesPane {
    fn render(&self) -> impl IntoElement {
        let defaults = &self.defaults;
        let profiles = defaults.profiles();
        let opened = profiles
            .iter()
            .position(|profile| Some(&profile.name) == self.opened.as_ref())
            .unwrap_or_default();
        let mut selected = use_state(|| opened);
        let mut section = use_state(|| 0usize);
        let index = (*selected.read()).min(profiles.len() - 1);
        let profile = profiles[index].clone();
        let default_name = defaults.default_profile_name();
        let rows = profiles
            .iter()
            .map(|profile| {
                let name = profile.name.clone();
                vec![if name == default_name {
                    tr!("profile-default-row", name = name)
                } else {
                    name
                }]
            })
            .collect();
        let several = profiles.len() > 1;
        let form = match *section.read() {
            0 => Form::new().section(general_section(defaults, index, &profile)),
            1 => Form::new().section(colors_section(defaults, index, &profile)),
            2 => Form::new().section(text_section(index, &profile)),
            3 => Form::new().section(window_section(index, &profile)),
            4 => Form::new()
                .section(terminal_section(index, &profile))
                .section(notifications_section(index, &profile)),
            5 => Form::new().section(session_section(index, &profile)),
            6 => Form::new().section(keys_section(index, &profile)),
            _ => Form::new().section(triggers_section(index, &profile)),
        };
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .direction(Direction::Horizontal)
            .content(Content::flex())
            .padding(Gaps::new_all(SPACE_3))
            .spacing(SPACE_3)
            .child(
                rect()
                    .width(Size::px(SIDEBAR.width as f32))
                    .height(Size::fill())
                    .direction(Direction::Vertical)
                    .content(Content::flex())
                    .spacing(SPACE_1)
                    .child(
                        rect().height(Size::flex(1.0)).child(
                            Table::new(tr!("pane-profiles"), vec![tr!("profile-name")])
                                .rows(rows)
                                .selected(Some(index))
                                .on_select(move |row| selected.set(row)),
                        ),
                    )
                    .child(
                        rect()
                            .direction(Direction::Horizontal)
                            .spacing(SPACE_1)
                            .child(IconButton::new(Symbol::Plus, tr!("profile-new")).on_press(
                                move |_| {
                                    change_defaults(|defaults| {
                                        defaults.ensure_profiles();
                                        let name =
                                            defaults.unique_profile_name(tr!("profile-new-name"));
                                        defaults.profiles.push(Profile {
                                            name,
                                            ..Profile::default()
                                        });
                                        selected.set(defaults.profiles.len() - 1);
                                    });
                                },
                            ))
                            .child(
                                IconButton::new(Symbol::Minus, tr!("profile-delete"))
                                    .disabled(!several)
                                    .on_press(move |_| {
                                        change_defaults(|defaults| {
                                            defaults.ensure_profiles();
                                            if defaults.profiles.len() > 1 {
                                                defaults.profiles.remove(index);
                                            }
                                        });
                                        selected.set(index.saturating_sub(1));
                                    }),
                            ),
                    ),
            )
            .child(
                rect()
                    .width(Size::flex(1.0))
                    .height(Size::fill())
                    .direction(Direction::Vertical)
                    .content(Content::flex())
                    .spacing(SPACE_3)
                    .child(
                        rect()
                            .width(Size::fill())
                            .main_align(Alignment::Center)
                            .child(
                                SegmentedControl::new(tr!("profile-sections"))
                                    .segment(tr!("section-general"))
                                    .segment(tr!("section-colors"))
                                    .segment(tr!("section-text"))
                                    .segment(tr!("section-window"))
                                    .segment(tr!("section-terminal"))
                                    .segment(tr!("section-session"))
                                    .segment(tr!("section-keys"))
                                    .segment(tr!("section-advanced"))
                                    .selected(*section.read())
                                    .on_change(move |choice| section.set(choice)),
                            ),
                    )
                    .child(ScrollView::new().height(Size::flex(1.0)).child(form)),
            )
    }
}

fn edit(index: usize, change: impl FnOnce(&mut Profile)) {
    change_defaults(|defaults| {
        defaults.ensure_profiles();
        if let Some(profile) = defaults.profiles.get_mut(index) {
            change(profile);
        }
    });
}

fn general_section(defaults: &Defaults, index: usize, profile: &Profile) -> Section {
    let is_default = defaults.default_profile_name() == profile.name;
    let (directory_kind, path) = match &profile.working_directory {
        WorkingDirectory::Home => (0, String::new()),
        WorkingDirectory::Previous => (1, String::new()),
        WorkingDirectory::Fixed(path) => (2, path.display().to_string()),
    };
    Section::titled(tr!("section-general"))
        .row(Row::new(tr!("profile-name")).control(ProfileText {
            index,
            field: Field::Name,
            value: profile.name.clone(),
        }))
        .row(
            Row::new(tr!("profile-command"))
                .detail(tr!("profile-command-detail"))
                .control(ProfileText {
                    index,
                    field: Field::Command,
                    value: profile.command.clone().unwrap_or_default(),
                }),
        )
        .row(
            Row::new(tr!("profile-initial-text"))
                .detail(tr!("profile-initial-text-detail"))
                .control(ProfileText {
                    index,
                    field: Field::InitialText,
                    value: profile.initial_text.clone(),
                }),
        )
        .row(
            Row::new(tr!("profile-badge"))
                .detail(tr!("profile-badge-detail"))
                .control(ProfileText {
                    index,
                    field: Field::Badge,
                    value: profile.badge.clone(),
                }),
        )
        .row(Row::new(tr!("profile-directory")).control(directory_choices(index, directory_kind)))
        .row(Row::new(tr!("directory-path")).control(ProfileText {
            index,
            field: Field::Directory,
            value: path,
        }))
        .row(
            Row::new(tr!("profile-default")).control(
                Switch::new(tr!("profile-default"))
                    .on(is_default)
                    .disabled(is_default)
                    .on_toggle({
                        let name = profile.name.clone();
                        move |_: bool| {
                            let name = name.clone();
                            change_defaults(|defaults| defaults.default_profile = Some(name));
                        }
                    }),
            ),
        )
        .row(
            Row::new(tr!("profile-duplicate")).control(Button::new(tr!("duplicate")).on_press({
                let profile = profile.clone();
                move |_| {
                    let profile = profile.clone();
                    change_defaults(|defaults| {
                        defaults.ensure_profiles();
                        let name = defaults.unique_profile_name(&profile.name);
                        defaults.profiles.push(Profile { name, ..profile });
                    });
                }
            })),
        )
}

// BK-021: RadioGroup is always horizontal; one single-option group per row stacks them like iTerm2.
fn directory_choices(index: usize, chosen: usize) -> Rect {
    [
        tr!("directory-home"),
        tr!("directory-previous"),
        tr!("directory-fixed"),
    ]
    .into_iter()
    .enumerate()
    .fold(
        rect().direction(Direction::Vertical).spacing(SPACE_1),
        |column, (kind, label)| {
            column.child(
                RadioGroup::new(label)
                    .option(label)
                    .selected(if kind == chosen { 0 } else { 1 })
                    .on_change(move |_: usize| {
                        edit(index, |profile| {
                            profile.working_directory = match kind {
                                0 => WorkingDirectory::Home,
                                1 => WorkingDirectory::Previous,
                                _ => WorkingDirectory::Fixed(
                                    std::env::var_os("HOME")
                                        .map(PathBuf::from)
                                        .unwrap_or_default(),
                                ),
                            };
                        });
                    }),
            )
        },
    )
}

fn colors_section(defaults: &Defaults, index: usize, profile: &Profile) -> Section {
    let schemes: Vec<&'static str> = defaults
        .schemes()
        .map(|scheme| intern(&scheme.name))
        .collect();
    let chosen = profile
        .color_scheme
        .as_deref()
        .and_then(|name| schemes.iter().position(|scheme| *scheme == name))
        .map_or(0, |position| position + 1);
    let scheme_button = schemes
        .iter()
        .fold(
            PopUpButton::new(tr!("scheme")).item(tr!("scheme-auto")),
            |button, name| button.item(name),
        )
        .selected(chosen)
        .on_change(move |choice: usize| {
            let name = choice
                .checked_sub(1)
                .map(|position| schemes[position].to_string());
            edit(index, |profile| profile.color_scheme = name);
        });
    Section::titled(tr!("section-colors"))
        .row(Row::new(tr!("scheme")).control(scheme_button))
        .row(
            Row::new(tr!("import-colors"))
                .detail(tr!("import-colors-detail"))
                .control(ImportColors { index }),
        )
        .row(
            Row::new(tr!("cursor-guide-row"))
                .detail(tr!("cursor-guide-detail"))
                .control(
                    Switch::new(tr!("cursor-guide-row"))
                        .on(profile.cursor_guide)
                        .on_toggle(move |on: bool| {
                            edit(index, |profile| profile.cursor_guide = on)
                        }),
                ),
        )
}

fn text_section(index: usize, profile: &Profile) -> Section {
    Section::titled(tr!("section-text"))
        .row(Row::new(tr!("font-family")).control(ProfileText {
            index,
            field: Field::Family,
            value: profile.font_family.clone(),
        }))
        .row(
            Row::new(tr!("font-size")).control(
                Stepper::new(
                    tr!("font-size"),
                    i64::from(TERMINAL.font_sizes.0),
                    i64::from(TERMINAL.font_sizes.1),
                )
                .value(i64::from(profile.font_size))
                .on_change(move |size: i64| edit(index, |profile| profile.font_size = size as u32)),
            ),
        )
        .row(Row::new(tr!("cursor-type")).control(profile_choice(
            index,
            tr!("cursor-type"),
            CursorType::ALL.map(|cursor| {
                let name = match cursor {
                    CursorType::Box => tr!("cursor-box"),
                    CursorType::VerticalBar => tr!("cursor-vertical-bar"),
                    CursorType::Underline => tr!("cursor-underline"),
                };
                (cursor, name)
            }),
            profile.cursor,
            |profile, cursor| profile.cursor = cursor,
        )))
        .row(
            Row::new(tr!("cursor-blinking")).control(
                Switch::new(tr!("cursor-blinking"))
                    .on(profile.blinking_cursor)
                    .on_toggle(move |on: bool| edit(index, |profile| profile.blinking_cursor = on)),
            ),
        )
}

fn profile_choice<T: Copy + PartialEq + 'static, const N: usize>(
    index: usize,
    label: &'static str,
    choices: [(T, &'static str); N],
    current: T,
    apply: fn(&mut Profile, T),
) -> PopUpButton {
    let selected = choices
        .iter()
        .position(|(choice, _)| *choice == current)
        .unwrap_or_default();
    choices
        .iter()
        .fold(PopUpButton::new(label), |button, (_, name)| {
            button.item(name)
        })
        .selected(selected)
        .on_change(move |position: usize| {
            let choice = choices[position].0;
            edit(index, |profile| apply(profile, choice));
        })
}

fn window_section(index: usize, profile: &Profile) -> Section {
    Section::titled(tr!("section-window"))
        .row(
            Row::new(tr!("window-columns"))
                .detail(tr!("window-size-detail"))
                .control(ProfileText {
                    index,
                    field: Field::Columns,
                    value: profile.columns.to_string(),
                }),
        )
        .row(Row::new(tr!("window-rows")).control(ProfileText {
            index,
            field: Field::Rows,
            value: profile.rows.to_string(),
        }))
        .row(
            Row::new(tr!("window-transparency"))
                .detail(tr!("window-transparency-detail"))
                .control(
                    Slider::stepped(TRANSPARENCY_STEPS)
                        .value(usize::from(profile.transparency / TRANSPARENCY_STEP))
                        .on_change(move |step: usize| {
                            change_defaults(|defaults| {
                                defaults.ensure_profiles();
                                if let Some(profile) = defaults.profiles.get_mut(index) {
                                    profile.transparency = step as u8 * TRANSPARENCY_STEP;
                                }
                            });
                        }),
                ),
        )
}

const TRANSPARENCY_STEP: u8 = 10;
const TRANSPARENCY_STEPS: usize = 10;

fn session_section(index: usize, profile: &Profile) -> Section {
    let endings = SessionEnd::ALL.map(|ending| {
        let name = match ending {
            SessionEnd::Close => tr!("session-end-close"),
            SessionEnd::Restart => tr!("session-end-restart"),
            SessionEnd::KeepOpen => tr!("session-end-keep"),
        };
        (ending, name)
    });
    let prompts = ClosePrompt::ALL.map(|prompt| {
        let name = match prompt {
            ClosePrompt::IfJobs => tr!("close-prompt-jobs"),
            ClosePrompt::Always => tr!("close-prompt-always"),
            ClosePrompt::Never => tr!("close-prompt-never"),
        };
        (prompt, name)
    });
    Section::titled(tr!("section-session"))
        .row(Row::new(tr!("session-end")).control(profile_choice(
            index,
            tr!("session-end"),
            endings,
            profile.when_session_ends,
            |profile, ending| profile.when_session_ends = ending,
        )))
        .row(Row::new(tr!("close-prompt")).control(profile_choice(
            index,
            tr!("close-prompt"),
            prompts,
            profile.prompt_before_closing,
            |profile, prompt| profile.prompt_before_closing = prompt,
        )))
        .row(
            Row::new(tr!("status-bar"))
                .detail(tr!("status-bar-detail"))
                .control(
                    Switch::new(tr!("status-bar"))
                        .on(profile.status_bar)
                        .on_toggle(move |on: bool| edit(index, |profile| profile.status_bar = on)),
                ),
        )
}

fn keys_section(index: usize, profile: &Profile) -> Section {
    let keys = AltKey::ALL.map(|key| {
        let name = match key {
            AltKey::Normal => tr!("alt-key-normal"),
            AltKey::Escape => tr!("alt-key-escape"),
        };
        (key, name)
    });
    Section::titled(tr!("section-keys")).row(
        Row::new(tr!("alt-key"))
            .detail(tr!("alt-key-detail"))
            .control(profile_choice(
                index,
                tr!("alt-key"),
                keys,
                profile.alt_key,
                |profile, key| profile.alt_key = key,
            )),
    )
}

fn terminal_section(index: usize, profile: &Profile) -> Section {
    Section::titled(tr!("section-terminal"))
        .row(
            Row::new(tr!("scrollback-lines"))
                .detail(intern(&tr!(
                    "scrollback-lines-detail",
                    limit = u64::from(SCROLLBACK_LIMIT)
                )))
                .control(ProfileText {
                    index,
                    field: Field::Scrollback,
                    value: profile.scrollback_lines.to_string(),
                }),
        )
        .row(
            Row::new(tr!("flash-visual-bell")).control(
                Switch::new(tr!("flash-visual-bell"))
                    .on(profile.flash_visual_bell)
                    .on_toggle(move |on: bool| {
                        edit(index, |profile| profile.flash_visual_bell = on)
                    }),
            ),
        )
        .row(
            Row::new(tr!("bell-icon-in-tabs")).control(
                Switch::new(tr!("bell-icon-in-tabs"))
                    .on(profile.bell_icon_in_tabs)
                    .on_toggle(move |on: bool| {
                        edit(index, |profile| profile.bell_icon_in_tabs = on)
                    }),
            ),
        )
}

// ProfilesTerminalPreferencesViewController.m: Notification Center alerts and Filter Alerts.
fn notifications_section(index: usize, profile: &Profile) -> Section {
    let switch = |label: &'static str, on: bool, set: fn(&mut Profile, bool)| {
        Row::new(label).control(
            Switch::new(label)
                .on(on)
                .on_toggle(move |on: bool| edit(index, |profile| set(profile, on))),
        )
    };
    Section::titled(tr!("section-notifications"))
        .row(switch(
            tr!("send-notifications"),
            profile.notifications,
            |profile, on| profile.notifications = on,
        ))
        .row(switch(
            tr!("notify-bell"),
            profile.notify_bell,
            |profile, on| profile.notify_bell = on,
        ))
        .row(switch(
            tr!("notify-idle"),
            profile.notify_idle,
            |profile, on| profile.notify_idle = on,
        ))
        .row(switch(
            tr!("notify-new-output"),
            profile.notify_new_output,
            |profile, on| profile.notify_new_output = on,
        ))
        .row(switch(
            tr!("notify-session-ended"),
            profile.notify_session_ended,
            |profile, on| profile.notify_session_ended = on,
        ))
        .row(switch(
            tr!("notify-escape-sequences"),
            profile.notify_escape_sequences,
            |profile, on| profile.notify_escape_sequences = on,
        ))
        .row(switch(
            tr!("suppress-alerts-in-active-session"),
            profile.suppress_alerts_in_active_session,
            |profile, on| profile.suppress_alerts_in_active_session = on,
        ))
}

fn triggers_section(index: usize, profile: &Profile) -> Section {
    let rows = profile
        .triggers
        .iter()
        .enumerate()
        .map(|(position, trigger)| {
            let number = position as u64 + 1;
            Row::new(intern(&tr!("trigger-number", number = number)))
                .control(trigger_editor(index, position, trigger))
        });
    rows.fold(Section::titled(tr!("section-triggers")), Section::row)
        .row(Row::new(tr!("trigger-add")).control(
            IconButton::new(Symbol::Plus, tr!("trigger-add")).on_press(move |_| {
                edit(index, |profile| profile.triggers.push(Trigger::default()))
            }),
        ))
}

fn trigger_editor(index: usize, position: usize, trigger: &Trigger) -> Rect {
    let action = TriggerAction::ALL
        .iter()
        .position(|action| *action == trigger.action)
        .unwrap_or_default();
    let text = |field, value: &str| {
        rect().width(Size::flex(1.0)).child(ProfileText {
            index,
            field,
            value: value.to_string(),
        })
    };
    rect()
        .width(Size::fill())
        .direction(Direction::Horizontal)
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(SPACE_1)
        .child(text(Field::TriggerRegex(position), &trigger.regex))
        .child(
            PopUpButton::new(tr!("trigger-action"))
                .item(tr!("trigger-highlight"))
                .item(tr!("trigger-bell"))
                .item(tr!("trigger-send-text"))
                .selected(action)
                .on_change(move |choice: usize| {
                    edit(index, |profile| {
                        if let Some(trigger) = profile.triggers.get_mut(position) {
                            trigger.action = TriggerAction::ALL[choice];
                        }
                    })
                }),
        )
        .child(text(Field::TriggerParameter(position), &trigger.parameter))
        .child(
            IconButton::new(Symbol::Minus, tr!("trigger-remove")).on_press(move |_| {
                edit(index, |profile| {
                    if position < profile.triggers.len() {
                        profile.triggers.remove(position);
                    }
                })
            }),
        )
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Name,
    Command,
    InitialText,
    Badge,
    Directory,
    Family,
    TriggerRegex(usize),
    TriggerParameter(usize),
    Scrollback,
    Columns,
    Rows,
}

impl Field {
    fn is_numeric(self) -> bool {
        matches!(self, Self::Scrollback | Self::Columns | Self::Rows)
    }

    fn placeholder(self) -> &'static str {
        match self {
            Self::Name => tr!("profile-name"),
            Self::Command => tr!("profile-command-placeholder"),
            Self::InitialText => tr!("profile-initial-text-placeholder"),
            Self::Badge => tr!("profile-badge-placeholder"),
            Self::Directory => tr!("directory-path"),
            Self::Family => tr!("font-family"),
            Self::TriggerRegex(_) => tr!("trigger-regex"),
            Self::TriggerParameter(_) => tr!("trigger-parameter"),
            Self::Scrollback => tr!("scrollback-lines"),
            Self::Columns => tr!("window-columns"),
            Self::Rows => tr!("window-rows"),
        }
    }

    fn store(self, index: usize, value: &str) {
        let value = value.trim().to_string();
        if value.is_empty()
            && !matches!(
                self,
                Self::Command | Self::InitialText | Self::Badge | Self::TriggerParameter(_)
            )
        {
            return;
        }
        change_defaults(|defaults| {
            defaults.ensure_profiles();
            let old = defaults
                .profiles
                .get(index)
                .map(|profile| profile.name.clone());
            let Some(profile) = defaults.profiles.get_mut(index) else {
                return;
            };
            match self {
                Self::Name => profile.name = value.clone(),
                Self::Command => profile.command = (!value.is_empty()).then(|| value.clone()),
                Self::InitialText => profile.initial_text = value.clone(),
                Self::Badge => profile.badge = value.clone(),
                Self::Directory => {
                    profile.working_directory = WorkingDirectory::Fixed(value.clone().into())
                }
                Self::Family => profile.font_family = value.clone(),
                Self::Columns | Self::Rows => {
                    if let Ok(cells) = value.parse::<u16>().map(|cells| cells.max(1)) {
                        match self {
                            Self::Columns => profile.columns = cells,
                            _ => profile.rows = cells,
                        }
                    }
                }
                Self::Scrollback => {
                    if let Ok(lines) = value.parse::<u32>() {
                        profile.scrollback_lines = lines.min(SCROLLBACK_LIMIT);
                    }
                }
                Self::TriggerRegex(position) | Self::TriggerParameter(position) => {
                    if let Some(trigger) = profile.triggers.get_mut(position) {
                        match self {
                            Self::TriggerRegex(_) => trigger.regex = value.clone(),
                            _ => trigger.parameter = value.clone(),
                        }
                    }
                }
            }
            if self == Self::Name && defaults.default_profile == old {
                defaults.default_profile = Some(value);
            }
        });
    }
}

#[derive(PartialEq)]
struct ProfileText {
    index: usize,
    field: Field,
    value: String,
}

impl Component for ProfileText {
    fn render(&self) -> impl IntoElement {
        let mut draft = use_state(|| self.value.clone());
        let mut saved = use_state(|| self.value.clone());
        if *saved.peek() != self.value {
            saved.set(self.value.clone());
            draft.set(self.value.clone());
        }
        let (index, field) = (self.index, self.field);
        use_side_effect(move || {
            let value = draft.read().clone();
            if field.is_numeric() && !value.chars().all(|c| c.is_ascii_digit()) {
                draft.set(value.chars().filter(char::is_ascii_digit).collect());
            } else if value != *saved.peek() {
                field.store(index, &value);
            }
        });
        TextField::new(draft, field.placeholder())
    }
}

#[derive(PartialEq)]
struct ImportColors {
    index: usize,
}

impl Component for ImportColors {
    fn render(&self) -> impl IntoElement {
        let path = use_state(String::new);
        let mut outcome = use_state(String::new);
        let index = self.index;
        let palette = Palette::current();
        rect()
            .width(Size::fill())
            .direction(Direction::Vertical)
            .spacing(SPACE_1)
            .child(
                rect()
                    .width(Size::fill())
                    .direction(Direction::Horizontal)
                    .content(Content::flex())
                    .spacing(SPACE_1)
                    .child(
                        TextField::new(path, tr!("import-colors-placeholder"))
                            .width(Size::flex(1.0)),
                    )
                    .child(Button::new(tr!("import")).on_press(move |_| {
                        outcome.set(import_into(index, Path::new(path.peek().trim())));
                    })),
            )
            .maybe_child(
                (!outcome.read().is_empty())
                    .then(|| text(outcome.read().clone(), CAPTION, palette.secondary_label)),
            )
    }
}

fn import_into(index: usize, path: &Path) -> String {
    match itermcolors::import(path) {
        Ok(scheme) => {
            let name = scheme.name.clone();
            change_defaults(|defaults| {
                defaults.schemes.retain(|known| known.name != scheme.name);
                defaults.schemes.push(scheme);
                defaults.ensure_profiles();
                if let Some(profile) = defaults.profiles.get_mut(index) {
                    profile.color_scheme = Some(name.clone());
                }
            });
            tr!("import-done", scheme = name)
        }
        Err(error) => tr!("import-failed", error = error.to_string()),
    }
}
