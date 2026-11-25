//! The server: a player's break of an ore or a tree's log starts a job; each tick the job breaks
//! up to `PER_TICK` more blocks through `PlayerRef::destroy_block`, so every block breaks as the
//! player's own (loot for the held tool and its enchantments, wear, the mined statistic, hunger,
//! spawn protection, other mods' cancels).
use crate::perks;
use crate::shape::{self, Pos};
use mod_api::{
    BlockMetadata, BlockState, ComponentId, Hand, Host, HostBlockStates, ItemPlace, LevelRef, PlayerRef, ServerHost, Tag,
};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::cell::Cell;
use std::sync::Mutex;

/// The ore families: an ore joins the vein of the family whose block tag `gathering:vein/<family>`
/// holds it (the game's own `minecraft:<family>_ores` inside, so deepslate and stone variants are
/// one vein; packs add their ores to these tags).
pub const FAMILIES: [&str; 10] = ["coal", "iron", "copper", "gold", "redstone", "lapis", "diamond", "emerald", "quartz", "debris"];
/// Blocks broken per player per tick: a 64-ore vein measured 32 ms in one go, so two ticks.
const PER_TICK: usize = 32;

thread_local! {
    /// Set while a job breaks, so those breaks don't start jobs of their own.
    static BUSY: Cell<bool> = const { Cell::new(false) };
}

/// What a job breaks: one ore family, or one kind of log.
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Vein(&'static str),
    Log(String),
}

struct Job {
    player: u128,
    dimension: String,
    kind: Kind,
    /// The held item's id when the job began: switching tools ends it.
    tool: String,
    left: Vec<Pos>,
}

static JOBS: Mutex<Vec<Job>> = Mutex::new(Vec::new());

