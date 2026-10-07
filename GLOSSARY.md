# MapleCompass

A helper overlay for MapleStory Classic World that recognises which map the player is on and tells them what is there: the mobs they can hunt and the hidden portals they would otherwise miss.

## Language

### Maps

**Map**:
A single in-game area the player occupies at one time; the unit MapleCompass identifies.
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
The street name and map name as rendered at the top of the in-game minimap; the only signal MapleCompass reads from the game.

**Candidate**:
One of several maps that share a Minimap Title, offered to the player when the map they came from doesn't settle which one they are on.

**Saved Copy**:
MeowDB data cached earlier and shown, with its date, because MeowDB can't be reached right now.

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

### Hunting

**Mob**:
A hostile monster that spawns on a map.
_Avoid_: monster, enemy, creature

**Spawn Count**:
How many of a given mob a map holds at once.

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
The fewest map changes it takes to get from the player's current map to another map. A map change is going through a portal (but not a Quest Portal) or a ship ride, and, if the player has turned them on, a taxi ride or a Teleport Scroll. MapleCompass knows only which map the player is on, never where on it they stand.
_Avoid_: position, proximity

**Ship**:
An NPC-run crossing between continents (e.g. Ellinia to Orbis). The only way to reach Ossyria, so it always counts as a map change.

**Taxi**:
An NPC who carries the player between towns for mesos.
_Avoid_: cab

**Teleport Scroll**:
An item that moves the player from any map to a town: either the map's own nearest town, or a named town.
_Avoid_: return scroll, portal scroll

**Map Summary**:
The hunting figures for a map as a whole, such as the EXP-per-hour ceiling and mobs killed per hour by party size.
