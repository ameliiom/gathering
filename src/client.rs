//! The client: while you aim at an ore or a tree's log with the tool that would take its vein or
//! tree, the blocks that would come with it are outlined as the game outlines the block you aim
//! at (thin, dark, see-through). Only faces you can see draw: buried ores stay hidden, so the
//! outline is no X-ray. The plan comes from the same rules the server breaks by (`plan.rs`).
use crate::perks::Comforts;
use crate::plan::{self, Plan};
use crate::shape::Pos;
use mod_api::{ClientHost, CrosshairTarget, EquipmentSlot, Host, RenderLevelStage, RenderLevelStageEvent, WorldDepth, WorldLine};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::Mutex;

/// The game's own block outline: black at 40%, a little wider so the group reads at a glance.
const COLOUR: u32 = 0x6600_0000;
const WIDTH: f32 = 2.0;

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

/// The twelve edges of the block at `p`, a hair outside it as the game draws its outline.
fn edges([x, y, z]: Pos) -> impl Iterator<Item = WorldLine> {
    let (lo, hi) = (-0.002, 1.002);
    let c = move |i: f64, j: f64, k: f64| [f64::from(x) + i, f64::from(y) + j, f64::from(z) + k];
    let corners = [(lo, lo, lo), (hi, lo, lo), (hi, lo, hi), (lo, lo, hi)];
    (0..4).flat_map(move |n| {
        let (a, b) = (corners[n], corners[(n + 1) % 4]);
        [
            WorldLine::new(c(a.0, a.1, a.2), c(b.0, b.1, b.2), COLOUR, WIDTH, WorldDepth::Tested),
            WorldLine::new(c(a.0, hi, a.2), c(b.0, hi, b.2), COLOUR, WIDTH, WorldDepth::Tested),
            WorldLine::new(c(a.0, lo, a.2), c(a.0, hi, a.2), COLOUR, WIDTH, WorldDepth::Tested),
        ]
    })
}

pub fn draw(host: &dyn ClientHost, event: &mut RenderLevelStageEvent<'_>) {
    if !matches!(event.stage, RenderLevelStage::AfterTranslucentBlocks) {
        return;
    }
    let blocks = aimed(host);
    if blocks.is_empty() {
        return;
    }
    let lines: Vec<WorldLine> = blocks.into_iter().flat_map(edges).collect();
    if let Err(e) = event.draw.lines(&lines) {
        eprintln!("[gathering] drawing the outline: {e:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_has_twelve_edges_just_outside_it() {
        let lines: Vec<WorldLine> = edges([2, 3, 4]).collect();
        assert_eq!(lines.len(), 12);
        for l in &lines {
            for end in [l.from, l.to] {
                for (v, base) in end.iter().zip([2.0, 3.0, 4.0]) {
                    assert!(*v > base - 0.01 && *v < base + 1.01);
                }
            }
            let length: f64 = (0..3).map(|i| (l.to[i] - l.from[i]).powi(2)).sum::<f64>().sqrt();
            assert!((length - 1.004).abs() < 1e-9, "an edge is one block long");
        }
    }
}
