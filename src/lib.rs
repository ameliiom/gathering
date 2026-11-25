//! Gathering: break one ore and its vein comes out, break one log and its tree comes down, as
//! comforts earned in the Skills tree Gathering. See `docs/design.md`.
//!
//! - **What a break takes** (`shape.rs`): joined blocks, nearest first, capped; trees top down.
//! - **The comforts** (`perks.rs`): the tags the tree gives, read with no code tie to Skills.
//! - **The server** (`server.rs`): a job per player, a few dozen blocks a tick, each broken as the
//!   player's own break.
pub mod perks;
pub mod shape;
mod server;

use mod_api::{BreakBlockEvent, Flow, Hooks, PlayerLoggedOutEvent, Registrar, RegistrationContext, ServerHost, ServerTickEvent, ServerTickPhase, SettingSpec};
#[allow(unused_imports)]
use mod_api::prelude::*;

pub(crate) const MOD_ID: &str = "gathering";

/// Settings (Options > Mods > Gathering).
pub(crate) mod setting {
    /// Opens every comfort for a world that doesn't use Skills.
    pub const ALL_COMFORTS: &str = "all_comforts";
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        if let Err(e) = r.register_setting(SettingSpec::toggle(setting::ALL_COMFORTS, "gathering.config.all_comforts", false)) {
            eprintln!("[gathering] setting {}: {e:?}", setting::ALL_COMFORTS);
        }
    }

    /// A player breaks a block: an ore or a tree's log may bring the rest with it.
    fn break_block(&self, host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
        server::broke(host, event.pos, event.state);
        Flow::Continue
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
