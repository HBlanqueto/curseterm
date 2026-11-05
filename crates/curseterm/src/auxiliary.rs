//! «About» and Settings, each in its own process (BK-001).

use barrier_kit::adornments::avatar;
use barrier_kit::app::{load_defaults, save_defaults, use_menu_action};
use barrier_kit::compact::Stepper;
use barrier_kit::controls::TextField;
use barrier_kit::patterns::about::about_window;
use barrier_kit::patterns::preferences::{Pane, PaneGroup, PreferencesWindow};
use barrier_kit::platform::{RunError, WindowControls, WindowOptions, run_window};
use barrier_kit::popup_button::PopUpButton;
use barrier_kit::style::Palette;
use barrier_kit::symbol::Symbol;
use barrier_kit::theme::color;
use barrier_kit::titlebar::Titlebar;
use barrier_kit::views::form::{Form, Row, Section};
use freya_core::integration::AppComponent;
use freya_core::prelude::*;
use torin::prelude::{Direction, Size};
use vow_design::{ACCENT_SWATCHES, WINDOW_MIN_SIZE};

use crate::config::{APP_ID, Defaults, SCHEMES};
use crate::localize::tr;
use crate::menu;
use crate::windows::{self, KeepTerminalSize};

const ABOUT_SIZE: (u32, u32) = (320, 280);
const SETTINGS_SIZE: (u32, u32) = (680, 440);

pub fn run_about() -> Result<(), RunError> {
    let _keep = KeepTerminalSize::new();
    run_window(
        AppComponent::new(About),
        WindowOptions::new(tr!("about"), "curseterm")
            .app_id(APP_ID)
            .size(ABOUT_SIZE)
            .min_size(Some(ABOUT_SIZE))
            .max_size(Some(ABOUT_SIZE))
            .menu(menu::auxiliary_menu()),
    )
}

pub fn run_settings() -> Result<(), RunError> {
    let _keep = KeepTerminalSize::new();
    run_window(
        AppComponent::new(Settings),
        WindowOptions::new(tr!("settings-title"), "curseterm")
            .app_id(APP_ID)
            .size(SETTINGS_SIZE)
            .min_size(Some(WINDOW_MIN_SIZE))
            .menu(menu::auxiliary_menu()),
    )
}

fn on_action(id: &str, controls: &WindowControls) {
    match id {
        _ if id == menu::ABOUT.id => windows::open(Some("--about")),
        _ if id == menu::SETTINGS.id => windows::open(Some("--settings")),
        _ if id == menu::NEW_WINDOW.id => windows::open(None),
        _ if id == menu::CLOSE.id => controls.close(),
        _ if id == menu::QUIT.id => {
            windows::quit_others();
            controls.close();
        }
        _ => {}
    }
}

struct About;

impl App for About {
    fn render(&self) -> impl IntoElement {
        let controls = WindowControls::get();
        use_menu_action(move |id| on_action(id, &controls));
        let palette = Palette::current();
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .direction(Direction::Vertical)
            .background(color(palette.window_background))
            .child(Titlebar::new().close_only(true))
            .child(about_window(
                avatar("CT"),
                "CurseTerm",
                Some(tr!("about-version", version = env!("CARGO_PKG_VERSION"))),
                tr!("about-copyright"),
            ))
    }
}

struct Settings;

impl App for Settings {
    fn render(&self) -> impl IntoElement {
        let controls = WindowControls::get();
        use_menu_action(move |id| on_action(id, &controls));
        let defaults = barrier_kit::app::use_defaults::<Defaults>(APP_ID);
        let family = use_state(|| defaults.peek().font_family().to_string());
        use_side_effect(move || {
            let family = family.read().trim().to_string();
            if !family.is_empty() {
                save(|defaults| defaults.font_family = Some(family.clone()));
            }
        });
        let current = defaults.read().clone();
        let scheme_index = current
            .color_scheme
            .as_deref()
            .and_then(|name| SCHEMES.iter().position(|scheme| scheme.name == name))
            .map_or(0, |index| index + 1);
        let schemes = SCHEMES
            .iter()
            .fold(
                PopUpButton::new(tr!("scheme")).item(tr!("scheme-auto")),
                |button, scheme| button.item(scheme.name.as_str()),
            )
            .selected(scheme_index)
            .on_change(|index: usize| {
                save(|defaults| {
                    defaults.color_scheme = index
                        .checked_sub(1)
                        .map(|index| SCHEMES[index].name.clone());
                });
            });

        PreferencesWindow::new().group(
            PaneGroup::new(None)
                .pane(Pane::new(
                    "text",
                    tr!("pane-text"),
                    Symbol::SlidersVertical,
                    ACCENT_SWATCHES[0].1,
                    Form::new().section(
                        Section::titled(tr!("section-font"))
                            .row(
                                Row::new(tr!("font-family"))
                                    .control(TextField::new(family, tr!("font-family"))),
                            )
                            .row(
                                Row::new(tr!("font-size")).control(
                                    Stepper::new(tr!("font-size"), 8, 36)
                                        .value(i64::from(current.font_size()))
                                        .on_change(|size: i64| {
                                            save(|defaults| defaults.font_size = Some(size as u32));
                                        }),
                                ),
                            ),
                    ),
                ))
                .pane(Pane::new(
                    "colors",
                    tr!("pane-colors"),
                    Symbol::CircleHalf,
                    ACCENT_SWATCHES[1].1,
                    Form::new().section(
                        Section::titled(tr!("section-scheme"))
                            .row(Row::new(tr!("scheme")).control(schemes)),
                    ),
                )),
        )
    }
}

fn save(change: impl FnOnce(&mut Defaults)) {
    let mut defaults: Defaults = load_defaults(APP_ID);
    let before = defaults.clone();
    change(&mut defaults);
    if defaults != before
        && let Err(error) = save_defaults(APP_ID, &defaults)
    {
        tracing::warn!(%error, "could not save the settings");
    }
}
