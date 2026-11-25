//! Wide planting: seeds right-clicked onto the top of farmland plant the joined empty farmland
//! around it too (3×3, 5×5 or the whole field, by the comfort), one seed each, paid from the
//! inventory. The game plants the farmland clicked as it always does; the comfort the rest.
use crate::shape::{self, Pos};
use mod_api::{BlockClick, GameMode, Hand, Host, ItemStack, LevelRef, PlayerRef, ServerHost};
#[allow(unused_imports)]
use mod_api::prelude::*;

/// The crop block a planting item sows on farmland (`BlockItem`'s block for the game's seeds).
pub fn crop_of(item: &str) -> Option<&'static str> {
    Some(match item {
        "minecraft:wheat_seeds" => "minecraft:wheat",
        "minecraft:carrot" => "minecraft:carrots",
        "minecraft:potato" => "minecraft:potatoes",
        "minecraft:beetroot_seeds" => "minecraft:beetroots",
        "minecraft:torchflower_seeds" => "minecraft:torchflower_crop",
        "minecraft:pitcher_pod" => "minecraft:pitcher_crop",
        "minecraft:melon_seeds" => "minecraft:melon_stem",
        "minecraft:pumpkin_seeds" => "minecraft:pumpkin_stem",
        _ => return None,
    })
}

/// The farmland a planting at `start` reaches: joined to it on its own level (corners count),
/// within `reach` blocks either way, nearest first, `start` left out, at most `cap`.
pub fn field(start: Pos, reach: i32, cap: usize, mut empty_farmland: impl FnMut(Pos) -> bool) -> Vec<Pos> {
    shape::joined(start, cap, |q| {
        q[1] == start[1] && (q[0] - start[0]).abs() <= reach && (q[2] - start[2]).abs() <= reach && empty_farmland(q)
    })
}

/// Who pays for `want` seeds from `slots` (slot, count of the seed): other slots first, then
/// the `held` slot down to its last seed, which the game spends on the farmland clicked.
/// Returns what to take from each slot; their sum is how many can be planted.
pub fn pay(slots: &[(u32, u32)], held: u32, want: u32) -> Vec<(u32, u32)> {
    let mut left = want;
    let mut take = Vec::new();
    let others = slots.iter().filter(|(s, _)| *s != held).map(|&(s, n)| (s, n));
    let hand = slots.iter().filter(|(s, _)| *s == held).map(|&(s, n)| (s, n.saturating_sub(1)));
    for (slot, n) in others.chain(hand) {
        let t = n.min(left);
        if t > 0 {
            take.push((slot, t));
            left -= t;
        }
    }
    take
}

const OFF_HAND_SLOT: u32 = 40;

/// The server's side of a click with seeds on farmland.
pub fn on_server(host: &mut dyn ServerHost, click: &BlockClick<'_>) {
    let read: &dyn Host = host;
    if click.hit.face != [0, 1, 0] {
        return;
    }
    let Some(stack) = click.stack.filter(|s| s.count() > 0) else { return };
    let item = stack.item().id(read);
    let Some(crop) = crop_of(&item) else { return };
    let Some(p) = read.player(click.player) else { return };
    if !matches!(p.game_mode(read), Some(GameMode::Survival | GameMode::Creative)) {
        return;
    }
    let reach = crate::server::comforts(read, p).plant;
    if reach == 0 || !crate::enabled(read, crate::setting::PLANTING) {
        return;
    }
    let Some(level) = p.level(read) else { return };
    let Some(state) = read.block_state_id(crop) else { return };
    let name_at = |q: Pos| level.block_state(read, q).map(|s| s.block().id(read));
    let empty_farmland = |q: Pos| {
        name_at(q).as_deref() == Some("minecraft:farmland") && name_at([q[0], q[1] + 1, q[2]]).as_deref() == Some("minecraft:air")
    };
    // The farmland clicked must be one the game plants, so the field joins to a real planting.
    if !empty_farmland(click.pos) {
        return;
    }
    let spots = field(click.pos, reach, crate::perks::FIELD_MAX, empty_farmland);
    if spots.is_empty() {
        return;
    }
    let free = p.has_infinite_materials(read) == Some(true);
    let held = match click.hand {
        Hand::Main => p.selected_slot(read).map(|s| s as u32),
        Hand::Off => Some(OFF_HAND_SLOT),
    };
    let Some(held) = held else { return };
    let slots: Vec<(u32, ItemStack)> = p
        .inventory_slots(read)
        .unwrap_or_default()
        .into_iter()
        .filter(|(s, st)| (*s < 36 || *s == OFF_HAND_SLOT) && st.item().id(read) == item)
        .collect();
    let counts: Vec<(u32, u32)> = slots.iter().map(|(s, st)| (*s, st.count())).collect();
    let paid = if free { Vec::new() } else { pay(&counts, held, spots.len() as u32) };
    let can = if free { spots.len() } else { paid.iter().map(|t| t.1 as usize).sum() };
    let mut planted = 0;
    for q in spots.into_iter().take(can) {
        if p.place_block(host, level, [q[0], q[1] + 1, q[2]], state, false).is_ok() {
            planted += 1;
        }
    }
    if free {
        return;
    }
    // Take what was planted, in the order `pay` chose.
    let mut owed = planted;
    for (slot, take) in paid {
        if owed == 0 {
            break;
        }
        let t = take.min(owed);
        owed -= t;
        let Some((_, original)) = slots.iter().find(|(s, _)| *s == slot) else { continue };
        if let Err(e) = p.set_inventory_slot(host, slot, original.clone().shrink(t)) {
            eprintln!("[gathering] paying seeds from slot {slot}: {e:?}");
        }
    }
    place_sound(host, p, level, click.pos);
}

