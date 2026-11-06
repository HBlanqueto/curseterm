use barrier_kit::controls::{Button, ButtonStyle, ScrollView, Switch, TextField};
use barrier_kit::popup_button::PopUpButton;
use barrier_kit::style::{Palette, text};
use barrier_kit::views::feedback::sheet;
use barrier_kit::views::form::{Form, Row, Section};
use barrier_kit_next::tokens::{ADVANCED_PASTE, MONO};
use freya_core::prelude::*;
use torin::prelude::{Alignment, Content, Direction, Size};
use vow_design::{CAPTION, HEADLINE, SPACE_2};

use crate::localize::tr;
use crate::paste_transform::{self, Tabs, Transform};

#[derive(Clone, Copy, PartialEq)]
pub struct AdvancedPaste {
    pub text: State<String>,
    pub transform: State<Transform>,
    pub regex: State<String>,
    pub substitution: State<String>,
    pub bracketed: State<bool>,
}

impl AdvancedPaste {
    pub fn open(self, clipboard: String) {
        let (mut text, mut transform, mut regex, mut substitution, mut bracketed) = (
            self.text,
            self.transform,
            self.regex,
            self.substitution,
            self.bracketed,
        );
        text.set(clipboard);
        transform.set(Transform::default());
        regex.set(String::new());
        substitution.set(String::new());
        bracketed.set(true);
    }

    pub fn result(self) -> String {
        let transform = Transform {
            regex: self.regex.peek().clone(),
            substitution: self.substitution.peek().clone(),
            ..self.transform.peek().clone()
        };
        paste_transform::apply(&self.text.peek(), &transform)
    }
}

// iTermPasteSpecialViewController.xib: Transforms, tabs, the regular expression and a preview.
pub fn advanced_paste_sheet(
    state: AdvancedPaste,
    on_paste: impl Fn(String, bool) + 'static,
    on_cancel: impl Fn() + 'static,
) -> Element {
    let transform = state.transform.read().clone();
    let switch = |label: &'static str, on: bool, set: fn(&mut Transform, bool)| {
        let mut shared = state.transform;
        Row::new(label).control(
            Switch::new(label)
                .on(on)
                .on_toggle(move |on: bool| set(&mut shared.write(), on)),
        )
    };
    let tabs = [
        Tabs::Keep,
        Tabs::EscapeWithCtrlV,
        Tabs::Spaces(ADVANCED_PASTE.spaces),
    ];
    let tab_choice = tabs
        .iter()
        .position(|choice| *choice == transform.tabs)
        .unwrap_or_default();
    let mut shared = state.transform;
    let tab_popup = [
        tr!("paste-with-tabs"),
        tr!("paste-escape-tabs"),
        tr!("paste-tabs-to-spaces"),
    ]
    .into_iter()
    .fold(PopUpButton::new(tr!("paste-tabs")), |button, name| {
        button.item(name)
    })
    .selected(tab_choice)
    .on_change(move |position: usize| shared.write().tabs = tabs[position]);
    let mut bracketed = state.bracketed;
    let preview: String = state
        .result()
        .chars()
        .take(ADVANCED_PASTE.preview_chars)
        .collect();
    let palette = Palette::current();
    sheet(
        rect()
            .width(Size::px(ADVANCED_PASTE.width))
            .height(Size::px(ADVANCED_PASTE.height))
            .direction(Direction::Vertical)
            .content(Content::flex())
            .spacing(SPACE_2)
            .child(text(tr!("advanced-paste"), HEADLINE, palette.label))
            .child(
                ScrollView::new().height(Size::flex(1.0)).child(
                    Form::new().section(
                        Section::titled(tr!("paste-transforms"))
                            .row(Row::new(tr!("paste-tabs")).control(tab_popup))
                            .row(switch(
                                tr!("paste-escape-shell"),
                                transform.escape_shell,
                                |transform, on| transform.escape_shell = on,
                            ))
                            .row(switch(
                                tr!("paste-convert-newlines"),
                                transform.convert_newlines,
                                |transform, on| transform.convert_newlines = on,
                            ))
                            .row(switch(
                                tr!("paste-remove-newlines"),
                                transform.remove_newlines,
                                |transform, on| transform.remove_newlines = on,
                            ))
                            .row(switch(
                                tr!("paste-remove-controls"),
                                transform.remove_controls,
                                |transform, on| transform.remove_controls = on,
                            ))
                            .row(switch(
                                tr!("paste-unicode-punctuation"),
                                transform.unicode_punctuation,
                                |transform, on| transform.unicode_punctuation = on,
                            ))
                            .row(switch(
                                tr!("paste-base64"),
                                transform.base64,
                                |transform, on| transform.base64 = on,
                            ))
                            .row(
                                Row::new(tr!("paste-bracketed")).control(
                                    Switch::new(tr!("paste-bracketed"))
                                        .on(*state.bracketed.read())
                                        .on_toggle(move |on: bool| bracketed.set(on)),
                                ),
                            )
                            .row(Row::new(tr!("paste-replace")).control(TextField::new(
                                state.regex,
                                tr!("paste-regex-placeholder"),
                            )))
                            .row(Row::new(tr!("paste-with")).control(TextField::new(
                                state.substitution,
                                tr!("paste-substitution-placeholder"),
                            ))),
                    ),
                ),
            )
            .child(text(tr!("paste-preview"), CAPTION, palette.secondary_label))
            .child(
                ScrollView::new()
                    .height(Size::px(ADVANCED_PASTE.preview_height))
                    .child(text(preview, MONO, palette.label)),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .direction(Direction::Horizontal)
                    .main_align(Alignment::End)
                    .spacing(SPACE_2)
                    .child(Button::new(tr!("close-cancel")).on_press(move |_| on_cancel()))
                    .child(
                        Button::new(tr!("paste-confirm"))
                            .style(ButtonStyle::Primary)
                            .on_press(move |_| on_paste(state.result(), *state.bracketed.peek())),
                    ),
            ),
    )
    .into_element()
}
