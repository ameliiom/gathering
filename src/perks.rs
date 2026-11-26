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
/// Mined ores drop their smelted result and the furnace's experience.
pub const SMELT: &str = "gathering.smelt";
/// Tags `gathering.ranch.<n>`: feeding or shearing one animal does the same kind within `n` blocks.
pub const RANCH: &str = "gathering.ranch.";
/// Tags `gathering.plant.<n>`: seeds on farmland plant the joined empty farmland in an `n`×`n`
/// square; `gathering.plant.field` the whole field (up to `FIELD_MAX` blocks).
pub const PLANT: &str = "gathering.plant.";
pub const PLANT_FIELD: &str = "gathering.plant.field";
/// A bite lands the catch in the inventory and brings the bobber back.
pub const FISHING: &str = "gathering.fishing";
/// Sapling items lying on grass or dirt plant themselves.
pub const SAPLINGS: &str = "gathering.saplings";
/// The most each may reach, and what `all_comforts` gives.
pub const VEIN_MAX: usize = 64;
pub const TIMBER_MAX: usize = 256;
pub const HARVEST_MAX: usize = 5;
pub const RANCH_MAX: usize = 7;
pub const PLANT_MAX: usize = 5;
/// The most blocks one planting sows ("the whole field").
pub const FIELD_MAX: usize = 256;
/// How far a planting reaches with `gathering.plant.field`: as far as the field goes.
pub const FIELD_REACH: i32 = i32::MAX;

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
    pub smelt: bool,
    /// How far feeding and shearing reach around the animal clicked, in blocks (0: just it).
    pub ranch: i32,
    /// How far a planting reaches around the farmland clicked: 0 (none), 1 (3×3), 2 (5×5),
    /// `FIELD_REACH` (the whole field).
    pub plant: i32,
    pub fishing: bool,
    pub saplings: bool,
}

impl Comforts {
    pub fn of(tags: &[String], all: bool) -> Self {
        if all {
            return Comforts {
                vein: VEIN_MAX,
                timber: TIMBER_MAX,
                reach: (HARVEST_MAX as i32 - 1) / 2,
                leaves: true,
                bag: true,
                smelt: true,
                ranch: RANCH_MAX as i32,
                plant: FIELD_REACH,
                fishing: true,
                saplings: true,
            };
        }
        let has = |tag: &str| tags.iter().any(|t| t == tag);
        let side = best(tags, HARVEST, HARVEST_MAX).max(1) as i32;
        let plant = if has(PLANT_FIELD) { FIELD_REACH } else { (best(tags, PLANT, PLANT_MAX) as i32 - 1).max(0) / 2 };
        Comforts {
            vein: best(tags, VEIN, VEIN_MAX),
            timber: best(tags, TIMBER, TIMBER_MAX),
            reach: (side - 1) / 2,
            leaves: has(LEAVES),
            bag: has(BAG),
            smelt: has(SMELT),
            ranch: best(tags, RANCH, RANCH_MAX) as i32,
            plant,
            fishing: has(FISHING),
            saplings: has(SAPLINGS),
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
        assert_eq!((all.smelt, all.ranch, all.plant, all.fishing), (true, 7, FIELD_REACH, true));
    }

    #[test]
    fn batch_two_comforts_read_from_tags() {
        let none = Comforts::of(&t(&[]), false);
        assert_eq!((none.smelt, none.ranch, none.plant, none.fishing), (false, 0, 0, false));
        let c = Comforts::of(&t(&["gathering.ranch.3", "gathering.ranch.5", "gathering.plant.3", "gathering.smelt"]), false);
        assert_eq!((c.smelt, c.ranch, c.plant, c.fishing), (true, 5, 1, false));
        assert_eq!(Comforts::of(&t(&["gathering.plant.5"]), false).plant, 2);
        assert_eq!(Comforts::of(&t(&["gathering.plant.3", "gathering.plant.field"]), false).plant, FIELD_REACH);
        assert!(Comforts::of(&t(&["gathering.fishing"]), false).fishing);
        assert!(Comforts::of(&t(&["gathering.saplings"]), false).saplings && !none.saplings);
        assert_eq!(Comforts::of(&t(&["gathering.ranch.99"]), false).ranch, 7);
    }

    /// The tree file gives exactly the tags Gathering reads, and every comfort has a node.
    #[test]
    fn the_shipped_tree_gives_exactly_the_tags_gathering_reads() {
        let text = crate::tree_files();
        let given: Vec<String> = text
            .split("\"tag\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').nth(1).map(str::to_owned))
            .collect();
        for tag in &given {
            let counted = [VEIN, TIMBER, HARVEST, RANCH, PLANT].iter().any(|p| tag.strip_prefix(p).is_some_and(|n| n.parse::<usize>().is_ok()));
            let flag = [LEAVES, BAG, SMELT, PLANT_FIELD, FISHING, SAPLINGS].contains(&tag.as_str());
            assert!(counted || flag, "the tree gives {tag}, which Gathering never reads");
        }
        assert_eq!(Comforts::of(&given, false), Comforts::of(&[], true), "some comfort has no node, or its node stops short");
    }
}
