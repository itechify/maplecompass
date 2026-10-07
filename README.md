<img src="assets/icon.svg" width="64" height="64" alt="">

# MapleCompass

A helper overlay for MapleStory Classic World. It reads the minimap title by ordinary screen capture and shows which map you are on. It never touches the game's memory, files or input ([ADR 0001](docs/adr/0001-observe-game-from-outside-only.md)).

Personal use only for now: see [ADR 0003](docs/adr/0003-fetch-meowdb-live-until-permission.md).

## Layout

- `crates/compass-core`: the headless core (no OS or UI dependencies). Title Region frames go in, a panel state comes out. It is the only test seam.
- `src-tauri`: the Tauri v2 shell. It finds the "MapleStory" window, captures the Title Region about 3 times a second, feeds the core and moves the overlay. It also fetches MeowDB's map index once at startup, which the core snaps readings to, and a map's page only once you are confirmed on that map. Pages are cached for a day in `%LOCALAPPDATA%\com.itechify.maplecompass\cache`, so revisits don't refetch.
- `src`, `index.html`: the overlay's web UI (vanilla TypeScript). It only renders the panel state.
- `samples/minimap`: test screenshots named `street-name__map-name[--variant].png`.

## Build, test, run (Windows)

Needs Rust (stable, MSVC), the Visual Studio C++ Build Tools, Node and pnpm.

```sh
cargo test                                      # core tests (the default workspace member)
pnpm install
pnpm tauri dev                                  # run the overlay beside the game
pnpm tauri build --debug --no-bundle            # build target/debug/maplecompass.exe
cargo run -p maplecompass --example probe       # no UI: capture the title and print what the core reads
```

Run the game windowed or borderless, with the minimap open.

The panel starts on the right edge of the game window, and clicks pass through it to the game. It sits just above the game rather than on top of everything, so an app you bring in front of the game covers it too, and it disappears while the game is minimised. Once it knows your map it shows the map's Map Summary (mobs and EXP per hour, solo and 6+, as MeowDB gives them), its mobs sorted by level, and its Hidden Portals, from NiaMeowDB (meowdb.com).

Hidden Portals are exits to other maps that MeowDB marks invisible (type 1), hidden (10) or script (11, shown as a Quest Portal). Each shows where it leads (tagged when that is a Hidden Street) and roughly where it is on the map. Invisible exits on a town are its building doors, which you can see, so they aren't listed. Walk-into portals (type 3) count as visible until checked in-game. When play shows a portal the rule gets wrong, add it to the override list in `crates/compass-core/src/portal_overrides.rs`, by map ID and MeowDB portal name.

These hotkeys work while the game has focus:

- **Ctrl+Shift+M** collapses the panel to an icon, or expands it again.
- **Ctrl+Shift+E** enters edit mode: drag the panel where you want it, then press Ctrl+Shift+E again to make it click-through.
- **Ctrl+Shift+D** switches placement: inside the game window, docked to its right edge, docked to its left edge, or detached.
- **Ctrl+Shift+O** opens the current map's MeowDB page in your browser.

Inside, the panel keeps its spot relative to the game window. Docked, it follows the game window's outside edge, and goes inside when there is no room on screen. Detached, it is an ordinary window: drag it anywhere, such as onto another monitor, and it stays there.

Settings, including the placement and the hotkeys, are saved in `%APPDATA%\com.itechify.maplecompass\settings.json`.

## Credits

Title recognition uses [ocrs](https://github.com/robertknight/ocrs). Its recognition model is CC BY-SA 4.0; see `crates/compass-core/models/NOTICE.md`.

Created in [T3 Code](https://t3.codes).
