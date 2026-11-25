//! The server: a player's break of an ore or a tree's log starts a job; each tick the job breaks
//! up to `PER_TICK` more blocks through `PlayerRef::destroy_block`, so every block breaks as the
//! player's own (loot for the held tool and its enchantments, wear, the mined statistic, hunger,
//! spawn protection, other mods' cancels).
use crate::bag;
use crate::payloads::{self, Caps};
use crate::perks::Comforts;
use crate::plan::{self, Kind, Plan, kind_at, natural_leaf};
use crate::shape::{self, Pos};
use mod_api::{BlockState, ComponentId, Hand, Host, HostBlockStates, ItemPlace, LevelRef, PlayerRef, ServerHost, Tag, VarInt};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

/// Blocks broken per player per tick: a 64-ore vein measured 32 ms in one go, so two ticks.
const PER_TICK: usize = 32;
/// Leaves: how long to wait after the last log (a leaf's distance spreads a block a tick, up to
/// 7), how many at most, and how many a tick.
const LEAF_WAIT: u32 = 8;
const LEAF_MAX: usize = 1024;
const LEAF_PER_TICK: usize = 64;

thread_local! {
    /// Set while a job breaks, so those breaks don't start jobs of their own.
    static BUSY: Cell<bool> = const { Cell::new(false) };
}

struct Job {
    player: u128,
    dimension: String,
    kind: Kind,
    /// The held item's id when the job began: switching tools ends it.
    tool: String,
    left: Vec<Pos>,
    /// Into The Bag: drops go to the inventory.
    bag: bool,
    /// Leaf Blower, for a tree: its leaves fall once the logs are down.
    leaves: Option<Leaves>,
}

struct Leaves {
    /// Every log the job felled (the first one included).
    felled: Vec<Pos>,
    /// Ticks still to wait for the leaves' distances to settle.
    wait: u32,
    /// The leaves to drop, found once the wait is over.
    queue: Option<Vec<Pos>>,
}

static JOBS: Mutex<Vec<Job>> = Mutex::new(Vec::new());

fn jobs<R>(f: impl FnOnce(&mut Vec<Job>) -> R) -> R {
    f(&mut JOBS.lock().unwrap_or_else(|e| e.into_inner()))
}

/// A player's comforts (their tags, or everything with `all_comforts`).
pub fn comforts(host: &dyn mod_api::Host, p: PlayerRef) -> Comforts {
    let all = host.setting(crate::MOD_ID, crate::setting::ALL_COMFORTS).is_some_and(|v| v.trim() == "true");
    Comforts::of(&p.entity().tags(host).unwrap_or_default(), all)
}

/// The held item's id, and how many more uses it has before it breaks (None: it doesn't wear).
fn held(host: &mut dyn ServerHost, p: PlayerRef) -> Option<(String, Option<i64>)> {
    held_in(host, p, Hand::Main)
}

/// As `held`, for either hand.
pub fn held_in(host: &mut dyn ServerHost, p: PlayerRef, hand: Hand) -> Option<(String, Option<i64>)> {
    let kind = ItemPlace::PlayerHand { player: p.uuid(), hand }.resource(host)?;
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
    let crouching = p.entity().is_crouching(&*host) == Some(true);
    let c = comforts(&*host, p);
    // Auto-smelt: chosen now for this break and the vein it starts; sneaking mines as usual.
    let smelt = c.smelt && !crouching && crate::enabled(&*host, crate::setting::SMELT);
    crate::smelt::choose(host, p.uuid(), smelt);
    if crouching {
        return;
    }
    let (Some(level), Some(dimension)) = (p.level(&*host), p.dimension(&*host)) else { return };
    let Some((tool, _)) = held(host, p) else { return };
    let correct = p.has_correct_tool_for_drops(&*host, state) == Some(true);
    let Some(Plan { kind, blocks }) = plan::plan(&*host, level, pos, state, &tool, correct, &c) else { return };
    let leaves = match kind {
        Kind::Log(_) if c.leaves => {
            let mut felled = blocks.clone();
            felled.push(pos);
            Some(Leaves { felled, wait: LEAF_WAIT, queue: None })
        }
        _ => None,
    };
    let left = blocks;
    let player = p.uuid();
    jobs(|j| {
        j.retain(|x| x.player != player);
        j.push(Job { player, dimension: dimension.to_string(), kind, tool, left, bag: c.bag, leaves });
    });
}

