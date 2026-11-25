//! Into The Bag: stacks a vein, a tree's leaves or a harvest drops go into the player's main
//! inventory instead (onto stacks of the same kind first, then empty slots, hotbar first, each
//! slot asked for no more than its room); what doesn't fit drops at `at`, as it would have.
use mod_api::{HostTransaction, ItemPlace, ItemResource, ItemStack, LevelRef, PlayerRef, ServerHost};
#[allow(unused_imports)]
use mod_api::prelude::*;

pub fn give(host: &mut dyn ServerHost, p: PlayerRef, level: LevelRef, at: [i32; 3], stacks: Vec<ItemStack>) {
    for stack in stacks.into_iter().filter(|s| s.count() > 0) {
        let kind = ItemResource::of(&stack);
        let want = stack.count();
        let max = host.max_stack_size(kind.item()).unwrap_or(64);
        let slots = p.inventory_slots(&*host).unwrap_or_default();
        let held: Vec<(u32, Option<u32>)> = slots
            .iter()
            .filter(|(s, st)| *s < 36 && st.count() > 0)
            .map(|(s, st)| (*s, kind.matches(st).then(|| max.saturating_sub(st.count()))))
            .collect();
        let same = held.iter().filter_map(|&(s, room)| Some((s, room?)));
        let empty = (0..36u32).filter(|s| !held.iter().any(|h| h.0 == *s)).map(|s| (s, max));
        let order: Vec<(u32, u32)> = same.chain(empty).filter(|&(_, room)| room > 0).collect();
        let given = host
            .transaction(|host, tx| {
                let mut left = want;
                for &(slot, room) in &order {
                    if left == 0 {
                        break;
                    }
                    let place = ItemPlace::PlayerSlot { player: p.uuid(), slot: slot as i32 };
                    left -= place.insert(host, tx, &kind, left.min(room)).min(left);
                }
                Ok::<u32, ()>(want - left)
            })
            .unwrap_or(0);
        if given < want
            && let Some(rest) = kind.to_stack(want - given)
            && let Err(e) = level.pop_resource(host, at, rest)
        {
            eprintln!("[gathering] dropping what didn't fit at {at:?}: {e:?}");
        }
    }
}
