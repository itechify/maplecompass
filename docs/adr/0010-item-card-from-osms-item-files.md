---
status: accepted
---

# The Item Card, from OSMS's item files, with links to MeowDB and MesoWatch

The Item Card shows, at the pointer, what the game's Item Tooltip doesn't say about the item the player hovers: what a shop pays for it, where it comes from and what it's used for. It is for deciding what to do with loot: sell it to a shop, sell it to players, or keep it. The player presses a hotkey (Ctrl+Shift+I by default) while the game shows the Item Tooltip. The name is read as Item Lookup reads it (ADR 0007), and matched against OSMS Data Explorer's items on the player's machine, so nothing is asked at the press. Opening the item's MeowDB page, Item Lookup, moves to Ctrl+Shift+O and is otherwise unchanged; the card's name opens it too.

## Where the card's figures come from

Every source passes the test OSMS passed in ADR 0008: its terms don't forbid automated access, we read its own data files or JSON rather than its pages, we send our honest user agent, we credit it, and we ship none of its data. A source that fails the test needs its owner's written permission first; it is never scraped meanwhile, and its blocks are never got around.

- **OSMS's item files** (`items.json`, `quests.json`, `crafting.json` and `cash_shop.json` from `data/current/`, about 1.8 MB): every item's game ID, name and kind, what a shop pays for it, the quests that give and need it and the recipes that make and use it, and the Cash Shop's items. They come from the game's client, as OSMS's other files do.
- **Market prices: none on the card.** BasilMarket.net's price list met the test when the card was designed, and the card showed its median sale price for a day. On 11 October 2026 BasilMarket began refusing automated access: its `robots.txt` disallows `/api/`, its server answers other clients "Automated access to BasilMarket isn't allowed", and it reserves its data against mining. MesoWatch, MeowDB's Free Market page and classicfreemarket.com forbid automated access too. MesoWatch's terms welcome links, so the card's Market row opens MesoWatch in the player's browser. It has no page per item and no search in its address, so the player looks the item up there.
- **Drops aren't on the card.** OSMS has no drop tables (the game keeps them on its servers), and the one source that meets the test, MapleClassic Wiki, covered 26 of the game's 119 mobs when we looked (#78). Sleepywood, MSCW Guidebook and Henesys.GG fail the test, and MeowDB forbids scrapers.

## What it shows

- **Shop price:** what a shop pays for one. Otherwise that a shop won't buy it (OSMS's `notSale`, which carries a placeholder price of 1), that it is a Cash item, or that no shop price is known: OSMS gives none for scrolls, starter gear and arrows, and for throwing stars and bullets it gives what recharging them costs, which a sale in the game hasn't yet confirmed is what a shop pays.
- **Market:** a link to MesoWatch.
- **Comes from** and **Used for:** the quests that give it (always, at random with OSMS's chance, or as a pick) and the recipes that make it; the quests that need it (with how many) and the recipes that use it. Three of each, then "and N more".
- Not "Untradeable", which the game's tooltip says.

## Matching, by name

The name read is matched against every item's name as Item Lookup matches MeowDB's answer: the name itself but for the letters the recogniser confuses (I, l and 1; O and 0), case, spacing and punctuation, or one clearly closer than any other.

Items that share a name, a kind and a shop price look alike in the game: the card shows one card for them with the quests and recipes of each (the game's two Red Potions differ only in which quests give them). Items that share a name but not their kind or shop price (the Blue Moon top and earring) are listed with their pictures and kinds for the player to pick the one their tooltip shows.

## At the pointer, until closed

The card is one of our windows, like the Personal Bests window: the panel's window bar, which the player drags it by, with its ×, and a body that scrolls under the bar when the card is taller than the space. It opens with its corner a few pixels below and right of the pointer's tip, like the game's tooltip, so a click without moving still lands on the item; it may cover the game's tooltip. It goes left of the pointer or above it at the game window's edges, and moves clear of the captured regions. Dragged, it stays where the player leaves it (on a monitor, clear of the captured regions, and with the game window as it moves) until it closes; the next card opens at the pointer again. One card shows at a time, until its × or the hotkey again: over another item that item's card replaces it, elsewhere the press just closes it. It hides while another app is in front. What keeps a card from showing (no tooltip, an unreadable name, no such item, the item files not in) goes in the Item Lookup alert, as Item Lookup's own outcomes do.

Its window (ADR 0001) is no-activate, on top while the game or one of our windows is in front, and it takes the clicks on itself while a card shows. MapleCompass doesn't hear of a click elsewhere: the game gets it. Closing the card on one would need Raw Input or a hook watching the mouse system-wide, a see-through window over the game that would stop it seeing the mouse, or the card taking the keyboard from the game. Each costs the game something or changes ADR 0001, so the card closes only by its × or the hotkey. Our windows show in our captures, so the shell moves the card off the screen before capturing for the next press.

## Considered options

- **A different hotkey for the card, keeping Ctrl+Shift+I for MeowDB.** The card is the more frequent use, and its name still opens MeowDB, so the key players pressed over an item goes to the card. A settings file from before it gives its Item Lookup key to the card, and Item Lookup Ctrl+Shift+O.
- **The card in the alert's spot, at the top centre.** The card is something to read beside the item, not a status; the player looks at the pointer.
- **Closing when the pointer leaves the item and the card.** It would need the pointer polled while the card shows; the player preferred a card that stays until closed.
- **The item files in the map set.** One set is one fetch to schedule, but a change in the item files would hold back maps too, and an older install's saved map set wouldn't read after the upgrade.
- **BasilMarket's price list with a browser's user agent.** It would get around a block BasilMarket put up on purpose.
- **Asking MesoWatch for a link to an item, or for its prices.** The player chose not to.

## Consequences

- OSMS's item files are fetched on the map set's schedule (a saved set under a day old is used as it is, afresh every 24 hours, a minute after OSMS couldn't be reached with none saved), but read, saved and swapped on their own, all or nothing. A card from item files over 7 days old says it is a Saved Copy, with its date. `probe osms` reads them in full with the map set.
- Item Lookup's MeowDB search now runs only when the player asks for the page: by Ctrl+Shift+O or the card's name. Among several MeowDB items with the name, the one of the card's kind opens, if just one is.
- A Bug Report (ADR 0009) writes the card's inputs down, the drags among them, and only the fingerprint of the item files, as of OSMS's set: a replay is given them (`probe replay --items`, or fetched live when the run had them).
- The card credits "Data: OSMS Data Explorer · Pages: NiaMeowDB, MesoWatch". Tests write made-up item files in OSMS's shape (`tests/common/items.rs`).
- Market prices on the card wait for a source that meets the test, or a site's written permission.
