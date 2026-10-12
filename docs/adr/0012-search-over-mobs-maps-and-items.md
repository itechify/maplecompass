---
status: accepted
---

# Search over mobs, maps and items

Mob search found a mob and the maps it spawns on, for a Hunting Target. The player also wants to go to a map with no mob in mind, and to look up an item they don't have in hand. So the search view becomes **Search**: what the player types matches mobs, maps and items, in one list with a section for each. A mob still leads to its map list and a Hunting Target. A map becomes the **Destination**, which the Navigation section shows the way to. An item shows its Item Card (ADR 0010) beside the panel. Item Lookup and the card's hotkey are unchanged.

## The matches

- One list, its sections in a fixed order: Mobs, then Maps, then Items. A section with nothing matching isn't shown. Each match keeps its kind's look: a mob's OSMS animation, name and level; a map's Map Mark, Map Name and street; an item's picture, name and kind.
- Each section shows its best five and counts the rest ("+23 more, keep typing"). Without the limit a two-letter query would list hundreds of items, and ask OSMS for every one of their pictures.
- Best first: names that start with the query, then names with a word that starts with it, then names that hold it anywhere, each group by name. Mobs are ranked so too, where mob search listed every match by name.
- A map matches by its Map Name alone. Its street shows beside it, but matching streets too would list every map on Victoria Road. Event and Dev maps are left out, as mob search leaves them out of a mob's maps.
- Every item in OSMS's item files can be matched, Cash items included: their market prices mean something too. Items that look alike (the same name, kind and shop price, as the card has it) are one match. Items that share a name and don't look alike are listed each with its own picture and kind, so a pick goes straight to that item's card, without the card's own list to pick from.
- ↑ and ↓ move through all the matches as one list, from one section on to the next; Enter picks the highlighted match, and the highlight starts on the first. While OSMS's item files aren't in, a line says so in the Items section's place, so an item the query would match isn't taken for one that doesn't exist.
- Esc and the back arrow go back a step: from a mob's map list to the matches as typed, and from there to the normal view. Mob search went straight back to the normal view, losing what was typed.

## The Destination

A Destination is a map picked in Search, to go to. The Navigation section shows it as it shows a Hunting Target, without the mob or the map's figures for it: the map with its Map Mark and street, its Distance, the route there (which the Map Preview rings the next portal of), and once the player is there, that they have arrived, with the trip. It ends as a Hunting Target does: when the player leaves the map having arrived, or with its ×. There is one target or the other: picking one replaces the other. Picked on the map the player is on, it says they are there at once, where a Hunting Target picked there waits for them to leave and come back, since the player may be hunting where they stand. It has no 🔍: there is no map list to reopen, and Search is a click away. The route ends in the Route Magenta ring, not the mob's red dot.

## An item's card, beside the panel

An item picked in Search closes the search view, and its card opens beside the panel, the tops level: towards the middle of the game first, or else on the other side, wholly inside the game's client area and clear of the captured regions and our other windows (the Grind Tracker sits beside the panel too, so a card that would cover it goes on the other side, or below it). It follows the panel until the player drags it, and otherwise behaves as a card from the hotkey: one card at a time, closed by its × or the hotkey. There is no pointer to open it at: the player clicked the panel, or pressed Enter.

## Pictures

Item pictures and mob animations come from OSMS when a match first shows, as the Map Preview's picture and the Mob Cards' animations do (ADR 0008), and are kept on disk; while the search view is open, its matches' pictures are asked for before any other. Nothing is bundled with Minerva: ADR 0008 and ADR 0010 ship none of OSMS's data, and five matches a section keep the requests few.

## Considered options

- **Separate searches, or a filter by kind.** One list answers "where is it, whatever it is" without the player saying first what kind of thing they want.
- **Items in Search keep the card's own list of those sharing a name.** One match per name would show one picture for items that look different; listing each makes the picture right and the pick final.
- **The card at the pointer, or in the alert's spot.** The pointer is over the panel, or nowhere the card belongs when Enter picks; the alert's spot is for statuses.
- **The search view stays open after an item pick**, to look through several items. The player chose to close it, as every other pick does, for now.
- **Bundling item pictures with the app**, for searches with OSMS out of reach. It would ship OSMS's data, which ADR 0008 and ADR 0010 don't do without its permission, for a need the limit of five already meets.
- **A map matching by its street too.** It would list dozens of maps for "Victoria Road" or "Hidden Street".
- **The Distance of each matched map.** The Navigation section shows it once the map is picked; the list stays quick to scan.

## Consequences

- The search state (`SearchState`) holds the matches by kind (`mobs`, `maps`, `items`, each its shown five and how many more), why no items are matched (`itemsMissing`), and the `destination` beside the `hunting` target. `SearchView::Mobs` becomes `SearchView::Matches`.
- New inputs write themselves down for Bug Reports (ADR 0009): a click on a map (`SearchPickDestination`) or on items (`SearchPickItem`). `SearchPick` keeps its meaning, a mob in the matches or a map in its map list, so earlier reports still read. Esc from a mob's map list now goes back to the matches, so a report from before replays differently from there.
- The search hotkey and the panel's 🔍 are named "Search". The site's Hunting tab describes Search and the Destination, and its Item Card topic the card from Search.
