//! Auto-smelt: an ore a player mines (one of `#gathering:veins`) drops what a furnace makes of
//! each drop, count for count (so Fortune's extra raw iron becomes as many ingots), and the
//! furnace's experience. Sneaking mines the ore as usual.
//!
//! The drops are changed in `block_drops`, which sees only the level; the player's choice (the
//! comfort, the setting, not sneaking) is taken when their break starts (`break_block`), and the
//! furnace's recipes are read then too, once for each set of recipes in force.
use mod_api::{BlockDropsEvent, Host, ItemKey, ItemStack, LevelHost, Recipe, ServerHost};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

/// The ore tag a block must be in for its drops to be smelted.
const ORES: &str = "gathering:veins";

/// What a furnace makes of one item: the result (one input's worth) and its experience.
#[derive(Clone, Debug)]
pub struct Cooked {
    pub result: ItemStack,
    pub experience: f32,
}

/// The smelting recipes, by the single item they take, built from the recipes in force.
struct Furnace {
    recipes: Arc<[Recipe]>,
    by_input: HashMap<ItemKey, Cooked>,
}

static FURNACE: Mutex<Option<Furnace>> = Mutex::new(None);
/// Each player's choice at their latest break: smelt or not.
static CHOICE: Mutex<BTreeMap<u128, bool>> = Mutex::new(BTreeMap::new());
/// The furnace's fractions of experience left over, per player, carried to their next ore.
static CARRY: Mutex<BTreeMap<u128, f32>> = Mutex::new(BTreeMap::new());

use crate::kit::sync::lock;

/// A player's break begins: remember whether its drops are smelted, and read the furnace's
/// recipes if they changed (a reload).
pub fn choose(host: &mut dyn ServerHost, player: u128, smelt: bool) {
    lock(&CHOICE).insert(player, smelt);
    if !smelt {
        return;
    }
    let read: &dyn Host = host;
    let recipes = read.recipes_of_type("minecraft:smelting");
    let mut furnace = lock(&FURNACE);
    if furnace.as_ref().is_some_and(|f| Arc::ptr_eq(&f.recipes, &recipes)) {
        return;
    }
    let mut by_input = HashMap::new();
    for recipe in recipes.iter() {
        let (Some(result), Some(ingredient)) = (recipe.results.first(), recipe.ingredients.first()) else { continue };
        let Ok(keys) = ingredient.item_keys(read) else { continue };
        for key in keys.iter() {
            by_input.entry(*key).or_insert_with(|| Cooked { result: result.clone(), experience: recipe.experience.unwrap_or(0.0) });
        }
    }
    *furnace = Some(Furnace { recipes, by_input });
}

pub fn forget(player: u128) {
    lock(&CHOICE).remove(&player);
    lock(&CARRY).remove(&player);
}

/// What `count` of an item cooks into, for a recipe making `each` from one at `experience`
/// apiece: how many result items (`None` past `u32`), the whole experience points with the
/// fraction carried from before added, and the new fraction to carry.
pub fn cook(each: u32, experience: f32, count: u32, carry: f32) -> (Option<u32>, u32, f32) {
    let total = experience * count as f32 + carry;
    let whole = total.floor().max(0.0);
    (each.checked_mul(count), whole as u32, total - whole)
}

/// The ore's drops become what the furnace makes of them, for a player who chose so.
pub fn drops(level: &dyn LevelHost, event: &mut BlockDropsEvent<'_>) {
    let Some(player) = event.breaker else { return };
    if lock(&CHOICE).get(&player) != Some(&true) {
        return;
    }
    if level.block_in_tag(event.state.block(), ORES) != Some(true) {
        return;
    }
    let furnace = lock(&FURNACE);
    let Some(furnace) = furnace.as_ref() else { return };
    let mut carry = lock(&CARRY).get(&player).copied().unwrap_or(0.0);
    for drop in event.drops.iter_mut() {
        let Some(cooked) = furnace.by_input.get(&drop.stack.item()) else { continue };
        let (made, xp, left) = cook(cooked.result.count(), cooked.experience, drop.stack.count(), carry);
        let Some(stack) = made.and_then(|n| ItemStack::new(cooked.result.item(), n)) else { continue };
        drop.stack = stack;
        event.experience = event.experience.saturating_add(xp);
        carry = left;
    }
    lock(&CARRY).insert(player, carry);
}

#[cfg(test)]
mod tests {
    use super::cook;

    #[test]
    fn counts_are_kept_and_experience_carries() {
        // Fortune III's four raw iron make four ingots: 0.7 x 4 = 2.8, so 2 now and 0.8 carried.
        let (made, xp, left) = cook(1, 0.7, 4, 0.0);
        assert_eq!((made, xp), (Some(4), 2));
        assert!((left - 0.8).abs() < 1e-5);
        // The carried 0.8 and one more ingot's 0.7 make 1 point, 0.5 carried.
        let (_, xp, left) = cook(1, 0.7, 1, left);
        assert_eq!(xp, 1);
        assert!((left - 0.5).abs() < 1e-5);
        // Gold gives 1 a piece, exactly; a recipe making two keeps the ratio.
        assert_eq!(cook(1, 1.0, 3, 0.0).1, 3);
        assert_eq!(cook(2, 0.1, 5, 0.0).0, Some(10));
        assert_eq!(cook(2, 0.1, u32::MAX, 0.0).0, None);
    }
}
