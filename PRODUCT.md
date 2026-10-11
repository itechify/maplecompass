# MapleCompass

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

The primary user is the average MapleStory Classic player, playing MapleStory Classic World. Product decisions should serve ordinary players rather than assume expert map knowledge or familiarity with MapleCompass internals.

MapleCompass is public: its repository and its releases (from 0.7.0) are on GitHub, and the MapleCompass site explains it to players before and after they install it. ADR 0008 lifted ADR 0003's ban on a public release, and the repo's history was cleaned of MeowDB's pages before it went public. `docs/releasing.md` says how a version is made and released.

## Product Purpose

MapleCompass identifies the player's current Map and shows its Map Preview, Hidden Portals, Map Summary, and mobs while they play. Success means the player can check useful information about where they are without interrupting play to look up the Map manually.

## Positioning

The companion follows the player's current Map by recognising the displayed Minimap Title through ordinary screen capture. It presents map context in its own overlay and automates no gameplay.

## Operating Context

- The app is Windows-first, with an HTML/CSS and vanilla TypeScript interface hosted in Tauri v2. `web` describes the UI implementation, not a standalone browser product. ADR 0002 preserves the possibility of macOS and Linux (Wine/Proton); those platforms are not confirmed shipping support.
- The game runs windowed or borderless, with the minimap open and both lines of the Minimap Title visible. Exclusive fullscreen is outside the supported workflow.
- First-run setup presents the account-risk warning, finds or lets the player pick the game window, explains game requirements, asks them to mark the Title Region, and verifies a recognised Map before finishing. A game-window size change requires marking again.
- During play, the panel can sit inside the game window, dock to either side, or remain detached, including on another monitor. It can collapse to an icon. Registered hotkeys control collapse, editing, placement, setup, opening the map's MeowDB page, choosing a Candidate, and Item Lookup (the MeowDB page of the item whose tooltip shows by the pointer); the defaults are documented on the MapleCompass site (`site/`), and the player rebinds any of them, or turns it off, in the panel's settings view (its gear, or Change hotkeys in setup's Overlay Guide) by pressing the new combination.
- The overlay panel is always clickable but never takes the keyboard on a click, so the game keeps focus; only the open search view and the open settings view take the keyboard. Title Region marking also receives interaction in MapleCompass's own window. The panel avoids covering the Title Region, never grows taller than the space where it is placed, and appears in screenshots and streams. Wherever it is placed, it and MapleCompass's other overlay windows (the Grind Tracker, the Item Lookup alert) show on top of everything, but only while the game window or one of MapleCompass's own windows is in front; with another app in front, or the game minimised or gone, they hide. Opening a MeowDB page in the browser therefore hides them until the player returns to the game. The setup and title-marking windows are outside this rule.
- Settings retain the marked Title Region, placement, selected game window, and hotkeys across sessions.

## Capabilities and Constraints

- The Minimap Title consists of Street Name and Map Name. Follow [GLOSSARY.md](GLOSSARY.md), especially its distinctions between a Hidden Portal, a Hidden Street, a Door, and a Quest Portal.
- Recognition uses one bundled `ocrs` recogniser, with preprocessing and matching against the map list. It must expose uncertainty rather than present an unresolved reading as certain.
- Maps can share a Minimap Title. The previous Map's connections can resolve the ambiguity. Equivalent displayed data requires no choice; otherwise the player sees Candidates and can cycle through them, with their choice retained until the Map changes. No Map Preview shows until it is settled which Map the player is on, since each has its own layout.
- OSMS Data Explorer's published files supply the map list, every Map's data and every mob's Mob Card ([ADR 0008](docs/adr/0008-map-and-mob-data-from-osms-data-explorer.md)); links open NiaMeowDB's pages. The panel shows the Portals section first: the Map Preview, with every portal marked and the Hidden Portals set apart, then the Hidden Portals' destinations with Hidden Street and Quest Portal tags where applicable. Then Map Summary figures for solo and 6+ party sizes, and mobs sorted by level with their statistics and spawn information (hovering a mob's name shows its Mob Card, with OSMS's animation of the mob; clicking it opens the mob's MeowDB page).
- Fetch OSMS's file set at launch when the saved one is a day old and every 24 hours while running, one file at a time, and swap it whole only once it reads. Fetch the shown Map's picture and its mobs' animations, and keep them on disk. When OSMS is unreachable, keep using the saved set, and say it is a Saved Copy, with its date, once it is over 7 days old. Without a saved set, explain that the map list couldn't be loaded, and retry.
- Preserve [ADR 0001](docs/adr/0001-observe-game-from-outside-only.md): ordinary screen capture and separate app windows only. No game memory or file access, injection, game input, hooks, cross-process window parenting, or `PrintWindow`. Hotkeys use operating-system registration.
- Preserve [ADR 0008](docs/adr/0008-map-and-mob-data-from-osms-data-explorer.md): fetch OSMS's files and ship none of them, and read no MeowDB page. Item Lookup's search is the one request to MeowDB, one per press at MeowDB's 2-second pace ([ADR 0007](docs/adr/0007-item-lookup-on-a-hotkey-through-meowdb-search.md)). Do not claim Nexon approval or guaranteed account safety.
- All product logic belongs in `crates/compass-core`, taking plain data and returning `PanelState` without OS, UI, network, or file dependencies. `src-tauri` adapts the platform; the web UI renders state.

