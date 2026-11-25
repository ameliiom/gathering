//! The comforts earned in the Skills tree Gathering, with no code tie to Skills: an unlocked node
//! gives the player a vanilla tag that Gathering reads (`gathering.vein.<n>`,
//! `gathering.timber.<n>`), and the tree ships as a data file only Skills reads. Without Skills,
//! the server setting `all_comforts` opens them.

/// Tags `gathering.vein.<n>`: how many more ores one break takes (none: just the one).
pub const VEIN: &str = "gathering.vein.";
/// Tags `gathering.timber.<n>`: how many more logs of a tree one break takes.
pub const TIMBER: &str = "gathering.timber.";
/// The most either may reach, and what `all_comforts` gives.
pub const VEIN_MAX: usize = 64;
pub const TIMBER_MAX: usize = 256;

/// The largest `<prefix><n>` among `tags` (0 without any), at most `max`.
pub fn best(tags: &[String], prefix: &str, max: usize) -> usize {
    tags.iter().filter_map(|t| t.strip_prefix(prefix)?.parse::<usize>().ok()).max().unwrap_or(0).min(max)
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
            let read = [VEIN, TIMBER].iter().any(|p| tag.strip_prefix(p).is_some_and(|n| n.parse::<usize>().is_ok()));
            assert!(read, "the tree gives {tag}, which Gathering never reads");
        }
        assert!(best(&given, VEIN, VEIN_MAX) == VEIN_MAX, "no node reaches the biggest vein");
        assert!(best(&given, TIMBER, TIMBER_MAX) == TIMBER_MAX, "no node reaches the biggest tree");
    }
}