fn place_sound(host: &mut dyn ServerHost, _p: PlayerRef, level: LevelRef, at: Pos) {
    let at = [at[0] as f64 + 0.5, at[1] as f64 + 1.0, at[2] as f64 + 0.5];
    level.play_sound(host, "minecraft:item.crop.plant", at, mod_api::SoundSource::Blocks, 1.0, 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_seed_has_its_crop() {
        assert_eq!(crop_of("minecraft:wheat_seeds"), Some("minecraft:wheat"));
        assert_eq!(crop_of("minecraft:potato"), Some("minecraft:potatoes"));
        assert_eq!(crop_of("minecraft:pumpkin_seeds"), Some("minecraft:pumpkin_stem"));
        assert_eq!(crop_of("minecraft:bone_meal"), None);
        // Each seed the harvest replants with is one wide planting sows.
        for block in ["minecraft:wheat", "minecraft:carrots", "minecraft:potatoes", "minecraft:beetroots", "minecraft:pitcher_crop"] {
            let item = crate::crops::crop_profile(block).unwrap().planting_item;
            assert_eq!(crop_of(item), Some(block));
        }
    }

    fn farm(points: &[Pos]) -> HashSet<Pos> {
        points.iter().copied().collect()
    }

    #[test]
    fn the_field_is_squared_joined_and_level() {
        // A 9×9 field at y 0, a lone farmland one gap away, and farmland a step up.
        let mut land: Vec<Pos> = (-4..=4).flat_map(|x| (-4..=4).map(move |z| [x, 0, z])).collect();
        land.push([6, 0, 0]);
        land.push([1, 1, 1]);
        let land = farm(&land);
        let at = |r: i32| field([0, 0, 0], r, 256, |q| land.contains(&q));
        assert_eq!(at(1).len(), 8);
        assert_eq!(at(2).len(), 24);
        let whole = at(i32::MAX);
        assert_eq!(whole.len(), 80);
        assert!(!whole.contains(&[6, 0, 0]) && !whole.contains(&[1, 1, 1]) && !whole.contains(&[0, 0, 0]));
        assert_eq!(field([0, 0, 0], i32::MAX, 10, |q| land.contains(&q)).len(), 10);
    }

    #[test]
    fn seeds_are_paid_from_other_slots_first_and_the_last_one_stays_in_hand() {
        // 3 in hand (slot 0), 2 in slot 5: 4 wanted takes both of slot 5 and 2 from the hand.
        assert_eq!(pay(&[(0, 3), (5, 2)], 0, 4), vec![(5, 2), (0, 2)]);
        // Only the hand's seeds, and more wanted than it holds: all but its last.
        assert_eq!(pay(&[(0, 3)], 0, 10), vec![(0, 2)]);
        // One seed in hand: the game plants the clicked one, nothing left for the rest.
        assert!(pay(&[(0, 1)], 0, 8).is_empty());
        assert_eq!(pay(&[(0, 64), (9, 64)], 0, 8), vec![(9, 8)]);
    }
}
