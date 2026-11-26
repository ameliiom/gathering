//! Gathering: break one ore and its vein comes out, break one log and its tree comes down,
//! right-click a ripe crop and it is harvested and replanted; the bigger comforts are earned in
//! the Skills tree Gathering. See `docs/design.md`.
//!
//! - **What a break takes** (`shape.rs`): joined blocks, nearest first, capped; trees top down.
//! - **The comforts** (`perks.rs`): the tags the tree gives, read with no code tie to Skills.
//! - **The server** (`server.rs`): a job per player, a few dozen blocks a tick, each broken as the
//!   player's own break; a felled tree's leaves after it.
//! - **Harvest** (`crops.rs`, `harvest.rs`): Right-Click Crops by AyanoKaito, ported, with the
//!   wide harvest around the crop clicked.
//! - **The bag**: drops into the inventory (`kit::inventory::give`).
//! - **Batch 2**: auto-smelt (`smelt.rs`), wide feeding and shearing (`ranch.rs`), wide planting
//!   (`planting.rs`), auto-fishing (`fishing.rs`) and saplings that plant themselves (`saplings.rs`).
// Kit (ameliiom/kit), the shared code: the installed Kit mod, or its source in place for
// local builds (`kit_local`, horizon-mods/.cargo/config.toml).
#[cfg(kit_local)]
#[allow(dead_code)]
#[path = "../../kit/src/lib.rs"]
mod kit;
#[cfg(not(kit_local))]
use ::kit;
mod client;
pub mod crops;
mod fishing;
mod harvest;
mod payloads;
pub mod perks;
mod plan;
pub mod planting;
mod ranch;
mod saplings;
mod server;
pub mod shape;
mod smelt;

use mod_api::{
    BlockDropsEvent, EntityInteractEvent, ItemSpawnEvent, LevelHost, PlayerFishEvent, PlayerShearEntityEvent, BreakBlockEvent, ClientHost, Flow, Hooks, HostSide, IPayloadContext, InteractionResult, PayloadId, PlayerLoggedOutEvent, Registrar,
    RegistrationContext, RenderLevelStageEvent, RightClickBlockEvent, ServerHost, ServerTickEvent, ServerTickPhase,
};
#[allow(unused_imports)]
use mod_api::prelude::*;

pub(crate) const MOD_ID: &str = "gathering";
/// `gathering:harvested`: every crop a right-click harvests (the tree's experience).
pub(crate) const HARVESTED: &str = "harvested";

/// Settings (Options > Mods > Gathering): every comfort without Skills (`kit::comforts::ALL`),
/// one on/off per comfort of batch 2 and one per plant (`harvest::setting`).
pub(crate) mod setting {
    use crate::kit::setting::Toggle;
    /// One on/off for each comfort of batch 2, on by default (it still has to be earned).
    pub const SMELT: Toggle = Toggle::on("smelt");
    pub const RANCH: Toggle = Toggle::on("ranch");
    pub const PLANTING: Toggle = Toggle::on("planting");
    pub const FISHING: Toggle = Toggle::on("fishing");
    pub const SAPLINGS: Toggle = Toggle::on("saplings");
    pub const COMFORTS: [Toggle; 5] = [SMELT, RANCH, PLANTING, FISHING, SAPLINGS];
}

/// Whether a comfort's setting is on (it is unless turned off).
pub(crate) fn enabled(host: &dyn mod_api::Host, key: kit::setting::Toggle) -> bool {
    key.read(host, MOD_ID)
}

