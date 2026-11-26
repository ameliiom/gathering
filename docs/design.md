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

## 4. The Gathering tree in Skills (until 0.3.0; see §11)

| Always | Earned in Gathering |
|---|---|
| Right-click harvest of one crop | **Vein Miner** I/II/III (8, 24, 64 ores) · **Timber** I/II (32, 256 logs) · **Leaf Blower** (the felled tree's leaves fall at once) · **Wide Harvest** I/II (3×3, 5×5 crops) · **Into The Bag** (a vein's, tree's or harvest's drops go straight into the inventory) |

The tree: row one Vein Miner I → II → III; row two Timber I → II → Leaf Blower; row three Wide
Harvest I → II → Into The Bag.

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

## 7. The full build (0.2.0), measured in the game (2026-11-25)

| What | Result |
|---|---|
| Right-click on ripe wheat, no comfort (7×7 field) | 1 harvested and replanted (age 0) |
| Wide Harvest I (3×3) | 9 replanted; Wide Harvest II (5×5) at a field's edge: the 20 inside the field |
| Into The Bag, 3×3 harvest | 9 wheat and 16 seeds in the inventory, nothing on the ground |
| Into The Bag, 64-ore vein | 64 raw iron in the inventory, nothing on the ground, every ore gone |
| Leaf Blower, felled oak | logs and leaves gone 30 ticks later; 7 drops (saplings, sticks, apples) as decay gives |
| Outline | aiming at a 10-ore vein with a pickaxe outlines all 10 (stone and deepslate) in the game's thin dark outline; buried faces don't draw |

- Harvest is Right-Click Crops 2.0.1 (16 plant families, its settings per plant, loot-paid
  replanting), ported to native with its own tests kept; the wide harvest asks its same rules at
  every crop around the one clicked, and a crop harvested counts toward the tree
  (`gathering:harvested`).
- Leaf Blower waits 8 ticks after the last log (a leaf's distance spreads a block a tick), then
  drops only natural leaves whose distance is 7: a neighbour tree's leaves stay.
- The client learns its vein and tree sizes from the server (`gathering:caps`, when they change,
  checked each second) and plans with the same code the server breaks by (`plan.rs`).

## 8. Risks (pre-mortem) and answers

| Risk | Answer |
|---|---|
| A build loses its wooden pillars | trees need natural leaves (`persistent=false`); sneak breaks one; measured on a wall |
| A big vein lags the server | the cap (64 at most) measured at 32 ms; a vein over 32 blocks breaks over two ticks (measured: see §6) |
| A favourite tool breaks mid-vein | stop one use before breaking |
| Too strong too early | small at first (8 ores, 32 logs), earned in the tree; hunger and wear stay the game's |
| A friend's mods or claims | `destroy_block` keeps spawn protection, adventure mode and other mods' cancels |

## 9. Phases (all built in 0.2.0)

1. Veins and trees, the Gathering tree, tags, settings (the core).
2. Right-click harvest (the forked Right-Click Crops, native) and Wide Harvest.
3. Leaf Blower, Into The Bag, and an outline of what will break while you mine (as FTB Ultimine
   shows), drawn in the game's own block-outline look.

## 10. Batch 2 and the saplings (0.3.0), measured in the game (2026-11-26)

Five more comforts, each a node of the Gathering tree (vanilla tags, as before) and each with its
own on/off in the mod's settings (on by default, still to be earned): Auto-Smelt (`gathering.smelt`,
after Vein Miner III), Self-Planting Saplings (`gathering.saplings`, after Leaf Blower), Wide
Planting I/II and Whole Field (`gathering.plant.3/5/field`, a row under Wide Harvest), Ranch
I/II/III (`gathering.ranch.3/5/7`) and Auto-Fishing (`gathering.fishing`). The tree also grows from
animals bred (1) and fish caught (2).

