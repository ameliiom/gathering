//! What one break takes with it, worked out on positions alone so it can be tested without a
//! world: the blocks joined to the first (all 26 neighbours, as ore veins and branches touch),
//! nearest first, at most a cap.
use std::collections::{HashSet, VecDeque};

pub type Pos = [i32; 3];

/// The 26 positions around `p`.
pub fn around(p: Pos) -> impl Iterator<Item = Pos> {
    (-1..=1)
        .flat_map(move |dx| (-1..=1).flat_map(move |dy| (-1..=1).map(move |dz| [p[0] + dx, p[1] + dy, p[2] + dz])))
        .filter(move |q| *q != p)
}

/// Every position joined to `start` through positions `same` accepts, nearest first (in steps),
/// `start` itself left out, at most `cap`.
pub fn joined(start: Pos, cap: usize, same: impl FnMut(Pos) -> bool) -> Vec<Pos> {
    joined_all(&[start], cap, same)
}

/// As `joined`, walking out from all of `starts` at once (a felled tree's logs, to its leaves).
pub fn joined_all(starts: &[Pos], cap: usize, mut same: impl FnMut(Pos) -> bool) -> Vec<Pos> {
    let mut seen: HashSet<Pos> = starts.iter().copied().collect();
    let mut queue: VecDeque<Pos> = starts.iter().copied().collect();
    let mut found = Vec::new();
    while let Some(p) = queue.pop_front() {
        for q in around(p) {
            if found.len() >= cap {
                return found;
            }
            if seen.insert(q) && same(q) {
                found.push(q);
                queue.push_back(q);
            }
        }
    }
    found
}

/// A tree comes down from the top, as it would fall; the cap already kept the logs nearest the
/// cut, so a tree too big for the player loses its trunk first and can be finished by hand.
pub fn top_down(mut logs: Vec<Pos>) -> Vec<Pos> {
    logs.sort_by_key(|p| std::cmp::Reverse(p[1]));
    logs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(points: &[Pos]) -> HashSet<Pos> {
        points.iter().copied().collect()
    }

    #[test]
    fn a_vein_joins_through_corners_and_stops_at_gaps() {
        // Three ores touching only by corners, and one two blocks away.
        let ores = set(&[[1, 1, 1], [2, 2, 2], [4, 2, 2]]);
        let found = joined([0, 0, 0], 64, |p| ores.contains(&p));
        assert_eq!(found, vec![[1, 1, 1], [2, 2, 2]]);
    }

    #[test]
    fn the_cap_keeps_the_nearest() {
        let line: Vec<Pos> = (1..=100).map(|x| [x, 0, 0]).collect();
        let ores = set(&line);
        let found = joined([0, 0, 0], 8, |p| ores.contains(&p));
        assert_eq!(found, (1..=8).map(|x| [x, 0, 0]).collect::<Vec<_>>());
        assert!(joined([0, 0, 0], 0, |p| ores.contains(&p)).is_empty());
    }

    #[test]
    fn each_block_is_asked_once() {
        let cube: Vec<Pos> = (0..4).flat_map(|x| (0..4).flat_map(move |y| (0..4).map(move |z| [x, y, z]))).collect();
        let ores = set(&cube);
        let mut asked = Vec::new();
        let found = joined([0, 0, 0], 1000, |p| {
            asked.push(p);
            ores.contains(&p)
        });
        assert_eq!(found.len(), 63);
        assert_eq!(asked.len(), set(&asked).len());
    }

    #[test]
    fn many_starts_share_one_walk() {
        // Two felled logs with leaves around and between them: each leaf found once, logs never.
        let leaves = set(&[[0, 2, 0], [1, 2, 0], [2, 2, 0], [5, 5, 5]]);
        let found = joined_all(&[[0, 1, 0], [2, 1, 0]], 100, |p| leaves.contains(&p));
        assert_eq!(set(&found), set(&[[0, 2, 0], [1, 2, 0], [2, 2, 0]]));
        assert_eq!(found.len(), 3);
    }

    #[test]
    fn trees_fall_from_the_top() {
        assert_eq!(top_down(vec![[0, 1, 0], [0, 3, 0], [1, 2, 0]]), vec![[0, 3, 0], [1, 2, 0], [0, 1, 0]]);
    }
}
