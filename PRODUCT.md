# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

One MapleStory Classic World player, the author, using it for themselves (see ADR 0003). They play windowed or borderless, with the minimap open, and look at the panel in short glances while hunting or travelling between maps. What they want from a glance: which map they are on, where its Hidden Portals are, and whether its mobs are worth grinding.

## Product Purpose

MapleCompass works out which map the player is on by reading the Minimap Title, then shows what is there: the Hidden Portals they would otherwise miss, the map's Map Summary (mobs and EXP per hour, solo and 6+), and its mobs. It works when the player finds hidden exits without looking them up and can judge a map's hunting at a glance.

## Positioning

It only watches the game from outside, by ordinary screen capture, and never touches the game's memory, files or input (ADR 0001). When it can't tell which map the player is on (titles that several maps share, or Roman numerals it can't read), it says so instead of guessing.

## Operating Context

- An overlay window that sits just above the game window: inside the game at its right edge by default, docked to its left or right edge, or detached. Clicks pass through it, except in edit mode or when it is detached.
- The game's own windows are on screen at the same time: the minimap at top-left, the Quest Helper at top-right, chat, the status bar. The panel never covers the Minimap Title, which it reads.
- Controlled only by global Win+Alt hotkeys: collapse/expand, edit mode, placement, open the map's MeowDB page, setup, next Candidate.
- The panel shows in the player's screenshots and streams.

## Capabilities and Constraints

- The fixed-width overlay window (380 px) grows to fit its content, up to 900 px tall. When collapsed it shrinks to a 44 px square.
- States: game not found, setup needed, map list unavailable, reading, a confirmed map, not sure (an unreadable numeral), Candidates (a title several maps share, chosen or not), MeowDB loading / unavailable / page format changed, Saved Copy (with its date), no mobs, no Hidden Portals, hotkeys taken by another app, edit mode, collapsed, hidden while the game is minimised.
- Data comes from NiaMeowDB, fetched live for personal use only (ADR 0003). MeowDB plans to rename itself NiaDB in late October 2026.
- All logic lives in `crates/compass-core`, which returns a `PanelState`. The web UI only renders it.
- Terminology is fixed in `GLOSSARY.md` (Map, Street Name, Hidden Portal, Hidden Street, Quest Portal, Candidate, Saved Copy, Map Summary, Spawn Share).

## Brand Commitments

- The overlay leans into MapleStory: it borrows the game's own window language so it reads as if it shipped with the game (chosen by the user in the October 2026 critique).
- Every screen that shows MeowDB data credits it as "NiaMeowDB (meowdb.com)" (ADR 0003).
- Copy uses the game's own terms and plain language, and never claims certainty it doesn't have.

## Evidence on Hand

- Real game screenshots in `samples/minimap/` (`--window` variants show the whole client, its minimap, Quest Helper and status bar).
- MeowDB page fixtures in `crates/compass-core/tests/fixtures/`.
- There are no other users, testimonials or usage figures, and none should be invented.

## Product Principles

- One glance per map change: the most useful thing comes first.
- Honest over clever: show doubt as doubt, and show stale data with its date.
- Stay out of the game's way: never cover what the player or the recogniser needs to see, and never interact with the client.
- Speak the game's language: its terms, and its look.

## Accessibility & Inclusion

The panel is read over busy, saturated game scenes, often on high-DPI screens or through stream compression. Text has to stay legible over any background, and states can't rely on colour alone.
