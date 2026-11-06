use barrier_kit::compact::IconButton;
use barrier_kit::controls::{Button, ButtonStyle, TextField};
use barrier_kit::popup_button::PopUpButton;
use barrier_kit::style::{Palette, text};
use barrier_kit::symbol::Symbol;
use barrier_kit::theme::color;
use barrier_kit::views::feedback::sheet;
use barrier_kit::views::form::{Form, Row, Section};
use barrier_kit::views::table::Table;
use barrier_kit_next::tokens::{MONO, SNIPPET_EDITOR};
use barrier_kit_next::views::text_view::TextView;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Content, Direction, Gaps, Size};
use vow_design::{CAPTION, HEADLINE, SPACE_1, SPACE_2, SPACE_3};

use crate::config::{Defaults, change_defaults};
use crate::localize::tr;
use crate::snippets::{Escaping, Snippet};

#[derive(PartialEq)]
pub struct SnippetsPane {
    pub defaults: Defaults,
}

// iTermSnippetsEditingViewController.xib: the snippets, add, remove and Edit.
impl Component for SnippetsPane {
    fn render(&self) -> impl IntoElement {
        let mut selected = use_state(|| None::<usize>);
        let editing = use_state(|| None::<Option<usize>>);
        let draft = Draft {
            title: use_state(String::new),
            value: use_state(String::new),
            escaping: use_state(Escaping::default),
            tags: use_state(String::new),
        };
        let snippets = &self.defaults.snippets;
        let chosen = selected.read().filter(|index| *index < snippets.len());
        let rows = snippets
            .iter()
            .map(|snippet| vec![snippet.title.clone(), snippet.value.replace('\n', " ")])
            .collect();
        let open = move |index: Option<usize>, snippet: Snippet| {
            draft.load(snippet);
            let mut editing = editing;
            editing.set(Some(index));
        };
        let edited = chosen.and_then(|index| snippets.get(index).cloned());
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .direction(Direction::Vertical)
            .content(Content::flex())
            .padding(Gaps::new_all(SPACE_3))
            .spacing(SPACE_1)
            .child(text(
                tr!("pane-snippets"),
                HEADLINE,
                Palette::current().label,
            ))
            .child(
                rect().height(Size::flex(1.0)).child(
                    Table::new(
                        tr!("pane-snippets"),
                        vec![tr!("snippet-title"), tr!("snippet-text")],
                    )
                    .rows(rows)
                    .selected(chosen)
                    .on_select(move |row| selected.set(Some(row))),
                ),
            )
            .child(
                rect()
                    .direction(Direction::Horizontal)
                    .cross_align(Alignment::Center)
                    .spacing(SPACE_1)
                    .child(
                        IconButton::new(Symbol::Plus, tr!("snippet-add"))
                            .on_press(move |_| open(None, Snippet::default())),
                    )
                    .child(
                        IconButton::new(Symbol::Minus, tr!("snippet-remove"))
                            .disabled(chosen.is_none())
                            .on_press(move |_| {
                                if let Some(index) = chosen {
                                    change_defaults(|defaults| {
                                        if index < defaults.snippets.len() {
                                            defaults.snippets.remove(index);
                                        }
                                    });
                                    selected.set(None);
                                }
                            }),
                    )
                    .child(
                        Button::new(tr!("snippet-edit"))
                            .disabled(edited.is_none())
                            .on_press(move |_| {
                                if let Some(snippet) = edited.clone() {
                                    open(chosen, snippet);
                                }
                            }),
                    ),
            )
            .maybe_child((*editing.read()).map(|index| {
                edit_sheet(draft, index, move || {
                    let mut editing = editing;
                    editing.set(None);
                })
            }))
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Draft {
    title: State<String>,
    value: State<String>,
    escaping: State<Escaping>,
    tags: State<String>,
}

impl Draft {
    fn load(self, snippet: Snippet) {
        let (mut title, mut value, mut escaping, mut tags) =
            (self.title, self.value, self.escaping, self.tags);
        title.set(snippet.title);
        value.set(snippet.value);
        escaping.set(snippet.escaping);
        tags.set(snippet.tags.join(", "));
    }

    fn snippet(self) -> Snippet {
        Snippet {
            title: self.title.peek().clone(),
            value: self.value.peek().clone(),
            escaping: *self.escaping.peek(),
            tags: self
                .tags
                .peek()
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
                .collect(),
        }
    }
}

// iTermEditSnippetWindowController.xib: Title, Value, Escaping and Tags, with OK and Cancel.
fn edit_sheet(draft: Draft, index: Option<usize>, close: impl Fn() + Clone + 'static) -> Element {
    let palette = Palette::current();
    let mut escaping = draft.escaping;
    let chosen = Escaping::ALL
        .iter()
        .position(|other| *other == *draft.escaping.read())
        .unwrap_or_default();
    let escapings = [
        tr!("snippet-escaping-literal"),
        tr!("snippet-escaping-compatibility"),
        tr!("snippet-escaping-c-style"),
    ]
    .into_iter()
    .fold(PopUpButton::new(tr!("snippet-escaping")), |button, name| {
        button.item(name)
    })
    .selected(chosen)
    .on_change(move |position: usize| escaping.set(Escaping::ALL[position]));
    let value = rect()
        .width(Size::fill())
        .height(Size::px(SNIPPET_EDITOR.value_height))
        .padding(Gaps::new_all(SPACE_1))
        .background(color(palette.content_background))
        .border(
            Border::new()
                .width(vow_design::HAIRLINE)
                .fill(color(palette.control_border)),
        )
        .child(
            TextView::new(
                draft.value,
                MONO.family,
                MONO.size as f32,
                MONO.line_height as f32,
                color(palette.label),
            )
            .height(Size::fill()),
        );
    let cancel = close.clone();
    sheet(
        rect()
            .width(Size::px(SNIPPET_EDITOR.width))
            .direction(Direction::Vertical)
            .spacing(SPACE_2)
            .child(text(tr!("snippet-edit-title"), HEADLINE, palette.label))
            .child(
                Form::new().section(
                    Section::new()
                        .row(
                            Row::new(tr!("snippet-title-label")).control(
                                TextField::new(draft.title, tr!("snippet-title-placeholder"))
                                    .width(Size::fill()),
                            ),
                        )
                        .row(Row::new(tr!("snippet-value-label")).control(value))
                        .row(Row::new(tr!("snippet-escaping")).control(escapings))
                        .row(
                            Row::new(tr!("snippet-tags-label")).control(
                                TextField::new(draft.tags, tr!("snippet-tags-placeholder"))
                                    .width(Size::fill()),
                            ),
                        ),
                ),
            )
            .child(text(
                tr!("snippet-tags-hint"),
                CAPTION,
                palette.secondary_label,
            ))
            .child(
                rect()
                    .width(Size::fill())
                    .direction(Direction::Horizontal)
                    .main_align(Alignment::End)
                    .spacing(SPACE_2)
                    .child(Button::new(tr!("close-cancel")).on_press(move |_| cancel()))
                    .child(
                        Button::new(tr!("snippet-ok"))
                            .style(ButtonStyle::Primary)
                            .on_press(move |_| {
                                let snippet = draft.snippet();
                                change_defaults(|defaults| match index {
                                    Some(index) if index < defaults.snippets.len() => {
                                        defaults.snippets[index] = snippet;
                                    }
                                    _ => defaults.snippets.push(snippet),
                                });
                                close();
                            }),
                    ),
            ),
    )
    .into_element()
}
