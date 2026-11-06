use barrier_kit::controls::{Button, ScrollView, SearchField};
use barrier_kit::popup_button::PopUpButton;
use barrier_kit::style::{Palette, text};
use barrier_kit::theme::color;
use barrier_kit_next::tokens::GLOBAL_SEARCH;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Content, Direction, Gaps, Position, Size};
use vow_design::{CAPTION, DIALOG_PADDING, HEADLINE, SPACE_1, SPACE_2, TEXT};

use crate::find_mode::FindMode;
use crate::localize::tr;
use crate::session::{SessionId, Snippet};

#[derive(Clone, PartialEq)]
pub struct SessionResults {
    pub session: SessionId,
    pub name: String,
    pub snippets: Vec<Snippet>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct GlobalSearch {
    pub open: State<bool>,
    pub query: State<String>,
    pub focused: State<bool>,
    pub results: State<Vec<SessionResults>>,
    pub selected: State<Option<(usize, usize)>>,
}

impl GlobalSearch {
    pub fn close(self) {
        let (mut open, mut focused, mut selected) = (self.open, self.focused, self.selected);
        open.set(false);
        focused.set(false);
        selected.set(None);
    }

    // NSOutlineView: the arrows walk the results of every session in order.
    pub fn step(self, forward: bool) -> Option<(usize, usize)> {
        let results = self.results.peek();
        let all: Vec<(usize, usize)> = results
            .iter()
            .enumerate()
            .flat_map(|(group, found)| (0..found.snippets.len()).map(move |row| (group, row)))
            .collect();
        let current = self
            .selected
            .peek()
            .and_then(|at| all.iter().position(|other| *other == at));
        let next = match (current, forward) {
            (None, true) => all.first(),
            (None, false) => all.last(),
            (Some(index), true) => all.get(index + 1),
            (Some(index), false) => index.checked_sub(1).and_then(|index| all.get(index)),
        };
        next.copied()
    }
}

// iTermGlobalSearch.xib: the search field, its mode and an outline of sessions and their matches.
pub fn global_search_panel(
    search: GlobalSearch,
    mode: FindMode,
    on_mode: impl Fn(FindMode) + 'static,
    on_reveal: impl Fn((usize, usize)) + Clone + 'static,
) -> Element {
    let palette = Palette::current();
    let names = [
        tr!("global-search-smart-case"),
        tr!("global-search-case-sensitive"),
        tr!("global-search-case-insensitive"),
        tr!("global-search-regex-case-sensitive"),
        tr!("global-search-regex-case-insensitive"),
    ];
    let modes = names
        .into_iter()
        .fold(PopUpButton::new(tr!("find-mode")), |button, name| {
            button.item(name)
        })
        .selected(
            FindMode::ALL
                .iter()
                .position(|other| *other == mode)
                .unwrap_or_default(),
        )
        .on_change(move |position: usize| on_mode(FindMode::ALL[position]));
    let navigate = on_reveal.clone();
    let selected = *search.selected.read();
    let rows: Vec<Element> = search
        .results
        .read()
        .iter()
        .enumerate()
        .flat_map(|(group, found)| {
            let header = text(found.name.clone(), TEXT, palette.label).into_element();
            let snippets = found.snippets.iter().enumerate().map({
                let on_reveal = on_reveal.clone();
                move |(row, snippet)| {
                    result_row(snippet, selected == Some((group, row)), {
                        let on_reveal = on_reveal.clone();
                        move || on_reveal((group, row))
                    })
                }
            });
            std::iter::once(header).chain(snippets)
        })
        .collect();
    let empty = rows.is_empty() && !search.query.read().is_empty();
    rect()
        .position(Position::new_absolute().top(SPACE_2).right(SPACE_2))
        .layer(Layer::Overlay)
        .a11y_role(AccessibilityRole::Dialog)
        .width(Size::px(GLOBAL_SEARCH.width))
        .max_width(Size::percent(95.0))
        .height(Size::px(GLOBAL_SEARCH.height))
        .max_height(Size::percent(90.0))
        .padding(Gaps::new_all(DIALOG_PADDING))
        .background(color(palette.raised_background))
        .border(
            Border::new()
                .width(vow_design::HAIRLINE)
                .alignment(BorderAlignment::Outer)
                .fill(color(palette.separator)),
        )
        .direction(Direction::Vertical)
        .content(Content::flex())
        .spacing(SPACE_2)
        .child(text(tr!("find-globally-title"), HEADLINE, palette.label))
        .child(
            rect()
                .width(Size::fill())
                .direction(Direction::Horizontal)
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(SPACE_1)
                .child(
                    SearchField::new(search.query, tr!("find-placeholder"))
                        .width(Size::flex(1.0))
                        .auto_focus(true)
                        .on_focus_change(move |focused| {
                            let mut state = search.focused;
                            state.set(focused);
                        })
                        .on_navigate(move |key| {
                            let forward = !matches!(key, NamedKey::ArrowUp);
                            if let Some(at) = search.step(forward) {
                                navigate(at);
                            }
                        }),
                )
                .child(modes)
                .child(Button::new(tr!("find-close")).on_press(move |_| search.close())),
        )
        .child(
            ScrollView::new().height(Size::flex(1.0)).child(
                rect()
                    .width(Size::fill())
                    .direction(Direction::Vertical)
                    .spacing(SPACE_1)
                    .children(rows),
            ),
        )
        .maybe_child(empty.then(|| text(tr!("find-none"), CAPTION, palette.secondary_label)))
        .into_element()
}

// iTermGlobalSearchEngineCursor.m: the match underlined on a translucent yellow.
fn result_row(snippet: &Snippet, selected: bool, on_press: impl Fn() + 'static) -> Element {
    let palette = Palette::current();
    let ink = if selected {
        palette.on_accent
    } else {
        palette.label
    };
    let alt = snippet.parts.concat();
    // Skia's paragraph width leaves out edge spaces, which would glue the three parts together.
    let [before, matched, after] = snippet
        .parts
        .clone()
        .map(|part| part.replace(' ', "\u{a0}"));
    rect()
        .a11y_role(AccessibilityRole::Button)
        .a11y_alt(alt)
        .width(Size::fill())
        .padding(Gaps::new(0.0, SPACE_1, 0.0, SPACE_2 * 2.0))
        .corner_radius(SPACE_1)
        .background(color(if selected {
            palette.accent
        } else {
            vow_design::Rgba::TRANSPARENT
        }))
        .overflow(Overflow::Clip)
        .direction(Direction::Horizontal)
        .on_press(move |_| on_press())
        .child(text(before, TEXT, ink))
        .child(
            rect()
                .direction(Direction::Vertical)
                .content(Content::fit())
                .background(color(GLOBAL_SEARCH.match_background))
                .child(text(matched, TEXT, ink))
                .child(
                    rect()
                        .width(Size::fill_minimum())
                        .height(Size::px(1.0))
                        .background(color(ink)),
                ),
        )
        .child(text(after, TEXT, ink))
        .into_element()
}
