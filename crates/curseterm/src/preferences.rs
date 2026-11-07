//! Settings window laid out like iTerm2's.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::tokens::{HIGHLIGHT_COLORS, SETTINGS_WINDOW_SIZE, TERMINAL, TRIGGER_EDITOR};
use alacritty_terminal::vte::ansi::Rgb;
use barrier_kit::app::{use_defaults, use_menu_action};
use barrier_kit::choices::RadioGroup;
use barrier_kit::compact::{IconButton, Stepper};
use barrier_kit::controls::{
    Button, Checkbox, ScrollView, SegmentedControl, Slider, Switch, TextField,
};
use barrier_kit::platform::{WindowControls, WindowOptions};
use barrier_kit::popup_button::PopUpButton;
use barrier_kit::style::{Palette, text};
use barrier_kit::symbol::Symbol;
use barrier_kit::theme::color;
use barrier_kit::titlebar::Titlebar;
use barrier_kit::views::form::{Form, Row, Section};
use barrier_kit::views::table::Table;
use barrier_kit::views::tabs::TabView;
use freya_core::integration::AppComponent;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Content, Direction, Gaps, Size};
use vow_design::{CAPTION, HEADLINE, SIDEBAR, SPACE_1, SPACE_3, WINDOW_MIN_SIZE};

use crate::advanced_pane::AdvancedPane;
use crate::archive;
use crate::arrangements_pane::ArrangementsPane;
use crate::auxiliary::on_action;
use crate::config::{
    APP_ID, AltKey, ClosePrompt, CursorType, Defaults, Profile, SCROLLBACK_LIMIT, SessionEnd,
    Theme, Trigger, TriggerAction, WindowStyle, WorkingDirectory, change_defaults,
};
use crate::itermcolors;
use crate::key_bindings::NamedAction;
use crate::key_bindings_pane::{KeyBindingsEditor, Owner};
use crate::localize::tr;
use crate::menu;
use crate::semantic_history::Action as SemanticAction;
use crate::smart_selection::{self, Precision, Rule};
use crate::snippets_pane::SnippetsPane;
use crate::triggers;

thread_local! {
    static INTERNED: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new());
}

// BK-020: pop-up items take `&'static str`; scheme names are interned once.
pub fn intern(text: &str) -> &'static str {
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
                            general_pane(&defaults),
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
                            Symbol::SlidersVertical,
                            tr!("pane-keys"),
                            rect()
                                .width(Size::fill())
                                .height(Size::fill())
                                .padding(Gaps::new_all(SPACE_3))
                                .spacing(SPACE_1)
                                .child(text(
                                    tr!("key-bindings"),
                                    HEADLINE,
                                    Palette::current().label,
                                ))
                                .child(bindings_editor(&defaults, Owner::Global)),
                        )
                        .tab(
                            Symbol::Tv,
                            tr!("pane-arrangements"),
                            ArrangementsPane {
                                defaults: defaults.clone(),
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
                            ShortcutsPane {
                                defaults: defaults.clone(),
                            },
                        )
                        .tab(
                            Symbol::SlidersVertical,
                            tr!("pane-advanced"),
                            AdvancedPane {
                                advanced: defaults.advanced.clone(),
                            },
                        )
                        .selected(*tab.read())
                        .on_change(move |index| tab.set(index)),
                ),
            )
    }
}

