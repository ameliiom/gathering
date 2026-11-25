# Gathering: design

The chores of gathering become comforts earned in Skills: break one ore and its whole vein comes
out, break one log of a tree and the tree comes down, right-click a ripe crop and it is harvested
and replanted. No new blocks, keys or screens: the player's own tools, the game's own breaking.

## 1. Outcome

A mining trip, a wood run or a farm round takes a fraction of the clicks, and the time saved is
earned in play (a new tree in Skills, **Gathering**), as Storage's Organization does.

## 2. Prior art (searched 2026-11-25)

| What | Where | Licence | Kind | Use |
|---|---|---|---|---|
| Right-Click Crops 2.0.1 | github.com/AyanoKaito/horizon-right-click-crops | MIT | Horizon mod, **wasm**, 16 plant families, settings per plant, loot-paid replant, tests | **Fork** under ameliiom keeping its notice, convert to native, add the area perks |
| FTB Ultimine, Veinminer | Java (NeoForge/Fabric) | various | vein mining with a held key, a shape preview, a block cap, hunger cost | design only |
| FallingTree | Java | various | whole tree from one log, leaves check, sneak for one log, fast leaf decay | design only |
| Horizon registry, GitHub `horizon-mod.toml` search | playhorizon.dev/mods (down today), GitHub | | no vein or tree mod for Horizon | build ours |

Horizon's own API was written for this: `PlayerRef::destroy_block` breaks a block "as FTB
Ultimine does for each block of a vein": the player's own checks, loot for the held tool with
Fortune and Silk Touch, the tool's wear, statistics and hunger.

## 3. How it plays

- **Veins**: break an ore with the right tool and every touching ore of that kind (deepslate and
  stone variants together) breaks too, up to the vein size you earned. Sneak to break just one.
- **Trees**: break a log and the logs above and around it come down, but only for a tree:
  logs touching natural leaves. Logs in a house or a wall break one at a time. Sneak for one.
- **Harvest**: right-click a ripe crop: harvested and replanted from its own drops (Right-Click
  Crops' rules: wheat, carrots, potatoes, beetroot, nether wart, cocoa, torchflower, pitcher; cane,
  cactus, bamboo, kelp and vines keep their base; pumpkin and melon keep the stem).
- **The tool is safe**: a vein or tree stops one use before the tool would break (measured: see §6).
- Drops fall where each block was, as the game drops them (or into the bag with the comfort).

## 4. The Gathering tree in Skills

| Always | Earned in Gathering |
|---|---|
| Right-click harvest of one crop | **Vein Miner** I/II/III (8, 24, 64 ores) · **Timber** I/II (32, 256 logs) · **Leaf Blower** (the felled tree's leaves fall at once) · **Wide Harvest** I/II (3×3, 5×5 crops) · **Into The Bag** (a vein's, tree's or harvest's drops go straight into the inventory) |

- Experience: ores and logs mined (the game's `mined` statistic), crops harvested (Gathering's
  own statistic).
- As with Storage, no code tie: each node gives a vanilla tag (`gathering.vein.24`…), the tree
  is a data file Skills reads; without Skills, the server setting `all_comforts` opens them.

## 5. Measured (owner's PC, Horizon f037744, a spike build, 2026-11-25)

| What | Result |
|---|---|
| One deepslate iron ore broken in a 72-ore vein (cap 64) | 65 broken, found in 0.17 ms, broken in **32 ms**; 7 left; diamond pickaxe wear **65** (one per block); 72 raw iron dropped |
| One oak log, small oak | 4 logs down, 0.6 ms |
| One log of a fancy oak | **20 logs** down, **1.9 ms** |
| One log of a 16-log wall with no leaves | **only that log** (the natural-leaves check) |
| Breaks run inside the break | our own guard keeps them from starting veins of their own |
| Ore kinds | the spike grouped ores by name (`deepslate_` and `nether_` stripped); the build groups them by the game's own block tags (`minecraft:iron_ores`, `minecraft:gold_ores`…), which data packs extend and other mods fill (to check in the game at build time) |

## 6. The skeleton (0.1.0), measured in the game (2026-11-25)

Veins, trees and the Gathering tree (Vein Miner I/II/III: 8, 24, 64; Timber I/II: 32, 256), with
the tags, the `all_comforts` setting, both languages and the gates (tree tags = tags read, each ore
family has its tag file, every shown key in both languages).

| Case (72-ore vein of stone and deepslate ore) | Result |
|---|---|
| No comfort | 1 broken (only the one hit) |
| Vein Miner I (8), diamond pickaxe | 9 broken, pickaxe wear 9 |
| Vein Miner III (64), gold and deepslate gold | 65 broken, wear 65: the game's own tag `minecraft:gold_ores` joins both |
| Sneaking | 1 broken |
| Wooden pickaxe on iron (no drops) | 1 broken: no vein without the right tool |
| Pickaxe with 10 uses left | 9 broken, stopped with **1 use left** (damage 1560 of 1561) |
| Fancy oak, axe, Timber I | every log down |
| Oak, axe, no comfort / oak, pickaxe | one log |
| 24-log wall, bare or under placed leaves | one log |
| Skills screen | the Gathering tab shows both rows; mining the tests earned 6 levels |

Blocks break 32 a tick from the next tick on, so a 64-ore vein takes two ticks.

## 7. Risks (pre-mortem) and answers

| Risk | Answer |
|---|---|
| A build loses its wooden pillars | trees need natural leaves (`persistent=false`); sneak breaks one; measured on a wall |
| A big vein lags the server | the cap (64 at most) measured at 32 ms; a vein over 32 blocks breaks over two ticks (measured: see §6) |
| A favourite tool breaks mid-vein | stop one use before breaking |
| Too strong too early | small at first (8 ores, 32 logs), earned in the tree; hunger and wear stay the game's |
| A friend's mods or claims | `destroy_block` keeps spawn protection, adventure mode and other mods' cancels |

## 8. Phases

1. Veins and trees, the Gathering tree, tags, settings (the core).
2. Right-click harvest (the forked Right-Click Crops, native) and Wide Harvest.
3. Leaf Blower, Into The Bag, and an outline of what will break while you mine (as FTB Ultimine
   shows), drawn in the game's own block-outline look.
