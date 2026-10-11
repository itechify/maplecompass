# The site's screenshots

The pictures on the MapleCompass site, taken by hand in the game. The site's build fails while one the page shows is missing, so the page is never published without it.

## How to take them

- The game at **1920×1200**, windowed or borderless, with the minimap open.
- MapleCompass from the release the site describes, with the panel inside the game window.
- Capture as PNG, for example with Snipping Tool: its window mode for `game-window.png`, a rectangle for the others.
- **`game-window.png`** is the whole game window as Windows draws it, title bar included. **The others** are crops at full size, never scaled, so their text stays readable, each framing what the table says at about the size it gives. Cut the hero from `game-window.png`: it is that window's top-left corner at full size, and the hero's film (`site/reel.ts`) ends on that corner of the window, so the two must match pixel for pixel.
- Save each under its file name here, replacing the old one, and set the version it was taken on. The page takes each picture's size from the file, and the build makes its WebP copy (`webp/`, not committed), so nothing else needs to change; if what a picture shows changes, update its alt text and caption in `site/index.html` to match.

## The shots

| File | What it shows | Size | Taken on |
| --- | --- | --- | --- |
| `hero.png` | The minimap on a hunting map (Henesys Hunting Ground I) and the panel under it naming the same map, down through its Map and Navigation sections, with a strip of the game beside it. Also the picture links to the site show (Discord, Reddit). | 600×872, the game window's top-left corner, its title bar included | 0.7.0 |
| `game-window.png` | The game on the same map, with the panel inside the game window under the minimap showing its Map, Navigation and Mobs sections. Also the hero's film, which marks and reads the minimap's title and drops the read into the panel's title: a retake that moves either title, or changes the map, means setting where they are and what the title says at the top of `site/reel.ts`. | 1922×1232, the whole window | 0.7.0 |
| `map.png` | The Map section on a map with a Hidden Portal: the Map Preview with its rings, the pointer resting on the dashed ring with its card showing, and the Hidden Portal's row. | 319×242, the panel's Map section | 0.7.0 |
| `navigation.png` | The Map section with the next portal's magenta ring, and Navigation with a Hunting Target: the mob, the figures, and the route from the map you're on. | 308×446, the panel's Map and Navigation sections | 0.7.0 |
| `grind-tracker.png` | The Grind Tracker: Time to Level, the rates, a Best row and the EXP gauge at its foot. | 280×159, the Grind Tracker's window | 0.7.0 |
| `item-lookup.png` | An item's tooltip in the inventory, and the Item Lookup alert above it saying the page it opened. | 611×742, the inventory, the tooltip and the alert | 0.7.0 |
| `setup.png` | Setup's step for marking the minimap title, on first run. | 522×592, the setup window | 0.7.0 |

## Retake

When a change alters something a shot shows, the PR that makes it lists the shot here, with what changed. At release, the smoke test retakes every shot listed (`docs/releasing.md`) and empties the list.

- `game-window.png`: the Mobs section lists every mob on the map, so Henesys Hunting Ground I shows all seven, with no "+2 more" line. The title shows the map's Map Mark (Henesys's shield) beside its two lines, and the route its stops' marks. The hero's film then lands its read on a title laid out beside that mark: set it to match at the top of `site/reel.ts`.
- `hero.png`: the title shows the map's Map Mark beside its two lines, and the route its stops' marks.
- `map.png`: the Hidden Portal's row has its destination's Map Mark between its ring and its name, and so does the mark card.
- `navigation.png`: the Hidden Portal's row, the Hunting Target's map and every stop on the route have their Map Marks.
- `item-card.png` (new, not taken yet): an Item Card at the pointer over the inventory, on an item with a shop price, a market price and a quest or recipe (Garnet, say), the game's tooltip beside or under it. Add it to the table above, and its figure to the Item Card topic in `site/index.html` (a `with-shot full-shot` topic, as Item Lookup's), once taken.
- `item-lookup.png`: Item Lookup's hotkey is Ctrl+Shift+O. The alert in the shot reads the same either way; retake only if it shows the hotkey.
- `setup.png`: the shot is setup opened again ("Step 3 of 6", with Keep the current title), and the Setup topic describes first run. Retake it on first run, at "Step 4 of 7", and set its alt text and caption back to first run.
