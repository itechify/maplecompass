---
status: accepted
---

# The Item Card, from OSMS's item files and BasilMarket's price list

The Item Card shows, at the pointer, what the game's Item Tooltip doesn't say about the item the player hovers: what a shop pays for it, what players pay for it, where it comes from and what it's used for. It is for deciding what to do with loot: sell it to a shop, sell it to players, or keep it. The player presses a hotkey (Ctrl+Shift+I by default) while the game shows the Item Tooltip. The name is read as Item Lookup reads it (ADR 0007), and matched against OSMS Data Explorer's items on the player's machine, so nothing is asked at the press. Opening the item's MeowDB page, Item Lookup, moves to Ctrl+Shift+O and is otherwise unchanged; the card's name opens it too.

## Where the card's figures come from

Every source passes the test OSMS passed in ADR 0008: its terms don't forbid automated access, we read its own data files or JSON rather than its pages, we send our honest user agent, we credit it, and we ship none of its data. A source that fails the test needs its owner's written permission first; it is never scraped meanwhile.

- **OSMS's item files** (`items.json`, `quests.json`, `crafting.json` and `cash_shop.json` from `data/current/`, about 1.8 MB): every item's game ID, name and kind, what a shop pays for it, the quests that give and need it and the recipes that make and use it, and the Cash Shop's items. They come from the game's client, as OSMS's other files do.
- **BasilMarket's price list** (`https://basilmarket.net/api/price-index`, 539 KB): the list its own "What's it worth?" search downloads. For each item it lists, the price players paid, by the median of their recent sales, and how many sales that comes from. Players record the sales on BasilMarket.net: trades on its site, and sales seen in Free Market shops. Its terms say nothing on automated access or reuse, and it has no `robots.txt`.

Drops aren't on the card. OSMS has no drop tables (the game keeps them on its servers), and the one source that meets the test, MapleClassic Wiki, covered 26 of the game's 119 mobs when we looked (#78). Sleepywood, MSCW Guidebook and Henesys.GG fail the test, and MeowDB forbids scrapers.

## What it shows

- **Shop price:** what a shop pays for one. Otherwise that a shop won't buy it (OSMS's `notSale`, which carries a placeholder price of 1), that it is a Cash item, or that no shop price is known: OSMS gives none for scrolls, starter gear and arrows, and for throwing stars and bullets it gives what recharging them costs, which a sale in the game hasn't yet confirmed is what a shop pays.
- **Market:** the price from sales and how many, only when BasilMarket has sales (`sales` or `fm-sales`). With none, "No sales seen yet": BasilMarket shows the NPC shop's price in their place, which the card never shows as a market price. A click opens the item's BasilMarket page.
- **Comes from** and **Used for:** the quests that give it (always, at random with OSMS's chance, or as a pick) and the recipes that make it; the quests that need it (with how many) and the recipes that use it. Three of each, then "and N more".
- Not "Untradeable", which the game's tooltip says.

## Matching, by name and by ID

The name read is matched against every item's name as Item Lookup matches MeowDB's answer: the name itself but for the letters the recogniser confuses (I, l and 1; O and 0), case, spacing and punctuation, or one clearly closer than any other. BasilMarket lists each name once, under one item's ID (in its picture's name), so its prices are matched by the game's item ID, never by name: a price never lands on another item that shares the name.

Items that share a name, a kind and a shop price look alike in the game: the card shows one card for them with the quests and recipes of each, and BasilMarket's price for whichever it lists (the game's two Red Potions differ only in which quests give them). Items that share a name but not their kind or shop price (the Blue Moon top and earring) are listed with their pictures and kinds for the player to pick the one their tooltip shows.

## At the pointer, until closed

The card's corner sits a few pixels below and right of the pointer's tip, like the game's tooltip, so a click without moving still lands on the item; it may cover the game's tooltip. It goes left of the pointer or above it at the game window's edges, and moves clear of the captured regions. One card shows at a time, until its × or the hotkey again: over another item that item's card replaces it, elsewhere the press just closes it. It hides while another app is in front, like our other windows. What keeps a card from showing (no tooltip, an unreadable name, no such item, the item files not in) goes in the Item Lookup alert, as Item Lookup's own outcomes do.

Its window is one of ours (ADR 0001): no-activate, on top while the game or one of our windows is in front, and it takes the clicks on itself while a card shows. MapleCompass doesn't hear of a click elsewhere: the game gets it. Closing the card on one would need Raw Input or a hook watching the mouse system-wide, a see-through window over the game that would stop it seeing the mouse, or the card taking the keyboard from the game. Each costs the game something or changes ADR 0001, so the card closes only by its × or the hotkey. Our windows show in our captures, so the shell moves the card off the screen before capturing for the next press.

## Considered options

- **A different hotkey for the card, keeping Ctrl+Shift+I for MeowDB.** The card is the more frequent use, and its name still opens MeowDB, so the key players pressed over an item goes to the card. A settings file from before it gives its Item Lookup key to the card, and Item Lookup Ctrl+Shift+O.
- **The card in the alert's spot, at the top centre.** The card is something to read beside the item, not a status; the player looks at the pointer.
- **Closing when the pointer leaves the item and the card.** It would need the pointer polled while the card shows; the player preferred a card that stays until closed.
- **The item files in the map set.** One set is one fetch to schedule, but a change in the item files would hold back maps too, and an older install's saved map set wouldn't read after the upgrade.
- **BasilMarket's per-item answer** (`/api/market/sightings`), with the range and the last sale's date. It is a request per card, and it carries players' usernames and shop names.

## Consequences

- OSMS's item files are fetched on the map set's schedule (a saved set under a day old is used as it is, afresh every 24 hours, a minute after OSMS couldn't be reached with none saved), but read, saved and swapped on their own, all or nothing. A card from item files over 7 days old says it is a Saved Copy, with its date. `probe osms` reads them in full with the map set.
- BasilMarket's price list is fetched only when a card opens and the list held is over 3 hours old, so a player who never opens a card never asks BasilMarket. A saved list shows meanwhile; one over a day old that couldn't be refreshed says when it is from. After BasilMarket couldn't be reached, the next card opening asks again no sooner than a minute on.
- Item Lookup's MeowDB search now runs only when the player asks for the page: by Ctrl+Shift+O or the card's name. Among several MeowDB items with the name, the one of the card's kind opens, if just one is.
- The card credits "Data: OSMS Data Explorer · Prices: BasilMarket.net". Tests write made-up item files and price lists in their shapes (`tests/common/items.rs`); nothing from BasilMarket is committed.
- BasilMarket's endpoints are undocumented and could change. A list that doesn't read keeps the one held, and the market line says it is unavailable when there is none. BasilMarket's prices are players' records: few items have sales yet, and the card shows how many each price comes from.
