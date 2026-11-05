//! «About» in its own process (BK-001) and the menu of the auxiliary windows.

use barrier_kit::adornments::avatar;
use barrier_kit::app::use_menu_action;
use barrier_kit::patterns::about::about_window;
use barrier_kit::platform::{RunError, WindowControls, WindowOptions, run_window};
use barrier_kit::style::Palette;
use barrier_kit::theme::color;
use barrier_kit::titlebar::Titlebar;
use freya_core::integration::AppComponent;
use freya_core::prelude::*;
use torin::prelude::{Direction, Size};

use crate::config::APP_ID;
use crate::localize::tr;
use crate::menu;
use crate::windows::{self, KeepTerminalSize};

const ABOUT_SIZE: (u32, u32) = (320, 280);

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

pub(crate) fn on_action(id: &str, controls: &WindowControls) {
    match id {
        _ if id == menu::ABOUT.id => windows::open(&["--about"]),
        _ if id == menu::SETTINGS.id => windows::open(&["--settings"]),
        _ if id == menu::NEW_WINDOW.id => windows::open(&[]),
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
