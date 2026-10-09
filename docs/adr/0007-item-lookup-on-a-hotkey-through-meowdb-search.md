---
status: accepted
---

# Item Lookup on a hotkey through MeowDB's search

Item Lookup opens the MeowDB page of the item the player is hovering in the game. The player presses a hotkey (Ctrl+Shift+I by default) while the game shows the item's Item Tooltip. MapleCompass then reads the pointer position, captures the game's client area once, finds the Item Tooltip box nearest the pointer, and reads the name on the line with the blue bullet. It sends that name to MeowDB's search and opens the page of the item whose name matches.

## A hotkey, not Ctrl+Shift+click

The natural gesture would be Ctrl+Shift+click on the item. ADR 0001 rules it out: keyboard input comes only through `RegisterHotKey`, and a mouse click can't be registered that way. Seeing one would need a low-level mouse hook (`WH_MOUSE_LL`) or polling key state (`GetAsyncKeyState`), and ADR 0001 bans both. The click would also reach the game, and clicking an inventory item picks it up, which closes the tooltip before it can be read. A hotkey reaches only MapleCompass, so the tooltip stays where it is. Reading the pointer position (`GetCursorPos`) is window-manager state, like the window pick at setup.

## MeowDB's search, not an index of every item

MeowDB's item pages are numbered by MeowDB's own IDs (`/msclassic/item-db/418`), not by the game's item IDs, so a name has to be looked up. We send one request per lookup to the endpoint MeowDB's own search box uses (`/msclassic/api/search?q=<name>&locale=en`). It answers with up to six close items, each with its ID, name and category. We open one only if its name matches what was read, allowing for the letters the recogniser confuses (I, l and 1; O and 0), or if it's clearly closer than the runner-up. Otherwise the panel says no MeowDB item has that name. When several results share the name (193 names are shared by 406 items, mostly Cash Equip colour and gender variants), the panel lists them with their categories and the player picks one. Answers are cached on disk, so a repeat lookup doesn't ask again.

## Considered options

- **An index of `/msclassic/item-db/all`**, fetched and cached like the map index, with the closest name picked. It works offline after the first fetch. But that page leaves out 68 items that have pages ("Hat MP Scroll: Greater" among them), so a perfect reading of a missing item would land on a near neighbour with full confidence. It is also 2.6 MB to fetch.
- **Both:** the index first, then search for names it lacks. This is the most code, and it still opens a wrong neighbour whenever the index has a close one.
- **A fixed spot below the pointer** instead of finding the box. The game moves the tooltip near the window's edges (up, or beside the pointer), so a fixed spot reads empty space there.

## Consequences

- The search endpoint isn't documented. If its answer changes shape, Item Lookup says it is unavailable, as a map page in a changed format does. It never guesses.
- The browser opens after a network round trip on a cache miss, not at the instant of the key press.
- This is the first reader of text over a translucent background (ADR 0005 turned the Pickup Log down for that reason). The name is bold white on the tooltip's dark fill, which holds up where the Pickup Log's thin fading lines don't. `samples/item-tooltip` keeps every frame the reader must get right, and a frame it ever reads wrong becomes one.
- The rest of ADR 0003 stands: one request per player's press, an honest user agent, the credit to NiaMeowDB, and no public release without MeowDB's written permission.
