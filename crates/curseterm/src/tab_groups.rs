// iTermTabGroupOrdering.swift and PseudoTerminal.m tab-group logic over the tabs' group ids,
// in tab order (None = ungrouped).

// -selectTabGroupByOffset: steps like Next/Previous Tab, skipping the rest of the current
// tab's group, wrapping around.
pub fn tab_group_step(groups: &[Option<u64>], selected: usize, offset: isize) -> Option<usize> {
    let count = groups.len() as isize;
    if count < 2 || selected as isize >= count {
        return None;
    }
    let step = |index: isize| (index + offset + count).rem_euclid(count);
    let mut target = step(selected as isize);
    if let Some(group) = groups[selected] {
        while target != selected as isize && groups[target as usize] == Some(group) {
            target = step(target);
        }
    }
    Some(target as usize)
}

// -indexOfNearestTabOutsideGroup:collapsed:group:: the visible tab nearest any member, the later
// one on a tie; None refuses the collapse.
pub fn nearest_outside(groups: &[Option<u64>], hidden: &[bool], group: u64) -> Option<usize> {
    let members: Vec<usize> = (0..groups.len())
        .filter(|index| groups[*index] == Some(group))
        .collect();
    (0..groups.len())
        .filter(|index| groups[*index] != Some(group) && !hidden[*index])
        .map(|index| {
            let distance = members.iter().map(|member| member.abs_diff(index)).min();
            (distance.unwrap_or(usize::MAX), index)
        })
        .fold(None, |best: Option<(usize, usize)>, found| match best {
            Some(kept) if kept.0 < found.0 => Some(kept),
            _ => Some(found),
        })
        .map(|(_, index)| index)
}

#[derive(Debug, PartialEq)]
pub enum TabMove {
    Reorder(Vec<usize>),
    Leave,
    Join(u64),
}

// -singleTabMoveForGroupIDs:pinned:selectedIndex:offset: (CurseTerm has no pinned tabs): a
// member leaves its group at the edge, an ungrouped tab joins the group it bumps into, a sole
// member jumps the next unit, and an end tab wraps.
pub fn single_tab_move(groups: &[Option<u64>], selected: usize, offset: isize) -> TabMove {
    let count = groups.len();
    let identity: Vec<usize> = (0..count).collect();
    if count < 2 || selected >= count || offset == 0 {
        return TabMove::Reorder(identity);
    }
    let right = offset > 0;
    let neighbor = (selected as isize + if right { 1 } else { -1 }) as usize;
    let inside = neighbor < count;
    let group = groups[selected];
    let sole = group
        .is_some_and(|group| groups.iter().filter(|other| **other == Some(group)).count() == 1);
    let wrapped = || {
        let mut order = identity.clone();
        order.remove(selected);
        order.insert(if right { 0 } else { count - 1 }, selected);
        TabMove::Reorder(order)
    };
    let swapped = || {
        let mut order = identity.clone();
        order.swap(selected, neighbor);
        TabMove::Reorder(order)
    };
    match group {
        Some(group) if !sole => {
            if inside && groups[neighbor] == Some(group) {
                swapped()
            } else {
                TabMove::Leave
            }
        }
        Some(_) => {
            if !inside {
                return wrapped();
            }
            let mut order = identity.clone();
            order.remove(selected);
            let at = match groups[neighbor] {
                Some(next) if right => {
                    let mut end = neighbor;
                    while end < count && groups[end] == Some(next) {
                        end += 1;
                    }
                    end - 1
                }
                Some(next) => {
                    let mut start = neighbor as isize;
                    while start >= 0 && groups[start as usize] == Some(next) {
                        start -= 1;
                    }
                    (start + 1) as usize
                }
                None => neighbor,
            };
            order.insert(at, selected);
            TabMove::Reorder(order)
        }
        None if !inside => wrapped(),
        None => match groups[neighbor] {
            Some(next) => TabMove::Join(next),
            None => swapped(),
        },
    }
}

// -tabsDidReorder: each group's members follow its first member, so a group stays one run.
pub fn contiguous_order(groups: &[Option<u64>]) -> Vec<usize> {
    let mut order = Vec::new();
    for index in 0..groups.len() {
        if order.contains(&index) {
            continue;
        }
        match groups[index] {
            Some(group) => {
                order.extend((index..groups.len()).filter(|other| groups[*other] == Some(group)))
            }
            None => order.push(index),
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Option<u64> = Some(1);
    const B: Option<u64> = Some(2);

    #[test]
    fn next_tab_group_skips_the_rest_of_the_current_group_and_wraps() {
        let groups = [None, A, A, None, B];
        assert_eq!(tab_group_step(&groups, 1, 1), Some(3));
        assert_eq!(tab_group_step(&groups, 2, -1), Some(0));
        assert_eq!(tab_group_step(&groups, 4, 1), Some(0));
    }

    #[test]
    fn collapsing_moves_to_the_nearest_visible_tab_preferring_the_later_one() {
        let groups = [None, A, A, None];
        assert_eq!(nearest_outside(&groups, &[false; 4], 1), Some(3));
        assert_eq!(nearest_outside(&[A, A], &[false; 2], 1), None);
        assert_eq!(nearest_outside(&[B, A], &[true, false], 1), None);
    }

    #[test]
    fn moving_a_tab_enters_and_leaves_groups_at_their_edges() {
        let groups = [None, A, A, None];
        assert_eq!(single_tab_move(&groups, 0, 1), TabMove::Join(1));
        assert_eq!(single_tab_move(&groups, 2, 1), TabMove::Leave);
        assert_eq!(
            single_tab_move(&groups, 1, 1),
            TabMove::Reorder(vec![0, 2, 1, 3])
        );
        assert_eq!(
            single_tab_move(&[B, A, A], 0, 1),
            TabMove::Reorder(vec![1, 2, 0])
        );
        assert_eq!(
            single_tab_move(&[None, None], 1, 1),
            TabMove::Reorder(vec![1, 0])
        );
    }

    #[test]
    fn a_group_split_by_another_tab_becomes_one_run_again() {
        assert_eq!(contiguous_order(&[A, None, A, B]), vec![0, 2, 1, 3]);
    }
}
