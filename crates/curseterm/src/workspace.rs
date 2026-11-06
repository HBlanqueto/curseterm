use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::advanced;
use crate::config::APP_NAME;
use crate::panes::{Axis, Pane, Toward};
use crate::session::SessionId;
use crate::tab_groups::{self, TabMove};

#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    pub root: Pane,
    pub active: SessionId,
    pub zoomed: bool,
    pub title: Option<String>,
    pub group: Option<TabGroup>,
}

// iTermTabGroup.swift: every member carries the group's name, colour and collapsed state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabGroup {
    pub id: u64,
    pub name: String,
    pub color: usize,
    pub collapsed: bool,
}

impl Tab {
    pub fn single(id: SessionId) -> Self {
        Self {
            root: Pane::Leaf(id),
            active: id,
            zoomed: false,
            title: None,
            group: None,
        }
    }
}

#[derive(Default)]
pub struct Workspace {
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub window_title: Option<String>,
    titles: HashMap<SessionId, String>,
    bells: HashSet<SessionId>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Broadcast {
    #[default]
    Off,
    CurrentTab,
    AllTabs,
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

    pub fn receivers(&self, broadcast: Broadcast) -> Vec<SessionId> {
        match broadcast {
            Broadcast::Off => self.active_session().into_iter().collect(),
            Broadcast::CurrentTab => self
                .active_tab()
                .map(|tab| tab.root.leaves())
                .unwrap_or_default(),
            Broadcast::AllTabs => self.tabs.iter().flat_map(|tab| tab.root.leaves()).collect(),
        }
    }

    pub fn title(&self, id: SessionId) -> &str {
        self.titles.get(&id).map_or(APP_NAME, String::as_str)
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
            .map(|tab| self.tab_title(tab).to_string())
            .collect()
    }

    fn tab_title<'a>(&'a self, tab: &'a Tab) -> &'a str {
        tab.title
            .as_deref()
            .unwrap_or_else(|| self.title(tab.active))
    }

    pub fn window_title(&self) -> &str {
        match (&self.window_title, self.active_tab()) {
            (Some(title), _) => title,
            (None, Some(tab)) => self.tab_title(tab),
            (None, None) => APP_NAME,
        }
    }

    pub fn rename_tab(&mut self, of: SessionId, title: &str) {
        if let Some(index) = self.tab_of(of) {
            self.tabs[index].title = typed_title(title);
        }
    }

    pub fn rename_window(&mut self, title: &str) {
        self.window_title = typed_title(title);
    }

    pub fn open_tab(&mut self, id: SessionId) {
        self.restore(Tab::single(id));
    }

    // -tabsDidReorder: a restored or duplicated member joins its group's run.
    pub fn restore(&mut self, tab: Tab) {
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.reorder(&tab_groups::contiguous_order(&self.group_ids()));
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
        let closed = self.detach(id)?;
        self.forget(id);
        Some(closed)
    }

    // Takes the session out of its tab and keeps its title, for moving or burying it.
    pub fn detach(&mut self, id: SessionId) -> Option<Closed> {
        let index = self.tab_of(id)?;
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

    // MovePaneController.m -moveSessionToTab:
    pub fn move_to_tab(&mut self, id: SessionId) {
        if self.detach(id).is_some() {
            self.open_tab(id);
        }
    }

    // MovePaneController.m -dropInSession:half:atPoint:
    pub fn move_beside(&mut self, id: SessionId, target: SessionId, axis: Axis, before: bool) {
        if id == target || self.tab_of(target).is_none() || self.detach(id).is_none() {
            return;
        }
        if let Some(index) = self.tab_of(target) {
            let tab = &mut self.tabs[index];
            tab.root.insert(target, id, axis, before);
            tab.active = id;
            tab.zoomed = false;
            self.active = index;
        }
    }

    pub fn replace(&mut self, old: SessionId, new: SessionId) {
        self.forget(old);
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
            self.forget(*id);
        }
        self.remove_tab(index);
        sessions
    }

    // iTermAdvancedSettingsModel moveLeftAfterClosingTab: closing the active tab selects its left
    // neighbour instead of the right one.
    fn remove_tab(&mut self, index: usize) {
        self.tabs.remove(index);
        let move_left =
            self.active == index && index > 0 && advanced::current().move_left_after_closing_tab;
        if self.active > index || self.active == self.tabs.len() || move_left {
            self.active = self.active.saturating_sub(1);
        }
        self.hear_active_tab();
    }

    fn forget(&mut self, id: SessionId) {
        self.titles.remove(&id);
        self.bells.remove(&id);
    }

    pub fn ring(&mut self, id: SessionId) {
        if self.tab_of(id) != Some(self.active) {
            self.bells.insert(id);
        }
    }

    fn hear_active_tab(&mut self) {
        if let Some(tab) = self.tabs.get(self.active) {
            for id in tab.root.leaves() {
                self.bells.remove(&id);
            }
        }
    }

    pub fn rang_tabs(&self) -> Vec<bool> {
        self.tabs
            .iter()
            .map(|tab| tab.root.leaves().iter().any(|id| self.bells.contains(id)))
            .collect()
    }

    pub fn tab_of(&self, id: SessionId) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.root.leaves().contains(&id))
    }

    pub fn focus(&mut self, id: SessionId) {
        if let Some(index) = self.tab_of(id) {
            self.active = index;
            self.tabs[index].active = id;
            self.hear_active_tab();
        }
    }

    // PSMTabBarControl.m: selecting a member of a collapsed group expands it.
    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            if let Some(id) = self.tabs[index]
                .group
                .as_ref()
                .filter(|group| group.collapsed)
                .map(|group| group.id)
            {
                for group in self.tabs.iter_mut().filter_map(|tab| tab.group.as_mut()) {
                    if group.id == id {
                        group.collapsed = false;
                    }
                }
            }
            self.active = index;
            self.hear_active_tab();
        }
    }

    pub fn step_tab(&mut self, step: isize) {
        if !self.tabs.is_empty() {
            self.active =
                (self.active as isize + step).rem_euclid(self.tabs.len() as isize) as usize;
            self.hear_active_tab();
        }
    }

    // PseudoTerminal.m -moveCurrentTabByOffset:.
    pub fn move_tab(&mut self, step: isize) {
        match tab_groups::single_tab_move(&self.group_ids(), self.active, step) {
            TabMove::Reorder(order) => self.reorder(&order),
            TabMove::Leave => self.tabs[self.active].group = None,
            TabMove::Join(id) => {
                let joined = self
                    .tabs
                    .iter()
                    .find_map(|tab| tab.group.clone().filter(|group| group.id == id));
                self.tabs[self.active].group = joined;
            }
        }
    }

    fn group_ids(&self) -> Vec<Option<u64>> {
        self.tabs
            .iter()
            .map(|tab| tab.group.as_ref().map(|group| group.id))
            .collect()
    }

    fn reorder(&mut self, order: &[usize]) {
        let active = self.active;
        let mut tabs: Vec<Option<Tab>> = self.tabs.drain(..).map(Some).collect();
        self.tabs = order
            .iter()
            .filter_map(|index| tabs[*index].take())
            .collect();
        self.active = order.iter().position(|index| *index == active).unwrap_or(0);
    }

    // PseudoTerminal.m -addTab:toNewGroupWithDefaultName: and -nextTabGroupColor.
    pub fn add_active_to_new_group(&mut self, name: String) {
        let ids = self.group_ids();
        // PseudoTerminal.m names a group by a UUID, so a restored one never meets another's id.
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos() as u64);
        let mut distinct: Vec<u64> = ids.into_iter().flatten().collect();
        distinct.dedup();
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.group = Some(TabGroup {
                id,
                name,
                color: distinct.len(),
                collapsed: false,
            });
        }
    }

    // -removeTabFromGroup:label: then -tabsDidReorder, so the tab lands just after its old group.
    pub fn remove_active_from_group(&mut self) {
        let Some(group) = self.tabs.get(self.active).and_then(|tab| tab.group.clone()) else {
            return;
        };
        self.tabs[self.active].group = None;
        let mut ids = self.group_ids();
        let last = ids.iter().rposition(|id| *id == Some(group.id));
        if let Some(last) = last.filter(|last| *last > self.active) {
            let mut order: Vec<usize> = (0..ids.len()).collect();
            order.remove(self.active);
            order.insert(last, self.active);
            self.reorder(&order);
            ids = self.group_ids();
        }
        self.reorder(&tab_groups::contiguous_order(&ids));
    }

    pub fn active_group(&self) -> Option<&TabGroup> {
        self.tabs
            .get(self.active)
            .and_then(|tab| tab.group.as_ref())
    }

    pub fn step_tab_group(&mut self, offset: isize) {
        if let Some(index) = tab_groups::tab_group_step(&self.group_ids(), self.active, offset) {
            self.select_tab(index);
        }
    }

    // PSMTabBarControl: pressing a group's chip collapses or expands it; the active tab never
    // stays hidden.
    pub fn toggle_group(&mut self, id: u64) {
        let Some(collapsed) = self.tabs.iter().find_map(|tab| {
            tab.group
                .as_ref()
                .filter(|group| group.id == id)
                .map(|group| group.collapsed)
        }) else {
            return;
        };
        if !collapsed && self.group_ids()[self.active] == Some(id) {
            let hidden: Vec<bool> = self
                .tabs
                .iter()
                .map(|tab| tab.group.as_ref().is_some_and(|group| group.collapsed))
                .collect();
            let Some(outside) = tab_groups::nearest_outside(&self.group_ids(), &hidden, id) else {
                return;
            };
            self.select_tab(outside);
        }
        for group in self.tabs.iter_mut().filter_map(|tab| tab.group.as_mut()) {
            if group.id == id {
                group.collapsed = !collapsed;
            }
        }
    }

    pub fn focus_toward(&mut self, toward: Toward) {
        if let Some(tab) = self.active_tab_mut()
            && let Some(next) = tab.root.neighbor(tab.active, toward)
        {
            tab.active = next;
        }
    }

    // PTYTab.m -swapSession:withSession:: the active session moves and keeps the focus.
    pub fn swap_active(&mut self, other: impl FnOnce(&Pane, SessionId) -> Option<SessionId>) {
        if let Some(tab) = self.active_tab_mut()
            && let Some(other) = other(&tab.root, tab.active)
        {
            tab.root.swap(tab.active, other);
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

    pub fn arrange_evenly(&mut self) {
        if let Some(tab) = self.active_tab_mut() {
            tab.root.arrange_evenly();
        }
    }

    pub fn toggle_zoom(&mut self) {
        if let Some(tab) = self.active_tab_mut() {
            tab.zoomed = !tab.zoomed && tab.root.is_split();
        }
    }
}

fn typed_title(title: &str) -> Option<String> {
    let title = title.trim();
    (!title.is_empty()).then(|| title.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bell_in_a_background_tab_shows_until_that_tab_is_selected() {
        let mut workspace = Workspace::default();
        workspace.open_tab(1);
        workspace.open_tab(2);
        workspace.ring(2);
        workspace.ring(1);
        assert_eq!(workspace.rang_tabs(), vec![true, false]);
        workspace.select_tab(0);
        workspace.select_tab(1);
        assert_eq!(workspace.rang_tabs(), vec![false, false]);
    }

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
    fn a_moved_session_leaves_its_pane_and_lands_beside_the_target() {
        let mut workspace = Workspace::default();
        workspace.open_tab(1);
        workspace.split(2, Axis::Row);
        workspace.open_tab(3);
        workspace.move_beside(3, 1, Axis::Column, true);
        assert_eq!(workspace.tabs.len(), 1);
        assert_eq!(workspace.tabs[0].root.leaves(), vec![3, 1, 2]);
        assert_eq!(workspace.active_session(), Some(3));
        workspace.move_to_tab(2);
        assert_eq!(workspace.tabs.len(), 2);
        assert_eq!(workspace.tabs[1].root, Pane::Leaf(2));
        workspace.move_beside(1, 1, Axis::Row, false);
        assert_eq!(workspace.tabs[0].root.leaves(), vec![3, 1]);
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
    fn moving_a_tab_keeps_it_selected_and_wraps_at_the_ends() {
        let mut workspace = Workspace::default();
        for id in 1..=3 {
            workspace.open_tab(id);
        }
        workspace.move_tab(-1);
        workspace.move_tab(-1);
        assert_eq!(workspace.active, 0);
        assert_eq!(workspace.active_session(), Some(3));
        workspace.move_tab(-1);
        assert_eq!(workspace.active, 2);
        assert_eq!(workspace.active_session(), Some(3));
    }

    #[test]
    fn a_typed_title_wins_over_the_shell_and_a_blank_one_gives_it_back() {
        let mut workspace = Workspace::default();
        workspace.open_tab(1);
        workspace.set_title(1, Some("vim".to_string()));
        workspace.rename_tab(1, " notas ");
        assert_eq!(workspace.tab_titles(), vec!["notas".to_string()]);
        assert_eq!(workspace.window_title(), "notas");
        workspace.rename_window("proyecto");
        assert_eq!(workspace.window_title(), "proyecto");
        workspace.rename_tab(1, "  ");
        workspace.rename_window("");
        assert_eq!(workspace.window_title(), "vim");
    }
}
