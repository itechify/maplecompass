<img src="assets/icon.svg" width="64" height="64" alt="">

# MapleCompass

A helper overlay for MapleStory Classic World. It reads the minimap title by ordinary screen capture and shows which map you are on. It never touches the game's memory, files or input ([ADR 0001](docs/adr/0001-observe-game-from-outside-only.md)).

Personal use only for now: see [ADR 0003](docs/adr/0003-fetch-meowdb-live-until-permission.md).

## Layout

- `crates/compass-core`: the headless core (no OS or UI dependencies). Title Region frames go in, a panel state comes out. It is the only test seam.
- `src-tauri`: the Tauri v2 shell. It finds the "MapleStory" window and moves the overlay with it (20 times a second), and on a thread of its own captures and reads the Title Region the player marked at setup about 3 times a second, feeding the readings to the core. It also fetches MeowDB's map index in the background at startup, which the core snaps readings to, and a map's page only once you are confirmed on that map (or on a title several maps share). Pages are cached for a day in `%LOCALAPPDATA%\com.itechify.maplecompass\cache`, so revisits don't refetch; when MeowDB can't be reached, an older cached copy is shown, marked with its date.
- `src`, `index.html`, `setup.html`, `region.html`: the web UI (vanilla TypeScript) for the overlay, the setup window and the title-marking window. It only renders state from the core.
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

On first run a setup window walks you through a ban-risk warning, finding the game window (or picking it by hand), the requirements, and marking the minimap title. Drag a box around its two lines, or use Arrow keys to move the box and Alt+Arrow to resize it; hold Shift for ten-pixel steps. Enter starts the test read and Esc returns to setup without changing the previous mark. These keys act only in MapleCompass's own marking window. A test read then shows the map it recognised; finish to start the overlay. The final step teaches the overlay shortcuts, and **Overlay guide** in setup lists the current bindings and explains placement and hunting figures. Closing setup on first run quits. MapleCompass asks you to set up again when the game window changes size.

The panel starts on the right edge of the game window, and clicks pass through it to the game, except on the × that clears a mob search. It shows up in your screenshots and streams, and it never covers the minimap title, which MapleCompass reads: dragged onto it, it moves just clear of it when you leave edit mode. It sits just above the game rather than on top of everything, so an app you bring in front of the game covers it too, and it disappears while the game is minimised. Once it knows your map it shows the map's Map Summary (mobs and EXP per hour, solo and 6+, as MeowDB gives them), its mobs sorted by level, and its Hidden Portals, from NiaMeowDB (meowdb.com).

Hidden Portals are exits to other maps that MeowDB marks invisible (type 1), hidden (10) or script (11, shown as a Quest Portal). Each shows where it leads (tagged when that is a Hidden Street) and roughly where it is on the map. Invisible exits on a town are its building doors, which you can see, so they aren't listed. Walk-into portals (type 3) count as visible until checked in-game. When play shows a portal the rule gets wrong, add it to the override list in `crates/compass-core/src/portal_overrides.rs`, by map ID and MeowDB portal name.

These hotkeys work while the game or one of MapleCompass's windows is in front; in any other app the same keys do what that app makes them do:

- **Ctrl+Shift+C** collapses the panel to an icon, or expands it again.
- **Ctrl+Shift+E** enters edit mode: drag the panel where you want it, then press Ctrl+Shift+E again to make it click-through.
- **Ctrl+Shift+P** switches placement: inside the game window, docked to its right edge, docked to its left edge, or detached.
- **Ctrl+Shift+M** opens the current map's MeowDB page in your browser.
- **Ctrl+Shift+S** opens setup again, to mark the minimap title anew or consult the Overlay guide.
- **Ctrl+Shift+N** shows the next map with the same title, when the panel can't tell which one you're on.
- **Ctrl+Shift+F** puts the panel in search mode (expanding it if it is collapsed, or bringing it back in front if you were already searching). The panel then takes your typing until you pick a mob (Enter) or press Esc, and the game has focus again after that. Type part of a mob's name, move the highlight with the arrow keys and press Enter. Search mode lists every map for that mob with the route to the highlighted one; after that, the panel's Mob search section keeps the top five maps as ranked (by Density Tier, then the nearest within a tier). Click the section's × at any time, or press Enter with nothing typed, to clear the search.

They are ordinary registered hotkeys (no keyboard hook), registered only while the game or one of MapleCompass's windows is in front, so other apps keep their own shortcuts, but the game never sees these combinations. If one clashes with a key you bind in the game, change it under `hotkeys` in the settings file (below). If another app already has one, the panel and the setup window say so. Win+Alt hotkeys saved by an earlier version are moved to these.

A few maps share a title, such as the two Maple Road / Mushroom Town maps. If they would show the same Map Summary, mobs and hidden portals anyway, the panel just shows them. Otherwise it picks the one connected to the map you came from, and failing that it shows the first and lists them all, each with its first mobs, mob levels and hidden portal count, with the map ID secondary; Ctrl+Shift+N then steps through them, and your pick holds until you change maps.

Many map names differ only in a Roman numeral, such as Rocky Road I, II and III. The recogniser can't read the game font's "II" and "III", so MapleCompass counts their strokes. When a numeral still can't be made out, the panel picks the map the one you came from leads to, if it leads to just one of them; otherwise it says it isn't sure rather than guess.

Inside, the panel keeps its spot relative to the game window. Docked, it follows the game window's outside edge, and goes inside when there is no room on screen. Detached, it is an ordinary window: drag it anywhere, such as onto another monitor, and it stays there.

Settings, including the placement, the marked title, a hand-picked game window and the hotkeys, are saved in `%APPDATA%\com.itechify.maplecompass\settings.json`.

## Credits

Title recognition uses [ocrs](https://github.com/robertknight/ocrs). Its recognition model is CC BY-SA 4.0; see `crates/compass-core/models/NOTICE.md`.

Created in [T3 Code](https://t3.codes).
