use barrier_kit::compact::IconButton;
use barrier_kit::controls::{Button, ButtonStyle, TextField};
use barrier_kit::popup_button::PopUpButton;
use barrier_kit::style::{Palette, text};
use barrier_kit::symbol::Symbol;
use barrier_kit::theme::color;
use barrier_kit::views::feedback::sheet;
use barrier_kit::views::form::{Form, Row, Section};
use barrier_kit::views::table::Table;
use barrier_kit_next::tokens::KEY_BINDING_EDITOR;
use freya_core::prelude::*;
use torin::prelude::{Alignment, Direction, Gaps, Size};
use vow_design::{CAPTION, HEADLINE, SPACE_1, SPACE_2, TEXT};

use crate::config::change_defaults;
use crate::key_bindings::{
    Binding, BindingAction, NamedAction, Parameter, Preset, combo, load_preset,
    preset_needs_question, shown,
};
use crate::localize::tr;
use crate::preferences::intern;

#[derive(Clone, Copy, PartialEq)]
pub enum Owner {
    Global,
    Profile(usize),
    // iTermActionsEditingViewController: the same editor, with a title where the shortcut goes.
    Actions,
}

impl Owner {
    fn change(self, change: impl FnOnce(&mut Vec<Binding>)) {
        change_defaults(|defaults| match self {
            Owner::Global => change(&mut defaults.key_bindings),
            Owner::Profile(index) => {
                defaults.ensure_profiles();
                if let Some(profile) = defaults.profiles.get_mut(index) {
                    change(&mut profile.key_mappings);
                }
            }
            Owner::Actions => {
                let mut bindings = defaults.actions.iter().map(NamedAction::binding).collect();
                change(&mut bindings);
                defaults.actions = bindings
                    .into_iter()
                    .map(NamedAction::from_binding)
                    .collect();
            }
        });
    }

