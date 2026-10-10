# MapleCompass

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

The primary user is the average MapleStory Classic player, playing MapleStory Classic World. Product decisions should serve ordinary players rather than assume expert map knowledge or familiarity with MapleCompass internals.

The current scope is personal use only. A broader audience does not imply permission to distribute the app publicly.

## Product Purpose

MapleCompass identifies the player's current Map and shows its Map Summary, mobs, and Hidden Portals while they play. Success means the player can check useful information about where they are without interrupting play to look up the Map manually.

## Positioning

The companion follows the player's current Map by recognising the displayed Minimap Title through ordinary screen capture. It presents map context in its own overlay and automates no gameplay.

## Operating Context

- The app is Windows-first, with an HTML/CSS and vanilla TypeScript interface hosted in Tauri v2. `web` describes the UI implementation, not a standalone browser product. ADR 0002 preserves the possibility of macOS and Linux (Wine/Proton); those platforms are not confirmed shipping support.
- The game runs windowed or borderless, with the minimap open and both lines of the Minimap Title visible. Exclusive fullscreen is outside the supported workflow.
- First-run setup presents the account-risk warning, finds or lets the player pick the game window, explains game requirements, asks them to mark the Title Region, and verifies a recognised Map before finishing. A game-window size change requires marking again.
- During play, the panel can sit inside the game window, dock to either side, or remain detached, including on another monitor. It can collapse to an icon. Registered hotkeys control collapse, editing, placement, setup, opening the source page, choosing a Candidate, and Item Lookup (the MeowDB page of the item whose tooltip shows by the pointer); the defaults are documented in [README.md](README.md), and the player rebinds any of them, or turns it off, in the panel's settings view (its gear, or Change hotkeys in setup's Overlay Guide) by pressing the new combination.
- The overlay panel is always clickable but never takes the keyboard on a click, so the game keeps focus; only the open search view and the open settings view take the keyboard. Title Region marking also receives interaction in MapleCompass's own window. The panel avoids covering the Title Region, never grows taller than the space where it is placed, and appears in screenshots and streams. Wherever it is placed, it and MapleCompass's other overlay windows (the Grind Tracker, the Item Lookup alert) show on top of everything, but only while the game window or one of MapleCompass's own windows is in front; with another app in front, or the game minimised or gone, they hide. Opening a MeowDB page in the browser therefore hides them until the player returns to the game. The setup and title-marking windows are outside this rule.
- Settings retain the marked Title Region, placement, selected game window, and hotkeys across sessions.

## Capabilities and Constraints

- The Minimap Title consists of Street Name and Map Name. Follow [GLOSSARY.md](GLOSSARY.md), especially its distinctions between a Hidden Portal, a Hidden Street, a Door, and a Quest Portal.
- Recognition uses one bundled `ocrs` recogniser, with preprocessing and matching against a known map index. It must expose uncertainty rather than present an unresolved reading as certain.
- Maps can share a Minimap Title. The previous Map's connections can resolve the ambiguity. Equivalent displayed data requires no choice; otherwise the player sees Candidates and can cycle through them, with their choice retained until the Map changes.
- MeowDB supplies the map index and map data. The panel shows Map Summary figures for solo and 6+ party sizes, mobs sorted by level with their statistics and spawn information (hovering a mob's name shows its Mob Card, with its image from maplestory.io; clicking it opens the mob's MeowDB page), and Hidden Portal destinations with approximate positions and Quest Portal labels where applicable.
- Fetch map pages only for confirmed map changes or the documented shared-title exception, plus the paced world crawl (ADR 0004) and the listed mobs' pages and images for their Mob Cards (ADR 0006). Cache pages for revisits; when the source is unreachable, show an available Saved Copy with its date. Without cached map data, retain the recognised Map and explain that its data is unavailable.
- Preserve [ADR 0001](docs/adr/0001-observe-game-from-outside-only.md): ordinary screen capture and separate app windows only. No game memory or file access, injection, game input, hooks, cross-process window parenting, or `PrintWindow`. Hotkeys use operating-system registration.
- Preserve [ADR 0003](docs/adr/0003-fetch-meowdb-live-until-permission.md): personal use only under the current fetching arrangement. Public release requires written MeowDB permission or revisiting that decision. Do not claim Nexon approval or guaranteed account safety.
- All product logic belongs in `crates/compass-core`, taking plain data and returning `PanelState` without OS, UI, network, or file dependencies. `src-tauri` adapts the platform; the web UI renders state.

## Brand Commitments

- Product name: MapleCompass. The app icon is the pixel-art gold compass in [assets/icon.png](assets/icon.png), with packaged variants in `src-tauri/icons`.
- The windows are set in NEXON's Maplestory typeface, bundled in `assets/fonts` with its provenance in `NOTICE.md`. Its terms for a third-party app need confirming before any public release.
- Every screen displaying MeowDB data credits it as `NiaMeowDB (meowdb.com)`; the panel adds `images: maplestory.io` while its Mob Cards show maplestory.io's mob images.
- Preserve the explicit first-run account-risk disclosure. The product is a third-party companion, not made or approved by Nexon.
- The windows lean into MapleStory: they borrow the game's own window language (the minimap and the Quest Helper), so they read as if they shipped with the game (chosen by the user in the October 2026 critique; see DESIGN.md).

## Evidence on Hand

- [README.md](README.md) documents current behavior, setup, hotkeys, and Windows build commands.
- [GLOSSARY.md](GLOSSARY.md) and [docs/adr](docs/adr/) hold the domain vocabulary and accepted technical decisions.
- `samples/minimap/` contains real title and game-window screenshots used for recognition and setup tests. These are evidence for recognition, not screenshots of the overlay design.
- `crates/compass-core/tests/` exercises public inputs and resulting panel state. Its `fixtures/` directory contains map-index and map-page data for parsing and behavior checks.
- [ADR 0002](docs/adr/0002-one-bundled-title-recogniser.md) records the recogniser selection and model attribution requirements. `crates/compass-core/models/NOTICE.md` contains the bundled model notice; public release also requires the attribution and licence work identified by that ADR.

## Product Principles

1. Help ordinary players understand their current Map with minimal interruption to play.
2. Observe the game from outside and leave gameplay under the player's control.
3. Make recognition uncertainty, ambiguous Maps, unavailable data, and Saved Copies clear.
4. Keep setup grounded in a successful test read, and preserve the player's placement and Candidate choices.
5. Credit source data and honor the existing limits on public distribution.
6. One glance per map change: the most useful thing comes first, starting with the Hidden Portals the game doesn't show.
7. Speak the game's language: its terms, and its look.

## Accessibility & Inclusion

The panel is read over busy, saturated game scenes, often on high-DPI screens or through stream compression. Text has to stay legible over any background, and states can't rely on colour alone.

## Open Decisions

- Release timing and support commitments beyond Windows remain undecided. Public distribution remains constrained by the accepted ADRs.
