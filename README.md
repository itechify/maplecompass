<img src="assets/icon.png" width="64" height="64" alt="">

# MapleCompass

A helper overlay for MapleStory Classic World. It reads the minimap title by ordinary screen capture and shows which map you are on. It never touches the game's memory, files or input ([ADR 0001](docs/adr/0001-observe-game-from-outside-only.md)).

Personal use only for now: see [ADR 0003](docs/adr/0003-fetch-meowdb-live-until-permission.md).

## Layout

- `crates/compass-core`: the headless core (no OS or UI dependencies). Title Region frames go in, a panel state comes out. It is the only test seam.
- `src-tauri`: the Tauri v2 shell. It finds the "MapleStory" window and moves the overlay with it (20 times a second), and on a thread of its own captures and reads the Title Region the player marked at setup about 3 times a second, feeding the readings to the core. It also fetches MeowDB's map index in the background at startup, which the core snaps readings to, and a map's page only once you are confirmed on that map (or on a title several maps share). Pages are cached for a day in `%LOCALAPPDATA%\com.itechify.maplecompass\cache`, so revisits don't refetch; when MeowDB can't be reached, an older cached copy is shown, marked with its date. Once a map's page is in, it fetches the pages of the mobs the panel lists (kept a week) and their pictures from maplestory.io (kept 30 days), one at a time and ahead of the world crawl (ADR 0006).
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

While it runs, the gold compass sits in the notification area (the tray). Click it for **Help & setup** or **Quit MapleCompass**: the one way to quit, whether or not the game is open. Only one MapleCompass runs at a time: starting it again shows the one that is running (its panel, expanded, or setup when there is no game window to show the panel on).

The panel starts on the right edge of the game window, its top a third of the way down, and grows downward as its content changes. You can click it at any time, and a click never takes the keyboard from the game: only the search view (below) does. Drag it by its header to move it, or by a side or a bottom corner to make it wider or narrower (never under 320 px); its height follows what it shows. It is never taller than the room it has (the game window, or your screen when docked or detached); a long route or map list scrolls instead. It shows up in your screenshots and streams, and it never covers the minimap title, which MapleCompass reads: dragged onto it, it moves just clear of it when you let go. It sits just above the game rather than on top of everything, so an app you bring in front of the game covers it too, and it disappears while the game is minimised. Once it knows your map it shows, from NiaMeowDB (meowdb.com), the map's Hidden Portals first, then its Map Summary (EXP and mobs per hour, solo and 6+, as MeowDB gives them) and its five most common mobs, sorted by level, with a line summing up the rest. Rest the pointer on a mob's name to see its Mob Card: its picture (from maplestory.io) and the figures its row doesn't show, such as its element, touch damage, accuracy, avoid and defences. Click the name to open the mob's MeowDB page. It is built like the game's own windows over the screen: a light grey bar with its name and blue buttons, the map's title on the Mini Map's sky blue, and the rest see-through like the Quest Helper. Collapsed, its icon shows the map's Hidden Portal count, with an orange dot when something needs a look.

Hidden Portals are exits to other maps that MeowDB marks invisible (type 1), hidden (10) or script (11, shown as a Quest Portal). Each shows where it leads (tagged when that is a Hidden Street) and roughly where it is on the map, in words and drawn on a tiny minimap. Invisible exits on a town are its building doors, which you can see, so they aren't listed. Walk-into portals (type 3) count as visible until checked in-game. When play shows a portal the rule gets wrong, add it to the override list in `crates/compass-core/src/portal_overrides.rs`, by map ID and MeowDB portal name.

Each of these also has a blue button in the bar at the top of the panel: a magnifier for search, two arrows for the next map with the same title (only when maps share it), an arrow out of a box for the MeowDB page (while a map is shown), a circular arrow to reset EXP tracking (while a Session runs), two squares for placement, a question mark for help & setup, and a dash to collapse; click the collapsed icon to expand the panel again. Hover an icon to see its hotkey. The hotkeys work while the game or one of MapleCompass's windows is in front; in any other app the same keys do what that app makes them do:

- **Ctrl+Shift+C** collapses the panel to an icon, or expands it again.
- **Ctrl+Shift+P** switches placement: inside the game window, docked to its right edge, docked to its left edge, or detached.
- **Ctrl+Shift+M** opens the current map's MeowDB page in your browser.
- **Ctrl+Shift+S** opens setup again, to mark the minimap title anew or consult the Overlay guide.
- **Ctrl+Shift+N** shows the next map with the same title, when the panel can't tell which one you're on.
- **Ctrl+Shift+F** opens the search view, like the magnifier button at the top of the panel (expanding it if it is collapsed, or bringing it back in front if it is already open).
- **Ctrl+Shift+R** resets EXP tracking: the Session starts afresh at your next EXP gain, like the reset-arrow button at the top of the panel.

The search view takes over the whole panel and takes your typing until it closes; then the game has focus again. Type part of a mob's name and pick the mob (arrow keys and Enter, or a click). The panel then lists every map it spawns on, ranked by Density Tier and then the nearest within a tier, each with its Density, Spawn Share and Distance, and the Taxis and Teleport Scrolls checkboxes. Pick a map the same way and it becomes your Hunting Target: the panel returns to the normal view with a Hunting section at the bottom, showing the mob and map, its figures, and the route there from the map you're on, worked out again as you move. When you arrive on that map the route gives way to "You're here", with how far you came, until you leave the map. Its magnifier reopens the mob's map list to pick another map, and its × clears it. The back arrow or Esc leaves the search view without changing anything. The Hunting Target lasts until you close MapleCompass.

They are ordinary registered hotkeys (no keyboard hook), registered only while the game or one of MapleCompass's windows is in front, so other apps keep their own shortcuts, but the game never sees these combinations. If one clashes with a key you bind in the game, change it under `hotkeys` in the settings file (below). If another app already has one, or one isn't a key combination, the panel and the setup window say which. Win+Alt hotkeys saved by an earlier version are moved to these.

A few maps share a title, such as the two Maple Road / Mushroom Town maps. If they would show the same Map Summary, mobs and hidden portals anyway, the panel just shows them. Otherwise it picks the one connected to the map you came from, and failing that it shows the first and lists them all, each with its first mobs, mob levels and hidden portal count, with the map ID secondary; Ctrl+Shift+N then steps through them, and your pick holds until you change maps.

Many map names differ only in a Roman numeral, such as Rocky Road I, II and III. The recogniser can't read the game font's "II" and "III", so MapleCompass counts their strokes. When a numeral still can't be made out, the panel picks the map the one you came from leads to, if it leads to just one of them; otherwise it says it isn't sure rather than guess.

Inside, the panel keeps the spot you dragged it to relative to the game window. Docked, it follows the game window's outside edge, and goes inside when there is no room on screen. Detached, you can drag it anywhere, such as onto another monitor, and it stays there; it also gets a taskbar entry. Wherever it is placed, it stays just above the game window and never above an app in front of the game, and it disappears while the game is minimised.

Settings, including the placement, the marked title, a hand-picked game window and the hotkeys, are saved in `%APPDATA%\com.itechify.maplecompass\settings.json`.

## Credits

Title recognition uses [ocrs](https://github.com/robertknight/ocrs). Its recognition model is CC BY-SA 4.0; see `crates/compass-core/models/NOTICE.md`.

Created in [T3 Code](https://t3.codes).