| What | Result |
|---|---|
| Auto-Smelt, Vein Miner I, 6-ore iron vein (stone and deepslate) | 6 iron ingots, 4 experience (0.7 × 6, the 0.2 left carried to the next ore) |
| Auto-Smelt with Fortune III, single ores | 3, 3, 4, 1, 1, 1 ingots: each drop smelted count for count |
| Auto-Smelt with Into The Bag, 4-ore vein | 9 ingots in the inventory, no raw iron |
| Sneaking, Fortune III | 2 raw iron: not smelted |
| Wide Planting I (3×3), 7×7 farmland, 20 seeds | 9 wheat planted, 9 seeds paid |
| Whole Field, fresh 7×7, 64 seeds / 5 seeds | 49 planted, 49 paid / 5 planted, none left |
| Ranch I, 7 cows (5 adults within 3 blocks, 1 at 6, 1 calf), 10 wheat | the 5 near adults in love, 5 wheat paid; the far cow and the calf left |
| Ranch I, shears on one of 4 sheep (plus one far, one already shorn) | the 4 near sheep shorn, the far one kept its wool, shears worn |
| Auto-Fishing, Lure III, 3 bites | 3 cod in the inventory, bobber gone each time, rod damage 1 a catch, 2 experience, `fish_caught` +1 a fish |
| Saplings, 3 oak saplings dropped on grass | after 5 s one planted where they lay; the other 2 wait (no sapling right beside another) |
| Saplings on stone / with the comfort off | stay items |

How each works:
- **Auto-Smelt** changes the drops in `block_drops` (the level's own hook, before anything
  spawns): a block of `#gathering:veins` broken by a player who chose it at `break_block` time.
  The furnace's recipes are read from `recipes_of_type("minecraft:smelting")` once per reload.
- **Wide Planting** runs beside the game's own planting of the farmland clicked: joined empty
  farmland on the same level (`shape::joined`), within the square, one seed each from other slots
  first, the hand's last seed left for the game's click.
- **Ranch** follows the game: the tick after a feeding or a shearing, if the animal clicked was
  fed (in love) or shorn, the same kind around it is done too (`InLove` 600 with the player as
  `LoveCause`, so the breeding is theirs; wool from `minecraft:shearing/sheep/<colour>`, since the
  `shearing/sheep` dispatcher rolls nothing from a mod's roll, measured). Shears stop one use
  before breaking.
- **Saplings** start a 5 s wait on `item_spawn`; then, on the ground, over `#minecraft:supports_vegetation`
  (the game's own rule for plants: in 26.3 grass is not in `#minecraft:dirt`, measured), with air
  above, no sapling in the 8 spots around, and a player with the comfort within 48 blocks, one of
  the stack is planted (looked at again up to 8 times while it waits).
- **Auto-Fishing** cancels the bite (`player_fish` state 1) and, the next tick, rolls
  `minecraft:gameplay/fishing` at the bobber with the bobber as the loot entity (open-water
  treasure kept) and the rod as tool. Luck of the Sea's bonus is not added (the roll has no luck).

## 11. No tree of its own (0.4.0, 2026-11-26)

The owner's rule: new work extends what exists and never repeats a meaning. Gathering's own tree
repeated Skills' Mining and Farming, so it is retired: `data/gathering/skills/mining.json` and
`farming.json` add the same nodes (same tags, so no code changed) with
`{"extends": "skills:mining"}` and `{"extends": "skills:farming"}`.

| Skills tree | Gathering's nodes (grid spots free beside the tree's own) | Sources it adds |
|---|---|---|
| Mining (Haste, Diver, Fireproof, Lucky in rows 0-3) | row 4 Vein Miner I-III, Auto-Smelt; row 5 Timber I-II, Leaf Blower, Into The Bag | logs mined (1): Mining counts pickaxe and shovel blocks and ores, not logs |
| Farming (Hearty, Fortune, Herder in rows 0-2) | Ranch I-III beside Herder (Ranch I needs it); row 3 Wide Harvest I-II; row 4 Wide Planting I-II, Whole Field; row 5 Auto-Fishing, Self-Planting Saplings | crops harvested by right-click (2, as Farming's broken ripe crops): the harvest replaces the crop without a break, so Farming would not see it. Animals bred and fish caught Farming already counts. |

A test keeps it so: only extension files, each extending its Skills tree, no tree title, and no
node id or grid spot that the Skills tree (read from the checkout beside this one) already uses.
Progress in the old Gathering tree is lost; its tags are removed by Skills on the next join and
earned again in Mining and Farming.

Measured in the game (2026-11-26): the Skills screen has no Gathering tab; Mining shows Vein Miner
I-III, Auto-Smelt, Timber I-II, Leaf Blower and Into The Bag in rows 5 and 6 under Lucky, and
Farming shows Ranch beside Herder, Wide Harvest, Wide Planting, Auto-Fishing and the saplings.
The test player's old tags were gone after joining (Skills' reconcile). Unlocking Vein Miner I in
Mining gave `gathering.vein.8`, and breaking one ore of a 6-ore iron vein took all 6.
