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
mod bag;
mod client;
pub mod crops;
mod harvest;
mod payloads;
pub mod perks;
mod plan;
mod server;
pub mod shape;

use mod_api::{
    BreakBlockEvent, ClientHost, Flow, Hooks, HostSide, IPayloadContext, InteractionResult, PayloadId, PlayerLoggedOutEvent, Registrar,
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
            HostSide::Server(host) => harvest::on_server(host, &click),
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
}
