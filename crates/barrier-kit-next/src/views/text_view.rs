//! Multi-line editable text without a frame, like `NSTextView`: Enter breaks the line and
//! Escape gives the focus back.

use freya_components::cursor_blink::use_cursor_blink;
use freya_core::prelude::*;
use freya_edit::*;
use torin::prelude::{Area, Size};
use vow_design::Font;

#[derive(PartialEq)]
pub struct TextView {
    value: State<String>,
    font: Font,
    ink: Color,
    width: Size,
    auto_focus: bool,
    on_change: Option<Callback<String, ()>>,
    on_focus_change: Option<Callback<bool, ()>>,
    key: DiffKey,
}

impl TextView {
    pub fn new(value: State<String>, font: Font, ink: Color) -> Self {
        Self {
            value,
            font,
            ink,
            width: Size::fill(),
            auto_focus: false,
            on_change: None,
            on_focus_change: None,
            key: DiffKey::None,
        }
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    pub fn auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    pub fn on_change(mut self, callback: impl Into<Callback<String, ()>>) -> Self {
        self.on_change = Some(callback.into());
        self
    }

    pub fn on_focus_change(mut self, callback: impl Into<Callback<bool, ()>>) -> Self {
        self.on_focus_change = Some(callback.into());
        self
    }
}

impl KeyExt for TextView {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for TextView {
    fn render(&self) -> impl IntoElement {
        let a11y_id = use_hook(AccessibilityId::new_unique);
        let focus = use_focus(a11y_id);
        let holder = use_state(ParagraphHolder::default);
        let mut area = use_state(Area::default);
        let mut dragging = use_state(|| false);
        let mut value = self.value;
        let mut editable = use_editable(|| value.read().to_string(), EditableConfig::new);
        let (mut movement, cursor_color) = use_cursor_blink(focus().is_focused(), self.ink);
        let on_focus_change = use_reactive(&self.on_focus_change);
        let mut reported = use_state(|| false);
        use_side_effect(move || {
            let focused = focus.read().is_focused();
            if *reported.peek() != focused {
                reported.set(focused);
                if let Some(callback) = &*on_focus_change.peek() {
                    callback.call(focused);
                }
            }
        });

        if *value.read() != editable.editor().read().committed_text() {
            let mut editor = editable.editor_mut().write();
            editor.clear_preedit();
            editor.set(&value.read());
            editor.editor_history().clear();
            editor.clear_selection();
        }

        let on_change = self.on_change.clone();
        let on_key_down = move |event: Event<KeyboardEventData>| {
            if event.key == Key::Named(NamedKey::Escape) {
                a11y_id.request_unfocus();
                return;
            }
            movement.reset();
            editable.process_event(EditableEvent::KeyDown {
                key: &event.key,
                modifiers: event.modifiers,
            });
            let text = editable.editor().read().committed_text();
            if *value.peek() != text {
                value.set(text.clone());
                if let Some(on_change) = &on_change {
                    on_change.call(text);
                }
            }
        };
        let on_key_up = move |event: Event<KeyboardEventData>| {
            editable.process_event(EditableEvent::KeyUp { key: &event.key });
        };
        let on_ime_preedit = move |event: Event<ImePreeditEventData>| {
            let mut editor = editable.editor_mut().write();
            if event.data().text.is_empty() {
                editor.clear_preedit();
            } else {
                editor.set_preedit(&event.data().text);
            }
        };
        let on_focus_press = move |event: Event<FocusPressEventData>| {
            event.stop_propagation();
            event.prevent_default();
            dragging.set_if_modified(true);
            movement.reset();
            editable.process_event(EditableEvent::Down {
                location: event.element_location(),
                editor_line: EditorLine::SingleParagraph,
                holder: &holder.read(),
            });
            a11y_id.request_focus();
        };
        let on_global_pointer_move = move |event: Event<PointerEventData>| {
            if a11y_id.is_focused() && *dragging.read() {
                let mut location = event.global_location();
                location.x -= area.read().min_x() as f64;
                location.y -= area.read().min_y() as f64;
                editable.process_event(EditableEvent::Move {
                    location,
                    editor_line: EditorLine::SingleParagraph,
                    holder: &holder.read(),
                });
            }
        };
        let on_global_pointer_press = move |_: Event<PointerEventData>| {
            editable.process_event(EditableEvent::Release);
            if *dragging.peek() {
                dragging.set(false);
            } else if a11y_id.is_focused() {
                a11y_id.request_unfocus();
            }
        };

        let focused = focus().is_focused();
        let editor = editable.editor().read();
        let mut paragraph = paragraph()
            .holder(holder.read().clone())
            .on_sized(move |event: Event<SizedEventData>| area.set(event.visible_area))
            .on_focus_press(on_focus_press)
            .width(self.width.clone())
            .font_family(self.font.family)
            .font_size(self.font.size as f32)
            .line_height(self.font.line_height as f32 / self.font.size as f32)
            .color(self.ink)
            .cursor_index(focused.then(|| editor.cursor_pos()))
            .cursor_color(cursor_color)
            .highlights(
                focused
                    .then(|| editor.get_visible_selection(EditorLine::SingleParagraph))
                    .flatten()
                    .map(|selection| vec![selection]),
            );
        paragraph = if editor.has_preedit() {
            let (before, preedit, after) = editor.preedit_text_segments();
            paragraph
                .span(before)
                .span(Span::new(preedit).text_decoration(TextDecoration::Underline))
                .span(after)
        } else {
            paragraph.span(editor.rope().to_string())
        };

        rect()
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_auto_focus(self.auto_focus)
            .a11y_role(AccessibilityRole::MultilineTextInput)
            .on_key_down(on_key_down)
            .on_key_up(on_key_up)
            .on_ime_preedit(on_ime_preedit)
            .on_global_pointer_move(on_global_pointer_move)
            .on_global_pointer_press(on_global_pointer_press)
            .on_pointer_enter(|_| Cursor::set(CursorIcon::Text))
            .on_pointer_leave(|_| Cursor::set(CursorIcon::default()))
            .width(self.width.clone())
            .child(paragraph)
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}