// GeneralPreferencesViewController.m: Startup › Open default window arrangement, and Selection.
fn general_pane(defaults: &Defaults) -> Rect {
    let switch = |label, on, apply: fn(&mut Defaults, bool)| {
        Row::new(label).control(
            Switch::new(label)
                .on(on)
                .on_toggle(move |on: bool| change_defaults(|defaults| apply(defaults, on))),
        )
    };
    rect().padding(Gaps::new_all(SPACE_3)).child(
        Form::new()
            .section(Section::titled(tr!("section-startup")).row(switch(
                tr!("open-arrangement-at-startup"),
                defaults.open_arrangement_at_startup,
                |defaults, on| defaults.open_arrangement_at_startup = on,
            )))
            .section(Section::titled(tr!("section-selection")).row(switch(
                tr!("copy-on-select"),
                defaults.copy_on_select,
                |defaults, on| defaults.copy_on_select = on,
            )))
            // PreferencePanel.xib General › Magic.
            .section(Section::titled(tr!("section-magic")).row(switch(
                tr!("maximize-throughput"),
                defaults.maximize_throughput,
                |defaults, on| defaults.maximize_throughput = on,
            ))),
    )
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
            7 => None,
            chosen => Some(match chosen {
                0 => Form::new().section(general_section(defaults, index, &profile)),
                1 => Form::new().section(colors_section(defaults, index, &profile)),
                2 => Form::new().section(text_section(index, &profile)),
                3 => Form::new().section(window_section(index, &profile)),
                4 => Form::new()
                    .section(terminal_section(index, &profile))
                    .section(notifications_section(index, &profile)),
                5 => Form::new().section(session_section(index, &profile)),
                _ => Form::new().section(keys_section(index, &profile)),
            }),
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
                                .selected(vec![index])
                                .on_select(move |rows: Vec<usize>| {
                                    if let Some(&row) = rows.first() {
                                        selected.set(row);
                                    }
                                }),
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
                    .child(
                        ScrollView::new().height(Size::flex(1.0)).child(
                            rect()
                                .width(Size::fill())
                                .direction(Direction::Vertical)
                                .spacing(SPACE_3)
                                .maybe_child(form)
                                .maybe_child((*section.read() == 6).then(|| {
                                    rect()
                                        .width(Size::fill())
                                        .direction(Direction::Vertical)
                                        .spacing(SPACE_1)
                                        .child(text(
                                            tr!("key-mappings"),
                                            HEADLINE,
                                            Palette::current().label,
                                        ))
                                        .child(bindings_editor(defaults, Owner::Profile(index)))
                                }))
                                .maybe_child(
                                    (*section.read() == 7).then(|| triggers_table(index, &profile)),
                                )
                                .maybe_child(
                                    (*section.read() == 7)
                                        .then(|| smart_selection_table(index, &profile)),
                                )
                                .maybe_child(
                                    (*section.read() == 7)
                                        .then(|| profile_switching_list(index, &profile)),
                                )
                                .maybe_child(
                                    (*section.read() == 7)
                                        .then(|| semantic_history_row(index, &profile)),
                                ),
                        ),
                    ),
            )
    }
}

#[derive(PartialEq)]
struct ShortcutsPane {
    defaults: Defaults,
}

// iTermShortcutsViewController: the Actions and Snippets tabs.
impl Component for ShortcutsPane {
    fn render(&self) -> impl IntoElement {
        let mut shown = use_state(|| 0usize);
        let content = if *shown.read() == 0 {
            rect()
                .width(Size::fill())
                .padding(Gaps::new_all(SPACE_3))
                .child(bindings_editor(&self.defaults, Owner::Actions))
                .into_element()
        } else {
            SnippetsPane {
                defaults: self.defaults.clone(),
            }
            .into_element()
        };
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .direction(Direction::Vertical)
            .content(Content::flex())
            .child(
                rect()
                    .width(Size::fill())
                    .padding(Gaps::new(SPACE_3, SPACE_3, 0.0, SPACE_3))
                    .main_align(Alignment::Center)
                    .child(
                        SegmentedControl::new(tr!("pane-shortcuts"))
                            .segment(tr!("shortcuts-actions"))
                            .segment(tr!("shortcuts-snippets"))
                            .selected(*shown.read())
                            .on_change(move |choice| shown.set(choice)),
                    ),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .child(content),
            )
    }
}

