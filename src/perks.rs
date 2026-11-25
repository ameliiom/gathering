//! The comforts earned in the Skills tree Gathering, with no code tie to Skills: an unlocked node
//! gives the player a vanilla tag that Gathering reads, and the tree ships as a data file only
//! Skills reads. Without Skills, the server setting `all_comforts` opens them.

/// Tags `gathering.vein.<n>`: how many more ores one break takes (none: just the one).
pub const VEIN: &str = "gathering.vein.";
/// Tags `gathering.timber.<n>`: how many more logs of a tree one break takes.
pub const TIMBER: &str = "gathering.timber.";
/// Tags `gathering.harvest.<n>`: a right-click harvests an `n`×`n` square of ripe crops.
pub const HARVEST: &str = "gathering.harvest.";
/// A felled tree's leaves fall at once.
pub const LEAVES: &str = "gathering.leaves";
/// What a vein, a tree or a harvest drops goes into the inventory.
pub const BAG: &str = "gathering.bag";
/// The most each may reach, and what `all_comforts` gives.
pub const VEIN_MAX: usize = 64;
pub const TIMBER_MAX: usize = 256;
pub const HARVEST_MAX: usize = 5;

/// The largest `<prefix><n>` among `tags` (0 without any), at most `max`.
pub fn best(tags: &[String], prefix: &str, max: usize) -> usize {
    tags.iter().filter_map(|t| t.strip_prefix(prefix)?.parse::<usize>().ok()).max().unwrap_or(0).min(max)
}

/// One player's comforts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Comforts {
    pub vein: usize,
    pub timber: usize,
    /// How far a harvest reaches around the crop clicked: 0 (one crop), 1 (3×3), 2 (5×5).
    pub reach: i32,
    pub leaves: bool,
    pub bag: bool,
}

impl Comforts {
    pub fn of(tags: &[String], all: bool) -> Self {
        if all {
            return Comforts { vein: VEIN_MAX, timber: TIMBER_MAX, reach: (HARVEST_MAX as i32 - 1) / 2, leaves: true, bag: true };
        }
        let side = best(tags, HARVEST, HARVEST_MAX).max(1) as i32;
        Comforts {
            vein: best(tags, VEIN, VEIN_MAX),
            timber: best(tags, TIMBER, TIMBER_MAX),
            reach: (side - 1) / 2,
            leaves: tags.iter().any(|t| t == LEAVES),
            bag: tags.iter().any(|t| t == BAG),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_biggest_tag_wins_within_bounds() {
        assert_eq!(best(&t(&[]), VEIN, VEIN_MAX), 0);
        assert_eq!(best(&t(&["gathering.vein.8", "gathering.vein.24", "gathering.timber.256"]), VEIN, VEIN_MAX), 24);
        assert_eq!(best(&t(&["gathering.vein.9999", "gathering.vein.x"]), VEIN, VEIN_MAX), 64);
        assert_eq!(best(&t(&["gathering.timber.32"]), TIMBER, TIMBER_MAX), 32);
    }

    #[test]
    fn comforts_read_from_tags() {
        assert_eq!(Comforts::of(&t(&[]), false), Comforts::default());
        let c = Comforts::of(&t(&["gathering.harvest.3", "gathering.bag", "gathering.vein.8"]), false);
        assert_eq!((c.reach, c.bag, c.leaves, c.vein, c.timber), (1, true, false, 8, 0));
        assert_eq!(Comforts::of(&t(&["gathering.harvest.5", "gathering.harvest.3"]), false).reach, 2);
        let all = Comforts::of(&t(&[]), true);
        assert_eq!((all.vein, all.timber, all.reach, all.leaves, all.bag), (64, 256, 2, true, true));
    }

    /// The tree file gives exactly the tags Gathering reads, and every comfort has a node.
    #[test]
    fn the_shipped_tree_gives_exactly_the_tags_gathering_reads() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/gathering/skills/gathering.json");
        let text = std::fs::read_to_string(path).unwrap();
        let given: Vec<String> = text
            .split("\"tag\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').nth(1).map(str::to_owned))
            .collect();
        for tag in &given {
            let counted = [VEIN, TIMBER, HARVEST].iter().any(|p| tag.strip_prefix(p).is_some_and(|n| n.parse::<usize>().is_ok()));
            assert!(counted || tag == LEAVES || tag == BAG, "the tree gives {tag}, which Gathering never reads");
        }
        assert_eq!(Comforts::of(&given, false), Comforts::of(&[], true), "some comfort has no node, or its node stops short");
    }
}
