//! Auto-fishing: when a fish bites, the catch goes straight into the inventory and the bobber
//! comes back, the rod taking its one use, as reeling in would.
//!
//! The bite is taken from the game (`player_fish` state 1, cancelled: the fish gets away), and
//! the next tick, with the bobber back in the level's lists, its own fishing loot is rolled at
//! it (`minecraft:gameplay/fishing` with the rod as the tool and the bobber as the entity, so
//! open water still decides treasure), into the inventory; the game's reel-in rewards follow:
//! the `fish_caught` statistic for a fish, 1 to 6 experience, one use of the rod.
use mod_api::{EntityRef, EquipmentSlot, Hand, Host, PlayerFishEvent, PlayerRef, ServerHost, StatType};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

const ROD: &str = "minecraft:fishing_rod";
const LOOT: &str = "minecraft:gameplay/fishing";
/// How far from its player a bobber is looked for (the game reels one in past 32 blocks).
const BOBBER_REACH: f64 = 40.0;
/// `player_fish` states: 1 is a bite.
const BITE: i32 = 1;

static PENDING: Mutex<Vec<(u128, Hand)>> = Mutex::new(Vec::new());
static SEED: AtomicU64 = AtomicU64::new(0x9e37_79b9_7f4a_7c15);

fn pending() -> std::sync::MutexGuard<'static, Vec<(u128, Hand)>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// The hand holding a rod, main hand first.
fn rod_hand(host: &mut dyn ServerHost, p: PlayerRef) -> Option<(Hand, Option<i64>)> {
    [Hand::Main, Hand::Off].into_iter().find_map(|hand| match crate::server::held_in(host, p, hand) {
        Some((item, left)) if item == ROD => Some((hand, left)),
        _ => None,
    })
}

/// A rod with a use to spare: the comfort never breaks it.
pub fn spares(left: Option<i64>) -> bool {
    left.is_none_or(|n| n > 1)
}

/// The game's reel-in experience, 1 to 6, from a random draw.
pub fn experience(draw: u64) -> u32 {
    (draw % 6) as u32 + 1
}

fn draw() -> u64 {
    // xorshift64: experience only, no fairness needed beyond spread.
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    x
}

/// A player's rod event: a bite is taken for the comfort (true: cancel it).
pub fn bite(host: &mut dyn ServerHost, event: &PlayerFishEvent<'_>) -> bool {
    if event.state != BITE {
        return false;
    }
    let read: &dyn Host = host;
    let Some(p) = host.players().server_players().into_iter().find(|p| p.uuid() == event.player) else { return false };
    if !crate::server::comforts(read, p).fishing || !crate::enabled(read, crate::setting::FISHING) {
        return false;
    }
    let Some((hand, left)) = rod_hand(host, p) else { return false };
    if !spares(left) {
        return false;
    }
    let mut list = pending();
    if list.iter().any(|(u, _)| *u == event.player) {
        return true;
    }
    list.push((event.player, hand));
    true
}

/// The player's own bobber in their level.
fn bobber(host: &dyn ServerHost, p: PlayerRef) -> Option<EntityRef> {
    let level = p.level(host)?;
    let at = p.entity().position(host)?;
    let r = BOBBER_REACH;
    let near = level.entities_in(host, [at[0] - r, at[1] - r, at[2] - r], [at[0] + r, at[1] + r, at[2] + r])?;
    near.into_iter().find(|e| host.fishing_hook(*e).and_then(|h| h.player_owner(host)) == Some(p.entity()))
}

pub fn tick(host: &mut dyn ServerHost) {
    let work = std::mem::take(&mut *pending());
    for (uuid, hand) in work {
        land(host, uuid, hand);
    }
}

fn land(host: &mut dyn ServerHost, uuid: u128, hand: Hand) {
    let read: &dyn Host = host;
    let Some(p) = host.players().server_players().into_iter().find(|p| p.uuid() == uuid) else { return };
    let (Some(level), Some(living)) = (p.level(read), read.living(p.entity())) else { return };
    let Some(hook) = bobber(&*host, p) else { return };
    let (Some(at), Some(pos)) = (hook.position(read), hook.block_position(read)) else { return };
    // Still the rod in the same hand, with a use to spare.
    match crate::server::held_in(host, p, hand) {
        Some((item, left)) if item == ROD && spares(left) => {}
        _ => return,
    }
    let read: &dyn Host = host;
    let Some(rod) = living.hand_item(read, hand) else { return };
    let catch = match level.roll_loot(host, LOOT, at, Some(&rod), Some(hook)) {
        Ok(Some(stacks)) => stacks,
        other => {
            eprintln!("[gathering] fishing loot: {other:?}");
            return;
        }
    };
    let read: &dyn Host = host;
    let fish = catch.iter().any(|s| crate::plan::item_in(read, &s.item().id(read), "minecraft:fishes"));
    hook.discard(host);
    crate::kit::inventory::give(host, crate::MOD_ID, p, level, pos, catch);
    if fish && !p.award_stat(host, StatType::Custom, "minecraft:fish_caught", 1) {
        eprintln!("[gathering] could not award fish_caught");
    }
    p.give_experience_points(host, experience(draw() ^ uuid as u64));
    let slot = match hand {
        Hand::Main => EquipmentSlot::MainHand,
        Hand::Off => EquipmentSlot::OffHand,
    };
    living.hurt_and_break(host, slot, 1);
    living.swing(host, hand, true);
    if let Some(me) = p.entity().position(&*host) {
        level.play_sound(host, "minecraft:entity.fishing_bobber.retrieve", me, mod_api::SoundSource::Neutral, 1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experience_is_one_to_six_like_a_reel_in() {
        let seen: std::collections::BTreeSet<u32> = (0..600).map(experience).collect();
        assert_eq!(seen.into_iter().collect::<Vec<_>>(), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn the_rod_is_never_broken() {
        assert!(spares(None));
        assert!(spares(Some(64)));
        assert!(spares(Some(2)));
        assert!(!spares(Some(1)));
    }
}