fn bindings_editor(defaults: &Defaults, owner: Owner) -> KeyBindingsEditor {
    let entries: Vec<(String, Vec<String>)> = defaults
        .profiles()
        .into_iter()
        .map(|profile| (profile.name, profile.tags))
        .collect();
    let profiles: Vec<String> = entries.iter().map(|(name, _)| name.clone()).collect();
    KeyBindingsEditor {
        owner,
        bindings: match owner {
            Owner::Global => defaults.key_bindings.clone(),
            Owner::Profile(index) => defaults
                .profiles()
                .get(index)
                .map(|profile| profile.key_mappings.clone())
                .unwrap_or_default(),
            Owner::Actions => defaults.actions.iter().map(NamedAction::binding).collect(),
        },
        menu_items: menu::action_items(&entries),
        profiles,
        snippets: defaults
            .snippets
            .iter()
            .map(|snippet| snippet.title.clone())
            .collect(),
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
            Row::new(tr!("profile-tags"))
                .detail(tr!("profile-tags-detail"))
                .control(ProfileText {
                    index,
                    field: Field::Tags,
                    value: profile.tags.join(", "),
                }),
        )
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
    let styles = [
        (WindowStyle::Normal, tr!("window-style-normal")),
        (WindowStyle::FullScreen, tr!("window-style-full-screen")),
        (WindowStyle::Maximized, tr!("window-style-maximized")),
        (WindowStyle::NoTitleBar, tr!("window-style-no-title-bar")),
    ];
    Section::titled(tr!("section-window"))
        .row(Row::new(tr!("window-style")).control(profile_choice(
            index,
            tr!("window-style"),
            styles,
            profile.window_style,
            |profile, style| profile.window_style = style,
        )))
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
        // ProfilesSessionPreferencesViewController.m KEY_ARCHIVE and KEY_ARCHIVEDIR.
        .row(
            Row::new(tr!("archive-on-close")).control(
                Switch::new(tr!("archive-on-close"))
                    .on(profile.archive_on_close)
                    .on_toggle(move |on: bool| {
                        edit(index, |profile| profile.archive_on_close = on)
                    }),
            ),
        )
        .row(Row::new(tr!("archive-folder")).control(ProfileText {
            index,
            field: Field::ArchiveFolder,
            value: profile.archive_directory.clone(),
        }))
}

