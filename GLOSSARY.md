# Minerva

A helper overlay for MapleStory Classic World that recognises which map the player is on and tells them what is there: the mobs they can hunt and the hidden portals they would otherwise miss. While they hunt, it also measures how fast they are levelling.

## Language

### Maps

**Map**:
A single in-game area the player occupies at one time; the unit Minerva identifies.
_Avoid_: zone, area, level, field

**Street Name**:
The broader location label a map belongs to (e.g. "Victoria Road"). Many maps share one street name.
_Avoid_: region, area

**Region**:
The island-level grouping a street belongs to (e.g. "Maple Island", "Victoria Island").
_Avoid_: continent, world

**Map Name**:
The specific name of a map (e.g. "Henesys Hunting Ground I"). Not guaranteed unique on its own; a map is identified by its street name and map name together.
_Avoid_: title, map title

**Minimap Title**:
The street name and map name as rendered at the top of the in-game minimap; the signal Minerva recognises the map from.

**Map Mark**:
The small emblem the in-game minimap draws beside its title, for the area a map belongs to (Henesys's shield, Kerning City's stone, Sleepywood's skull). Many maps share one; a few have none. Minerva shows it beside a map's name, so a glance says where in the world the map is.
_Avoid_: map icon, region icon

**Candidate**:
One of several maps that share a Minimap Title, offered to the player when the map they came from doesn't settle which one they are on.

**Saved Copy**:
Map and mob data cached earlier and shown, with its date, because its source can't be reached right now.

**Title Region**:
The rectangle the player marks around the minimap title, saved relative to the game window.
_Avoid_: minimap region, capture area

**Portal**:
A spot on a map that moves the player to another map (or another spot on the same map).

**Hidden Portal**:
A portal that is not shown on the minimap or visibly in the world, so the player must already know it is there. Its destination may be any map, not only a Hidden Street.
_Avoid_: hidden street (when meaning the portal), secret exit

**Hidden Street**:
A map whose street name is literally "Hidden Street". Not the same thing as a hidden portal: a Hidden Street can be reached through a visible portal, and a hidden portal can lead somewhere that is not a Hidden Street.

**Town**:
A map that is a settlement (shops, NPCs, no mobs) rather than a hunting field, as marked in the map data.
_Avoid_: city, village (except inside map names)

**Door**:
A portal from a town into a building interior, drawn as part of the scenery. The player can see it, so it is not a hidden portal even though the minimap doesn't show it.
_Avoid_: entrance, building portal

**Quest Portal**:
A hidden portal that only works while the player meets a quest condition.
_Avoid_: script portal

**Map Preview**:
A picture of the whole of the shown map, with every portal marked where it is, its hidden portals set apart, and every Spawn Point dotted.
_Avoid_: minimap (the game's own), map image

### Hunting

**Mob**:
A hostile monster that spawns on a map.
_Avoid_: monster, enemy, creature

**Mob Card**:
The card that appears when the player hovers a mob's name in the panel: its image and the figures the row doesn't show. Clicking the name opens the mob's MeowDB page.
_Avoid_: tooltip, popup, mob info

**Spawn Point**:
A spot on a map where one of its mobs appears, again and again once killed. The Map Preview dots each one in the minimap's red.
_Avoid_: spawn, spawner

**Spawn Count**:
How many of a given mob a map holds at once: its spawn points there.

**Spawn Share**:
A mob's spawn count as a fraction of all mob spawns on the map.

**Mob Rate**:
A map's multiplier on how fast its mobs respawn (e.g. 0.5x, 1.6x).

**Density**:
How many of a given mob a player can kill on a map: its spawn count times the map's mob rate. A map with more of the mob, or that refills faster, is denser for that mob.
_Avoid_: spawn rate, mob density (when meaning spawn share)

**Density Tier**:
High, Medium or Low: how a map's density for a mob compares with that mob's densest map (High from two-thirds of it, Medium from one-third).

**Distance**:
The fewest map changes it takes to get from the player's current map to another map. A map change is going through a portal (but not a Quest Portal) or a ship ride, and, if the player has turned them on, a taxi ride or a Teleport Scroll. Minerva knows only which map the player is on, never where on it they stand.
_Avoid_: position, proximity

**Search**:
The search view the 🔍 or the search hotkey (Ctrl+Shift+F by default) opens over the panel: the mobs, maps and items whose name holds what the player types, a section for each, with each kind's best five. A mob leads to the maps it spawns on, to pick a Hunting Target; a map becomes the Destination; an item shows its Item Card.
_Avoid_: mob search, item search

**Hunting Target**:
The mob and map the player picked in Search (the mob, then one of its maps) to go and hunt. Minerva shows the way there from the player's current map, then that they have arrived, until they leave that map or clear it.
_Avoid_: waypoint, goal

**Destination**:
A map the player picked in Search to go to, with no mob in mind. Minerva shows the way there as for a Hunting Target, in its place: there is one or the other, whichever the player picked last. Picked on the map the player is on, it says they are there at once.
_Avoid_: waypoint, goal, target (alone)

**Ship**:
An NPC-run crossing between regions (e.g. Ellinia, on Victoria Island, to Orbis, in Ossyria). The only way to reach Ossyria, so it always counts as a map change.

**Taxi**:
An NPC who carries the player between towns for mesos.
_Avoid_: cab

**Teleport Scroll**:
An item that moves the player from any map to a town: either the map's own nearest town, or a named town.
_Avoid_: return scroll, portal scroll

**Map Summary**:
The hunting figures for a map as a whole, such as the EXP-per-hour ceiling and mobs killed per hour by party size.

### Tracking

**Grind Tracker**:
The window beside the panel, turned on at setup by marking the EXP Bar Region, that measures a Session from the EXP bar: Time to Level, the Rolling Rate and Session rates, how the Rolling Rate compares with the Personal Best, the Pace Trace, deaths, and where the EXP bar will be at the Rolling Rate. Its window bar opens the Personal Bests window. The player closes it with its ×, and opens it again from the panel; closing it doesn't stop the Session.
_Avoid_: EXP tracking, EXP tracker

**Session**:
The stretch of play Minerva measures rates over, from the first EXP gain until the player resets it or switches character. A reset can be undone until the next EXP gain. Idle stretches pause it rather than count against it, and so does a stretch when the EXP bar can't be read; what the EXP did over one too long to be a gap in play isn't counted either. A misread that leads Minerva to learn the level's total wrong doesn't end it: the total is learned again.
_Avoid_: run, grind (the Grind Tracker measures a Session; "grind" alone names nothing)

**EXP Bar Region**:
The rectangle the player marks around the EXP figures on the in-game EXP bar (e.g. "EXP 266 [6.33%]"), saved relative to the game window.
_Avoid_: status bar region, EXP region

**Rolling Rate**:
A rate measured over the last few minutes of active play in a session, as opposed to the average over the whole session.
_Avoid_: current rate, live rate

**Pace Trace**:
The last 5 minutes of active play drawn kill by kill on the Grind Tracker, raced against the 5 minutes before them: how far ahead of that earlier pace the player is, or behind it. On a map with a Personal Best, once the player comes within reach of its pace, it races that too.
_Avoid_: graph, chart, sparkline

**Personal Best**:
The highest Rolling Rate, in EXP/h, the player has had over a full 5 minutes of active play on one map, shared by all their characters: the best in the map's Visit History. The Grind Tracker shows it and marks the Rolling Rate while it beats it.
_Avoid_: record, high score, PB, max

**Personal Bests window**:
Every map's Personal Best side by side, in a window over the game that the Grind Tracker opens and closes, to compare maps: each with its mob levels and how close the best came to the map's solo EXP/h ceiling, since the bests don't say which character or level set them. Each map opens to its Visit History, the Visit under way's best among them. It shows the bests as they stand, the Visit under way's included, and keeps up with them while it is open.
_Avoid_: leaderboard, records, best list, Personal Bests page

**Visit**:
The stretch from the panel confirming a map until it confirms another. Losing the title for a while doesn't end it; closing Minerva does.
_Avoid_: stay, trip

**Visit History**:
A map's best Visits that have ended, up to 10, highest first: the first is its Personal Best. The Visit under way's best joins it when the Visit ends. The player can remove any Visit in it that came from a bug.
_Avoid_: runs, attempts, records

**Time to Level**:
How long the player would take to reach the next level at their net EXP rate.
_Avoid_: ETA, time to next level

**Pickup Log**:
The fading lines in the bottom-right corner of the game screen that announce gained EXP, gained mesos and picked-up items.
_Avoid_: chat, drop log, kill feed

**Meso**:
The in-game currency; the game itself says "mesos".
_Avoid_: miso, gold, money

### Items

**Item**:
Something the player can hold in the inventory: equipment, potions, scrolls, chairs, ores, Cash items. Each has its own MeowDB page.
_Avoid_: drop, loot (when meaning the item itself)

**Item Tooltip**:
The box the game draws by the mouse pointer when the player hovers an item: the item's name on the line with the blue bullet (a crafted item's maker sits above it), then its picture and figures. It is the game's, not ours: the Item Card and the Mob Card are Minerva's own.

**Item Card**:
The card the hotkey (Ctrl+Shift+I by default) shows at the pointer for the item whose Item Tooltip the game shows there, or Search shows beside the panel for an item picked there: what a shop pays for it, and the market prices the player keeps for it on their Market Lines (typed into the card, with a button that opens MesoWatch to look it up). It stays until the player closes it or presses the hotkey again, where the player drags it by its bar, and its name opens the item's MeowDB page. When items that don't look alike share the name, it first lists them, with their pictures, for the player to pick.
_Avoid_: item tooltip (the game's), price check, popup

**Market Line**:
One of the market prices the player keeps for every item, under a name they give it ("Your price", "Typical", "Lowest ask"). Every Item Card lists every line, in the player's order, each with the item's price on it or "Not set"; the player adds, renames and removes lines on the card. A player starts with one, "Your price".
_Avoid_: column, field, price type

**Market Import**:
A file of prices a tool of the player's own leaves in the `market-imports` folder beside `market-prices.json`, for their Market Lines. Minerva takes each in within a couple of seconds, at launch if it was closed, onto the lines of the same IDs, the newest price winning, and deletes it. It adds a line it names, but removes nothing. Minerva fetches no market prices itself.
_Avoid_: sync, feed (the tool's business, not Minerva's)

**Item Lookup**:
The hotkey (Ctrl+Shift+U by default) that reads the Item Tooltip nearest the pointer and opens the item's MeowDB page. An alert at the top centre of the game window says what came of it, and when several items share the name, lists them for the player to pick. The same alert says what kept an Item Card from showing.
_Avoid_: item search (Search is the search view's)

### Reporting bugs

**Bug Report**:
The `.zip` Minerva saves when the player chooses Bug Report in the tray menu, to go with a bug's issue: everything it read and made of it since launch (up to a day), with crops of the regions it captured. Fed back to the core, it goes through the same states again (ADR 0009).
_Avoid_: log, diagnostics, dump, trace (to the player; "trace" is its record of inputs, in the code)