    fn first_column(self) -> &'static str {
        match self {
            Owner::Actions => tr!("action-title"),
            _ => tr!("key-binding-shortcut"),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct KeyBindingsEditor {
    pub owner: Owner,
    pub bindings: Vec<Binding>,
    pub profiles: Vec<String>,
    pub snippets: Vec<String>,
    pub menu_items: Vec<(String, String)>,
}

// PreferencePanel.xib Keys › Key Bindings and the profile's Keys › Key Mappings: the table,
// + and − and Edit.
impl Component for KeyBindingsEditor {
    fn render(&self) -> impl IntoElement {
        let mut selected = use_state(|| None::<usize>);
        let editing = use_state(|| None::<Option<usize>>);
        let draft = use_state(Binding::default);
        let pending = use_state(|| None::<Preset>);
        let presets_shown = use_state(|| 0_u32);
        let owner = self.owner;
        let chosen = selected.read().filter(|index| *index < self.bindings.len());
        let rows = self
            .bindings
            .iter()
            .map(|binding| {
                let first = match owner {
                    Owner::Actions => binding.combo.clone(),
                    _ => shown(&binding.combo),
                };
                vec![first, describe(binding, &self.menu_items)]
            })
            .collect();
        let open = move |index: Option<usize>, binding: Binding| {
            let (mut draft, mut editing) = (draft, editing);
            draft.set(binding);
            editing.set(Some(index));
        };
        let edited = chosen.and_then(|index| self.bindings.get(index).cloned());
        rect()
            .width(Size::fill())
            .direction(Direction::Vertical)
            .spacing(SPACE_1)
            .child(
                rect()
                    .height(Size::px(KEY_BINDING_EDITOR.table_height))
                    .child(
                        Table::new(
                            tr!("key-bindings"),
                            vec![owner.first_column(), tr!("key-binding-action")],
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
                        IconButton::new(Symbol::Plus, tr!("key-binding-add"))
                            .on_press(move |_| open(None, Binding::default())),
                    )
                    .child(
                        IconButton::new(Symbol::Minus, tr!("key-binding-remove"))
                            .disabled(chosen.is_none())
                            .on_press(move |_| {
                                if let Some(index) = chosen {
                                    owner.change(|bindings| {
                                        if index < bindings.len() {
                                            bindings.remove(index);
                                        }
                                    });
                                    selected.set(None);
                                }
                            }),
                    )
                    .child(
                        Button::new(tr!("key-binding-edit"))
                            .disabled(edited.is_none())
                            .on_press(move |_| {
                                if let Some(binding) = edited.clone() {
                                    open(chosen, binding);
                                }
                            }),
                    )
                    .maybe_child(presets_popup(owner, &self.bindings, pending, presets_shown)),
            )
            .maybe_child((*pending.read()).map(|preset| preset_question(owner, preset, pending)))
            .maybe_child((*editing.read()).map(|index| {
                edit_sheet(self, draft, index, move || {
                    let mut editing = editing;
                    editing.set(None);
                })
            }))
    }
}

fn describe(binding: &Binding, menu_items: &[(String, String)]) -> String {
    let name = binding.action.name();
    match binding.action.parameter() {
        Parameter::None if binding.action == BindingAction::Sequence => binding
            .steps
            .iter()
            .map(|step| describe(step, menu_items))
            .collect::<Vec<_>>()
            .join(tr!("binding-sequence-separator")),
        Parameter::None => name.to_string(),
        Parameter::MenuItem => {
            let label = menu_items
                .iter()
                .find(|(id, _)| *id == binding.parameter)
                .map_or(binding.parameter.as_str(), |(_, label)| label.as_str());
            format!("{name} {label}")
        }
        _ if binding.parameter.is_empty() => name.to_string(),
        _ => format!("{name} {}", binding.parameter),
    }
}

fn options(editor: &KeyBindingsEditor, action: BindingAction) -> Vec<(String, String)> {
    let same = |names: &[String]| -> Vec<(String, String)> {
        names
            .iter()
            .map(|name| (name.clone(), name.clone()))
            .collect()
    };
    match action.parameter() {
        Parameter::Profile => same(&editor.profiles),
        Parameter::Snippet => same(&editor.snippets),
        Parameter::MenuItem => editor.menu_items.clone(),
        Parameter::None | Parameter::Text => Vec::new(),
    }
}

// OK saves the first choice of a pop-up nobody touched.
fn with_default_parameter(editor: &KeyBindingsEditor, mut binding: Binding) -> Binding {
    if binding.parameter.is_empty()
        && let Some((first, _)) = options(editor, binding.action).first()
    {
        binding.parameter = first.clone();
    }
    binding.steps = binding
        .steps
        .into_iter()
        .map(|step| with_default_parameter(editor, step))
        .collect();
    binding
}

// iTermEditKeyActionWindowController.m -groups:: Composition (Sequence…) only for the binding
// itself, not for one of its steps.
fn action_controls(
    editor: &KeyBindingsEditor,
    draft: State<Binding>,
    primary: bool,
) -> (Element, Option<Element>) {
    let binding = draft.read().clone();
    let mut actions = draft;
    let group = binding.action.group();
    let mut grouped = draft;
    let groups = BindingAction::GROUPS.len() - usize::from(!primary);
    let group_popup = (0..groups)
        .fold(
            PopUpButton::new(tr!("key-binding-group")),
            |button, group| button.item(BindingAction::group_name(group)),
        )
        .selected(group)
        .on_change(move |position: usize| {
            let mut binding = grouped.write();
            binding.action = BindingAction::GROUPS[position][0];
            binding.parameter.clear();
        });
    let members = BindingAction::GROUPS[group];
    let action_popup = members
        .iter()
        .fold(
            PopUpButton::new(tr!("key-binding-action")),
            |button, action| button.item(action.name()),
        )
        .selected(
            members
                .iter()
                .position(|action| *action == binding.action)
                .unwrap_or_default(),
        )
        .on_change(move |position: usize| {
            let mut binding = actions.write();
            binding.action = members[position];
            binding.parameter.clear();
        });
    let action_popups = rect()
        .direction(Direction::Horizontal)
        .cross_align(Alignment::Center)
        .spacing(SPACE_1)
        .child(group_popup)
        .child(action_popup)
        .into_element();
    let options = options(editor, binding.action);
    let choices = |names: Vec<(String, String)>| {
        let mut choice = draft;
        let ids: Vec<String> = names.iter().map(|(id, _)| id.clone()).collect();
        let current = ids
            .iter()
            .position(|id| *id == binding.parameter)
            .unwrap_or_default();
        names
            .into_iter()
            .fold(
                PopUpButton::new(tr!("key-binding-parameter")),
                |button, (_, name)| button.item(intern(&name)),
            )
            .selected(current)
            .on_change(move |position: usize| {
                if let Some(id) = ids.get(position) {
                    choice.write().parameter = id.clone();
                }
            })
            .into_element()
    };
    let parameter = match binding.action.parameter() {
        Parameter::None => None,
        Parameter::Text => Some(ParameterText { draft }.into_element()),
        Parameter::MenuItem => Some(
            MenuItemChooser {
                draft,
                items: options,
            }
            .into_element(),
        ),
        Parameter::Profile | Parameter::Snippet => Some(choices(options)),
    };
    (action_popups, parameter)
}

// iTermEditKeyActionWindowController.xib: Keyboard Shortcut, Action and its parameter.
fn edit_sheet(
    editor: &KeyBindingsEditor,
    draft: State<Binding>,
    index: Option<usize>,
    close: impl Fn() + Clone + 'static,
) -> Element {
    let owner = editor.owner;
    let palette = Palette::current();
    let binding = draft.read().clone();
    let (action_popups, parameter) = action_controls(editor, draft, true);
    let steps = (binding.action == BindingAction::Sequence).then(|| {
        Steps {
            draft,
            editor: editor.clone(),
        }
        .into_element()
    });
    let cancel = close.clone();
    let ready = !binding.combo.is_empty();
    let saving = editor.clone();
    sheet(
        rect()
            .width(Size::px(KEY_BINDING_EDITOR.width))
            .direction(Direction::Vertical)
            .spacing(SPACE_2)
            .child(text(
                match owner {
                    Owner::Actions => tr!("action-sheet-title"),
                    _ => tr!("key-binding-sheet-title"),
                },
                HEADLINE,
                palette.label,
            ))
            .child(Form::new().section({
                let first = match owner {
                    Owner::Actions => ActionTitle { draft }.into_element(),
                    _ => Recorder { draft }.into_element(),
                };
                let section = Section::new()
                    .row(Row::new(owner.first_column()).control(first))
                    .row(Row::new(tr!("key-binding-action")).control(action_popups));
                match parameter {
                    Some(control) => {
                        section.row(Row::new(tr!("key-binding-parameter")).control(control))
                    }
                    None => section,
                }
            }))
            .maybe_child(steps)
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
                            .disabled(!ready)
                            .on_press(move |_| {
                                let mut binding = draft.peek().clone();
                                if binding.action != BindingAction::Sequence {
                                    binding.steps.clear();
                                }
                                let binding = with_default_parameter(&saving, binding);
                                owner.change(|bindings| match index {
                                    Some(index) if index < bindings.len() => {
                                        bindings[index] = binding;
                                    }
                                    _ => bindings.push(binding),
                                });
                                close();
                            }),
                    ),
            ),
    )
    .into_element()
}

// KeyActionSequenceTableViewController.swift: the steps with + and −; the selected one is edited
// below, and a new one is Next Tab, after the selection.
#[derive(PartialEq)]
struct Steps {
    draft: State<Binding>,
    editor: KeyBindingsEditor,
}

impl Component for Steps {
    fn render(&self) -> impl IntoElement {
        let selected = use_state(|| None::<usize>);
        let step = use_state(Binding::default);
        let mut draft = self.draft;
        use_side_effect(move || {
            let edited = step.read().clone();
            if let Some(index) = *selected.peek()
                && draft
                    .peek()
                    .steps
                    .get(index)
                    .is_some_and(|kept| *kept != edited)
            {
                draft.write().steps[index] = edited;
            }
        });
        let steps = draft.read().steps.clone();
        let chosen = selected.read().filter(|index| *index < steps.len());
        let rows = steps
            .iter()
            .map(|step| vec![describe(step, &self.editor.menu_items)])
            .collect();
        let pick = move |index: Option<usize>| {
            let (mut selected, mut step) = (selected, step);
            if let Some(found) = index.and_then(|index| draft.peek().steps.get(index).cloned()) {
                step.set(found);
            }
            selected.set(index);
        };
        let controls = chosen.map(|index| {
            let (action_popups, parameter) = action_controls(&self.editor, step, false);
            rect()
                .key(index)
                .width(Size::fill())
                .direction(Direction::Vertical)
                .spacing(SPACE_1)
                .child(action_popups)
                .maybe_child(parameter)
        });
        rect()
            .width(Size::fill())
            .direction(Direction::Vertical)
            .spacing(SPACE_1)
            .child(
                rect()
                    .height(Size::px(KEY_BINDING_EDITOR.steps_height))
                    .child(
                        Table::new(
                            tr!("binding-sequence-steps"),
                            vec![tr!("key-binding-action")],
                        )
                        .rows(rows)
                        .selected(chosen)
                        .on_select(move |row| pick(Some(row))),
                    ),
            )
            .child(
                rect()
                    .direction(Direction::Horizontal)
                    .spacing(SPACE_1)
                    .child(
                        IconButton::new(Symbol::Plus, tr!("key-binding-add")).on_press(move |_| {
                            let at = chosen.map_or(draft.peek().steps.len(), |index| index + 1);
                            draft.write().steps.insert(
                                at,
                                Binding {
                                    action: BindingAction::NextTab,
                                    ..Binding::default()
                                },
                            );
                            pick(Some(at));
                        }),
                    )
                    .child(
                        IconButton::new(Symbol::Minus, tr!("key-binding-remove"))
                            .disabled(chosen.is_none())
                            .on_press(move |_| {
                                if let Some(index) = chosen {
                                    draft.write().steps.remove(index);
                                    pick(None);
                                }
                            }),
                    ),
            )
            .maybe_child(controls)
    }
}

// iTermKeyMappingViewController.xib: the Presets… pop-up under the table; Actions has none.
fn presets_popup(
    owner: Owner,
    bindings: &[Binding],
    pending: State<Option<Preset>>,
    mut shown: State<u32>,
) -> Option<Element> {
    let presets: &'static [Preset] = match owner {
        Owner::Global => &Preset::GLOBAL,
        Owner::Profile(_) => &Preset::PROFILE,
        Owner::Actions => return None,
    };
    let kept = bindings.to_vec();
    let popup = presets
        .iter()
        .fold(
            PopUpButton::new(tr!("key-binding-presets")).item(tr!("key-binding-presets")),
            |button, preset| button.item(preset.name()),
        )
        .selected(0)
        .on_change(move |position: usize| {
            *shown.write() += 1;
            let Some(preset) = position.checked_sub(1).and_then(|index| presets.get(index)) else {
                return;
            };
            if preset_needs_question(&kept, *preset) {
                let mut pending = pending;
                pending.set(Some(*preset));
            } else {
                owner.change(|bindings| load_preset(bindings, *preset, true));
            }
        });
    // BK-042: the kit pop-up keeps its last choice, so a new key brings back «Presets…».
    Some(rect().key(*shown.read()).child(popup).into_element())
}