fn keys_section(index: usize, profile: &Profile) -> Section {
    let keys = AltKey::ALL.map(|key| {
        let name = match key {
            AltKey::Normal => tr!("alt-key-normal"),
            AltKey::Escape => tr!("alt-key-escape"),
        };
        (key, name)
    });
    let switch = |label: &'static str, on: bool, set: fn(&mut Profile, bool)| {
        Row::new(label).control(
            Switch::new(label)
                .on(on)
                .on_toggle(move |on: bool| edit(index, |profile| set(profile, on))),
        )
    };
    Section::titled(tr!("section-keys"))
        .row(
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
        .row(switch(
            tr!("report-keys-csi-u"),
            profile.report_keys_csi_u,
            |profile, on| profile.report_keys_csi_u = on,
        ))
        .row(switch(
            tr!("apps-change-key-reporting"),
            profile.apps_change_key_reporting,
            |profile, on| profile.apps_change_key_reporting = on,
        ))
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

// TriggerController.m: the profile's trigger table, one row per trigger, and + to add one.
fn triggers_table(index: usize, profile: &Profile) -> Rect {
    let rows = profile
        .triggers
        .iter()
        .enumerate()
        .map(|(position, trigger)| trigger_editor(index, position, trigger).into_element());
    rect()
        .width(Size::fill())
        .direction(Direction::Vertical)
        .spacing(SPACE_1)
        .child(text(
            tr!("section-triggers"),
            HEADLINE,
            Palette::current().label,
        ))
        .children(rows)
        .child(
            IconButton::new(Symbol::Plus, tr!("trigger-add")).on_press(move |_| {
                edit(index, |profile| profile.triggers.push(Trigger::default()))
            }),
        )
}

// TriggerController.m: the table's Enabled, Regular Expression, Action, Parameters and Instant
// columns in one row.
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
    let parameter = match trigger.action {
        TriggerAction::Highlight | TriggerAction::HighlightLine => {
            Some(highlight_colors(index, position, &trigger.parameter))
        }
        action if action.takes_parameter() => Some(rect().width(Size::fill()).child(ProfileText {
            index,
            field: Field::TriggerParameter(position, action),
            value: trigger.parameter.clone(),
        })),
        _ => None,
    };
    let column = |width| rect().width(Size::px(width));
    rect()
        .width(Size::fill())
        .direction(Direction::Horizontal)
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(SPACE_1)
        .child(
            Checkbox::new(tr!("trigger-enabled"))
                .checked(trigger.enabled)
                .on_toggle(move |on| edit_trigger(index, position, |trigger| trigger.enabled = on)),
        )
        .child(text(Field::TriggerRegex(position), &trigger.regex))
        .child(
            column(TRIGGER_EDITOR.action_width).child(
                TriggerAction::ALL
                    .iter()
                    .fold(PopUpButton::new(tr!("trigger-action")), |popup, action| {
                        popup.item(trigger_action_title(*action))
                    })
                    .selected(action)
                    .on_change(move |choice: usize| {
                        edit_trigger(index, position, |trigger| {
                            trigger.action = TriggerAction::ALL[choice];
                        })
                    }),
            ),
        )
        .child(column(TRIGGER_EDITOR.parameter_width).maybe_child(parameter))
        .child(
            Checkbox::new(tr!("trigger-instant"))
                .checked(trigger.instant)
                .on_toggle(move |on| edit_trigger(index, position, |trigger| trigger.instant = on)),
        )
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

// SmartSelectionController.m: Notes, Regular Expression and Precision, with + and − and
// Load Defaults.
fn smart_selection_table(index: usize, profile: &Profile) -> Rect {
    let rows = profile
        .smart_selection_rules
        .iter()
        .enumerate()
        .map(|(position, rule)| {
            let text = |field, value: &str| {
                rect().width(Size::flex(1.0)).child(ProfileText {
                    index,
                    field,
                    value: value.to_string(),
                })
            };
            let precision = Precision::ALL
                .iter()
                .position(|precision| *precision == rule.precision)
                .unwrap_or_default();
            rect()
                .width(Size::fill())
                .direction(Direction::Horizontal)
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(SPACE_1)
                .child(text(Field::SmartNotes(position), &rule.notes))
                .child(text(Field::SmartRegex(position), &rule.regex))
                .child(
                    rect().width(Size::px(TRIGGER_EDITOR.action_width)).child(
                        [
                            tr!("precision-very-low"),
                            tr!("precision-low"),
                            tr!("precision-normal"),
                            tr!("precision-high"),
                            tr!("precision-very-high"),
                        ]
                        .into_iter()
                        .fold(
                            PopUpButton::new(tr!("smart-selection-precision")),
                            |popup, name| popup.item(name),
                        )
                        .selected(precision)
                        .on_change(move |choice: usize| {
                            edit(index, |profile| {
                                if let Some(rule) = profile.smart_selection_rules.get_mut(position)
                                {
                                    rule.precision = Precision::ALL[choice];
                                }
                            })
                        }),
                    ),
                )
                .child(
                    IconButton::new(Symbol::Minus, tr!("smart-selection-remove")).on_press(
                        move |_| {
                            edit(index, |profile| {
                                if position < profile.smart_selection_rules.len() {
                                    profile.smart_selection_rules.remove(position);
                                }
                            })
                        },
                    ),
                )
                .into_element()
        });
    rect()
        .width(Size::fill())
        .direction(Direction::Vertical)
        .spacing(SPACE_1)
        .child(text(
            tr!("section-smart-selection"),
            HEADLINE,
            Palette::current().label,
        ))
        .children(rows)
        .child(
            rect()
                .direction(Direction::Horizontal)
                .cross_align(Alignment::Center)
                .spacing(SPACE_1)
                .child(
                    IconButton::new(Symbol::Plus, tr!("smart-selection-add")).on_press(move |_| {
                        edit(index, |profile| {
                            profile.smart_selection_rules.push(Rule::default())
                        })
                    }),
                )
                .child(
                    Button::new(tr!("smart-selection-defaults")).on_press(move |_| {
                        edit(index, |profile| {
                            profile.smart_selection_rules = smart_selection::default_rules()
                        })
                    }),
                ),
        )
}

// iTermSemanticHistoryPrefsController.m: what ⌘-click on a file does, and its text with \1 path,
// \2 line, \3 text before, \4 text after and \5 working directory.
fn semantic_history_row(index: usize, profile: &Profile) -> Rect {
    let history = &profile.semantic_history;
    let chosen = SemanticAction::ALL
        .iter()
        .position(|action| *action == history.action)
        .unwrap_or_default();
    let action = [
        tr!("semantic-default-app"),
        tr!("semantic-url"),
        tr!("semantic-command"),
        tr!("semantic-raw-command"),
        tr!("semantic-coprocess"),
        tr!("semantic-send-text"),
    ]
    .into_iter()
    .fold(
        PopUpButton::new(tr!("section-semantic-history")),
        |popup, name| popup.item(name),
    )
    .selected(chosen)
    .on_change(move |choice: usize| {
        edit(index, |profile| {
            profile.semantic_history.action = SemanticAction::ALL[choice];
        })
    });
    let parameter = (history.action != SemanticAction::DefaultApp).then(|| {
        rect().width(Size::flex(1.0)).child(ProfileText {
            index,
            field: Field::SemanticText(history.action),
            value: history.text.clone(),
        })
    });
    rect()
        .width(Size::fill())
        .direction(Direction::Vertical)
        .spacing(SPACE_1)
        .child(text(
            tr!("section-semantic-history"),
            HEADLINE,
            Palette::current().label,
        ))
        .child(text(
            tr!("semantic-history-help"),
            CAPTION,
            Palette::current().secondary_label,
        ))
        .child(
            rect()
                .width(Size::fill())
                .direction(Direction::Horizontal)
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(SPACE_1)
                .child(action)
                .maybe_child(parameter),
        )
}

// ProfilesAdvancedPreferencesViewController.m Automatic Profile Switching: one rule per row,
// [user@][host][:path][&job], with + and −.
fn profile_switching_list(index: usize, profile: &Profile) -> Rect {
    let rows = profile
        .bound_hosts
        .iter()
        .enumerate()
        .map(|(position, rule)| {
            rect()
                .width(Size::fill())
                .direction(Direction::Horizontal)
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(SPACE_1)
                .child(rect().width(Size::flex(1.0)).child(ProfileText {
                    index,
                    field: Field::BoundHost(position),
                    value: rule.clone(),
                }))
                .child(
                    IconButton::new(Symbol::Minus, tr!("profile-switching-remove")).on_press(
                        move |_| {
                            edit(index, |profile| {
                                if position < profile.bound_hosts.len() {
                                    profile.bound_hosts.remove(position);
                                }
                            })
                        },
                    ),
                )
                .into_element()
        });
    rect()
        .width(Size::fill())
        .direction(Direction::Vertical)
        .spacing(SPACE_1)
        .child(text(
            tr!("section-profile-switching"),
            HEADLINE,
            Palette::current().label,
        ))
        .child(text(
            tr!("profile-switching-help"),
            CAPTION,
            Palette::current().secondary_label,
        ))
        .children(rows)
        .child(
            IconButton::new(Symbol::Plus, tr!("profile-switching-add"))
                .on_press(move |_| edit(index, |profile| profile.bound_hosts.push(String::new()))),
        )
}

fn edit_trigger(index: usize, position: usize, change: impl FnOnce(&mut Trigger)) {
    edit(index, |profile| {
        if let Some(trigger) = profile.triggers.get_mut(position) {
            change(trigger);
        }
    });
}

pub fn trigger_action_title(action: TriggerAction) -> &'static str {
    match action {
        TriggerAction::Annotate => tr!("trigger-annotate"),
        TriggerAction::HighlightLine => tr!("trigger-highlight-line"),
        TriggerAction::Highlight => tr!("trigger-highlight"),
        TriggerAction::Hyperlink => tr!("trigger-hyperlink"),
        TriggerAction::PasswordManager => tr!("trigger-password-manager"),
        TriggerAction::Notification => tr!("trigger-notification"),
        TriggerAction::PromptDetected => tr!("trigger-prompt-detected"),
        TriggerAction::ReportDirectory => tr!("trigger-report-directory"),
        TriggerAction::Bell => tr!("trigger-bell"),
        TriggerAction::RunCommand => tr!("trigger-run-command"),
        TriggerAction::Coprocess => tr!("trigger-coprocess"),
        TriggerAction::SendText => tr!("trigger-send-text"),
        TriggerAction::Mark => tr!("trigger-mark"),
        TriggerAction::SetTitle => tr!("trigger-set-title"),
        TriggerAction::Alert => tr!("trigger-alert"),
        TriggerAction::Stop => tr!("trigger-stop"),
    }
}

