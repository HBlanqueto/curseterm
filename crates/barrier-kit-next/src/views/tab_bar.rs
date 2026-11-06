//! Document tabs drawn like PSMTabBarControl: equal tabs that fill the bar, a close button on
//! the left on hover and the colors of a `TabStyle`.

use barrier_kit::interaction::use_interaction;
use barrier_kit::style::text;
use barrier_kit::symbol::{Symbol, symbol};
use barrier_kit::theme::color;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Content, Direction, Gaps, Position, Size};
use vow_design::{CAPTION, Font, HEADLINE, Rgba};

use crate::tokens::{TAB_BAR, TAB_GROUP, TAB_STRIP, TabStyle};

#[derive(Clone, Copy, PartialEq)]
pub struct Badge {
    pub symbol: Symbol,
    pub label: &'static str,
}

// PSMTabGroup: the group a run of tabs belongs to.
#[derive(Clone, Debug, PartialEq)]
pub struct TabGroupMark {
    pub id: u64,
    pub name: String,
    pub color: Rgba,
    pub collapsed: bool,
}

#[derive(PartialEq)]
pub struct TabBar {
    label: &'static str,
    close_label: &'static str,
    tabs: Vec<(String, Option<Badge>, Option<TabGroupMark>)>,
    on_group_press: Option<Callback<u64, ()>>,
    selected: usize,
    style: TabStyle,
    active: bool,
    leading: f32,
    on_select: Option<Callback<usize, ()>>,
    on_close: Option<Callback<usize, ()>>,
    on_empty_mouse_down: Option<Callback<Event<MouseEventData>, ()>>,
    add: Option<(&'static str, Callback<(), ()>)>,
}

impl TabBar {
    pub fn new(label: &'static str, close_label: &'static str, style: TabStyle) -> Self {
        Self {
            label,
            close_label,
            tabs: Vec::new(),
            on_group_press: None,
            selected: 0,
            style,
            active: true,
            leading: 0.0,
            on_select: None,
            on_close: None,
            on_empty_mouse_down: None,
            add: None,
        }
    }

    pub fn on_add(mut self, label: &'static str, callback: impl Into<Callback<(), ()>>) -> Self {
        self.add = Some((label, callback.into()));
        self
    }

    pub fn tab(mut self, title: impl Into<String>) -> Self {
        self.tabs.push((title.into(), None, None));
        self
    }

    pub fn badged_tab(mut self, title: impl Into<String>, badge: Badge) -> Self {
        self.tabs.push((title.into(), Some(badge), None));
        self
    }

    // The last tab added belongs to `group`; a group's tabs are consecutive.
    pub fn in_group(mut self, group: TabGroupMark) -> Self {
        if let Some(tab) = self.tabs.last_mut() {
            tab.2 = Some(group);
        }
        self
    }

    pub fn on_group_press(mut self, callback: impl Into<Callback<u64, ()>>) -> Self {
        self.on_group_press = Some(callback.into());
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn leading(mut self, width: f32) -> Self {
        self.leading = width;
        self
    }

    pub fn on_select(mut self, callback: impl Into<Callback<usize, ()>>) -> Self {
        self.on_select = Some(callback.into());
        self
    }

    pub fn on_close(mut self, callback: impl Into<Callback<usize, ()>>) -> Self {
        self.on_close = Some(callback.into());
        self
    }

    pub fn on_empty_mouse_down(
        mut self,
        callback: impl Into<Callback<Event<MouseEventData>, ()>>,
    ) -> Self {
        self.on_empty_mouse_down = Some(callback.into());
        self
    }
}

impl Component for TabBar {
    fn render(&self) -> impl IntoElement {
        let style = self.style;
        let row_height = style.height - 1.0;
        let empty = |width: Size| {
            let on_mouse_down = self.on_empty_mouse_down.clone();
            edge(&style, rect().width(width), row_height).on_mouse_down(
                move |event: Event<MouseEventData>| {
                    if let Some(on_mouse_down) = &on_mouse_down {
                        on_mouse_down.call(event);
                    }
                },
            )
        };
        // PSMTabBarControl.m: the add button takes the template tint of the bar's appearance.
        let mut add_ink = contrasting(over(style.unselected, style.backing));
        if !self.active {
            add_ink.a *= TAB_STRIP.inactive_close_alpha;
        }
        let add = self.add.clone().map(|(label, on_add)| {
            empty(Size::px(TAB_STRIP.add_width)).child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .main_align(Alignment::Center)
                    .cross_align(Alignment::Center)
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(label)
                    .on_press(move |event: Event<PressEventData>| {
                        event.stop_propagation();
                        on_add.call(());
                    })
                    .child(symbol(Symbol::Plus, TAB_STRIP.close, add_ink)),
            )
        });
        rect()
            .width(Size::fill())
            .height(Size::px(style.height))
            .direction(Direction::Vertical)
            .background(color(style.backing))
            .child(
                rect()
                    .a11y_role(AccessibilityRole::TabList)
                    .a11y_alt(self.label)
                    .width(Size::fill())
                    .height(Size::px(row_height))
                    .direction(Direction::Horizontal)
                    .content(Content::flex())
                    .maybe_child((self.leading > 0.0).then(|| empty(Size::px(self.leading))))
                    .children(self.runs(style))
                    .maybe_child(add),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(1.0))
                    .background(color(style.bottom_line)),
            )
    }
}

