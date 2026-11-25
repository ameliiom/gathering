//! Ranch: feeding an animal feeds the adults of its kind around it too, and shearing a sheep
//! shears the sheep around it, within the comfort's reach (3, 5 or 7 blocks).
//!
//! The game feeds or shears the animal clicked, as always; the next tick, once it has, the
//! comfort does the same to the rest: one food from the hand for each animal it brings into
//! love (`InLove` 600 and the player as `LoveCause`, so the breeding counts as theirs), and one
//! shears' use per sheep, stopping one use before the shears would break.
use mod_api::{Compound, EntityInteractEvent, EntityRef, EquipmentSlot, Hand, Host, LevelRef, PlayerRef, PlayerShearEntityEvent, ServerHost, Tag};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::Mutex;

/// How long an animal fed by the comfort stays in love, as the game's own feeding.
const LOVE_TICKS: i32 = 600;
const SHEEP: &str = "minecraft:sheep";

enum Pending {
    Feed { player: u128, animal: u32, hand: Hand, food: String, count: u32 },
    Shear { player: u128, sheep: u32, hand: Hand },
}

static PENDING: Mutex<Vec<Pending>> = Mutex::new(Vec::new());

fn pending() -> std::sync::MutexGuard<'static, Vec<Pending>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// The player's reach, when the comfort is earned and on.
fn reach(host: &dyn Host, p: PlayerRef) -> Option<i32> {
    let r = crate::server::comforts(host, p).ranch;
    (r > 0 && crate::enabled(host, crate::setting::RANCH)).then_some(r)
}

fn int(data: &Compound, key: &str) -> i32 {
    match data.get(&key.into()) {
        Some(Tag::Int(v)) => *v,
        Some(Tag::Short(v)) => i32::from(*v),
        Some(Tag::Byte(v)) => i32::from(*v),
        _ => 0,
    }
}

/// What tells whether an animal can be fed into love now, read from its saved data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Animal {
    /// `Age`: below 0 a baby, above 0 resting after breeding; only 0 breeds.
    pub age: i32,
    pub in_love: i32,
    /// Tamed (`Tame`, or an `Owner`): a tamed horse or wolf is fed only along with tamed ones.
    pub tame: bool,
}

impl Animal {
    fn read(data: &Compound) -> Self {
        Animal { age: int(data, "Age"), in_love: int(data, "InLove"), tame: int(data, "Tame") != 0 || data.contains_key(&"Owner".into()) }
    }

    /// Whether the comfort feeds this animal along with `clicked`, which the game just fed.
    pub fn feeds_with(&self, clicked: &Animal) -> bool {
        self.age == 0 && self.in_love <= 0 && self.tame == clicked.tame
    }
}

/// The wool colours by a sheep's saved `Color` (`DyeColor` ids): the game's per-colour shearing
/// tables, `minecraft:shearing/sheep/<colour>` (its `shearing/sheep` dispatcher rolls nothing
/// from a mod's roll, measured).
const COLORS: [&str; 16] = [
    "white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black",
];

pub fn wool_color(id: i32) -> Option<&'static str> {
    usize::try_from(id).ok().and_then(|i| COLORS.get(i).copied())
}

/// A player's four-int UUID, as the game saves `LoveCause`.
pub fn uuid_ints(u: u128) -> Vec<i32> {
    (0..4).map(|i| (u >> (96 - 32 * i)) as u32 as i32).collect()
}

/// A right-click on an animal with something in hand: remember it if the comfort may follow.
pub fn interacted(host: &mut dyn ServerHost, event: &EntityInteractEvent<'_>) {
    let Some(stack) = event.stack.filter(|s| s.count() > 0) else { return };
    let read: &dyn Host = host;
    let Some(p) = host.players().acting_player() else { return };
    if reach(read, p).is_none() || event.kind == SHEEP && stack.item().id(read) == "minecraft:shears" {
        return;
    }
    let food = stack.item().id(read);
    let Some(animal) = read.entity(event.entity as u32) else { return };
    // Only an animal the game may bring into love now: the comfort follows a feeding, not a heal.
    let Some(data) = animal.nbt(host) else { return };
    let a = Animal::read(&data);
    if !data.contains_key(&"InLove".into()) || a.age != 0 || a.in_love > 0 {
        return;
    }
    let count = stack.count();
    pending().push(Pending::Feed { player: p.uuid(), animal: event.entity as u32, hand: event.hand, food, count });
}

