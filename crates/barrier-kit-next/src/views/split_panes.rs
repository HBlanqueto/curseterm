//! Panes side by side or stacked, with dividers the user drags; nests.

use barrier_kit::style::Palette;
use barrier_kit::theme::color;
use freya_core::prelude::*;
use torin::prelude::{Content, Direction, Size};

use crate::tokens::SPLIT_PANES;

#[derive(PartialEq)]
pub struct SplitPanes {
    label: &'static str,
    direction: Direction,
    panes: Vec<Element>,
    ratios: Vec<f32>,
    on_ratios: Option<Callback<Vec<f32>, ()>>,
}

impl SplitPanes {
    pub fn new(label: &'static str, direction: Direction) -> Self {
        Self {
            label,
            direction,
            panes: Vec::new(),
            ratios: Vec::new(),
            on_ratios: None,
        }
    }

    pub fn pane(mut self, pane: impl IntoElement) -> Self {
        self.panes.push(pane.into_element());
        self
    }

    pub fn ratios(mut self, ratios: Vec<f32>) -> Self {
        self.ratios = ratios;
        self
    }

    pub fn on_ratios(mut self, callback: impl Into<Callback<Vec<f32>, ()>>) -> Self {
        self.on_ratios = Some(callback.into());
        self
    }
}

impl Component for SplitPanes {
    fn render(&self) -> impl IntoElement {
        let mut length = use_state(|| 0.0f32);
        let direction = self.direction;
        let horizontal = direction == Direction::Horizontal;
        let count = self.panes.len();
        let ratios: Vec<f32> = if self.ratios.len() == count {
            self.ratios.clone()
        } else {
            vec![1.0 / count.max(1) as f32; count]
        };
        let along = |ratio: f32| Size::flex(ratio);
        let mut row = rect()
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt(self.label)
            .width(Size::fill())
            .height(Size::fill())
            .direction(direction)
            .content(Content::flex())
            .on_sized(move |event: Event<SizedEventData>| {
                let area = event.data().area;
                length.set_if_modified(if horizontal {
                    area.width()
                } else {
                    area.height()
                });
            });
        for (index, pane) in self.panes.iter().enumerate() {
            if index > 0 {
                let ratios = ratios.clone();
                let on_ratios = self.on_ratios.clone();
                row = row.child(Divider {
                    direction,
                    on_drag: Callback::new(move |delta: f32| {
                        let free = *length.peek() - SPLIT_PANES.divider * (count - 1) as f32;
                        if let (Some(on_ratios), true) = (&on_ratios, free > 0.0) {
                            on_ratios.call(dragged(&ratios, index - 1, delta, free));
                        }
                    }),
                });
            }
            let cell = rect().overflow(Overflow::Clip).child(pane.clone());
            row = row.child(if horizontal {
                cell.width(along(ratios[index])).height(Size::fill())
            } else {
                cell.height(along(ratios[index])).width(Size::fill())
            });
        }
        row
    }
}

fn dragged(ratios: &[f32], divider: usize, delta: f32, free: f32) -> Vec<f32> {
    let total: f32 = ratios.iter().sum();
    let minimum = SPLIT_PANES.pane_min / free * total;
    let shift = delta / free * total;
    let pair = ratios[divider] + ratios[divider + 1];
    let left = (ratios[divider] + shift).clamp(minimum, pair - minimum);
    let mut next = ratios.to_vec();
    next[divider] = left;
    next[divider + 1] = pair - left;
    next
}

#[derive(PartialEq)]
struct Divider {
    direction: Direction,
    on_drag: Callback<f32, ()>,
}

impl Component for Divider {
    fn render(&self) -> impl IntoElement {
        let palette = Palette::current();
        let mut last = use_state(|| None::<f32>);
        let horizontal = self.direction == Direction::Horizontal;
        let on_drag = self.on_drag.clone();
        let along = move |(x, y): (f64, f64)| if horizontal { x } else { y } as f32;
        let icon = if horizontal {
            CursorIcon::ColResize
        } else {
            CursorIcon::RowResize
        };
        let line = rect().background(color(palette.separator));
        let line = if horizontal {
            line.width(Size::px(SPLIT_PANES.divider))
                .height(Size::fill())
        } else {
            line.height(Size::px(SPLIT_PANES.divider))
                .width(Size::fill())
        };
        line.a11y_role(AccessibilityRole::Splitter)
            .on_pointer_enter(move |_| Cursor::set(icon))
            .on_pointer_leave(move |_| {
                if last.peek().is_none() {
                    Cursor::set(CursorIcon::default());
                }
            })
            .on_mouse_down(move |event: Event<MouseEventData>| {
                last.set(Some(along(event.data().global_location.to_tuple())));
            })
            .on_global_pointer_move(move |event: Event<PointerEventData>| {
                let Some(from) = *last.peek() else {
                    return;
                };
                let to = along(event.data().global_location().to_tuple());
                last.set(Some(to));
                on_drag.call(to - from);
            })
            .on_global_pointer_press(move |_| {
                if last.peek().is_some() {
                    last.set(None);
                    Cursor::set(CursorIcon::default());
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::dragged;
    use crate::tokens::SPLIT_PANES;

    #[test]
    fn dragging_moves_only_the_two_panes_beside_the_divider_and_stops_at_the_minimum() {
        let moved = dragged(&[0.25, 0.25, 0.5], 1, 100.0, 1000.0);
        assert!((moved[1] - 0.35).abs() < 1e-6);
        assert!((moved[2] - 0.4).abs() < 1e-6);
        assert_eq!(moved[0], 0.25);
        let stopped = dragged(&[0.5, 0.5], 0, 10_000.0, 1000.0);
        assert!((stopped[1] - SPLIT_PANES.pane_min / 1000.0).abs() < 1e-6);
    }
}
