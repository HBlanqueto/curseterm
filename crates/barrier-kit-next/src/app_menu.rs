//! Live state for entries of the window's global menu.

use barrier_kit::app_menu::MenuHandle;
use barrier_kit::menu::{CheckState, MenuItem};
use freya_core::prelude::*;

/// Applies `value` to the entry `id` whenever it changes between renders.
pub fn use_menu_update<T: PartialEq + Clone + 'static>(
    id: &'static str,
    value: T,
    apply: fn(&mut MenuItem, &T),
) {
    let handle = use_hook(try_consume_root_context::<MenuHandle>);
    let mut published = use_state(|| None::<T>);
    if published.peek().as_ref() == Some(&value) {
        return;
    }
    published.set(Some(value.clone()));
    if let Some(handle) = handle {
        handle.update(id, |item| apply(item, &value));
    }
}

pub fn use_menu_enabled(id: &'static str, enabled: bool) {
    use_menu_update(id, enabled, |item, enabled| {
        if let MenuItem::Action { enabled: shown, .. } | MenuItem::Submenu { enabled: shown, .. } =
            item
        {
            *shown = *enabled;
        }
    });
}

pub fn use_menu_checked(id: &'static str, checked: bool) {
    use_menu_update(id, checked, |item, checked| {
        if let MenuItem::Action { check, .. } = item {
            *check = CheckState::Checkbox(*checked);
        }
    });
}

pub fn use_submenu_items(id: &'static str, items: Vec<MenuItem>) {
    use_menu_update(id, items, |item, items| {
        if let MenuItem::Submenu { items: shown, .. } = item {
            *shown = items.clone();
        }
    });
}

pub fn submenu(id: &'static str, label: impl Into<String>, items: Vec<MenuItem>) -> MenuItem {
    MenuItem::Submenu {
        id: Some(id.to_string()),
        label: label.into(),
        enabled: true,
        items,
        icon: None,
    }
}