/// A player shears a sheep: remember it for the rest of the flock.
pub fn sheared(host: &mut dyn ServerHost, event: &PlayerShearEntityEvent<'_>) {
    let read: &dyn Host = host;
    let Some(p) = host.players().server_players().into_iter().find(|p| p.uuid() == event.player) else { return };
    if reach(read, p).is_none() || read.entity(event.entity_id as u32).and_then(|e| e.kind(read)).map(|k| k.id(read)).as_deref() != Some(SHEEP) {
        return;
    }
    let hand = if event.off_hand { Hand::Off } else { Hand::Main };
    pending().push(Pending::Shear { player: event.player, sheep: event.entity_id as u32, hand });
}

pub fn tick(host: &mut dyn ServerHost) {
    let work = std::mem::take(&mut *pending());
    for job in work {
        match job {
            Pending::Feed { player, animal, hand, food, count } => feed(host, player, animal, hand, &food, count),
            Pending::Shear { player, sheep, hand } => shear(host, player, sheep, hand),
        }
    }
}

fn player(host: &dyn ServerHost, uuid: u128) -> Option<PlayerRef> {
    host.players().server_players().into_iter().find(|p| p.uuid() == uuid)
}

/// The same kind's entities within `r` blocks of `at` (the one clicked left out).
fn around(host: &dyn Host, level: LevelRef, clicked: EntityRef, r: i32) -> Vec<EntityRef> {
    let read = host;
    let (Some(at), Some(kind)) = (clicked.position(read), clicked.kind(read)) else { return Vec::new() };
    let r = f64::from(r);
    let found = level.entities_in(read, [at[0] - r, at[1] - r, at[2] - r], [at[0] + r, at[1] + r, at[2] + r]).unwrap_or_default();
    let mut same: Vec<(f64, EntityRef)> = found
        .into_iter()
        .filter(|e| *e != clicked && e.kind(read) == Some(kind))
        .filter_map(|e| {
            let q = e.position(read)?;
            let d = (0..3).map(|i| (q[i] - at[i]).powi(2)).sum::<f64>();
            (d <= r * r).then_some((d, e))
        })
        .collect();
    same.sort_by(|a, b| a.0.total_cmp(&b.0));
    same.into_iter().map(|(_, e)| e).collect()
}

fn hearts(host: &mut dyn ServerHost, at: [f64; 3]) {
    host.commands().run_command(&format!("particle minecraft:heart {:.2} {:.2} {:.2} 0.3 0.3 0.3 0 3", at[0], at[1] + 0.8, at[2]));
}

fn feed(host: &mut dyn ServerHost, uuid: u128, id: u32, hand: Hand, food: &str, had: u32) {
    let read: &dyn Host = host;
    let Some(p) = player(&*host, uuid) else { return };
    let (Some(level), Some(r)) = (p.level(read), reach(read, p)) else { return };
    let Some(clicked) = read.entity(id) else { return };
    let Some(data) = clicked.nbt(host) else { return };
    let fed = Animal::read(&data);
    // The game turned the food down (wrong food, a baby, a wild horse): nothing follows.
    if fed.in_love <= 0 {
        return;
    }
    let read: &dyn Host = host;
    let Some(living) = read.living(p.entity()) else { return };
    let free = p.has_infinite_materials(read) == Some(true);
    let stack = living.hand_item(read, hand).filter(|s| s.item().id(read) == food);
    // Only the hand whose food the game took (in creative, the main hand) feeds the rest.
    let now = stack.as_ref().map_or(0, |s| s.count());
    if !(now < had || free && hand == Hand::Main) {
        return;
    }
    // The last food went to the one clicked.
    let Some(stack) = stack else { return };
    let others = around(read, level, clicked, r);
    let mut used = 0;
    for animal in others {
        if !free && used >= stack.count() {
            break;
        }
        let Some(data) = animal.nbt(host) else { continue };
        if !Animal::read(&data).feeds_with(&fed) {
            continue;
        }
        let mut love = Compound::new();
        love.insert("InLove".into(), Tag::Int(LOVE_TICKS));
        love.insert("LoveCause".into(), Tag::IntArray(uuid_ints(uuid)));
        if !animal.merge_nbt(host, love) {
            continue;
        }
        if let Some(at) = animal.position(&*host) {
            hearts(host, at);
        }
        used += 1;
    }
    if used > 0
        && !free
        && let Err(e) = living.set_hand_item(host, hand, stack.shrink(used))
    {
        eprintln!("[gathering] paying the food: {e:?}");
    }
}

