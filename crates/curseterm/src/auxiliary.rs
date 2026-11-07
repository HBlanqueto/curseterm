//! About window and the menu of the auxiliary windows.

use crate::tokens::ABOUT_WINDOW_SIZE;
use barrier_kit::adornments::avatar;
use barrier_kit::app::use_menu_action;
use barrier_kit::patterns::about::about_window;
use barrier_kit::platform::{WindowControls, WindowOptions};
use barrier_kit::style::Palette;
use barrier_kit::theme::color;
use barrier_kit::titlebar::Titlebar;
use freya_core::integration::AppComponent;
use freya_core::prelude::*;
use torin::prelude::{Direction, Size};

use crate::config::{APP_ID, APP_NAME};
use crate::localize::tr;
use crate::menu;
use crate::windows;

pub fn about() -> (AppComponent, WindowOptions) {
    let options = WindowOptions::new(tr!("about"), "curseterm")
        .app_id(APP_ID)
        .size(ABOUT_WINDOW_SIZE)
        .min_size(Some(ABOUT_WINDOW_SIZE))
        .max_size(Some(ABOUT_WINDOW_SIZE))
        .menu(menu::auxiliary_menu());
    (AppComponent::new(About), options)
}

pub(crate) fn on_action(id: &str, controls: &WindowControls) {
    match id {
        _ if id == menu::ABOUT.id => windows::open(&["--about"]),
        _ if id == menu::SETTINGS.id => windows::open(&["--settings"]),
        _ if id == menu::NEW_WINDOW.id => windows::open(&[]),
        _ if id == menu::CLOSE.id => controls.close(),
        _ if id == menu::QUIT.id => windows::quit(),
        _ => {}
    }
}

struct About;

impl App for About {
    fn render(&self) -> impl IntoElement {
        windows::use_registered_window();
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
                APP_NAME,
                Some(tr!("about-version", version = env!("CARGO_PKG_VERSION"))),
                tr!("about-copyright"),
            ))
    }
}
