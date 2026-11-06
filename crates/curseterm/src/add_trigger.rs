use barrier_kit::controls::{Button, ButtonStyle, Checkbox, TextField};
use barrier_kit::popup_button::PopUpButton;
use barrier_kit::style::{Palette, text};
use barrier_kit::views::feedback::sheet;
use barrier_kit::views::form::{Form, Row, Section};
use barrier_kit_next::tokens::KEY_BINDING_EDITOR;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Direction, Size};
use vow_design::{HEADLINE, SPACE_2};

use crate::config::{Trigger, TriggerAction};
use crate::localize::tr;
use crate::preferences::{trigger_action_title, trigger_placeholder};

// iTermAddTriggerViewController: the selection as a literal expression, an action, its parameter
// and Instant; OK adds it to the session's profile.
#[derive(PartialEq)]
pub struct AddTriggerSheet {
    pub selection: String,
    pub on_done: Callback<Option<Trigger>, ()>,
}

impl Component for AddTriggerSheet {
    fn render(&self) -> impl IntoElement {
        let regex = use_state(|| regex::escape(&self.selection));
        let parameter = use_state(String::new);
        let mut action = use_state(TriggerAction::default);
        let mut instant = use_state(|| false);
        let palette = Palette::current();
        let chosen = *action.read();
        let action_popup = TriggerAction::ALL
            .iter()
            .fold(PopUpButton::new(tr!("trigger-action")), |popup, action| {
                popup.item(trigger_action_title(*action))
            })
            .selected(
                TriggerAction::ALL
                    .iter()
                    .position(|action| *action == chosen)
                    .unwrap_or_default(),
            )
            .on_change(move |choice: usize| action.set(TriggerAction::ALL[choice]));
        let section = Section::new()
            .row(
                Row::new(tr!("trigger-regex"))
                    .control(TextField::new(regex, tr!("trigger-regex")).width(Size::fill())),
            )
            .row(Row::new(tr!("trigger-action")).control(action_popup));
        // A new highlight keeps its default colours here; Settings has the colour pop-ups.
        let typed = chosen.takes_parameter()
            && !matches!(
                chosen,
                TriggerAction::Highlight | TriggerAction::HighlightLine
            );
        let section = if typed {
            section.row(Row::new(tr!("key-binding-parameter")).control(
                TextField::new(parameter, trigger_placeholder(chosen)).width(Size::fill()),
            ))
        } else {
            section
        };
        let section = section.row(
            Row::new(tr!("trigger-instant")).control(
                Checkbox::new(tr!("trigger-instant"))
                    .checked(*instant.read())
                    .on_toggle(move |on| instant.set(on)),
            ),
        );
        let (cancel, confirm) = (self.on_done.clone(), self.on_done.clone());
        let ready = !regex.read().is_empty();
        sheet(
            rect()
                .width(Size::px(KEY_BINDING_EDITOR.width))
                .direction(Direction::Vertical)
                .spacing(SPACE_2)
                .child(text(tr!("add-trigger-title"), HEADLINE, palette.label))
                .child(Form::new().section(section))
                .child(
                    rect()
                        .width(Size::fill())
                        .direction(Direction::Horizontal)
                        .main_align(Alignment::End)
                        .spacing(SPACE_2)
                        .child(
                            Button::new(tr!("close-cancel")).on_press(move |_| cancel.call(None)),
                        )
                        .child(
                            Button::new(tr!("snippet-ok"))
                                .style(ButtonStyle::Primary)
                                .disabled(!ready)
                                .on_press(move |_| {
                                    confirm.call(Some(Trigger {
                                        regex: regex.peek().clone(),
                                        action: *action.peek(),
                                        parameter: parameter.peek().clone(),
                                        instant: *instant.peek(),
                                        ..Trigger::default()
                                    }))
                                }),
                        ),
                ),
        )
    }
}
