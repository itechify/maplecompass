# The site's screenshots

The pictures on the MapleCompass site, taken by hand in the game. The site's build fails while one the page shows is missing, so the page is never published without it.

## How to take them

- The game at **1920×1200**, windowed or borderless, with the minimap open.
- MapleCompass from the release the site describes, with the panel in its default placement, inside the game window.
- Capture the game window only (for example with Snipping Tool's window mode, or Win+Shift+S and a rectangle), as PNG.
- **The hero** is the whole game window. **The others** are crops of that capture at full size, never scaled, so their text stays readable: crop to the rectangle in the table, in pixels from the game window's top left corner (x, y, width, height). Where the panel sits elsewhere, keep the size and move the rectangle with it, and note the new one here.
- Save each under its file name here, replacing the old one, and set the version it was taken on.

## The shots

| File | What it shows | Crop (x, y, w, h) | Taken on |
| --- | --- | --- | --- |
| `hero.png` | The game on a hunting map, with the panel showing its Map, Navigation and Mobs sections. Also the picture links to the site show (Discord, Reddit). | Whole window | |
| `map.png` | The Map section on a map with Hidden Portals: the Map Preview with its rings, the Hidden Portal rows, the pointer resting on a ring with its card showing. | The panel's Map section | |
| `navigation.png` | Navigation with a Hunting Target: the mob, the figures, and the route from the map you're on, with the next portal's magenta ring on the Map Preview. | The panel's Map and Navigation sections | |
| `grind-tracker.png` | The Grind Tracker beside the panel, with Time to Level, the rates, a Best row and the EXP gauge at its foot. | The Grind Tracker's window | |
| `item-lookup.png` | An item's tooltip in the inventory, and the Item Lookup alert at the top of the game window saying the page it opened. | The tooltip and the alert | |
| `setup.png` | Setup's test read: the marked minimap title, recognised. | The setup window | |

## Retake

When a change alters something a shot shows, the PR that makes it lists the shot here, with what changed. At release, the smoke test retakes every shot listed (`docs/releasing.md`) and empties the list.

- `hero.png`, `map.png`, `navigation.png`, `grind-tracker.png`, `item-lookup.png`, `setup.png`: not taken yet.