// iTermKeyMappingViewController.m -removeBeforeLoadingWithTitle:: Keep, Remove or Cancel.
fn preset_question(owner: Owner, preset: Preset, pending: State<Option<Preset>>) -> Element {
    let palette = Palette::current();
    let answer = move |replace_all: Option<bool>| {
        move |_| {
            let mut pending = pending;
            pending.set(None);
            if let Some(replace_all) = replace_all {
                owner.change(|bindings| load_preset(bindings, preset, replace_all));
            }
        }
    };
    sheet(
        rect()
            .a11y_role(AccessibilityRole::AlertDialog)
            .a11y_alt(tr!("preset-load-heading"))
            .width(Size::px(KEY_BINDING_EDITOR.width))
            .direction(Direction::Vertical)
            .spacing(SPACE_2)
            .child(text(tr!("preset-load-heading"), HEADLINE, palette.label))
            .child(
                text(
                    tr!("preset-remove-before-loading"),
                    TEXT,
                    palette.secondary_label,
                )
                .width(Size::fill())
                .max_lines(None),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .direction(Direction::Horizontal)
                    .main_align(Alignment::End)
                    .spacing(SPACE_2)
                    .child(Button::new(tr!("close-cancel")).on_press(answer(None)))
                    .child(Button::new(tr!("preset-remove")).on_press(answer(Some(true))))
                    .child(
                        Button::new(tr!("preset-keep"))
                            .style(ButtonStyle::Primary)
                            .on_press(answer(Some(false))),
                    ),
            ),
    )
    .into_element()
}

