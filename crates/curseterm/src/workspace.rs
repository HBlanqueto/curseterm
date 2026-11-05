use std::collections::HashMap;

use crate::panes::{Axis, Pane, Toward};
use crate::session::SessionId;

pub const DEFAULT_TITLE: &str = "CurseTerm";

#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    pub root: Pane,
    pub active: SessionId,
    pub zoomed: bool,
}

#[derive(Default)]
pub struct Workspace {
    pub tabs: Vec<Tab>,
    pub active: usize,
    titles: HashMap<SessionId, String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Closed {
    Pane,
    Tab,
    Window,
}

impl Workspace {
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active)
    }

    pub fn active_session(&self) -> Option<SessionId> {
        self.active_tab().map(|tab| tab.active)
    }

    pub fn title(&self, id: SessionId) -> &str {
        self.titles.get(&id).map_or(DEFAULT_TITLE, String::as_str)
    }

    pub fn set_title(&mut self, id: SessionId, title: Option<String>) {
        match title {
            Some(title) => self.titles.insert(id, title),
            None => self.titles.remove(&id),
        };
    }

    pub fn tab_titles(&self) -> Vec<String> {
        self.tabs
            .iter()
            .map(|tab| self.title(tab.active).to_string())
            .collect()
    }

    pub fn open_tab(&mut self, id: SessionId) {
        self.tabs.push(Tab {
            root: Pane::Leaf(id),
            active: id,
            zoomed: false,
        });
        self.active = self.tabs.len() - 1;
    }

    pub fn split(&mut self, id: SessionId, axis: Axis) -> bool {
        let Some(tab) = self.active_tab_mut() else {
            return false;
        };
        let split = tab.root.split(tab.active, id, axis);
        if split {
            tab.active = id;
            tab.zoomed = false;
        }
        split
    }

    pub fn close(&mut self, id: SessionId) -> Option<Closed> {
        let index = self.tab_of(id)?;
        self.titles.remove(&id);
        let tab = &mut self.tabs[index];
        if tab.root == Pane::Leaf(id) {
            self.remove_tab(index);
            return Some(if self.tabs.is_empty() {
                Closed::Window
            } else {
                Closed::Tab
            });
        }
        tab.root.remove(id);
        if tab.active == id {
            tab.active = tab.root.leaves()[0];
        }
        tab.zoomed &= tab.root.is_split();
        Some(Closed::Pane)
    }

    pub fn replace(&mut self, old: SessionId, new: SessionId) {
        self.titles.remove(&old);
        for tab in &mut self.tabs {
            if tab.root.replace(old, new) && tab.active == old {
                tab.active = new;
            }
        }
    }

    pub fn close_tab(&mut self, index: usize) -> Vec<SessionId> {
        let Some(tab) = self.tabs.get(index) else {
            return Vec::new();
        };
        let sessions = tab.root.leaves();
        for id in &sessions {
            self.titles.remove(id);
        }
        self.remove_tab(index);
        sessions
    }

    fn remove_tab(&mut self, index: usize) {
        self.tabs.remove(index);
        if self.active > index || self.active == self.tabs.len() {
            self.active = self.active.saturating_sub(1);
        }
    }

    fn tab_of(&self, id: SessionId) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.root.leaves().contains(&id))
    }

    pub fn focus(&mut self, id: SessionId) {
        if let Some(index) = self.tab_of(id) {
            self.active = index;
            self.tabs[index].active = id;
        }
    }

    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    pub fn step_tab(&mut self, step: isize) {
        if !self.tabs.is_empty() {
            self.active =
                (self.active as isize + step).rem_euclid(self.tabs.len() as isize) as usize;
        }
    }

    pub fn move_tab(&mut self, step: isize) {
        let target = self.active as isize + step;
        if (0..self.tabs.len() as isize).contains(&target) {
            self.tabs.swap(self.active, target as usize);
            self.active = target as usize;
        }
    }

    pub fn focus_toward(&mut self, toward: Toward) {
        if let Some(tab) = self.active_tab_mut()
            && let Some(next) = tab.root.neighbor(tab.active, toward)
        {
            tab.active = next;
        }
    }

    pub fn cycle_pane(&mut self, step: isize) {
        if let Some(tab) = self.active_tab_mut() {
            tab.active = tab.root.cycle(tab.active, step);
        }
    }

    pub fn move_divider(&mut self, toward: Toward, amount: f32) {
        if let Some(tab) = self.active_tab_mut() {
            tab.root.move_divider(tab.active, toward, amount);
        }
    }

    pub fn toggle_zoom(&mut self) {
        if let Some(tab) = self.active_tab_mut() {
            tab.zoomed = !tab.zoomed && tab.root.is_split();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_panes_collapses_the_tab_then_the_window() {
        let mut workspace = Workspace::default();
        workspace.open_tab(1);
        workspace.split(2, Axis::Row);
        workspace.open_tab(3);
        assert_eq!(workspace.close(2), Some(Closed::Pane));
        assert_eq!(workspace.tabs[0].active, 1);
        assert_eq!(workspace.close(3), Some(Closed::Tab));
        assert_eq!(workspace.active, 0);
        assert_eq!(workspace.close(1), Some(Closed::Window));
    }

    #[test]
    fn closing_a_tab_before_the_active_one_keeps_the_same_tab_selected() {
        let mut workspace = Workspace::default();
        for id in 1..=3 {
            workspace.open_tab(id);
        }
        assert_eq!(workspace.close_tab(0), vec![1]);
        assert_eq!(workspace.active_session(), Some(3));
    }

    #[test]
    fn moving_a_tab_keeps_it_selected_and_stops_at_the_ends() {
        let mut workspace = Workspace::default();
        for id in 1..=3 {
            workspace.open_tab(id);
        }
        workspace.move_tab(-1);
        workspace.move_tab(-1);
        workspace.move_tab(-1);
        assert_eq!(workspace.active, 0);
        assert_eq!(workspace.active_session(), Some(3));
        workspace.step_tab(-1);
        assert_eq!(workspace.active, 2);
    }
}