/// Each tick: every job breaks up to `PER_TICK` more, as long as its player is here, in the same
/// dimension, holding the same tool with uses to spare, and the block is still what it was.
pub fn tick(host: &mut dyn ServerHost) {
    let due = TICKS.fetch_add(1, Ordering::Relaxed).is_multiple_of(TELL_EVERY);
    if due {
        tell(host);
    }
    crate::ranch::tick(host);
    crate::fishing::tick(host);
    crate::saplings::tick(host);
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
    if job.left.is_empty() {
        return match &mut job.leaves {
            Some(leaves) => fall(host, p, level, leaves, job.bag),
            None => false,
        };
    }
    for _ in 0..PER_TICK {
        let Some(pos) = job.left.first().copied() else { break };
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
        p.destroy_block(host, pos, job.bag);
        BUSY.set(false);
    }
    !job.left.is_empty() || job.leaves.is_some()
}

/// Leaf Blower: once the logs are down and the leaves have learnt it (the game spreads a leaf's
/// distance from the nearest log a block per tick), the felled tree's natural leaves that no
/// log holds any more fall with their drops (saplings, sticks, apples), as decay would drop them.
fn fall(host: &mut dyn ServerHost, p: PlayerRef, level: LevelRef, leaves: &mut Leaves, bag: bool) -> bool {
    if leaves.wait > 0 {
        leaves.wait -= 1;
        return true;
    }
    let queue = leaves.queue.get_or_insert_with(|| {
        let read: &dyn Host = &*host;
        let meta = read.block_states();
        let loose = |q: Pos| level.block_state(read, q).is_some_and(|s| natural_leaf(read, s));
        let mut found = shape::joined_all(&leaves.felled, LEAF_MAX, loose);
        // Only the ones too far from any standing log: a neighbour tree's leaves stay.
        found.retain(|q| level.block_state(read, *q).and_then(|s| s.property(&meta, "distance").ok().flatten().map(str::to_owned)).as_deref() == Some("7"));
        found
    });
    for _ in 0..LEAF_PER_TICK {
        let Some(pos) = queue.pop() else { return false };
        match level.destroy_block(host, pos, true, None, bag) {
            Ok(Some(stacks)) if bag => bag::give(host, p, level, pos, stacks),
            Ok(_) => {}
            Err(e) => eprintln!("[gathering] a leaf at {pos:?}: {e:?}"),
        }
    }
    !queue.is_empty()
}

/// A player left: their job ends.
pub fn forget(player: u128) {
    jobs(|j| j.retain(|x| x.player != player));
    told(|t| t.remove(&player));
    crate::smelt::forget(player);
}

/// Ticks between looks at each player's comforts (Skills changes their tags at any time).
const TELL_EVERY: u32 = 20;
static TICKS: AtomicU32 = AtomicU32::new(0);
/// What each player's client was last told.
static TOLD: Mutex<BTreeMap<u128, (usize, usize)>> = Mutex::new(BTreeMap::new());

fn told<R>(f: impl FnOnce(&mut BTreeMap<u128, (usize, usize)>) -> R) -> R {
    f(&mut TOLD.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Tells each client its vein and tree sizes when they changed, for the outline.
fn tell(host: &mut dyn ServerHost) {
    let Some(token) = payloads::CAPS.get() else { return };
    for p in host.players().server_players() {
        let c = comforts(&*host, p);
        let now = (c.vein, c.timber);
        if told(|t| t.get(&p.uuid()) == Some(&now)) {
            continue;
        }
        let caps = Caps { vein: VarInt(now.0 as i32), timber: VarInt(now.1 as i32) };
        // A client not ready yet (just joining) is asked again next time.
        if token.send(host, p, &caps).is_ok() {
            told(|t| t.insert(p.uuid(), now));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

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
        for key in crate::setting::COMFORTS {
            assert!(en.contains(&format!("gathering.config.{key}")), "no line for the setting {key}");
        }
    }
}