// iTermSearchableComboView: the menu items narrowed by what is typed, since a long kit pop-up
// does not scroll (BK-040).
#[derive(PartialEq)]
struct MenuItemChooser {
    draft: State<Binding>,
    items: Vec<(String, String)>,
}

impl Component for MenuItemChooser {
    fn render(&self) -> impl IntoElement {
        let filter = use_state(String::new);
        let mut binding = self.draft;
        let typed = filter.read().to_lowercase();
        let chosen = binding.read().parameter.clone();
        let mut shown: Vec<(String, String)> = self
            .items
            .iter()
            .filter(|(id, label)| *id == chosen || label.to_lowercase().contains(&typed))
            .take(KEY_BINDING_EDITOR.menu_matches)
            .cloned()
            .collect();
        if shown.is_empty() {
            shown = self.items.iter().take(1).cloned().collect();
        }
        let ids: Vec<String> = shown.iter().map(|(id, _)| id.clone()).collect();
        let selected = ids.iter().position(|id| *id == chosen).unwrap_or_default();
        // What the pop-up shows is what OK saves, also before it is touched.
        use_side_effect({
            let items = self.items.clone();
            move || {
                let typed = filter.read().to_lowercase();
                let chosen = binding.peek().parameter.clone();
                let matches = |label: &String| label.to_lowercase().contains(&typed);
                let kept = items
                    .iter()
                    .any(|(id, label)| *id == chosen && matches(label));
                if let Some((first, _)) = items.iter().find(|(_, label)| matches(label))
                    && !kept
                {
                    binding.write().parameter = first.clone();
                }
            }
        });
        rect()
            .width(Size::fill())
            .direction(Direction::Vertical)
            .spacing(SPACE_1)
            .child(TextField::new(filter, tr!("key-binding-menu-filter")).width(Size::fill()))
            .child(
                shown
                    .iter()
                    .fold(
                        PopUpButton::new(tr!("key-binding-parameter")),
                        |button, (_, label)| button.item(intern(label)),
                    )
                    .selected(selected)
                    .on_change(move |position: usize| {
                        if let Some(id) = ids.get(position) {
                            binding.write().parameter = id.clone();
                        }
                    }),
            )
    }
}