fn on_caps(caps: payloads::Caps, _: &mut dyn IPayloadContext) -> Result<(), String> {
    client::caps(caps.vein.0.max(0) as usize, caps.timber.0.max(0) as usize);
    Ok(())
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let specs = std::iter::once(kit::comforts::ALL)
            .chain(setting::COMFORTS)
            .chain(crops::Plant::ALL.into_iter().map(harvest::setting))
            .map(|t| t.spec(MOD_ID));
        if let Err(e) = r.register_custom_stat(HARVESTED) {
            eprintln!("[gathering] declaring statistic {HARVESTED}: {e}");
        }
        for spec in specs {
            if let Err(e) = r.register_setting(spec) {
                eprintln!("[gathering] a setting: {e:?}");
            }
        }
        let mut handlers = r.payload_handlers();
        let mut v1 = handlers.registrar("1").optional();
        let _ = payloads::CAPS.set(v1.play_to_client_typed(PayloadId("gathering:caps".into()), on_caps));
    }

    /// The outline of what the aimed break would take.
    fn render_level_stage(&self, host: &dyn ClientHost, event: &mut RenderLevelStageEvent<'_>) {
        client::draw(host, event);
    }

    /// A player breaks a block: an ore or a tree's log may bring the rest with it.
    fn break_block(&self, host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
        server::broke(host, event.pos, event.state);
        Flow::Continue
    }

    /// A right-click on a ripe crop harvests it (and its neighbours, with the comfort). The
    /// client makes the same choice from what it sees, so it doesn't go on to use the item.
    fn right_click_block(&self, host: HostSide, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let click = *event.click;
        let handled = match host {
            HostSide::Server(host) => {
                let harvested = harvest::on_server(host, &click);
                if !harvested {
                    // Seeds on farmland: the game plants the one clicked, the comfort the rest.
                    planting::on_server(host, &click);
                }
                harvested
            }
            HostSide::Client(host) => harvest::on_client(host, &click),
        };
        if handled {
            *event.cancellation_result = InteractionResult::Success;
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            server::tick(host);
        }
    }

    fn player_logged_out(&self, _: &mut dyn ServerHost, event: &mut PlayerLoggedOutEvent<'_>) {
        server::forget(event.player);
    }

    /// Saplings plant themselves: a sapling item starts its wait.
    fn item_spawn(&self, host: &mut dyn ServerHost, event: &mut ItemSpawnEvent<'_>) {
        saplings::spawned(&*host, event);
    }

    /// Auto-smelt: a mined ore's drops, before they spawn.
    fn block_drops(&self, level: &dyn LevelHost, event: &mut BlockDropsEvent<'_>) -> Flow {
        smelt::drops(level, event);
        Flow::Continue
    }

    /// Wide feeding: the game feeds the animal clicked; next tick the same kind around it too.
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        if let HostSide::Server(host) = host {
            ranch::interacted(host, event);
        }
        None
    }

    /// Wide shearing: the game shears the sheep clicked; next tick the rest around it.
    fn player_shear_entity(&self, host: &mut dyn ServerHost, event: &mut PlayerShearEntityEvent<'_>) -> Flow {
        ranch::sheared(host, event);
        Flow::Continue
    }

    /// Auto-fishing: a bite is taken from the game and landed in the inventory next tick.
    fn player_fish(&self, host: &mut dyn ServerHost, event: &mut PlayerFishEvent<'_>) -> Flow {
        if fishing::bite(host, event) { Flow::Cancel } else { Flow::Continue }
    }
}

/// The Skills trees Gathering adds its nodes to (no tree of its own: the comforts extend Mining
/// and Farming), and their files' text joined, for the tests.
#[cfg(test)]
pub(crate) const EXTENDS: [(&str, &str); 2] = [("mining", "skills:mining"), ("farming", "skills:farming")];

#[cfg(test)]
pub(crate) fn tree_files() -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/gathering/skills");
    EXTENDS.iter().map(|(file, _)| std::fs::read_to_string(dir.join(format!("{file}.json"))).unwrap()).collect::<Vec<_>>().join("
")
}

#[cfg(test)]
mod tree_tests {
    use super::EXTENDS;
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Each `"<field>": "value"` string in a tree file, in order.
    fn strings(text: &str, field: &str) -> Vec<String> {
        text.split(&format!("\"{field}\"")).skip(1).filter_map(|r| r.split('"').nth(1).map(str::to_owned)).collect()
    }

    /// Each node's grid spot, whitespace removed: `[1,4]`.
    fn spots(text: &str) -> Vec<String> {
        text.split("\"at\"").skip(1).filter_map(|r| Some(r[r.find('[')?..=r.find(']')?].split_whitespace().collect())).collect()
    }

    /// Gathering ships no tree of its own: only additions to Skills' trees, and each node keeps
    /// clear of the Skills tree's own ids and grid spots (read from the Skills checkout beside
    /// this one when it is there).
    #[test]
    fn nodes_extend_skills_trees_without_clashing() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let dir = root.join("data/gathering/skills");
        let files: BTreeSet<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        let wanted: BTreeSet<String> = EXTENDS.iter().map(|(f, _)| format!("{f}.json")).collect();
        assert_eq!(files, wanted, "only extension files, one per Skills tree");
        let skills = root.join("../skills/assets/skills/packs/defaults/data/skills/skills");
        for (file, tree) in EXTENDS {
            let ours = std::fs::read_to_string(dir.join(format!("{file}.json"))).unwrap();
            assert_eq!(strings(&ours, "extends"), vec![tree.to_string()]);
            assert!(strings(&ours, "title").iter().all(|t| t.starts_with("gathering.node.")), "a tree title in {file}");
            let theirs = std::fs::read_to_string(skills.join(format!("{file}.json"))).unwrap_or_default();
            let mut ids: BTreeSet<String> = strings(&theirs, "id").into_iter().collect();
            let mut taken: BTreeSet<String> = spots(&theirs).into_iter().collect();
            let (our_ids, our_spots) = (strings(&ours, "id"), spots(&ours));
            assert_eq!(our_ids.len(), our_spots.len());
            for (id, at) in our_ids.iter().zip(&our_spots) {
                assert!(ids.insert(id.clone()), "{tree}: node id {id} taken");
                assert!(taken.insert(at.clone()), "{tree}: spot {at} of {id} taken");
            }
        }
    }
}