fn jobs<R>(f: impl FnOnce(&mut Vec<Job>) -> R) -> R {
    f(&mut JOBS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn name(host: &dyn Host, state: BlockState) -> Option<String> {
    host.block_states().name(state.block()).ok().map(|s| s.to_string())
}

fn block_in(host: &dyn Host, block: &str, tag: &str) -> bool {
    host.registry_entry("minecraft:block", block).and_then(|e| host.registry_entry_in_tag(&e, tag)).unwrap_or(false)
}

fn item_in(host: &dyn Host, item: &str, tag: &str) -> bool {
    host.registry_entry("minecraft:item", item).and_then(|e| host.registry_entry_in_tag(&e, tag)).unwrap_or(false)
}

fn family(host: &dyn Host, block: &str) -> Option<&'static str> {
    FAMILIES.into_iter().find(|f| block_in(host, block, &format!("gathering:vein/{f}")))
}

fn kind_at(host: &dyn Host, level: LevelRef, pos: Pos) -> Option<Kind> {
    let block = name(host, level.block_state(host, pos)?)?;
    if let Some(f) = family(host, &block) {
        return Some(Kind::Vein(f));
    }
    block_in(host, &block, "minecraft:logs").then_some(Kind::Log(block))
}

/// Natural leaves (grown, not placed) touching any of `logs`: what tells a tree from a house.
fn natural_leaves(host: &dyn Host, level: LevelRef, logs: &[Pos]) -> bool {
    let meta = host.block_states();
    logs.iter().flat_map(|p| shape::around(*p)).any(|q| {
        let Some(state) = level.block_state(host, q) else { return false };
        name(host, state).is_some_and(|n| block_in(host, &n, "minecraft:leaves"))
            && state.property(&meta, "persistent").ok().flatten() == Some("false")
    })
}

fn tags(host: &dyn ServerHost, p: PlayerRef) -> Vec<String> {
    p.entity().tags(host).unwrap_or_default()
}

fn all_comforts(host: &dyn ServerHost) -> bool {
    host.setting(crate::MOD_ID, crate::setting::ALL_COMFORTS).is_some_and(|v| v.trim() == "true")
}

/// The held item's id, and how many more uses it has before it breaks (None: it doesn't wear).
fn held(host: &mut dyn ServerHost, p: PlayerRef) -> Option<(String, Option<i64>)> {
    let kind = ItemPlace::PlayerHand { player: p.uuid(), hand: Hand::Main }.resource(host)?;
    let int = |id: &str| match ComponentId::new(id).and_then(|c| kind.value(&*host, &c)) {
        Some(Tag::Int(n)) => Some(i64::from(n)),
        _ => None,
    };
    let left = int("minecraft:max_damage").map(|max| max - int("minecraft:damage").unwrap_or(0));
    Some((kind.item().id(&*host), left))
}

/// A player broke the block `state` at `pos`: with the comfort and the right tool, the rest of
/// its vein or tree is queued. Sneaking breaks just the one.
pub fn broke(host: &mut dyn ServerHost, pos: Pos, state: BlockState) {
    if BUSY.get() {
        return;
    }
    let Some(p) = host.players().acting_player() else { return };
    if p.entity().is_crouching(&*host) == Some(true) {
        return;
    }
    let (Some(level), Some(dimension)) = (p.level(&*host), p.dimension(&*host)) else { return };
    let Some(block) = name(&*host, state) else { return };
    let Some((tool, _)) = held(host, p) else { return };
    let tags = tags(&*host, p);
    let all = all_comforts(&*host);
    let (kind, cap) = if let Some(f) = family(&*host, &block) {
        // The right tool for drops, or the vein would break into nothing.
        if p.has_correct_tool_for_drops(&*host, state) != Some(true) {
            return;
        }
        (Kind::Vein(f), if all { perks::VEIN_MAX } else { perks::best(&tags, perks::VEIN, perks::VEIN_MAX) })
    } else if block_in(&*host, &block, "minecraft:logs") && item_in(&*host, &tool, "minecraft:axes") {
        (Kind::Log(block), if all { perks::TIMBER_MAX } else { perks::best(&tags, perks::TIMBER, perks::TIMBER_MAX) })
    } else {
        return;
    };
    if cap == 0 {
        return;
    }
    let host_ref: &dyn Host = &*host;
    let found = shape::joined(pos, cap, |q| kind_at(host_ref, level, q).as_ref() == Some(&kind));
    if found.is_empty() {
        return;
    }
    let left = match &kind {
        Kind::Log(_) => {
            let mut all_logs = found.clone();
            all_logs.push(pos);
            if !natural_leaves(host_ref, level, &all_logs) {
                return;
            }
            shape::top_down(found)
        }
        Kind::Vein(_) => found,
    };
    let player = p.uuid();
    jobs(|j| {
        j.retain(|x| x.player != player);
        j.push(Job { player, dimension: dimension.to_string(), kind, tool, left });
    });
}

/// Each tick: every job breaks up to `PER_TICK` more, as long as its player is here, in the same
/// dimension, holding the same tool with uses to spare, and the block is still what it was.
pub fn tick(host: &mut dyn ServerHost) {
    if jobs(|j| j.is_empty()) {
        return;
    }
    let mut work = jobs(std::mem::take);
    work.retain_mut(|job| step(host, job));
    jobs(|j| {
        // Jobs queued while these ran come after them.
        let newer = std::mem::take(j);
        *j = work;
        j.extend(newer);
    });
}

/// One tick of a job; false when it is over.
fn step(host: &mut dyn ServerHost, job: &mut Job) -> bool {
    let Some(p) = host.players().server_players().into_iter().find(|p| p.uuid() == job.player) else { return false };
    let (Some(level), Some(dimension)) = (p.level(&*host), p.dimension(&*host)) else { return false };
    if dimension.to_string() != job.dimension {
        return false;
    }
    for _ in 0..PER_TICK {
        let Some(pos) = job.left.first().copied() else { return false };
        match held(host, p) {
            Some((tool, left)) if tool == job.tool && left.is_none_or(|n| n > 1) => {}
            // Another tool, an empty hand, or one use left: the tool is kept, the job ends.
            _ => return false,
        }
        job.left.remove(0);
        if kind_at(&*host, level, pos).as_ref() != Some(&job.kind) {
            continue;
        }
        BUSY.set(true);
        p.destroy_block(host, pos, false);
        BUSY.set(false);
    }
    !job.left.is_empty()
}

/// A player left: their job ends.
pub fn forget(player: u128) {
    jobs(|j| j.retain(|x| x.player != player));
}

#[cfg(test)]
mod tests {
    use super::FAMILIES;
    use std::path::Path;

    /// Every ore family the code knows has its tag file, and no tag file is left without one.
    #[test]
    fn each_family_has_exactly_its_tag_file() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/gathering/tags/block/vein");
        let mut files: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().trim_end_matches(".json").to_owned()).collect();
        files.sort();
        let mut known: Vec<String> = FAMILIES.iter().map(|f| f.to_string()).collect();
        known.sort();
        assert_eq!(files, known);
    }

    /// Both languages hold the same keys, and every node and perk the tree shows has its line.
    #[test]
    fn every_shown_key_is_in_both_languages() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let keys = |lang: &str| -> Vec<String> {
            let text = std::fs::read_to_string(root.join(format!("assets/gathering/lang/{lang}.json"))).unwrap();
            let mut k: Vec<String> = text.lines().filter_map(|l| l.trim().strip_prefix('"')?.split('"').next().map(str::to_owned)).collect();
            k.sort();
            k
        };
        let (en, ar) = (keys("en_us"), keys("ar_sa"));
        assert_eq!(en, ar);
        let tree = std::fs::read_to_string(root.join("data/gathering/skills/gathering.json")).unwrap();
        let quoted = |field: &str| -> Vec<String> {
            tree.split(&format!("\"{field}\"")).skip(1).filter_map(|r| r.split('"').nth(1).map(str::to_owned)).collect()
        };
        for title in quoted("title") {
            assert!(en.contains(&title), "no line for {title}");
        }
        for tag in quoted("tag") {
            assert!(en.contains(&format!("skills.perk.{tag}")), "no perk line for {tag}");
        }
        assert!(en.contains(&format!("gathering.config.{}", crate::setting::ALL_COMFORTS)));
    }
}
