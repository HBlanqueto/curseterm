//! Live state for entries of the window's global menu.

use barrier_kit::app_menu::MenuHandle;
use barrier_kit::menu::MenuItem;
use freya_core::prelude::*;

pub fn use_menu_enabled(id: &'static str, enabled: bool) {
    let handle = use_hook(try_consume_root_context::<MenuHandle>);
    let mut published = use_state(|| None::<bool>);
    if *published.peek() == Some(enabled) {
        return;
    }
    published.set(Some(enabled));
    if let Some(handle) = handle {
        handle.update(id, |item| {
            if let MenuItem::Action { enabled: shown, .. } = item {
                *shown = enabled;
            }
        });
    }
}
