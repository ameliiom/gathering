//! What one break would take, read from a level on either side: the server breaks it, the client
//! outlines it while you aim. One set of rules, so the outline always shows what will break.
use crate::perks::Comforts;
use crate::shape::{self, Pos};
use mod_api::{BlockMetadata, BlockState, Host, HostBlockStates, LevelRef};
#[allow(unused_imports)]
use mod_api::prelude::*;

/// The ore families: an ore joins the vein of the family whose block tag `gathering:vein/<family>`
/// holds it (the game's own `minecraft:<family>_ores` inside, so deepslate and stone variants are
/// one vein; packs add their ores to these tags).
pub const FAMILIES: [&str; 10] = ["coal", "iron", "copper", "gold", "redstone", "lapis", "diamond", "emerald", "quartz", "debris"];

/// What a break takes along: one ore family, or one kind of log.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Vein(&'static str),
    Log(String),
}

/// The blocks a break at `pos` takes along (the block itself left out), and their kind.
pub struct Plan {
    pub kind: Kind,
    /// In breaking order: nearest first for a vein, top down for a tree.
    pub blocks: Vec<Pos>,
}

pub fn name(host: &dyn Host, state: BlockState) -> Option<String> {
    host.block_states().name(state.block()).ok().map(|s| s.to_string())
}

pub fn block_in(host: &dyn Host, block: &str, tag: &str) -> bool {
    host.registry_entry("minecraft:block", block).and_then(|e| host.registry_entry_in_tag(&e, tag)).unwrap_or(false)
}

fn item_in(host: &dyn Host, item: &str, tag: &str) -> bool {
    host.registry_entry("minecraft:item", item).and_then(|e| host.registry_entry_in_tag(&e, tag)).unwrap_or(false)
}

fn family(host: &dyn Host, block: &str) -> Option<&'static str> {
    FAMILIES.into_iter().find(|f| block_in(host, block, &format!("gathering:vein/{f}")))
}

pub fn kind_at(host: &dyn Host, level: LevelRef, pos: Pos) -> Option<Kind> {
    let block = name(host, level.block_state(host, pos)?)?;
    if let Some(f) = family(host, &block) {
        return Some(Kind::Vein(f));
    }
    block_in(host, &block, "minecraft:logs").then_some(Kind::Log(block))
}

/// Grown (not placed) leaves: `persistent=false`.
pub fn natural_leaf(host: &dyn Host, state: BlockState) -> bool {
    name(host, state).is_some_and(|n| block_in(host, &n, "minecraft:leaves"))
        && state.property(&host.block_states(), "persistent").ok().flatten() == Some("false")
}

/// Natural leaves touching any of `logs`: what tells a tree from a house.
fn natural_leaves(host: &dyn Host, level: LevelRef, logs: &[Pos]) -> bool {
    logs.iter().flat_map(|p| shape::around(*p)).any(|q| level.block_state(host, q).is_some_and(|s| natural_leaf(host, s)))
}

/// What breaking `state` at `pos` with the held item `tool` takes along. `correct`: whether that
/// tool gets the block's drops (a vein never breaks into nothing). None: just the one block.
pub fn plan(host: &dyn Host, level: LevelRef, pos: Pos, state: BlockState, tool: &str, correct: bool, c: &Comforts) -> Option<Plan> {
    let block = name(host, state)?;
    let (kind, cap) = if let Some(f) = family(host, &block) {
        if !correct {
            return None;
        }
        (Kind::Vein(f), c.vein)
    } else if block_in(host, &block, "minecraft:logs") && item_in(host, tool, "minecraft:axes") {
        (Kind::Log(block), c.timber)
    } else {
        return None;
    };
    if cap == 0 {
        return None;
    }
    let found = shape::joined(pos, cap, |q| kind_at(host, level, q).as_ref() == Some(&kind));
    if found.is_empty() {
        return None;
    }
    let blocks = match &kind {
        Kind::Log(_) => {
            let mut all = found.clone();
            all.push(pos);
            if !natural_leaves(host, level, &all) {
                return None;
            }
            shape::top_down(found)
        }
        Kind::Vein(_) => found,
    };
    Some(Plan { kind, blocks })
}

#[cfg(test)]
mod tests {
    use super::FAMILIES;
    use std::path::Path;

    /// Every ore family the code knows has its tag file, and no tag file is left without one.
    #[test]
    fn each_family_has_exactly_its_tag_file() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/gathering/tags/block/vein");
        let mut files: Vec<String> =
            std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().trim_end_matches(".json").to_owned()).collect();
        files.sort();
        let mut known: Vec<String> = FAMILIES.iter().map(|f| f.to_string()).collect();
        known.sort();
        assert_eq!(files, known);
    }
}
