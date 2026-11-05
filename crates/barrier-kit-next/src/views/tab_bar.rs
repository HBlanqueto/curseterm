//! Document tabs: one tab per document with a live title and a close button.

use barrier_kit::adornments::divider;
use barrier_kit::compact::IconButton;
use barrier_kit::interaction::use_interaction;
use barrier_kit::style::{Palette, text};
use barrier_kit::symbol::Symbol;
use barrier_kit::theme::color;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Content, Direction, Gaps, Size};
use vow_design::CAPTION;

use crate::tokens::TAB_BAR;

#[derive(PartialEq)]
pub struct TabBar {
    label: &'static str,
    close_label: &'static str,
    titles: Vec<String>,
    selected: usize,
    on_select: Option<Callback<usize, ()>>,
    on_close: Option<Callback<usize, ()>>,
    add: Option<(&'static str, Callback<(), ()>)>,
}

impl TabBar {
    pub fn new(label: &'static str, close_label: &'static str) -> Self {
        Self {
            label,
            close_label,
            titles: Vec::new(),
            selected: 0,
            on_select: None,
            on_close: None,
            add: None,
        }
    }

    pub fn on_add(mut self, label: &'static str, callback: impl Into<Callback<(), ()>>) -> Self {
        self.add = Some((label, callback.into()));
        self
    }

    pub fn tab(mut self, title: impl Into<String>) -> Self {
        self.titles.push(title.into());
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
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
}

impl Component for TabBar {
    fn render(&self) -> impl IntoElement {
        let palette = Palette::current();
        rect()
            .width(Size::fill())
            .direction(Direction::Vertical)
            .background(color(palette.window_background))
            .child(
                rect()
                    .a11y_role(AccessibilityRole::TabList)
                    .a11y_alt(self.label)
                    .width(Size::fill())
                    .height(Size::px(TAB_BAR.height))
                    .direction(Direction::Horizontal)
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .children(self.titles.iter().enumerate().map(|(index, title)| {
                        Tab {
                            title: title.clone(),
                            close_label: self.close_label,
                            selected: index == self.selected,
                            index,
                            on_select: self.on_select.clone(),
                            on_close: self.on_close.clone(),
                        }
                        .into_element()
                    }))
                    .maybe_child(self.add.clone().map(|(label, on_add)| {
                        IconButton::new(Symbol::Plus, label).on_press(move |_| on_add.call(()))
                    })),
            )
            .child(divider())
    }
}

#[derive(PartialEq)]
struct Tab {
    title: String,
    close_label: &'static str,
    selected: bool,
    index: usize,
    on_select: Option<Callback<usize, ()>>,
    on_close: Option<Callback<usize, ()>>,
}

impl Component for Tab {
    fn render(&self) -> impl IntoElement {
        let palette = Palette::current();
        let interaction = use_interaction();
        let index = self.index;
        let on_select = self.on_select.clone();
        let on_close = self.on_close.clone();
        let background = if self.selected {
            palette.content_background
        } else if interaction.hovered() {
            palette.control_hover
        } else {
            palette.window_background
        };
        let ink = if self.selected {
            palette.label
        } else {
            palette.secondary_label
        };
        let close = (self.selected || interaction.hovered()).then(|| {
            IconButton::new(Symbol::XMark, self.close_label).on_press(move |_| {
                if let Some(on_close) = &on_close {
                    on_close.call(index);
                }
            })
        });
        let tab = interaction
            .attach(rect(), true)
            .a11y_role(AccessibilityRole::Tab)
            .a11y_alt(self.title.clone())
            .width(Size::flex(1.0))
            .max_width(Size::px(TAB_BAR.tab_max_width))
            .height(Size::fill())
            .padding(Gaps::new(0.0, TAB_BAR.padding, 0.0, TAB_BAR.padding))
            .direction(Direction::Horizontal)
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .background(color(background))
            .on_press(move |_| {
                if let Some(on_select) = &on_select {
                    on_select.call(index);
                }
            })
            .child(
                rect()
                    .width(Size::flex(1.0))
                    .main_align(Alignment::Center)
                    .child(text(self.title.clone(), CAPTION, ink)),
            )
            .maybe_child(close);
        interaction.focus_ring(tab, 0.0)
    }
}
