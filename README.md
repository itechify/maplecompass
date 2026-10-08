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

The panel starts on the right edge of the game window. You can click it at any time, and a click never takes the keyboard from the game: only the search view (below) does. Drag it by its header to move it. It is never taller than the room it has (the game window, or your screen when docked or detached); a long route or map list scrolls instead. It shows up in your screenshots and streams, and it never covers the minimap title, which MapleCompass reads: dragged onto it, it moves just clear of it when you let go. It sits just above the game rather than on top of everything, so an app you bring in front of the game covers it too, and it disappears while the game is minimised. Once it knows your map it shows the map's Map Summary (mobs and EXP per hour, solo and 6+, as MeowDB gives them), its mobs sorted by level, and its Hidden Portals, from NiaMeowDB (meowdb.com).

Hidden Portals are exits to other maps that MeowDB marks invisible (type 1), hidden (10) or script (11, shown as a Quest Portal). Each shows where it leads (tagged when that is a Hidden Street) and roughly where it is on the map. Invisible exits on a town are its building doors, which you can see, so they aren't listed. Walk-into portals (type 3) count as visible until checked in-game. When play shows a portal the rule gets wrong, add it to the override list in `crates/compass-core/src/portal_overrides.rs`, by map ID and MeowDB portal name.

Each of these also has an icon at the top of the panel: 🔍 search, ⇆ next map with the same title (only when maps share it), ↗ MeowDB page (while a map is shown), ↺ reset EXP tracking (while a Session runs), ⧉ placement, ? help & setup, and – collapse; click the collapsed icon to expand the panel again. Hover an icon to see its hotkey. The hotkeys work while the game or one of MapleCompass's windows is in front; in any other app the same keys do what that app makes them do:

- **Ctrl+Shift+C** collapses the panel to an icon, or expands it again.
- **Ctrl+Shift+P** switches placement: inside the game window, docked to its right edge, docked to its left edge, or detached.
- **Ctrl+Shift+M** opens the current map's MeowDB page in your browser.
- **Ctrl+Shift+S** opens setup again, to mark the minimap title anew or consult the Overlay guide.
- **Ctrl+Shift+N** shows the next map with the same title, when the panel can't tell which one you're on.
- **Ctrl+Shift+F** opens the search view, like the 🔍 at the top of the panel (expanding it if it is collapsed, or bringing it back in front if it is already open).
- **Ctrl+Shift+R** resets EXP tracking: the Session starts afresh at your next EXP gain, like the ↺ at the top of the panel.

The search view takes over the whole panel and takes your typing until it closes; then the game has focus again. Type part of a mob's name and pick the mob (arrow keys and Enter, or a click). The panel then lists every map it spawns on, ranked by Density Tier and then the nearest within a tier, each with its Density, Spawn Share and Distance, and the Taxis and Teleport Scrolls checkboxes. Pick a map the same way and it becomes your Hunting Target: the panel returns to the normal view with a Hunting section at the bottom, showing the mob and map, its figures, and the route there from the map you're on, worked out again as you move. When you arrive on that map the section goes away. Its 🔍 reopens the mob's map list to pick another map, and its × clears it. The back arrow or Esc leaves the search view without changing anything. The Hunting Target lasts until you close MapleCompass.

They are ordinary registered hotkeys (no keyboard hook), registered only while the game or one of MapleCompass's windows is in front, so other apps keep their own shortcuts, but the game never sees these combinations. If one clashes with a key you bind in the game, change it under `hotkeys` in the settings file (below). If another app already has one, the panel and the setup window say so. Win+Alt hotkeys saved by an earlier version are moved to these.

A few maps share a title, such as the two Maple Road / Mushroom Town maps. If they would show the same Map Summary, mobs and hidden portals anyway, the panel just shows them. Otherwise it picks the one connected to the map you came from, and failing that it shows the first and lists them all, each with its first mobs, mob levels and hidden portal count, with the map ID secondary; Ctrl+Shift+N then steps through them, and your pick holds until you change maps.

Many map names differ only in a Roman numeral, such as Rocky Road I, II and III. The recogniser can't read the game font's "II" and "III", so MapleCompass counts their strokes. When a numeral still can't be made out, the panel picks the map the one you came from leads to, if it leads to just one of them; otherwise it says it isn't sure rather than guess.

Inside, the panel keeps the spot you dragged it to relative to the game window. Docked, it follows the game window's outside edge, and goes inside when there is no room on screen. Detached, it is an ordinary window: drag it anywhere, such as onto another monitor, and it stays there.

Settings, including the placement, the marked title, a hand-picked game window and the hotkeys, are saved in `%APPDATA%\com.itechify.maplecompass\settings.json`.

## Credits

Title recognition uses [ocrs](https://github.com/robertknight/ocrs). Its recognition model is CC BY-SA 4.0; see `crates/compass-core/models/NOTICE.md`.

Created in [T3 Code](https://t3.codes).
