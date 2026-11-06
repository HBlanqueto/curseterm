use crate::session::SessionId;

const MIN_RATIO: f32 = 0.1;
const EDGE: f32 = 1e-4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    Up,
    Down,
    Left,
    Right,
}

impl Toward {
    fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Right => Axis::Row,
            Self::Up | Self::Down => Axis::Column,
        }
    }

    fn forward(self) -> bool {
        matches!(self, Self::Right | Self::Down)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pane {
    Leaf(SessionId),
    Split {
        axis: Axis,
        children: Vec<Pane>,
        ratios: Vec<f32>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

const WHOLE: Area = Area {
    x: 0.0,
    y: 0.0,
    width: 1.0,
    height: 1.0,
};

impl Pane {
    pub fn leaves(&self) -> Vec<SessionId> {
        match self {
            Self::Leaf(id) => vec![*id],
            Self::Split { children, .. } => children.iter().flat_map(Self::leaves).collect(),
        }
    }

    pub fn replace(&mut self, old: SessionId, new: SessionId) -> bool {
        match self {
            Self::Leaf(id) if *id == old => {
                *id = new;
                true
            }
            Self::Leaf(_) => false,
            Self::Split { children, .. } => {
                children.iter_mut().any(|child| child.replace(old, new))
            }
        }
    }

    pub fn is_split(&self) -> bool {
        matches!(self, Self::Split { .. })
    }

    pub fn split(&mut self, target: SessionId, new: SessionId, axis: Axis) -> bool {
        match self {
            Self::Leaf(id) if *id == target => {
                *self = Self::Split {
                    axis,
                    children: vec![Self::Leaf(target), Self::Leaf(new)],
                    ratios: vec![0.5, 0.5],
                };
                true
            }
            Self::Leaf(_) => false,
            Self::Split {
                axis: own,
                children,
                ratios,
            } => {
                let sibling = children
                    .iter()
                    .position(|child| *child == Self::Leaf(target));
                if let (true, Some(index)) = (*own == axis, sibling) {
                    let half = ratios[index] / 2.0;
                    ratios[index] = half;
                    ratios.insert(index + 1, half);
                    children.insert(index + 1, Self::Leaf(new));
                    return true;
                }
                children
                    .iter_mut()
                    .any(|child| child.split(target, new, axis))
            }
        }
    }

    // PTYTab.m -arrangeSplitPanesEvenlyInSplitView:
    pub fn arrange_evenly(&mut self) {
        if let Self::Split {
            children, ratios, ..
        } = self
        {
            let share = 1.0 / children.len() as f32;
            ratios.iter_mut().for_each(|ratio| *ratio = share);
            children.iter_mut().for_each(Self::arrange_evenly);
        }
    }

    pub fn remove(&mut self, target: SessionId) -> bool {
        let Self::Split {
            children, ratios, ..
        } = self
        else {
            return false;
        };
        let removed = match children
            .iter()
            .position(|child| *child == Self::Leaf(target))
        {
            Some(index) => {
                children.remove(index);
                ratios.remove(index);
                let total: f32 = ratios.iter().sum();
                ratios.iter_mut().for_each(|ratio| *ratio /= total);
                true
            }
            None => children.iter_mut().any(|child| child.remove(target)),
        };
        if children.len() == 1 {
            *self = children.remove(0);
        }
        removed
    }

    pub fn areas(&self) -> Vec<(SessionId, Area)> {
        let mut out = Vec::new();
        self.collect_areas(WHOLE, &mut out);
        out
    }

    fn collect_areas(&self, area: Area, out: &mut Vec<(SessionId, Area)>) {
        match self {
            Self::Leaf(id) => out.push((*id, area)),
            Self::Split {
                axis,
                children,
                ratios,
            } => {
                let mut offset = 0.0;
                for (child, ratio) in children.iter().zip(ratios) {
                    let part = match axis {
                        Axis::Row => Area {
                            x: area.x + offset * area.width,
                            width: ratio * area.width,
                            ..area
                        },
                        Axis::Column => Area {
                            y: area.y + offset * area.height,
                            height: ratio * area.height,
                            ..area
                        },
                    };
                    child.collect_areas(part, out);
                    offset += ratio;
                }
            }
        }
    }

    pub fn neighbor(&self, from: SessionId, toward: Toward) -> Option<SessionId> {
        let areas = self.areas();
        let (_, origin) = areas.iter().find(|(id, _)| *id == from)?;
        areas
            .iter()
            .filter(|(id, _)| *id != from)
            .filter_map(|(id, area)| {
                let (touches, overlap) = match toward {
                    Toward::Left => (near(area.x + area.width, origin.x), vertical(area, origin)),
                    Toward::Right => (
                        near(origin.x + origin.width, area.x),
                        vertical(area, origin),
                    ),
                    Toward::Up => (
                        near(area.y + area.height, origin.y),
                        horizontal(area, origin),
                    ),
                    Toward::Down => (
                        near(origin.y + origin.height, area.y),
                        horizontal(area, origin),
                    ),
                };
                (touches && overlap > EDGE).then_some((*id, overlap))
            })
            .fold(
                None,
                |best: Option<(SessionId, f32)>, candidate| match best {
                    Some((_, overlap)) if overlap >= candidate.1 => best,
                    _ => Some(candidate),
                },
            )
            .map(|(id, _)| id)
    }

    pub fn cycle(&self, from: SessionId, step: isize) -> SessionId {
        let leaves = self.leaves();
        let index = leaves.iter().position(|id| *id == from).unwrap_or(0) as isize;
        leaves[(index + step).rem_euclid(leaves.len() as isize) as usize]
    }

    pub fn move_divider(&mut self, target: SessionId, toward: Toward, amount: f32) -> bool {
        self.divider_search(target, toward, amount) == Search::Moved
    }

    fn divider_search(&mut self, target: SessionId, toward: Toward, amount: f32) -> Search {
        let Self::Split {
            axis,
            children,
            ratios,
        } = self
        else {
            return if *self == Self::Leaf(target) {
                Search::Inside
            } else {
                Search::Absent
            };
        };
        let found = children.iter_mut().enumerate().find_map(|(index, child)| {
            match child.divider_search(target, toward, amount) {
                Search::Absent => None,
                other => Some((index, other)),
            }
        });
        match found {
            None => Search::Absent,
            Some((_, Search::Moved)) => Search::Moved,
            Some((index, _)) if *axis == toward.axis() => {
                let divider = if index + 1 < children.len() {
                    index
                } else {
                    index - 1
                };
                let delta = if toward.forward() { amount } else { -amount };
                let (left, right) = (ratios[divider] + delta, ratios[divider + 1] - delta);
                if left >= MIN_RATIO && right >= MIN_RATIO {
                    ratios[divider] = left;
                    ratios[divider + 1] = right;
                }
                Search::Moved
            }
            Some(_) => Search::Inside,
        }
    }

    pub fn ratios_at(&mut self, path: &[usize]) -> Option<&mut Vec<f32>> {
        let Self::Split {
            children, ratios, ..
        } = self
        else {
            return None;
        };
        match path.split_first() {
            None => Some(ratios),
            Some((first, rest)) => children.get_mut(*first)?.ratios_at(rest),
        }
    }
}

#[derive(PartialEq)]
enum Search {
    Absent,
    Inside,
    Moved,
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < EDGE
}

fn vertical(a: &Area, b: &Area) -> f32 {
    (a.y + a.height).min(b.y + b.height) - a.y.max(b.y)
}

fn horizontal(a: &Area, b: &Area) -> f32 {
    (a.x + a.width).min(b.x + b.width) - a.x.max(b.x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_left_two_stacked_right() -> Pane {
        let mut pane = Pane::Leaf(1);
        pane.split(1, 2, Axis::Row);
        pane.split(2, 3, Axis::Column);
        pane
    }

    #[test]
    fn splitting_along_the_parent_axis_adds_a_sibling_and_halves_the_target() {
        let mut pane = Pane::Leaf(1);
        pane.split(1, 2, Axis::Row);
        pane.split(1, 3, Axis::Row);
        assert_eq!(
            pane,
            Pane::Split {
                axis: Axis::Row,
                children: vec![Pane::Leaf(1), Pane::Leaf(3), Pane::Leaf(2)],
                ratios: vec![0.25, 0.25, 0.5],
            }
        );
    }

    #[test]
    fn removing_a_pane_collapses_a_split_left_with_one_child() {
        let mut pane = one_left_two_stacked_right();
        assert!(pane.remove(3));
        assert_eq!(pane.leaves(), vec![1, 2]);
        assert!(pane.remove(2));
        assert_eq!(pane, Pane::Leaf(1));
    }

    #[test]
    fn neighbours_follow_the_geometry() {
        let pane = one_left_two_stacked_right();
        assert_eq!(pane.neighbor(1, Toward::Right), Some(2));
        assert_eq!(pane.neighbor(2, Toward::Down), Some(3));
        assert_eq!(pane.neighbor(3, Toward::Left), Some(1));
        assert_eq!(pane.neighbor(1, Toward::Left), None);
        assert_eq!(pane.cycle(3, 1), 1);
        assert_eq!(pane.cycle(1, -1), 3);
    }

    #[test]
    fn moving_a_divider_uses_the_nearest_split_on_that_axis_and_keeps_a_minimum() {
        let mut pane = one_left_two_stacked_right();
        assert!(pane.move_divider(3, Toward::Left, 0.2));
        assert!(pane.move_divider(3, Toward::Up, 0.1));
        let Pane::Split {
            ratios, children, ..
        } = &pane
        else {
            panic!("root is a split");
        };
        assert!((ratios[0] - 0.3).abs() < 1e-6);
        let Pane::Split { ratios: inner, .. } = &children[1] else {
            panic!("right side is a split");
        };
        assert!((inner[0] - 0.4).abs() < 1e-6);
        assert!(pane.move_divider(1, Toward::Left, 0.5));
        assert!(
            pane.ratios_at(&[])
                .is_some_and(|ratios| ratios[0] >= MIN_RATIO)
        );
    }
}