## Brand Commitments

- Product name: MapleCompass. The app icon is the pixel-art gold compass in [assets/icon.png](assets/icon.png), with packaged variants in `src-tauri/icons`.
- The windows are set in NEXON's Maplestory typeface, bundled in `assets/fonts` with its provenance in `NOTICE.md`. Help & Setup's About & Credits and `THIRD-PARTY-NOTICES.md` credit it.
- Each screen credits what it shows. The panel's footer (as the tooltip of its info icon) and the Personal Bests window credit `Data: OSMS Data Explorer (osmsdataexplorer.com) · Pages: NiaMeowDB (meowdb.com)`; the search view, which opens no pages, credits `Data: OSMS Data Explorer (osmsdataexplorer.com)`; setup's test read says `Map names from OSMS Data Explorer`.
- Preserve the explicit first-run account-risk disclosure. The product is a third-party companion, not made or approved by Nexon.
- The windows lean into MapleStory: they borrow the game's own window language (the minimap and the Quest Helper), so they read as if they shipped with the game (chosen by the user in the October 2026 critique; see DESIGN.md).

## Evidence on Hand

- The MapleCompass site (`site/`, published at itechify.github.io/maplecompass from each release's tag) is the player manual: current behavior, setup, hotkeys, what MapleCompass fetches and keeps, and the account-risk warning word for word. A change a player notices updates it in the same PR. [README.md](README.md) holds the layout and the Windows build commands.
- [GLOSSARY.md](GLOSSARY.md) and [docs/adr](docs/adr/) hold the domain vocabulary and accepted technical decisions.
- `samples/minimap/` contains real title and game-window screenshots used for recognition and setup tests. These are evidence for recognition, not screenshots of the overlay design.
- `crates/compass-core/tests/` exercises public inputs and resulting panel state, on made-up worlds in OSMS's shape. Its `fixtures/` directory holds only the map names fixture (every Map's ID, Street Name, Map Name and Region), which recognition is tested against.
- [ADR 0002](docs/adr/0002-one-bundled-title-recogniser.md) records the recogniser selection and model attribution requirements. `crates/compass-core/models/NOTICE.md` contains the bundled model notice, and Help & Setup's About & Credits and `THIRD-PARTY-NOTICES.md` carry its attribution and licence, as that ADR requires of a public release.

## Product Principles

1. Help ordinary players understand their current Map with minimal interruption to play.
2. Observe the game from outside and leave gameplay under the player's control.
3. Make recognition uncertainty, ambiguous Maps, unavailable data, and Saved Copies clear.
4. Keep setup grounded in a successful test read, and preserve the player's placement and Candidate choices.
5. Credit where the data and the linked pages come from, and redistribute neither.
6. One glance per map change: the most useful thing comes first, starting with the Hidden Portals the game doesn't show.
7. Speak the game's language: its terms, and its look.

## Accessibility & Inclusion

The panel is read over busy, saturated game scenes, often on high-DPI screens or through stream compression. Text has to stay legible over any background, and states can't rely on colour alone.

## Open Decisions

- Support commitments beyond Windows remain undecided.
- Item Lookup's MeowDB search is a grey area under MeowDB's terms, to raise with MeowDB (ADR 0007).