// Each trigger's -triggerOptionalParameterPlaceholderWithInterpolation:.
pub fn trigger_placeholder(action: TriggerAction) -> &'static str {
    match action {
        TriggerAction::Alert => tr!("trigger-alert-placeholder"),
        TriggerAction::Annotate => tr!("trigger-annotate-placeholder"),
        TriggerAction::Notification => tr!("trigger-notification-placeholder"),
        TriggerAction::RunCommand => tr!("trigger-run-command-placeholder"),
        TriggerAction::Coprocess => tr!("trigger-coprocess-placeholder"),
        TriggerAction::ReportDirectory => tr!("trigger-report-directory-placeholder"),
        TriggerAction::SetTitle => tr!("trigger-set-title-placeholder"),
        TriggerAction::Hyperlink => tr!("trigger-hyperlink-placeholder"),
        TriggerAction::PasswordManager => tr!("account-name"),
        _ => tr!("trigger-send-text-placeholder"),
    }
}

// HighlightTrigger.m -paramIsTwoColorWells: the text and the background, each optional.
fn highlight_colors(index: usize, position: usize, parameter: &str) -> Rect {
    let (text, background) = triggers::colors(parameter);
    let popup = |label, chosen: Option<Rgb>, background_side: bool| {
        let selected = chosen.map_or(0, |chosen| {
            HIGHLIGHT_COLORS
                .iter()
                .position(|swatch| triggers::rgb(*swatch) == chosen)
                .map_or(0, |found| found + 1)
        });
        HIGHLIGHT_COLORS
            .iter()
            .enumerate()
            .fold(
                PopUpButton::new(label).item(tr!("highlight-unchanged")),
                |popup, (swatch, _)| popup.item(highlight_color_name(swatch)),
            )
            .selected(selected)
            .on_change(move |choice: usize| {
                let picked = choice
                    .checked_sub(1)
                    .map(|swatch| triggers::rgb(HIGHLIGHT_COLORS[swatch]));
                edit_trigger(index, position, |trigger| {
                    let (text, background) = triggers::colors(&trigger.parameter);
                    trigger.parameter = if background_side {
                        triggers::colors_parameter(text, picked)
                    } else {
                        triggers::colors_parameter(picked, background)
                    };
                });
            })
    };
    rect()
        .direction(Direction::Horizontal)
        .spacing(SPACE_1)
        .child(popup(tr!("highlight-text-color"), text, false))
        .child(popup(tr!("highlight-background-color"), background, true))
}

