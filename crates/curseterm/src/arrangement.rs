use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::panes::{Axis, Pane};
use crate::session::SessionId;
use crate::workspace::TabGroup;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arrangement {
    pub name: String,
    pub tabs: Vec<TabLayout>,
    #[serde(default)]
    pub layout_locked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabLayout {
    pub title: Option<String>,
    pub root: Layout,
    #[serde(default)]
    pub group: Option<TabGroup>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Layout {
    Session {
        profile: String,
        directory: Option<PathBuf>,
    },
    Split {
        axis: Axis,
        ratios: Vec<f32>,
        children: Vec<Layout>,
    },
}

pub fn capture(
    pane: &Pane,
    describe: &impl Fn(SessionId) -> Option<(String, Option<PathBuf>)>,
) -> Option<Layout> {
    match pane {
        Pane::Leaf(id) => {
            let (profile, directory) = describe(*id)?;
            Some(Layout::Session { profile, directory })
        }
        Pane::Split {
            axis,
            children,
            ratios,
        } => Some(Layout::Split {
            axis: *axis,
            ratios: ratios.clone(),
            children: children
                .iter()
                .map(|child| capture(child, describe))
                .collect::<Option<_>>()?,
        }),
    }
}

pub fn build(
    layout: &Layout,
    spawn: &mut impl FnMut(&str, Option<PathBuf>) -> Option<SessionId>,
) -> Option<Pane> {
    match layout {
        Layout::Session { profile, directory } => spawn(profile, directory.clone()).map(Pane::Leaf),
        Layout::Split {
            axis,
            ratios,
            children,
        } => {
            let children: Vec<Pane> = children
                .iter()
                .filter_map(|child| build(child, spawn))
                .collect();
            match children.len() {
                0 => None,
                1 => children.into_iter().next(),
                count => Some(Pane::Split {
                    axis: *axis,
                    ratios: if ratios.len() == count {
                        ratios.clone()
                    } else {
                        vec![1.0 / count as f32; count]
                    },
                    children,
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_captured_split_is_rebuilt_with_the_same_shape_profiles_and_directories() {
        let pane = Pane::Split {
            axis: Axis::Row,
            ratios: vec![0.3, 0.7],
            children: vec![
                Pane::Leaf(1),
                Pane::Split {
                    axis: Axis::Column,
                    ratios: vec![0.5, 0.5],
                    children: vec![Pane::Leaf(2), Pane::Leaf(3)],
                },
            ],
        };
        let describe =
            |id: SessionId| Some((format!("p{id}"), Some(PathBuf::from(format!("/d{id}")))));
        let layout = capture(&pane, &describe).expect("captured");
        let text = toml::to_string(&TabLayout {
            title: None,
            root: layout,
            group: None,
        })
        .expect("serialize");
        let back: TabLayout = toml::from_str(&text).expect("deserialize");
        let mut spawned = Vec::new();
        let mut next = 10;
        let rebuilt = build(&back.root, &mut |profile, directory| {
            spawned.push((profile.to_string(), directory));
            next += 1;
            Some(next)
        })
        .expect("built");
        let Pane::Split {
            ratios, children, ..
        } = &rebuilt
        else {
            panic!("not a split");
        };
        assert_eq!(ratios, &vec![0.3, 0.7]);
        assert!(matches!(
            &children[1],
            Pane::Split {
                axis: Axis::Column,
                ..
            }
        ));
        assert_eq!(spawned[2], ("p3".to_string(), Some(PathBuf::from("/d3"))));
    }

    #[test]
    fn a_split_whose_sessions_fail_but_one_collapses_to_that_pane() {
        let layout = Layout::Split {
            axis: Axis::Row,
            ratios: vec![0.5, 0.5],
            children: vec![
                Layout::Session {
                    profile: "falla".into(),
                    directory: None,
                },
                Layout::Session {
                    profile: "vale".into(),
                    directory: None,
                },
            ],
        };
        let rebuilt = build(&layout, &mut |profile, _| (profile == "vale").then_some(7));
        assert_eq!(rebuilt, Some(Pane::Leaf(7)));
    }
}