impl TabBar {
    fn tab_element(&self, index: usize, style: TabStyle) -> Element {
        let (title, badge, _) = &self.tabs[index];
        Tab {
            title: title.clone(),
            badge: *badge,
            close_label: self.close_label,
            selected: index == self.selected,
            last: index + 1 == self.tabs.len(),
            style,
            active: self.active,
            index,
            on_select: self.on_select.clone(),
            on_close: self.on_close.clone(),
        }
        .into_element()
    }

    // PSMYosemiteTabStyle.m -drawSquaredTabGroupRunHorizontal:… and
    // -drawSquaredCollapsedTabGroupChip:…: the name on a block of the group's colour before its
    // tabs, a 1 pt outline round the run, and a collapsed run as one “Name  N” chip.
    fn runs(&self, style: TabStyle) -> Vec<Element> {
        let mut runs = Vec::new();
        let mut index = 0;
        while index < self.tabs.len() {
            let Some(group) = self.tabs[index].2.clone() else {
                runs.push(self.tab_element(index, style));
                index += 1;
                continue;
            };
            let end = (index..self.tabs.len())
                .find(|other| self.tabs[*other].2.as_ref().map(|mark| mark.id) != Some(group.id))
                .unwrap_or(self.tabs.len());
            let members = end - index;
            let chip = self.chip(&group, members, style);
            let run = rect()
                .height(Size::fill())
                .direction(Direction::Horizontal)
                .border(
                    Border::new()
                        .width(TAB_GROUP.outline)
                        .alignment(BorderAlignment::Inner)
                        .fill(color(group.color)),
                )
                .child(chip);
            runs.push(if group.collapsed {
                run.into_element()
            } else {
                run.width(Size::flex(members as f32))
                    .content(Content::flex())
                    .children((index..end).map(|member| self.tab_element(member, style)))
                    .into_element()
            });
            index = end;
        }
        runs
    }

    fn chip(&self, group: &TabGroupMark, members: usize, style: TabStyle) -> Rect {
        let ink = contrasting(group.color);
        let font = Font {
            size: style.font_size as u32,
            weight: 700,
            ..HEADLINE
        };
        let label = match (group.collapsed, group.name.is_empty()) {
            (false, _) => group.name.clone(),
            (true, true) => members.to_string(),
            (true, false) => format!("{}  {members}", group.name),
        };
        let trailing = if group.collapsed {
            TAB_GROUP.name_pad
        } else {
            TAB_GROUP.name_pad + TAB_GROUP.colored_area
        };
        let id = group.id;
        let on_group_press = self.on_group_press.clone();
        rect()
            .height(Size::fill())
            .padding(Gaps::new(0.0, trailing, 0.0, TAB_GROUP.name_pad))
            .direction(Direction::Horizontal)
            .cross_align(Alignment::Center)
            .background(color(group.color))
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(label.clone())
            .on_press(move |event: Event<PressEventData>| {
                event.stop_propagation();
                if let Some(on_group_press) = &on_group_press {
                    on_group_press.call(id);
                }
            })
            .child(text(label, font, ink))
            .maybe_child(group.collapsed.then(|| {
                rect()
                    .width(Size::px(TAB_GROUP.chevron))
                    .main_align(Alignment::End)
                    .child(symbol(Symbol::ChevronDown, TAB_BAR.badge, ink))
            }))
    }
}

// An area outside the tabs: the unselected color, the top line and the bottom of the outline.
fn edge(style: &TabStyle, area: Rect, height: f32) -> Rect {
    area.height(Size::px(height))
        .background(color(style.unselected))
        .child(line(
            style.top_line,
            Position::new_absolute().top(0.0).left(0.0),
            true,
        ))
        .child(line(
            style.outline,
            Position::new_absolute().bottom(0.0).left(0.0),
            true,
        ))
}

fn line(ink: Rgba, position: Position, horizontal: bool) -> Rect {
    let line = rect().position(position).background(color(ink));
    if horizontal {
        line.width(Size::fill()).height(Size::px(1.0))
    } else {
        line.width(Size::px(1.0)).height(Size::fill())
    }
}

#[derive(PartialEq)]
struct Tab {
    title: String,
    badge: Option<Badge>,
    close_label: &'static str,
    selected: bool,
    last: bool,
    style: TabStyle,
    active: bool,
    index: usize,
    on_select: Option<Callback<usize, ()>>,
    on_close: Option<Callback<usize, ()>>,
}

