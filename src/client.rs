//! The client: while you aim at an ore or a tree's log with the tool that would take its vein or
//! tree, the blocks that would come with it are outlined as the game outlines the block you aim
//! at (thin, dark, see-through). Only faces you can see draw: buried ores stay hidden, so the
//! outline is no X-ray. The plan comes from the same rules the server breaks by (`plan.rs`).
use crate::perks::Comforts;
use crate::plan::{self, Plan};
use crate::shape::Pos;
use mod_api::{ClientHost, CrosshairTarget, EquipmentSlot, Host, RenderLevelStage, RenderLevelStageEvent};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::Mutex;

struct Outline {
    /// The vein and tree sizes this player has (the server tells).
    caps: (usize, usize),
    /// The last plan and what it was for (block, its state, the held item), so it is walked
    /// once per aim, not once per frame.
    last: Option<((Pos, String, String), Vec<Pos>)>,
}

static OUTLINE: Mutex<Outline> = Mutex::new(Outline { caps: (0, 0), last: None });

fn with<R>(f: impl FnOnce(&mut Outline) -> R) -> R {
    f(&mut OUTLINE.lock().unwrap_or_else(|e| e.into_inner()))
}

/// The server told this player's vein and tree sizes.
pub fn caps(vein: usize, timber: usize) {
    with(|o| {
        o.caps = (vein, timber);
        o.last = None;
    });
}

/// The blocks the aimed break would take along, from this client's copy of the world.
fn aimed(host: &dyn ClientHost) -> Vec<Pos> {
    let read: &dyn Host = host;
    let (vein, timber) = with(|o| o.caps);
    if vein == 0 && timber == 0 {
        return Vec::new();
    }
    let Some(CrosshairTarget::Block { pos, state, .. }) = host.crosshair_target() else { return Vec::new() };
    let pos = [pos.x, pos.y, pos.z];
    let Some(p) = host.local_player() else { return Vec::new() };
    if p.entity().is_crouching(read) == Some(true) {
        return Vec::new();
    }
    let Some(level) = p.level(read) else { return Vec::new() };
    let tool = read
        .living(p.entity())
        .and_then(|l| l.equipment(read, EquipmentSlot::MainHand))
        .filter(|s| s.count() > 0)
        .map(|s| s.item().id(read))
        .unwrap_or_default();
    let key = (pos, plan::name(read, state).unwrap_or_default(), tool.clone());
    if let Some(blocks) = with(|o| o.last.as_ref().filter(|(k, _)| *k == key).map(|(_, b)| b.clone())) {
        return blocks;
    }
    let correct = p.has_correct_tool_for_drops(read, state) == Some(true);
    let c = Comforts { vein, timber, ..Comforts::default() };
    let blocks = plan::plan(read, level, pos, state, &tool, correct, &c).map(|Plan { blocks, .. }| blocks).unwrap_or_default();
    with(|o| o.last = Some((key, blocks.clone())));
    blocks
}

pub fn draw(host: &dyn ClientHost, event: &mut RenderLevelStageEvent<'_>) {
    if !matches!(event.stage, RenderLevelStage::AfterTranslucentBlocks) {
        return;
    }
    let blocks = aimed(host);
    if blocks.is_empty() {
        return;
    }
    if let Err(e) = event.draw.lines(&crate::kit::outline::lines(blocks)) {
        eprintln!("[gathering] drawing the outline: {e:?}");
    }
}
