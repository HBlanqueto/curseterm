use barrier_kit::compact::IconButton;
use barrier_kit::controls::{Button, TextField};
use barrier_kit::style::{Palette, text};
use barrier_kit::symbol::Symbol;
use barrier_kit::theme::color;
use barrier_kit::views::table::Table;
use barrier_kit_next::tokens::ARRANGEMENT_PREVIEW;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Content, Direction, Gaps, Size};
use vow_design::{CAPTION, HAIRLINE, HEADLINE, SIDEBAR, SPACE_1, SPACE_3};

use crate::arrangement::{Arrangement, Layout};
use crate::config::{Defaults, change_defaults};
use crate::localize::tr;
use crate::panes::Axis;

#[derive(PartialEq)]
pub struct ArrangementsPane {
    pub defaults: Defaults,
}

// WindowArrangements.m: the saved arrangements by name, the default starred, Set Default, delete
// and a preview of the selected one.
impl Component for ArrangementsPane {
    fn render(&self) -> impl IntoElement {
        let names = self.defaults.arrangement_names();
        let default = self.defaults.default_arrangement.clone();
        let saved = &self.defaults.arrangements;
        let slot_of = |name: &String| {
            saved
                .iter()
                .position(|arrangement| &arrangement.name == name)
        };
        let mut selected = use_state(|| None::<usize>);
        let slot = (*selected.read())
            .filter(|slot| *slot < saved.len())
            .or_else(|| names.first().and_then(slot_of));
        let chosen = slot.map(|slot| saved[slot].name.clone());
        let rows = names
            .iter()
            .map(|name| {
                vec![if Some(name) == default.as_ref() {
                    tr!("profile-default-row", name = name)
                } else {
                    name.clone()
                }]
            })
            .collect();
        let row = chosen
            .as_ref()
            .and_then(|chosen| names.iter().position(|name| name == chosen));
        let arrangement = slot.map(|slot| (slot, saved[slot].clone()));
        let slots: Vec<Option<usize>> = names.iter().map(slot_of).collect();
        let (deleting, defaulting) = (chosen.clone(), chosen.clone());
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .direction(Direction::Horizontal)
            .padding(Gaps::new_all(SPACE_3))
            .spacing(SPACE_3)
            .child(
                rect()
                    .width(Size::px(SIDEBAR.width as f32))
                    .height(Size::fill())
                    .direction(Direction::Vertical)
                    .content(Content::flex())
                    .spacing(SPACE_1)
                    .child(
                        rect().height(Size::flex(1.0)).child(
                            Table::new(tr!("pane-arrangements"), vec![tr!("arrangement-name")])
                                .rows(rows)
                                .selected(row)
                                .on_select(move |row: usize| {
                                    selected.set(slots.get(row).copied().flatten())
                                }),
                        ),
                    )
                    .child(
                        rect()
                            .direction(Direction::Horizontal)
                            .cross_align(Alignment::Center)
                            .spacing(SPACE_1)
                            .child(
                                IconButton::new(Symbol::Minus, tr!("arrangement-delete"))
                                    .disabled(deleting.is_none())
                                    .on_press(move |_| {
                                        if let Some(name) = &deleting {
                                            change_defaults(|defaults| {
                                                defaults.delete_arrangement(name)
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Button::new(tr!("arrangement-set-default"))
                                    .disabled(defaulting.is_none() || defaulting == default)
                                    .on_press(move |_| {
                                        let name = defaulting.clone();
                                        change_defaults(|defaults| {
                                            defaults.default_arrangement = name
                                        });
                                    }),
                            ),
                    ),
            )
            .child(
                rect()
                    .width(Size::flex(1.0))
                    .height(Size::fill())
                    .direction(Direction::Vertical)
                    .spacing(SPACE_1)
                    .maybe_child(arrangement.map(|(slot, arrangement)| {
                        rect()
                            .width(Size::fill())
                            .direction(Direction::Vertical)
                            .spacing(SPACE_1)
                            .child(text(
                                tr!("arrangement-name"),
                                HEADLINE,
                                Palette::current().label,
                            ))
                            .child(ArrangementName {
                                slot,
                                name: arrangement.name.clone(),
                            })
                            .child(preview(&arrangement))
                    })),
            )
    }
}

// The field starts over only for another arrangement: a rename that is still being saved
// must not undo the letters typed after it.
#[derive(PartialEq)]
struct ArrangementName {
    slot: usize,
    name: String,
}

impl Component for ArrangementName {
    fn render(&self) -> impl IntoElement {
        let mut draft = use_state(|| self.name.clone());
        let mut shown = use_state(|| self.name.clone());
        let mut editing = use_state(|| self.slot);
        if *editing.peek() != self.slot {
            editing.set(self.slot);
            shown.set(self.name.clone());
            draft.set(self.name.clone());
        }
        use_side_effect(move || {
            let typed = draft.read().trim().to_string();
            let old = shown.peek().clone();
            if typed == old {
                return;
            }
            let mut renamed = false;
            change_defaults(|defaults| renamed = defaults.rename_arrangement(&old, &typed));
            if renamed {
                shown.set(typed);
            }
        });
        TextField::new(draft, tr!("arrangement-name"))
    }
}

// ArrangementPreviewView.m -drawRect: the screen, and on it the window with its tabs; here the
// tab strip and the first tab's panes.
fn preview(arrangement: &Arrangement) -> Rect {
    let palette = Palette::current();
    let tabs = arrangement.tabs.iter().enumerate().map(|(index, tab)| {
        let number = index as u64 + 1;
        let title = tab
            .title
            .clone()
            .unwrap_or_else(|| tr!("arrangement-tab", number = number));
        rect()
            .width(Size::flex(1.0))
            .height(Size::fill())
            .main_align(Alignment::Center)
            .cross_align(Alignment::Center)
            .border(frame())
            .child(text(title, CAPTION, palette.label))
            .into_element()
    });
    let window = rect()
        .width(Size::fill())
        .height(Size::fill())
        .direction(Direction::Vertical)
        .content(Content::flex())
        .background(color(ARRANGEMENT_PREVIEW.window))
        .border(frame())
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(ARRANGEMENT_PREVIEW.tab_height))
                .direction(Direction::Horizontal)
                .content(Content::flex())
                .children(tabs),
        )
        .maybe_child(arrangement.tabs.first().map(|tab| {
            rect()
                .width(Size::fill())
                .height(Size::flex(1.0))
                .child(panes(&tab.root))
        }));
    rect()
        .width(Size::fill())
        .height(Size::px(ARRANGEMENT_PREVIEW.height))
        .padding(Gaps::new_all(ARRANGEMENT_PREVIEW.padding))
        .background(color(ARRANGEMENT_PREVIEW.backdrop))
        .child(
            rect()
                .width(Size::fill())
                .height(Size::fill())
                .padding(Gaps::new_all(ARRANGEMENT_PREVIEW.padding))
                .background(color(ARRANGEMENT_PREVIEW.screen))
                .border(frame())
                .child(window),
        )
}

fn panes(layout: &Layout) -> Element {
    match layout {
        Layout::Session { profile, .. } => rect()
            .width(Size::fill())
            .height(Size::fill())
            .main_align(Alignment::Center)
            .cross_align(Alignment::Center)
            .border(frame())
            .child(text(
                profile.clone(),
                CAPTION,
                Palette::current().secondary_label,
            ))
            .into_element(),
        Layout::Split {
            axis,
            ratios,
            children,
        } => {
            let horizontal = *axis == Axis::Row;
            let children = children.iter().zip(ratios).map(|(child, ratio)| {
                let share = Size::flex(*ratio);
                let (width, height) = if horizontal {
                    (share, Size::fill())
                } else {
                    (Size::fill(), share)
                };
                rect()
                    .width(width)
                    .height(height)
                    .child(panes(child))
                    .into_element()
            });
            rect()
                .width(Size::fill())
                .height(Size::fill())
                .direction(if horizontal {
                    Direction::Horizontal
                } else {
                    Direction::Vertical
                })
                .content(Content::flex())
                .children(children)
                .into_element()
        }
    }
}

fn frame() -> Border {
    Border::new()
        .width(HAIRLINE)
        .fill(color(ARRANGEMENT_PREVIEW.frame))
}