impl Component for Tab {
    fn render(&self) -> impl IntoElement {
        let style = self.style;
        let interaction = use_interaction();
        let hovered = interaction.hovered();
        let index = self.index;
        let on_select = self.on_select.clone();
        let on_close = self.on_close.clone();
        let fill = match (self.selected, hovered) {
            (true, false) => style.selected,
            (true, true) => style.selected_hover,
            (false, false) => style.unselected,
            (false, true) => style.unselected_hover,
        };
        let ink = match (self.selected, self.active) {
            (true, true) => style.text_selected,
            (false, true) => style.text_unselected,
            (true, false) => style.text_selected_inactive,
            (false, false) => style.text_unselected_inactive,
        };
        let separator = if self.active {
            style.separator
        } else {
            style.separator_inactive
        };
        // PSMYosemiteTabStyle.m -tabColorBrightness: white on a dark tab, black on a light one.
        let mut close_ink = contrasting(over(fill, style.backing));
        if !self.active {
            close_ink.a *= TAB_STRIP.inactive_close_alpha;
        }
        let close = hovered.then(|| {
            rect()
                .position(
                    Position::new_absolute()
                        .left(TAB_STRIP.close_x)
                        .top((style.height - 1.0 - TAB_STRIP.close) / 2.0),
                )
                .layer(1)
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(self.close_label)
                .on_press(move |event: Event<PressEventData>| {
                    event.stop_propagation();
                    if let Some(on_close) = &on_close {
                        on_close.call(index);
                    }
                })
                .child(symbol(Symbol::XMark, TAB_STRIP.close, close_ink))
        });
        let shadow = (style.shadow && !self.selected).then(|| {
            rect()
                .position(Position::new_absolute().top(0.0).left(0.0))
                .width(Size::fill())
                .children(TAB_STRIP.shadow.map(|ink| {
                    rect()
                        .width(Size::fill())
                        .height(Size::px(1.0))
                        .background(color(ink))
                        .into_element()
                }))
        });
        let outline: Vec<Element> = if self.selected {
            vec![
                line(
                    style.outline,
                    Position::new_absolute().top(0.0).left(0.0),
                    false,
                )
                .into(),
                line(
                    style.outline,
                    Position::new_absolute().top(0.0).right(0.0),
                    false,
                )
                .into(),
            ]
        } else {
            vec![
                line(
                    style.outline,
                    Position::new_absolute().bottom(0.0).left(0.0),
                    true,
                )
                .into(),
            ]
        };
        let font = Font {
            size: style.font_size as u32,
            ..CAPTION
        };
        let tab = interaction
            .attach(rect(), true)
            .a11y_role(AccessibilityRole::Tab)
            .a11y_alt(match self.badge {
                Some(badge) => format!("{}, {}", self.title, badge.label),
                None => self.title.clone(),
            })
            .width(Size::flex(1.0))
            .min_width(Size::px(TAB_STRIP.min_tab_width))
            .height(Size::fill())
            .background(color(fill))
            .on_press(move |_| {
                if let Some(on_select) = &on_select {
                    on_select.call(index);
                }
            })
            .maybe_child(shadow)
            .maybe_child((!self.selected).then(|| {
                line(
                    style.top_line,
                    Position::new_absolute().top(0.0).left(0.0),
                    true,
                )
            }))
            .child(line(
                separator,
                Position::new_absolute()
                    .top(1.0)
                    .right(if self.last { -1.0 } else { 0.0 }),
                false,
            ))
            .children(outline)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .padding(Gaps::new(
                        0.0,
                        TAB_STRIP.label_inset,
                        0.0,
                        TAB_STRIP.label_inset,
                    ))
                    .direction(Direction::Horizontal)
                    .main_align(Alignment::Center)
                    .cross_align(Alignment::Center)
                    .spacing(TAB_BAR.badge_gap)
                    .maybe_child(
                        self.badge
                            .map(|badge| symbol(badge.symbol, TAB_BAR.badge, ink)),
                    )
                    .child(text(self.title.clone(), font, ink)),
            )
            .maybe_child(close);
        interaction.focus_ring(tab, 0.0)
    }
}

// NSColor+PSM.m -it_hspBrightness
pub fn brightness(ink: Rgba) -> f32 {
    (0.241 * ink.r * ink.r + 0.691 * ink.g * ink.g + 0.068 * ink.b * ink.b).sqrt()
}

fn over(top: Rgba, bottom: Rgba) -> Rgba {
    let blend = |above: f32, below: f32| above * top.a + below * (1.0 - top.a);
    Rgba {
        r: blend(top.r, bottom.r),
        g: blend(top.g, bottom.g),
        b: blend(top.b, bottom.b),
        a: 1.0,
    }
}

fn contrasting(background: Rgba) -> Rgba {
    if brightness(background) < 0.5 {
        Rgba::rgba8(255, 255, 255, 1.0)
    } else {
        Rgba::rgba8(0, 0, 0, 1.0)
    }
}