fn shear(host: &mut dyn ServerHost, uuid: u128, id: u32, hand: Hand) {
    let read: &dyn Host = host;
    let Some(p) = player(&*host, uuid) else { return };
    let (Some(level), Some(r)) = (p.level(read), reach(read, p)) else { return };
    let Some(clicked) = read.entity(id) else { return };
    let Some(living) = read.living(p.entity()) else { return };
    let slot = match hand {
        Hand::Main => EquipmentSlot::MainHand,
        Hand::Off => EquipmentSlot::OffHand,
    };
    let flock = around(read, level, clicked, r);
    for sheep in flock {
        // The same shears, with a use to spare after this one.
        match crate::server::held_in(host, p, hand) {
            Some((tool, left)) if tool == "minecraft:shears" && left.is_none_or(|n| n > 1) => {}
            _ => return,
        }
        let read: &dyn Host = host;
        if read.living(sheep).and_then(|l| l.baby(read)) != Some(false) {
            continue;
        }
        let Some(data) = sheep.nbt(host) else { continue };
        if int(&data, "Sheared") != 0 {
            continue;
        }
        let Some(color) = wool_color(int(&data, "Color")) else { continue };
        let read: &dyn Host = host;
        let (Some(at), Some(pos), Some(shears)) = (sheep.position(read), sheep.block_position(read), living.hand_item(read, hand)) else { continue };
        let wool = match level.roll_loot(host, &format!("minecraft:shearing/sheep/{color}"), at, Some(&shears), Some(sheep)) {
            Ok(Some(stacks)) => stacks,
            other => {
                eprintln!("[gathering] shearing loot: {other:?}");
                continue;
            }
        };
        let mut cut = Compound::new();
        cut.insert("Sheared".into(), Tag::Byte(1));
        if !sheep.merge_nbt(host, cut) {
            continue;
        }
        for stack in wool {
            if let Err(e) = level.pop_resource(host, pos, stack) {
                eprintln!("[gathering] dropping wool: {e:?}");
            }
        }
        level.play_sound(host, "minecraft:entity.sheep.shear", at, mod_api::SoundSource::Players, 1.0, 1.0);
        living.hurt_and_break(host, slot, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_ready_adults_of_the_same_taming_are_fed() {
        let clicked = Animal { age: 0, in_love: 600, tame: false };
        assert!(Animal { age: 0, in_love: 0, tame: false }.feeds_with(&clicked));
        assert!(!Animal { age: -24000, in_love: 0, tame: false }.feeds_with(&clicked), "a baby");
        assert!(!Animal { age: 6000, in_love: 0, tame: false }.feeds_with(&clicked), "resting after breeding");
        assert!(!Animal { age: 0, in_love: 300, tame: false }.feeds_with(&clicked), "already in love");
        assert!(!Animal { age: 0, in_love: 0, tame: true }.feeds_with(&clicked), "a tame one beside wild ones");
        assert!(Animal { age: 0, in_love: 0, tame: true }.feeds_with(&Animal { tame: true, ..clicked }));
    }

    #[test]
    fn wool_colours_follow_the_dye_ids() {
        assert_eq!(wool_color(0), Some("white"));
        assert_eq!(wool_color(3), Some("light_blue"));
        assert_eq!(wool_color(15), Some("black"));
        assert_eq!(wool_color(16), None);
        assert_eq!(wool_color(-1), None);
    }

    #[test]
    fn love_cause_is_the_players_uuid_in_four_ints() {
        assert_eq!(uuid_ints(1), vec![0, 0, 0, 1]);
        assert_eq!(uuid_ints(0xffff_ffff_0000_0002_0000_0003_8000_0000), vec![-1, 2, 3, i32::MIN]);
    }
}
