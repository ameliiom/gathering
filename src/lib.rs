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
//! - **The bag** (`bag.rs`): drops into the inventory.
//! - **Batch 2**: auto-smelt (`smelt.rs`), wide feeding and shearing (`ranch.rs`), wide planting
//!   (`planting.rs`), auto-fishing (`fishing.rs`) and saplings that plant themselves (`saplings.rs`).
mod bag;
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
    RegistrationContext, RenderLevelStageEvent, RightClickBlockEvent, ServerHost, ServerTickEvent, ServerTickPhase, SettingSpec,
};
#[allow(unused_imports)]
use mod_api::prelude::*;

pub(crate) const MOD_ID: &str = "gathering";
/// `gathering:harvested`: every crop a right-click harvests (the tree's experience).
pub(crate) const HARVESTED: &str = "harvested";

/// Settings (Options > Mods > Gathering): every comfort without Skills, and one on/off per plant.
pub(crate) mod setting {
    /// Opens every comfort for a world that doesn't use Skills.
    pub const ALL_COMFORTS: &str = "all_comforts";
    /// One on/off for each comfort of batch 2, on by default (it still has to be earned).
    pub const SMELT: &str = "smelt";
    pub const RANCH: &str = "ranch";
    pub const PLANTING: &str = "planting";
    pub const FISHING: &str = "fishing";
    pub const SAPLINGS: &str = "saplings";
    pub const COMFORTS: [&str; 5] = [SMELT, RANCH, PLANTING, FISHING, SAPLINGS];
}

fn comfort_setting(key: &'static str) -> SettingSpec {
    SettingSpec::toggle(key, match key {
        setting::SMELT => "gathering.config.smelt",
        setting::RANCH => "gathering.config.ranch",
        setting::PLANTING => "gathering.config.planting",
        setting::SAPLINGS => "gathering.config.saplings",
        _ => "gathering.config.fishing",
    }, true)
}

/// Whether a comfort's setting is on (it is unless turned off).
pub(crate) fn enabled(host: &dyn mod_api::Host, key: &'static str) -> bool {
    let stored = host.setting(MOD_ID, key);
    !matches!(comfort_setting(key).read(stored.as_deref()), mod_api::SettingValue::Toggle(false))
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
        let specs = std::iter::once(SettingSpec::toggle(setting::ALL_COMFORTS, "gathering.config.all_comforts", false))
            .chain(setting::COMFORTS.into_iter().map(comfort_setting))
            .chain(crops::Plant::ALL.into_iter().map(harvest::setting));
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
