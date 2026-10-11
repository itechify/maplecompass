# The site's screenshots

The pictures on the MapleCompass site, taken by hand in the game. The site's build fails while one the page shows is missing, so the page is never published without it.

## How to take them

- The game at **1920×1200**, windowed or borderless, with the minimap open.
- MapleCompass from the release the site describes, with the panel inside the game window.
- Capture as PNG, for example with Snipping Tool: its window mode for the hero, a rectangle for the others.
- **The hero** is the whole game window as Windows draws it, title bar included. **The others** are crops at full size, never scaled, so their text stays readable, each framing what the table says at about the size it gives.
- Save each under its file name here, replacing the old one, and set the version it was taken on. The page takes each picture's size from the file, so nothing else needs to change; if what a picture shows changes, update its alt text and caption in `site/index.html` to match.

## The shots

| File | What it shows | Size | Taken on |
| --- | --- | --- | --- |
| `hero.png` | The game on a hunting map (Henesys Hunting Ground I), with the panel under the minimap showing its Map, Navigation and Mobs sections. Also the picture links to the site show (Discord, Reddit). | 1922×1232, the whole window | 0.7.0 |
| `map.png` | The Map section on a map with a Hidden Portal: the Map Preview with its rings, the pointer resting on the dashed ring with its card showing, and the Hidden Portal's row. | 319×242, the panel's Map section | 0.7.0 |
| `navigation.png` | The Map section with the next portal's magenta ring, and Navigation with a Hunting Target: the mob, the figures, and the route from the map you're on. | 308×446, the panel's Map and Navigation sections | 0.7.0 |
| `grind-tracker.png` | The Grind Tracker: Time to Level, the rates, a Best row and the EXP gauge at its foot. | 280×159, the Grind Tracker's window | 0.7.0 |
| `item-lookup.png` | An item's tooltip in the inventory, and the Item Lookup alert above it saying the page it opened. | 611×742, the inventory, the tooltip and the alert | 0.7.0 |
| `setup.png` | Setup's step for marking the minimap title. | 522×592, the setup window | 0.7.0 |

## Retake

When a change alters something a shot shows, the PR that makes it lists the shot here, with what changed. At release, the smoke test retakes every shot listed (`docs/releasing.md`) and empties the list.

Nothing to retake.
