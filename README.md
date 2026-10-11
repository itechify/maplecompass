<img src="assets/icon.png" width="64" height="64" alt="">

# MapleCompass

A helper overlay for MapleStory Classic World. It reads the minimap title by ordinary screen capture and shows which map you are on. It never touches the game's memory, files or input ([ADR 0001](docs/adr/0001-observe-game-from-outside-only.md)).

**[The MapleCompass site](https://itechify.github.io/maplecompass/)** explains what it does and how to use it: setup, the panel, hunting, the Grind Tracker, Item Lookup and the hotkeys. This README is the developer's side.

Its map and mob data come from the files [OSMS Data Explorer](https://osmsdataexplorer.com) publishes, and its links open [NiaMeowDB](https://meowdb.com)'s pages ([ADR 0008](docs/adr/0008-map-and-mob-data-from-osms-data-explorer.md)).

## Install

Download the latest Windows installer from [Releases](https://github.com/itechify/maplecompass/releases/latest). Each release says which file to pick. MapleCompass isn't made, endorsed or supported by NEXON, and using any third-party tool with MapleStory is at your own risk. Help & Setup's About & Credits credits the data, the recognition model and the typeface, and [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) lists everything bundled.

## Layout

- `crates/compass-core`: the headless core (no OS or UI dependencies). Title Region frames go in, a panel state comes out. It is the only test seam.
- `src-tauri`: the Tauri v2 shell. It finds the "MapleStory" window and moves the overlay with it (20 times a second), and on a thread of its own captures and reads the Title Region the player marked at setup about 3 times a second, feeding the readings to the core. It also fetches OSMS Data Explorer's five data files (under 1 MB, one at a time) at startup when its saved set is a day old, and again every 24 hours while it runs. From them the core knows every map's mobs, portals and Map Summary and every mob's Mob Card, snaps readings to the map list, and searches the whole world from the first launch. The set is saved in `%LOCALAPPDATA%\com.itechify.maplecompass\cache` and used offline after that; once OSMS hasn't been reached for over a week, the panel says it shows a saved copy, with its date. Every Map Mark (first), the shown map's picture and its mobs' animations are fetched from OSMS once, one at a time, and kept in the same folder until they go 30 days unused. MeowDB's caches from earlier versions are deleted at startup.
- `src`, `index.html`, `setup.html`, `region.html`: the web UI (vanilla TypeScript) for the overlay, the setup window and the title-marking window. It only renders state from the core.
- `site`: the MapleCompass site, the player manual on GitHub Pages: one hand-written page built with Vite from `site/vite.config.ts`, sharing the app's tokens (`src/theme.css`) and hotkey names (`src/hotkeys.ts`). It is published from a release's tag when the release is published (`.github/workflows/site.yml`), and a pull request that touches it is built by `.github/workflows/site-check.yml`. Its screenshots are taken by hand: `site/screenshots/README.md`.
- `samples/minimap`: test screenshots named `street-name__map-name[--variant].png`.

## Build, test, run (Windows)

Needs Rust (stable, MSVC), the Visual Studio C++ Build Tools, Node and pnpm.

```sh
cargo test                                      # core tests (the default workspace member)
cargo test -p maplecompass --lib                # the app's own unit tests
pnpm install
pnpm tauri dev                                  # run the overlay beside the game
pnpm tauri build --debug --no-bundle            # build target/debug/maplecompass.exe
cargo run -p maplecompass --example probe       # no UI: capture the title and print what the core reads
cargo run -p maplecompass --example probe osms  # no game: fetch OSMS's live data and read it in full
cargo run -p maplecompass --example probe replay <report.zip>  # no game: a Bug Report's timeline, replayed through the core
pnpm site:dev                                   # the site, at http://localhost:5173/maplecompass/
pnpm site:build                                 # the site, into site/dist (fails while a screenshot is missing)
```

The tests run on made-up worlds in OSMS's shape and a list of the game's map names; `probe osms` checks the live data, by hand when OSMS changes and before a release.

Run the game windowed or borderless, with the minimap open.

When play shows a portal the Map Preview's rule gets wrong, add it to the override list in `crates/compass-core/src/portal_overrides.rs`, by map ID and the portal's name in OSMS's `portals.json`.

## Credits

Map and mob data, map pictures, Map Marks and mob animations come from [OSMS Data Explorer](https://osmsdataexplorer.com), fetched as it runs and never bundled. Map, mob and item links open [NiaMeowDB](https://meowdb.com)'s pages, and Item Lookup uses its search ([ADR 0007](docs/adr/0007-item-lookup-on-a-hotkey-through-meowdb-search.md)). The Map Summary is worked out as NiaMeowDB works it out.

The windows are set in NEXON's Maplestory typeface; see `assets/fonts/NOTICE.md`.

Title recognition uses [ocrs](https://github.com/robertknight/ocrs). Its recognition model is CC BY-SA 4.0; see `crates/compass-core/models/NOTICE.md`.

Created in [T3 Code](https://t3.codes).
