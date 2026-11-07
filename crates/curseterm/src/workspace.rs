use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use barrier_kit::views::floating_pane::PaneFrame;

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
    pub floats: Vec<Float>,
    pub floats_hidden: bool,
    last_tiled: Option<SessionId>,
}

// floating-panes-design.md §3: a float holds one session above the tiled panes; the list goes
// back to front.
#[derive(Clone, Debug, PartialEq)]
pub struct Float {
    pub id: SessionId,
    pub frame: PaneFrame,
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
            floats: Vec::new(),
            floats_hidden: false,
            last_tiled: None,
        }
    }

    pub fn restored(
        root: Pane,
        title: Option<String>,
        group: Option<TabGroup>,
        floats: Vec<Float>,
        floats_hidden: bool,
    ) -> Self {
        let active = root.leaves()[0];
        Self {
            root,
            active,
            title,
            group,
            floats,
            floats_hidden,
            ..Self::single(active)
        }
    }

    // The tiled panes, then the floats back to front.
    pub fn sessions(&self) -> Vec<SessionId> {
        let mut sessions = self.root.leaves();
        sessions.extend(self.floats.iter().map(|float| float.id));
        sessions
    }

    pub fn is_float(&self, id: SessionId) -> bool {
        self.floats.iter().any(|float| float.id == id)
    }

    // PTYTab.m -mostRecentlyActiveTiledSession.
    pub fn tiled_target(&self) -> SessionId {
        let leaves = self.root.leaves();
        self.last_tiled
            .filter(|id| leaves.contains(id))
            .or_else(|| leaves.contains(&self.active).then_some(self.active))
            .unwrap_or(leaves[0])
    }

    // §6.5: focus entering a float unhides the floats.
    fn activate(&mut self, id: SessionId) {
        if !self.is_float(self.active) {
            self.last_tiled = Some(self.active);
        }
        self.active = id;
        if self.is_float(id) {
            self.floats_hidden = false;
        }
    }

    fn raise(&mut self, id: SessionId) {
        if let Some(at) = self.floats.iter().position(|float| float.id == id) {
            let float = self.floats.remove(at);
            self.floats.push(float);
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
            Broadcast::CurrentTab => self.active_tab().map(Tab::sessions).unwrap_or_default(),
            Broadcast::AllTabs => self.tabs.iter().flat_map(Tab::sessions).collect(),
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
        // floating-panes-design.md §6.6: a float cannot be split.
        let split = !tab.is_float(tab.active) && tab.root.split(tab.active, id, axis);
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
        if let Some(at) = tab.floats.iter().position(|float| float.id == id) {
            tab.floats.remove(at);
            // §6.3: the next float in z-order, else the last active tiled session.
            if tab.active == id {
                tab.active = tab
                    .floats
                    .last()
                    .map_or_else(|| tab.tiled_target(), |float| float.id);
            }
            return Some(Closed::Pane);
        }
        // §3.2: a tab keeps a tiled session, so the topmost float takes the last one's place.
        if tab.root == Pane::Leaf(id)
            && let Some(float) = tab.floats.pop()
        {
            tab.root = Pane::Leaf(float.id);
            tab.active = float.id;
            return Some(Closed::Pane);
        }
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
        let sessions = tab.sessions();
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
            for id in tab.sessions() {
                self.bells.remove(&id);
            }
        }
    }

    pub fn rang_tabs(&self) -> Vec<bool> {
        self.tabs
            .iter()
            .map(|tab| tab.sessions().iter().any(|id| self.bells.contains(id)))
            .collect()
    }

    pub fn tab_of(&self, id: SessionId) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.sessions().contains(&id))
    }

    pub fn focus(&mut self, id: SessionId) {
        if let Some(index) = self.tab_of(id) {
            self.active = index;
            self.tabs[index].activate(id);
            self.tabs[index].raise(id);
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

    // PseudoTerminal.m -addTabToGroup: and -tabsDidReorder.
    pub fn add_active_to_group(&mut self, id: u64) {
        let Some(group) = self.groups().into_iter().find(|group| group.id == id) else {
            return;
        };
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.group = Some(group);
        }
        self.reorder(&tab_groups::contiguous_order(&self.group_ids()));
    }

    pub fn groups(&self) -> Vec<TabGroup> {
        let mut groups: Vec<TabGroup> = Vec::new();
        for group in self.tabs.iter().filter_map(|tab| tab.group.as_ref()) {
            if groups.iter().all(|known| known.id != group.id) {
                groups.push(group.clone());
            }
        }
        groups
    }

    pub fn rename_group(&mut self, id: u64, name: &str) {
        for group in self.tabs.iter_mut().filter_map(|tab| tab.group.as_mut()) {
            if group.id == id {
                group.name = name.to_string();
            }
        }
    }

    // PseudoTerminal.m -newTabToTheRight:: a tab made beside a grouped one joins that group.
    pub fn place_active(&mut self, at: usize, group: Option<TabGroup>) {
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.group = group;
        }
        let mut order: Vec<usize> = (0..self.tabs.len()).collect();
        let moved = order.remove(self.active);
        order.insert(at.min(order.len()), moved);
        self.reorder(&order);
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

    // §6.4: Next and Previous Pane go through the tiled panes, then the floats in z-order.
    pub fn cycle_pane(&mut self, step: isize) {
        if let Some(tab) = self.active_tab_mut() {
            let order = tab.sessions();
            let count = order.len() as isize;
            let at = order.iter().position(|id| *id == tab.active).unwrap_or(0) as isize;
            let next = order[(at + step).rem_euclid(count) as usize];
            tab.activate(next);
        }
    }

    pub fn open_float(&mut self, id: SessionId, frame: PaneFrame) {
        if let Some(tab) = self.active_tab_mut() {
            tab.floats.push(Float { id, frame });
            tab.activate(id);
        }
    }

    pub fn set_float_frame(&mut self, id: SessionId, frame: PaneFrame) {
        let float = self
            .tabs
            .iter_mut()
            .flat_map(|tab| tab.floats.iter_mut())
            .find(|float| float.id == id);
        if let Some(float) = float {
            float.frame = frame;
        }
    }

    pub fn active_float(&self) -> Option<&Float> {
        let tab = self.active_tab()?;
        tab.floats.iter().find(|float| float.id == tab.active)
    }

    // PTYTab.m -bringFloatingPaneToFront: and -sendFloatingPaneToBack:.
    pub fn restack_active_float(&mut self, to_front: bool) {
        if let Some(tab) = self.active_tab_mut()
            && let Some(at) = tab.floats.iter().position(|float| float.id == tab.active)
        {
            let float = tab.floats.remove(at);
            if to_front {
                tab.floats.push(float);
            } else {
                tab.floats.insert(0, float);
            }
        }
    }

    // PseudoTerminal.m -toggleFloatingPanesHidden: and §6.3: hiding an active float hands the
    // focus to the last active tiled session.
    pub fn toggle_floats_hidden(&mut self) {
        if let Some(tab) = self.active_tab_mut() {
            tab.floats_hidden = !tab.floats_hidden;
            if tab.floats_hidden && tab.is_float(tab.active) {
                tab.active = tab.tiled_target();
            }
        }
    }

    // PseudoTerminal.m -dockFloatingSession:: to the right of the tiled pane used last.
    pub fn dock_active_float(&mut self) {
        if let Some(tab) = self.active_tab_mut()
            && let Some(at) = tab.floats.iter().position(|float| float.id == tab.active)
        {
            let id = tab.floats.remove(at).id;
            let target = tab.tiled_target();
            tab.root.split(target, id, Axis::Row);
            tab.zoomed = false;
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
        if let Some(tab) = self.active_tab_mut()
            && !tab.is_float(tab.active)
        {
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
    fn floats_come_after_the_tiled_panes_and_the_last_tiled_pane_is_never_left_empty() {
        let mut workspace = Workspace::default();
        workspace.open_tab(1);
        workspace.open_float(2, PaneFrame::default());
        assert_eq!(workspace.tabs[0].sessions(), vec![1, 2]);
        workspace.cycle_pane(1);
        assert_eq!(workspace.active_session(), Some(1));
        workspace.focus(2);
        assert!(!workspace.split(4, Axis::Row));
        assert_eq!(workspace.close(1), Some(Closed::Pane));
        assert_eq!(workspace.tabs[0].root.leaves(), vec![2]);
        assert!(workspace.tabs[0].floats.is_empty());
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
