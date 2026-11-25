//! Saplings plant themselves: a sapling item (`#minecraft:saplings`) lying on grass or dirt
//! (`#minecraft:supports_vegetation`) for 5 seconds plants one of itself where it lies, if the spot is free and
//! no other sapling grows right beside it (at most one per 2 blocks). Earned in the Gathering
//! tree; it works for items near a player who has it, so a friend's felled tree counts too.
use crate::shape::Pos;
use mod_api::{BlockUpdateFlags, EntityRef, Host, ItemSpawnEvent, LevelRef, ServerHost};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Ticks a sapling must lie before it plants itself (5 s).
pub const WAIT: u64 = 100;
/// Ticks before looking again at one that could not plant yet (still falling, a busy spot).
const RETRY: u64 = 40;
/// Looks before giving up on one item (it may lie on stone forever).
const TRIES: u8 = 8;
/// Where a sapling may grow: the game's own rule for plants (grass, dirt, podzol, moss, mud...);
/// in 26.3 grass is not in `#minecraft:dirt` (measured).
const SOIL: &str = "minecraft:supports_vegetation";
/// How near a player with the comfort the item must be.
const NEAR: f64 = 48.0;

struct Lying {
    entity: u32,
    due: u64,
    tries: u8,
}

static LYING: Mutex<Vec<Lying>> = Mutex::new(Vec::new());
static NOW: AtomicU64 = AtomicU64::new(0);

fn lying() -> std::sync::MutexGuard<'static, Vec<Lying>> {
    LYING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Whether a sapling at `at` would have one growing in any of the 8 spots around it (and a
/// step up or down): what keeps saplings at most one per 2 blocks.
pub fn crowded(at: Pos, mut sapling: impl FnMut(Pos) -> bool) -> bool {
    (-1..=1).any(|dx| (-1..=1).any(|dy| (-1..=1).any(|dz| (dx, dz) != (0, 0) && sapling([at[0] + dx, at[1] + dy, at[2] + dz]))))
}

/// A dropped item joined the level: a sapling starts its wait.
pub fn spawned(host: &dyn Host, event: &ItemSpawnEvent<'_>) {
    if !crate::plan::item_in(host, event.item, "minecraft:saplings") {
        return;
    }
    let due = NOW.load(Ordering::Relaxed) + WAIT;
    lying().push(Lying { entity: event.entity, due, tries: 0 });
}

pub fn tick(host: &mut dyn ServerHost) {
    let now = NOW.fetch_add(1, Ordering::Relaxed) + 1;
    let work: Vec<Lying> = {
        let mut list = lying();
        if !list.iter().any(|l| l.due <= now) {
            return;
        }
        let (due, wait): (Vec<Lying>, Vec<Lying>) = std::mem::take(&mut *list).into_iter().partition(|l| l.due <= now);
        *list = wait;
        due
    };
    let mut again = Vec::new();
    for mut l in work {
        match plant(host, l.entity) {
            Outcome::Gone => {}
            Outcome::Wait if l.tries + 1 < TRIES => {
                l.tries += 1;
                l.due = now + RETRY;
                again.push(l);
            }
            Outcome::Wait => {}
            // More saplings in the stack: the next one may plant beside, once spaced.
            Outcome::Planted => {
                l.due = now + WAIT;
                l.tries = 0;
                again.push(l);
            }
        }
    }
    lying().extend(again);
}

enum Outcome {
    Gone,
    Wait,
    Planted,
}

/// Whether a player with the comfort, its setting on, is near `at` in `level`.
fn earned_near(host: &mut dyn ServerHost, level: LevelRef, at: [f64; 3]) -> bool {
    if !crate::enabled(&*host, crate::setting::SAPLINGS) {
        return false;
    }
    let players = host.players().server_players();
    let read: &dyn Host = host;
    players.into_iter().any(|p| {
        p.level(read) == Some(level)
            && p.entity().position(read).is_some_and(|q| (0..3).map(|i| (q[i] - at[i]).powi(2)).sum::<f64>() <= NEAR * NEAR)
            && crate::server::comforts(read, p).saplings
    })
}

fn plant(host: &mut dyn ServerHost, id: u32) -> Outcome {
    let read: &dyn Host = host;
    let Some(entity): Option<EntityRef> = read.entity(id) else { return Outcome::Gone };
    let Some(item) = read.item_entity(entity) else { return Outcome::Gone };
    let (Some(level), Some(at), Some(pos), Some(stack)) = (entity.level(read), entity.position(read), entity.block_position(read), item.stack(read)) else {
        return Outcome::Gone;
    };
    if entity.on_ground(read) != Some(true) {
        return Outcome::Wait;
    }
    let name = stack.item().id(read);
    let block_at = |q: Pos| level.block_state(read, q).map(|s| s.block().id(read));
    let soil = block_at([pos[0], pos[1] - 1, pos[2]]).is_some_and(|b| crate::plan::block_in(read, &b, SOIL));
    let free = block_at(pos).as_deref() == Some("minecraft:air");
    let near = crowded(pos, |q| block_at(q).is_some_and(|b| crate::plan::block_in(read, &b, "minecraft:saplings")));
    let Some(state) = read.block_state_id(&name) else { return Outcome::Gone };
    if !soil || !free || near {
        return Outcome::Wait;
    }
    if !earned_near(host, level, at) {
        return Outcome::Wait;
    }
    if !matches!(level.set_block(host, pos, state, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS), Ok(true)) {
        return Outcome::Wait;
    }
    let centre = [pos[0] as f64 + 0.5, pos[1] as f64, pos[2] as f64 + 0.5];
    level.play_sound(host, "minecraft:block.grass.place", centre, mod_api::SoundSource::Blocks, 1.0, 1.0);
    match stack.shrink(1) {
        Some(left) => {
            let _ = item.set_stack(host, left);
            Outcome::Planted
        }
        None => {
            entity.discard(host);
            Outcome::Gone
        }
    }
}

#[cfg(test)]
mod tests {
    use super::crowded;

    #[test]
    fn saplings_keep_one_block_apart() {
        assert!(!crowded([0, 0, 0], |_| false));
        assert!(crowded([0, 0, 0], |q| q == [1, 0, 1]), "a corner neighbour");
        assert!(crowded([0, 0, 0], |q| q == [0, 1, -1]), "a step up");
        assert!(!crowded([0, 0, 0], |q| q == [2, 0, 0]), "two blocks away is fine");
        assert!(!crowded([0, 0, 0], |q| q == [0, 0, 0]), "the spot itself is not a neighbour");
    }
}
