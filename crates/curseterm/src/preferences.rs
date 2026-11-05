//! Settings window with iTerm2's layout: toolbar tabs on top and, under
//! Profiles, the profile list beside the profile editor's sections. It runs in
//! its own process (BK-001) and writes the app's `.domain`.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use barrier_kit::app::{use_defaults, use_menu_action};
use barrier_kit::choices::RadioGroup;
use barrier_kit::compact::{IconButton, Stepper};
use barrier_kit::controls::{Button, ScrollView, SegmentedControl, Switch, TextField};
use barrier_kit::platform::{RunError, WindowControls, WindowOptions, run_window};
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
use vow_design::{CAPTION, SIDEBAR, SPACE_1, SPACE_3, WINDOW_MIN_SIZE};

use crate::auxiliary::on_action;
use crate::config::{APP_ID, Defaults, Profile, WorkingDirectory, change_defaults};
use crate::itermcolors;
use crate::localize::tr;
use crate::menu;
use crate::windows::KeepTerminalSize;

const SIZE: (u32, u32) = (860, 560);
const FONT_SIZES: (i64, i64) = (6, 72);
const DEFAULT_MARK: &str = "★ ";

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

pub fn run() -> Result<(), RunError> {
    let _keep = KeepTerminalSize::new();
    run_window(
        AppComponent::new(Settings),
        WindowOptions::new(tr!("settings-title"), "curseterm")
            .app_id(APP_ID)
            .size(SIZE)
            .min_size(Some(WINDOW_MIN_SIZE))
            .menu(menu::auxiliary_menu()),
    )
}

struct Settings;

impl App for Settings {
    fn render(&self) -> impl IntoElement {
        let controls = WindowControls::get();
        use_menu_action(move |id| on_action(id, &controls));
        let defaults = use_defaults::<Defaults>(APP_ID).read().clone();
        let mut tab = use_state(|| 1usize);
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
                            Symbol::CircleHalf,
                            tr!("pane-appearance"),
                            appearance(&defaults),
                        )
                        .tab(
                            Symbol::ListBullet,
                            tr!("pane-profiles"),
                            ProfilesPane {
                                defaults: defaults.clone(),
                            },
                        )
                        .selected(*tab.read())
                        .on_change(move |index| tab.set(index)),
                ),
            )
    }
}

fn appearance(defaults: &Defaults) -> Rect {
    rect().padding(Gaps::new_all(SPACE_3)).child(
        Form::new().section(
            Section::titled(tr!("section-tabs")).row(
                Row::new(tr!("show-tab-bar-always")).control(
                    Switch::new(tr!("show-tab-bar-always"))
                        .on(defaults.show_tab_bar)
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
}

impl Component for ProfilesPane {
    fn render(&self) -> impl IntoElement {
        let mut selected = use_state(|| 0usize);
        let mut section = use_state(|| 0usize);
        let defaults = &self.defaults;
        let profiles = defaults.profiles();
        let index = (*selected.read()).min(profiles.len() - 1);
        let profile = profiles[index].clone();
        let default_name = defaults.default_profile_name();
        let rows = profiles
            .iter()
            .map(|profile| {
                let mark = if profile.name == default_name {
                    DEFAULT_MARK
                } else {
                    ""
                };
                vec![format!("{mark}{}", profile.name)]
            })
            .collect();
        let several = profiles.len() > 1;
        let form = match *section.read() {
            0 => general_section(defaults, index, &profile),
            1 => colors_section(defaults, index, &profile),
            _ => text_section(index, &profile),
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
                                    .selected(*section.read())
                                    .on_change(move |choice| section.set(choice)),
                            ),
                    )
                    .child(
                        ScrollView::new()
                            .height(Size::flex(1.0))
                            .child(Form::new().section(form)),
                    ),
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
                Stepper::new(tr!("font-size"), FONT_SIZES.0, FONT_SIZES.1)
                    .value(i64::from(profile.font_size))
                    .on_change(move |size: i64| {
                        edit(index, |profile| profile.font_size = size as u32)
                    }),
            ),
        )
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Name,
    Command,
    Directory,
    Family,
}

impl Field {
    fn placeholder(self) -> &'static str {
        match self {
            Self::Name => tr!("profile-name"),
            Self::Command => tr!("profile-command-placeholder"),
            Self::Directory => tr!("directory-path"),
            Self::Family => tr!("font-family"),
        }
    }

    fn store(self, index: usize, value: &str) {
        let value = value.trim().to_string();
        if value.is_empty() && self != Self::Command {
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
                Self::Directory => {
                    profile.working_directory = WorkingDirectory::Fixed(value.clone().into())
                }
                Self::Family => profile.font_family = value.clone(),
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
            if value != *saved.peek() {
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
