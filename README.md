# Gathering

For [Horizon](https://playhorizon.dev) (Minecraft: Java Edition 26.3 in Rust). The chores of gathering become comforts you earn: break one ore and its whole vein comes out, break one log of a tree and the tree comes down, right-click a ripe crop and it is harvested and replanted. Your own tools, the game's own breaking (Fortune, Silk Touch, wear, hunger all as usual). English and Arabic.

## How it plays
- **Harvest**: right-click a ripe crop: harvested and replanted from its own drops. Wheat, carrots, potatoes, beetroot, nether wart, cocoa, torchflower, pitcher; sugar cane, cactus, bamboo, kelp and the nether vines keep their base; pumpkin and melon keep their stem. Each plant can be turned off in the settings.
- **Veins**: break an ore with a tool that gets its drops, and the touching ores of that kind (stone and deepslate together) come too.
- **Trees**: break a log with an axe and the tree comes down from the top. Only trees: logs touching natural leaves. Your house's logs break one at a time.
- **An outline** shows what will come with the block you aim at, in the game's own outline. Buried ores stay hidden.
- **Sneak** to break just one block.
- **Your tool is safe**: a vein or a tree stops while the tool has one use left.

## Comforts from Skills
With [Skills](https://github.com/ameliiom/skills) 0.2 or later, a tree **Gathering** grows as you mine ores, cut logs and harvest crops:
- **Vein Miner** I/II/III: 8, 24, 64 more ores.
- **Timber** I/II: 32, 256 more logs.
- **Leaf Blower**: a felled tree's leaves fall at once, with their saplings and apples.
- **Wide Harvest** I/II: a right-click harvests a 3×3, then 5×5 square.
- **Into The Bag**: what a vein, a tree or a harvest drops goes straight into your inventory.

Without Skills, the server setting **All comforts without Skills** opens them all.

## Install
1. Copy `gathering.hmod` into `%APPDATA%\horizon\horizon-native-mods\`.
2. Add the line `mod.gathering:true` to `%APPDATA%\horizon\horizon-mods.txt`.
3. Build: the launcher's Mods page **Build**, or `horizon-launcher native-build --instance horizon --trust-native-code`. A native mod runs with your rights: install it only from a source you trust.

The server and every player need the mod. Built for Horizon Nightly `f037744` (2026-11-21).

## Add your ores
Each ore family is a block tag `gathering:vein/<family>` (coal, iron, copper, gold, redstone, lapis, diamond, emerald, quartz, debris) holding the game's own `minecraft:<family>_ores`; a data pack adds a mod's ores to them.

## Credits
Harvesting is [Right-Click Crops](https://github.com/AyanoKaito/horizon-right-click-crops) by AyanoKaito (MIT), ported to a native mod: see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Licence
MIT, see [LICENSE](LICENSE). Not affiliated with the Horizon project.
