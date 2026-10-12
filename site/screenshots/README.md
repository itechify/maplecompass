# The site's screenshots

The pictures on the Minerva site, taken by hand in the game. The site's build fails while one the page shows is missing, so the page is never published without it.

## How to take them

- The game at **1920×1200**, windowed or borderless, with the minimap open.
- Minerva from the release the site describes, with the panel inside the game window.
- Capture as PNG, for example with Snipping Tool: its window mode for `game-window.png`, a rectangle for the others.
- **`game-window.png`** is the whole game window as Windows draws it, title bar included. **The others** are crops at full size, never scaled, so their text stays readable, each framing what the table says at about the size it gives. Cut the hero from `game-window.png`: it is that window's top-left corner at full size, and the hero's film (`site/reel.ts`) ends on that corner of the window, so the two must match pixel for pixel.
- Save each under its file name here, replacing the old one, and set the version it was taken on. The page takes each picture's size from the file, and the build makes its WebP copy (`webp/`, not committed), so nothing else needs to change; if what a picture shows changes, update its alt text and caption in `site/index.html` to match.

## The shots

| File | What it shows | Size | Taken on |
| --- | --- | --- | --- |
| `hero.png` | The minimap on a hunting map (Henesys Hunting Ground I) and the panel under it naming the same map beside its Map Mark, down through its Map and Mobs sections to the panel's foot, with a strip of the game beside it. Also the picture links to the site show (Discord, Reddit). | 600×960, the game window's top-left corner, its title bar included | 0.8.0 |
| `game-window.png` | The game on the same map, with the panel inside the game window under the minimap showing its Map and Mobs sections, every mob listed, and the Grind Tracker under it. Also the hero's film, which marks and reads the minimap's title and drops the read into the panel's title: a retake that moves either title, or changes the map, means setting where they are and what the title says at the top of `site/reel.ts`. | 1922×1232, the whole window | 0.8.0 |
| `map.png` | The Map section on a map with a Hidden Portal: the Map Preview with its rings, the pointer resting on the dashed ring with its card showing, and the Hidden Portal's row, each with its destination's Map Mark. | 332×247, the panel's Map section | 0.8.0 |
| `navigation.png` | The Map section with the next portal's magenta ring, and Navigation with a Hunting Target: the mob, the figures, and the route from the map you're on, each map with its Map Mark. | 317×475, the panel's Map and Navigation sections | 0.8.0 |
| `grind-tracker.png` | The Grind Tracker: Time to Level, the rates, a Best row and the EXP gauge at its foot. | 280×159, the Grind Tracker's window | 0.7.0 |
| `item-card.png` | An Item Card at the pointer over the inventory, on an item with a shop price and a market price you set (Garnet), with its window bar and the game's tooltip under it. | 488×386, the card and the tooltip | 0.8.0 |
| `item-lookup.png` | An item's tooltip in the inventory, and the Item Lookup alert above it saying the page it opened. | 611×742, the inventory, the tooltip and the alert | 0.7.0 |
| `setup.png` | Setup's step for marking the minimap title, on first run. | 522×592, the setup window | 0.8.0 |

## Retake

When a change alters something a shot shows, the PR that makes it lists the shot here, with what changed. At release, the smoke test retakes every shot listed (`docs/releasing.md`) and empties the list.

- `item-card.png`: the card's market section is now Market Lines: a "Market" heading with its + and MesoWatch buttons, then a row per line. Retake it with two or three lines, some with a price (for example "Your price" and "Lowest ask"), and update its alt text to say so. The card also heads with the item's picture now, beside its name: the alt text says so too. Its window bar shows the owl now, not the compass.
- `game-window.png` and `hero.png`: the app is Minerva now. The panel's window bar shows the owl and "Minerva" where it showed the compass and "MapleCompass". Retake the game window and cut the hero from it again, keeping the same map with the minimap's title and the panel's where they are, so `site/reel.ts` needs no change.
- `setup.png`: the window's title and heading say "Minerva Setup", and its bar shows the 32px owl where it showed the 28px compass.
