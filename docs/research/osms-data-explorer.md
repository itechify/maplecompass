# OSMS Data Explorer as the data source in place of MeowDB

Checked on 10 October 2026, against OSMS's `data/current` (its "Public Release" set, dated 6 October 2026), the MeowDB pages then kept in `crates/compass-core/tests/fixtures` (saved 7 October, since removed), and three live MeowDB pages. The decisions it led to, and what shipped, are in ADR 0008.

## Answer

OSMS Data Explorer can supply nearly everything MapleCompass reads from MeowDB, and it doesn't have MeowDB's scraping problem in the same form. It is a static site that loads a handful of plain JSON files. No terms forbid fetching them, and the five files the whole world needs are about 0.8 MB in one go, not about 420 map pages and the listed mobs' pages, fetched one every 2 seconds. Most of the panel's data maps over directly, the Map Summary can be computed exactly, and three Mob Card figures can be derived with fitted rules. Two things have no OSMS equivalent: minimap bounds, which place Hidden Portals, and whether a mob jumps between floors. Item Lookup can keep opening MeowDB pages, but it still needs one MeowDB search per press to find the page.

## What OSMS is

[osmsdataexplorer.com](https://osmsdataexplorer.com/#overview) is a GitHub Pages site behind Cloudflare, built from the public repo [ohmi69/osms_datamine_dashboard](https://github.com/ohmi69/osms_datamine_dashboard) ("Made by @ohmi on Discord"). The repo was last pushed on 10 October 2026. The browser fetches these files from `https://osmsdataexplorer.com/data/current/`:

| File | Size | What MapleCompass would use |
| --- | --- | --- |
| `maps.json` | 349 KB | every map: ID, name, street, region, `is_town`, `return_map_id`, `exits`, `mob_rate`, `mob_positions` (one entry per spawn point), `map_mark` (the Mini Map's emblem for its area, `"None"` for none); and `map_mark_icons`, each mark's 38×38 PNG under `data/current/images/map_marks/` (11 in October 2026) |
| `portals.json` | 121 KB | every map's portals: `name`, `type`, `dest_map`, `x`, `y` |
| `monsters.json` | 90 KB | every mob's stats, elements, and its maps with spawn counts and `mob_time` |
| `lookups.json` | 223 KB | names by ID: 7,431 items, 318 maps, 101 mobs, 226 NPCs |
| `items.json` | 511 KB | items and scrolls with game IDs and categories |
| `map_manifest.json` | 15 KB | map ID to rendered map image (`data/maps/<hash>.webp`) |

Past datasets are kept and fixed under `data/patches/<version>/` (`cot1`, `cot2`, `v49`, ...), listed in `data/patches/index.json`. `patch_notes.json` lists what changed from COT2 to the Public Release.

## The scraping question

- **MeowDB:** its terms (§4) forbid "automated scripts, bots, or scrapers ... without permission" (ADR 0003). MapleCompass parses its HTML and Next.js flight data, page by page.
- **OSMS:** the site has no terms page, and the repo has no licence (`license: null` on GitHub). Its `robots.txt` is Cloudflare's content-signals preamble with no rules and no signals set, which by its own wording "neither grants nor restricts". Nothing forbids automated access, and Cloudflare served `maps.json` and `portals.json` as JSON to MapleCompass's own user agent. The files are the site's own data files, so reading them parses no HTML and uses no undocumented page structure. Fetching all of them is about as much load as one visit to the site.
- **What's still open:** having no licence means no one has granted reuse either. Fetching the files live for personal use is fine. Bundling a snapshot in a public release, or redistributing it, still needs permission, but from one person (@ohmi on the [Classic World Discord](https://discord.gg/AK9tZ2TvN8)), not a commercial site. MapleCompass would credit OSMS as it now credits MeowDB.
- **Nexon:** both sites' data is mined from Nexon's client, so this part doesn't change.

## Field by field

### Map index (ADR 0003)

`maps.json` has every map's ID, name, street and region, which is everything `MapEntry` needs. IDs are numbers (`10001010`), not MeowDB's 9-digit strings, so they need zero-padding.

### A map's data (`MapData`)

Compared on the 9 fixture maps OSMS has:

| `MapData` field | MeowDB | OSMS | Result |
| --- | --- | --- | --- |
| mobs and Spawn Counts | `monsters[].count` | `mob_positions` grouped by mob, or `monsters[].maps[].count` | identical on all 9 |
| mob name, level, EXP, HP | flight data + Monsters table | `monsters.json` | identical |
| `connected_maps` | `connectedMaps` | `exits` | identical, except that Kerning City also leads to Backalley Semi-Basement (`088001000`), new in the Public Release |
| `portals` (name, type, destination) | `portals` | `portals.json` | identical on all 9 after the core's existing filter (OSMS also lists portals to the same map, which the core drops) |
| portal types | 1 invisible, 10 hidden, 11 script | same numbers | identical, so the Hidden Portal rule and `PORTAL_OVERRIDES` carry over unchanged |
| `mob_rate_percent` | `mobRate` | `mob_rate`, absent when ×1 | identical once absent is read as ×1. The fixtures' differences (Sunset Sky ×0.5→×0.6, Northern Top ×2.0→×1.8) are Public Release changes, and live MeowDB has them too |
| `nearest_town` | `returnMap` | `return_map_id` | identical where OSMS gives one. OSMS leaves it out on 18 of its 92 Towns (Henesys, Kerning City, the Mushroom Towns), where MeowDB gives the Town itself, and on 4 Hidden Street event maps. The reader would take a missing one as the map itself on a Town, and as none elsewhere |
| `is_town` | `isTown` | `is_town`, absent when false | identical: the same as MeowDB's town flag on all 298 maps both have. It also covers shops and event rooms (92 maps), so Doors work as they do now |
| Hidden Portal destination name and street | `toMapName`, `toStreetName` | `lookups.json` names + `maps.json` streets | available |
| Hidden Portal position | portal x, y with `minimapWidth/Height`, `miniMapCenterX/Y` | portal x, y in the rendered map image's pixels; **no minimap bounds** | **gap**, see below |

### Map Summary

MeowDB computes the Map Summary itself. Its figures come out exactly from OSMS's spawn data with this rule, built on the same 7.56 s respawn that OSMS's own map table uses:

- A spawn point without a timer gives `3600 / 7.56` mobs an hour to a full party, and 0.75 of that alone.
- A timed spawn point gives `3600 / mob_time` an hour, alone or in a party.
- Each mob's figure is rounded, and the map's figure is the sum of its mobs'. The map's Mob Rate isn't applied, as MeowDB doesn't apply it.

All four figures (mobs and EXP an hour, alone and with a full party) matched to the unit on the 6 fixture maps with mobs, and on live MeowDB's The Pig Beach, whose Iron Hog spawns every 3600 s. Ranged timers weren't checked: OSMS gives some as a range with an average (`"180.00-1800.00 [720.00]"`), and MeowDB has a `timedRespawnAvg`, but whether the two averages agree is unknown.

### Mob Card (ADR 0006)

Compared on the 12 fixture mobs OSMS has:

| `MobDetails` field | OSMS | Result |
| --- | --- | --- |
| elements | `elements` (`{"Fire":"Weak"}`) | identical |
| ACC, AVOID, P.DEF, M.DEF | `acc`, `eva`, `PDDamage`, `MDDamage`; absent or 0 where MeowDB shows "—" | identical |
| MP | `mp` | available. MeowDB omits Mushmom's (200), while OSMS gives it |
| HP per EXP | `hp / exp` | identical to 2 decimals. The rank becomes "of 92" (mobs in the game now), not MeowDB's "of 182", which includes COT2-only mobs |
| Touch DMG | `PADamage` (one number) | MeowDB's range fits `floor(1.1 × P.ATK)` to `ceil(1.5 × P.ATK) − 1` on all 12 mobs, Blue Snail (5) to Mushmom (532). This is fitted, not documented: OSMS's formulas pages don't cover mob damage |
| Speed | `speed` | fits on all 12: below 0 is "Slow", absent is "Normal", above 0 is "Fast". Fitted |
| Movement | the animation names in `gifs` | "Flies" when there is a `fly` animation. **"Jumps between floors" vs "Stays on its floor" is a gap**: Slime and Orange Mushroom jump, but OSMS lists no `jump` animation for them |

### Mob images

Each mob in `monsters.json` names its animations (`move`, `stand`, `fly`, `attack1`, ...) as animated WebP files under `data/images/monsters/<hash>.webp`, plus a PNG thumbnail. They are served by Cloudflare with a 4-hour cache. They could replace maplestory.io, and as animations, which ADR 0006 passed on for lack of frame timings.

### Item Lookup (ADR 0007)

OSMS has every item's name with its game ID (7,431 in `lookups.json`), straight from the game's data. A reading could be matched offline against that list, so ADR 0007's problem of MeowDB's list missing 68 items goes away. "Hat MP Scroll: Greater", ADR 0007's example of a missing item, is in it, as are all 54 item names in the MeowDB search answers the tests then kept. But MeowDB's item pages are numbered by MeowDB's own IDs, and OSMS doesn't carry them. Opening the right MeowDB page still needs one request per press to MeowDB's search, as now. Otherwise Item Lookup could open OSMS's own listing, which needs no request to anyone. OSMS has no page per item. Its Items tab (consumables, etc., setup, scrolls) and Equipment tab take a search in the link, such as `#items?q=<name>` or `id:<game ID>`, which its own cross-links use. That wasn't tried for the Equipment tab.

## Coverage and freshness

- **Maps:** OSMS has 305 maps. Live MeowDB lists 428: those 305 (less OSMS's Dev map), plus 124 more: 92 in Ossyria (Orbis and El Nath), 31 in Forgotten Hollow, and the Yellow Mushroom House. OSMS's patch notes list all 124 as removed in the Public Release client. Launch coverage says Orbis and El Nath are held back from the opening build and Forgotten Hollow is closed for now, to come later ([Noisy Pixel](https://noisypixel.net/maplestory-classic-world-october-21-launch-founders-access/), [GosuGamers](https://www.gosugamers.net/entertainment/news/79063-maplestory-classic-world-announces-21-october-release-date)). So OSMS covers what a player can reach today. When those areas return, OSMS gets them once they're in the client it mines, and that lag is unknown.
- **Mobs:** OSMS has 92 mobs in `monsters.json`. The 9 more in `lookups.json` (Nependeath, Dark Nependeath, the 810000-series boss variants, and a "Monsters Near Your Level" placeholder) appear on no map.
- **Freshness:** both are current. OSMS's repo was pushed on 10 October, and live MeowDB already has the Public Release Mob Rates.

## Gaps and risks

1. **Hidden Portal positions.** OSMS's portal x and y are pixels in its rendered map image, which adds margins around the minimap bounds, and the margins vary by map (left 50–115 px, bottom 124–185 px). Dividing by the image's size, read from the first bytes of the WebP, put 31 of the 38 fixture portals in MeowDB's minimap cell. Every miss was "lower" read as "middle". Subtracting typical margins would close most of that, but it stays an approximation. The clean fix is to ask OSMS to add each map's minimap bounds to `maps.json`.
2. **Mob movement.** Losing "Jumps between floors" leaves the Mob Card without that line, unless OSMS adds the field (the game has it, as MeowDB shows).
3. **Undocumented JSON.** The format has changed before: `data.js` falls back for a missing `beauty.json` and a missing `map_manifest.json`. The reader would stay all or nothing (`FormatChanged`, never partial figures), as it is now. Reading a fixed `data/patches/<version>/` set would trade freshness for stability.
4. **Fitted rules.** Touch DMG and Speed match MeowDB on 12 mobs but aren't documented by either site.

## What a switch would change

As foreseen before the switch; ADR 0008 records what changed.

- **No world crawl.** One fetch of the five files, about 0.8 MB, gives the whole world's portal graph, mobs and Mob Rates, so ADR 0004's crawl of about 420 pages at 2-second intervals, its week-old refreshes and the "Mapping the world" progress all go. Search and Distance work from the first launch, and offline after the first fetch.
- **No Mob Card fetches.** Every mob's figures arrive with the rest, so ADR 0006's per-mob page fetches go.
- **The core's interface barely moves.** `MapData`, `Mob`, `Portal`, `MobDetails` and `PanelState` stay as they are. The `meowdb` module's HTML and flight readers become a JSON reader of OSMS's files. Clicking a mob's name can still open its MeowDB page, since MeowDB's mob IDs are the game's (`mob_url` is unchanged).
- **Tests.** New fixtures would be OSMS JSON files, and the MeowDB HTML fixtures would go. AGENTS.md's rule for `tests/fixtures/meowdb-monsters` would move to the new fixtures.

## Decisions

Recorded in ADR 0008:

1. **Hidden Portal positions:** neither. The panel shows OSMS's picture of the map as a Map Preview, with every portal marked and the Hidden Portals set apart. Hovering a mark shows where it leads, and hovering a row shows where the portal is. OSMS's portal positions are already in that picture's pixels, so no minimap bounds are needed.
2. **Mob movement:** the Mob Card drops the line.
3. **Item Lookup:** for now it keeps opening MeowDB pages through MeowDB's search (ADR 0007), the one MeowDB request left. The letter of MeowDB's terms arguably covers it, so it's a grey area to raise with MeowDB, but not a bar to release. Pointing the browser at a MeowDB search instead was checked and isn't possible: `/msclassic/search?q=` is a 404, and `/msclassic/item-db?q=` loads the item database without applying the query. OSMS may supply an item card on hover later.
4. **Asking @ohmi:** not needed, since nothing restricts fetching. MapleCompass fetches the files and doesn't bundle them.
5. **Tests:** no OSMS file is committed. The tests run on made-up worlds in OSMS's shape, title recognition on a committed list of the game's map names (IDs, streets, names and regions only), and `probe osms` reads the live set in full by hand. The MeowDB fixtures go.

## How this was checked

OSMS's site code (`main.js`, `lib/data.js`, `tabs/maps/MapBrowser.js`, `tabs/maps/MapPortalOverlay.js`, `tabs/formulas.js`) and its `data/current` and `data/patches/cot2` files were read. Node scripts compared the 12 MeowDB map fixtures and 14 mob fixtures with OSMS, field by field: 9 maps and 12 mobs overlap, and the rest are Ossyria content. Three live MeowDB requests were made, each identifying itself as MapleCompass: Sunset Sky (`010003070`, freshness), the map index (coverage), and The Pig Beach (`010000021`, timed spawns). OSMS's `robots.txt` was read, and its files were fetched once more with MapleCompass's user agent. Later, two MeowDB search URLs were tried, the second also in a browser.