fn highlight_color_name(swatch: usize) -> &'static str {
    match swatch {
        0 => tr!("highlight-black"),
        1 => tr!("highlight-dark-gray"),
        2 => tr!("highlight-light-gray"),
        3 => tr!("highlight-white"),
        4 => tr!("highlight-gray"),
        5 => tr!("highlight-red"),
        6 => tr!("highlight-green"),
        7 => tr!("highlight-blue"),
        8 => tr!("highlight-cyan"),
        9 => tr!("highlight-yellow"),
        10 => tr!("highlight-magenta"),
        11 => tr!("highlight-orange"),
        12 => tr!("highlight-purple"),
        _ => tr!("highlight-brown"),
    }
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
    SmartNotes(usize),
    SmartRegex(usize),
    BoundHost(usize),
    Tags,
    SemanticText(SemanticAction),
    TriggerParameter(usize, TriggerAction),
    Scrollback,
    Columns,
    Rows,
    ArchiveFolder,
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
            Self::TriggerRegex(_) | Self::SmartRegex(_) => tr!("trigger-regex"),
            Self::SmartNotes(_) => tr!("smart-selection-notes"),
            Self::BoundHost(_) => tr!("profile-switching-placeholder"),
            Self::Tags => tr!("profile-tags-placeholder"),
            Self::SemanticText(SemanticAction::Url) => tr!("semantic-url-placeholder"),
            Self::SemanticText(SemanticAction::SendText) => tr!("semantic-text-placeholder"),
            Self::SemanticText(_) => tr!("semantic-command-placeholder"),
            Self::TriggerParameter(_, action) => trigger_placeholder(action),
            Self::Scrollback => tr!("scrollback-lines"),
            Self::Columns => tr!("window-columns"),
            Self::Rows => tr!("window-rows"),
            Self::ArchiveFolder => archive_folder_placeholder(),
        }
    }

    fn store(self, index: usize, value: &str) {
        let value = value.trim().to_string();
        if value.is_empty()
            && !matches!(
                self,
                Self::Command
                    | Self::InitialText
                    | Self::Badge
                    | Self::TriggerParameter(..)
                    | Self::SemanticText(_)
                    | Self::Tags
                    | Self::ArchiveFolder
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
                Self::ArchiveFolder => profile.archive_directory = value.clone(),
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
                Self::TriggerRegex(position) | Self::TriggerParameter(position, _) => {
                    if let Some(trigger) = profile.triggers.get_mut(position) {
                        match self {
                            Self::TriggerRegex(_) => trigger.regex = value.clone(),
                            _ => trigger.parameter = value.clone(),
                        }
                    }
                }
                Self::SemanticText(_) => profile.semantic_history.text = value.clone(),
                // ProfilesGeneralPreferencesViewController.m: tags are typed separated by commas.
                Self::Tags => {
                    profile.tags = value
                        .split(',')
                        .map(str::trim)
                        .filter(|tag| !tag.is_empty())
                        .map(str::to_string)
                        .collect()
                }
                Self::BoundHost(position) => {
                    if let Some(rule) = profile.bound_hosts.get_mut(position) {
                        *rule = value.clone();
                    }
                }
                Self::SmartNotes(position) | Self::SmartRegex(position) => {
                    if let Some(rule) = profile.smart_selection_rules.get_mut(position) {
                        match self {
                            Self::SmartNotes(_) => rule.notes = value.clone(),
                            _ => rule.regex = value.clone(),
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

// An empty folder means the default one (PTYSession.swift -saveArchive).
fn archive_folder_placeholder() -> &'static str {
    static FOLDER: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    FOLDER.get_or_init(|| {
        archive::default_directory()
            .map(|folder| folder.display().to_string())
            .unwrap_or_default()
    })
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
        let mut outcome = use_state(String::new);
        let index = self.index;
        let palette = Palette::current();
        rect()
            .width(Size::fill())
            .direction(Direction::Vertical)
            .spacing(SPACE_1)
            .child(Button::new(tr!("import-colors-choose")).on_press(move |_| {
                // iTermColorPresets.m -importColorPresetFromFile:: an open panel, one file.
                spawn(async move {
                    if let Some(path) =
                        barrier_kit::portal::open_files(tr!("import-colors"), false, false)
                            .await
                            .and_then(|paths| paths.into_iter().next())
                    {
                        outcome.set(import_into(index, &path));
                    }
                });
            }))
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
