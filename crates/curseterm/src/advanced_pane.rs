use barrier_kit::controls::{Checkbox, ScrollView, Switch, TextField};
use barrier_kit::views::form::{Form, Row, Section};
use freya_core::prelude::*;
use torin::prelude::{Direction, Gaps, Size};
use vow_design::{SPACE_2, SPACE_3};

use crate::advanced::{self, Advanced, ENTRIES, Entry, Value};
use crate::config::change_defaults;
use crate::localize::tr;

// PreferencePanel.xib Advanced: the search field, “Show only non-default values” and the settings
// grouped by section, sorted by section and then by description.
#[derive(PartialEq)]
pub struct AdvancedPane {
    pub advanced: Advanced,
}

impl Component for AdvancedPane {
    fn render(&self) -> impl IntoElement {
        let query = use_state(String::new);
        let mut modified_only = use_state(|| false);
        let defaults = Advanced::default();
        let typed = query.read().clone();
        let mut shown: Vec<usize> = (0..ENTRIES.len())
            .filter(|index| advanced::matches(&ENTRIES[*index], &typed))
            .filter(|index| {
                !*modified_only.read() || !same(&ENTRIES[*index], &self.advanced, &defaults)
            })
            .collect();
        shown.sort_by_key(|index| ((ENTRIES[*index].section)(), (ENTRIES[*index].description)()));
        let mut sections: Vec<(&'static str, Section)> = Vec::new();
        for index in shown {
            let entry = &ENTRIES[index];
            let section = (entry.section)();
            let row = Row::new((entry.description)()).control(control(index, &self.advanced));
            match sections.last_mut() {
                Some((title, rows)) if *title == section => {
                    *rows = std::mem::replace(rows, Section::new()).row(row);
                }
                _ => sections.push((section, Section::titled(section).row(row))),
            }
        }
        let form = sections
            .into_iter()
            .fold(Form::new(), |form, (_, section)| form.section(section));
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(Gaps::new_all(SPACE_3))
            .direction(Direction::Vertical)
            .spacing(SPACE_2)
            .child(TextField::new(query, tr!("advanced-search")).width(Size::fill()))
            .child(
                Checkbox::new(tr!("advanced-non-default-only"))
                    .checked(*modified_only.read())
                    .on_toggle(move |on| modified_only.set(on)),
            )
            .child(ScrollView::new().height(Size::flex(1.0)).child(form))
    }
}

fn same(entry: &Entry, advanced: &Advanced, defaults: &Advanced) -> bool {
    match entry.value {
        Value::Switch(get, _) => get(advanced) == get(defaults),
        Value::Number(get, _) => (get(advanced) - get(defaults)).abs() < 0.0001,
        Value::Text(get, _) => get(advanced) == get(defaults),
    }
}

fn control(index: usize, advanced: &Advanced) -> Element {
    let entry = &ENTRIES[index];
    match entry.value {
        Value::Switch(get, set) => Switch::new((entry.description)())
            .on(get(advanced))
            .on_toggle(move |on: bool| change_defaults(|defaults| set(&mut defaults.advanced, on)))
            .into_element(),
        Value::Number(get, _) => AdvancedField {
            index,
            value: get(advanced).to_string(),
        }
        .into_element(),
        Value::Text(get, _) => AdvancedField {
            index,
            value: get(advanced).to_string(),
        }
        .into_element(),
    }
}

// iTermAdvancedSettingsViewController.m -controlTextDidChange:: a number is saved once it parses.
#[derive(PartialEq)]
struct AdvancedField {
    index: usize,
    value: String,
}

impl Component for AdvancedField {
    fn render(&self) -> impl IntoElement {
        let mut draft = use_state(|| self.value.clone());
        let mut saved = use_state(|| self.value.clone());
        if *saved.peek() != self.value {
            saved.set(self.value.clone());
            draft.set(self.value.clone());
        }
        let value = ENTRIES[self.index].value;
        use_side_effect(move || {
            let typed = draft.read().clone();
            if typed == *saved.peek() {
                return;
            }
            match value {
                Value::Number(_, set) => {
                    if let Ok(number) = typed.trim().parse::<f64>() {
                        change_defaults(|defaults| set(&mut defaults.advanced, number));
                    }
                }
                Value::Text(_, set) => {
                    change_defaults(|defaults| set(&mut defaults.advanced, typed.clone()))
                }
                Value::Switch(..) => {}
            }
        });
        TextField::new(draft, (ENTRIES[self.index].description)()).width(Size::fill())
    }
}