#[derive(PartialEq)]
struct ParameterText {
    draft: State<Binding>,
}

impl Component for ParameterText {
    fn render(&self) -> impl IntoElement {
        let mut binding = self.draft;
        let value = use_state(|| binding.peek().parameter.clone());
        use_side_effect(move || {
            let typed = value.read().clone();
            if binding.peek().parameter != typed {
                binding.write().parameter = typed;
            }
        });
        TextField::new(value, tr!("key-binding-text-placeholder")).width(Size::fill())
    }
}

#[derive(PartialEq)]
struct ActionTitle {
    draft: State<Binding>,
}

impl Component for ActionTitle {
    fn render(&self) -> impl IntoElement {
        let mut binding = self.draft;
        let value = use_state(|| binding.peek().combo.clone());
        use_side_effect(move || {
            let typed = value.read().trim().to_string();
            if binding.peek().combo != typed {
                binding.write().combo = typed;
            }
        });
        TextField::new(value, tr!("action-title")).width(Size::fill())
    }
}

// iTermShortcutInputView: click it, then press the shortcut.
#[derive(PartialEq)]
struct Recorder {
    draft: State<Binding>,
}

impl Component for Recorder {
    fn render(&self) -> impl IntoElement {
        let a11y_id = use_hook(AccessibilityId::new_unique);
        let focus = use_focus(a11y_id);
        let focused = focus.read().is_focused();
        let mut draft = self.draft;
        let palette = Palette::current();
        let recorded = draft.read().combo.clone();
        let label = match (focused, recorded.is_empty()) {
            (true, _) => tr!("key-binding-recording").to_string(),
            (false, true) => tr!("key-binding-click-to-set").to_string(),
            (false, false) => shown(&recorded),
        };
        rect()
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::TextInput)
            .a11y_alt(tr!("key-binding-shortcut"))
            .width(Size::fill())
            .padding(Gaps::new(SPACE_1, SPACE_2, SPACE_1, SPACE_2))
            .corner_radius(SPACE_1)
            .background(color(palette.control_background))
            .border(
                Border::new()
                    .width(if focused { 2.0 } else { vow_design::HAIRLINE })
                    .fill(color(if focused {
                        palette.accent
                    } else {
                        palette.control_border
                    })),
            )
            .on_press(move |_| a11y_id.request_focus())
            .on_key_down(move |event: Event<KeyboardEventData>| {
                let data = event.data();
                if let Some(recorded) = combo(&data.key, data.modifiers) {
                    event.stop_propagation();
                    draft.write().combo = recorded;
                    a11y_id.request_unfocus();
                }
            })
            .child(text(
                label,
                if focused { CAPTION } else { TEXT },
                palette.label,
            ))
    }
}
