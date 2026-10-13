# The site's screenshots

The pictures on the Minerva site, taken by hand in the game. The site's build fails while one the page shows is missing, so the page is never published without it.

## How to take them

- The game at **1920×1200**, windowed or borderless, with the minimap open.
- Minerva from the release the site describes, with the panel inside the game window.
- Capture as PNG, for example with Snipping Tool: its window mode for `game-window.png`, a rectangle for the others.
- **`game-window.png`** is the whole game window as Windows draws it, title bar included. **The others** are crops at full size, never scaled, so their text stays readable, each framing what the table says at about the size it gives. Cut the hero from `game-window.png`: it is that window's top-left corner at full size, and the hero's film (`site/reel.ts`) ends on that corner of the window, so the two must match pixel for pixel.
- Save each under its file name here, replacing the old one, and set the version it was taken on. The page takes each picture's size from the file, and the build makes its WebP copy (`webp/`, not committed), so nothing else needs to change; if what a picture shows changes, update its alt text and caption in the page that shows it (`site/index.html`, or `site/faq/index.html`) to match.

## The shots

| File | What it shows | Size | Taken on |
| --- | --- | --- | --- |
| `hero.png` | The minimap on a hunting map (Henesys Hunting Ground I) and the panel under it naming the same map beside its Map Mark, down through its Map and Mobs sections to the panel's foot, with a strip of the game beside it. Also the picture links to the site show (Discord, Reddit). | 600×960, the game window's top-left corner, its title bar included | 0.9.0 |
| `game-window.png` | The game on the same map, with the panel inside the game window under the minimap showing its Map and Mobs sections, every mob listed. Also the hero's film, which marks and reads the minimap's title and drops the read into the panel's title: a retake that moves either title, or changes the map, means setting where they are and what the title says at the top of `site/reel.ts`. | 1922×1232, the whole window | 0.9.0 |
| `map.png` | The Map section on a map with a Hidden Portal: the Map Preview with its rings, the pointer resting on the dashed ring with its card showing, and the Hidden Portal's row, each with its destination's Map Mark. | 332×247, the panel's Map section | 0.8.0 |
| `mobs.png` | The panel's Mobs section on the hero's hunting map (Henesys Hunting Ground I): the Map Summary's EXP/h and Mobs/h, solo and 6+ party, and its seven mobs. | 335×232, the panel's Mobs section, cut from `hero.png` | 0.9.0 |
| `navigation.png` | The Map section with the next portal's magenta ring, and Navigation with a Hunting Target: the mob, the figures, and the route from the map you're on, each map with its Map Mark. | 317×475, the panel's Map and Navigation sections | 0.8.0 |
| `grind-tracker.png` | The Grind Tracker: Time to Level, the rates, a Best row and the EXP gauge at its foot. | 280×159, the Grind Tracker's window | 0.7.0 |
| `item-card.png` | An Item Card at the pointer over the inventory, on an item with two Market Lines (Claw Attack Scroll: Intermediate): its window bar, the item's picture beside its name, Shop pays, and Your price and Lowest ask under Market, with the game's tooltip under it. | 659×546, the inventory, the card and the tooltip | 0.9.0 |
| `item-lookup.png` | An item's tooltip in the inventory, and the Item Lookup alert above it saying the page it opened. | 611×742, the inventory, the tooltip and the alert | 0.7.0 |
| `setup.png` | Setup's step for marking the minimap title, reopened from Help & Setup (step 3 of 6), with Keep the current title. | 522×592, the setup window | 0.9.0 |

## Retake

When a change alters something a shot shows, the PR that makes it lists the shot here, with what changed. At release, the smoke test retakes every shot listed (`docs/releasing.md`) and empties the list.

- `item-card.png`: the card's credit line at its foot is now an info icon, its words in its tooltip.
- `mobs.png`: cut from `hero.png` for now; take it on its own, with the pointer on a mob's name and its Mob Card showing. Its row is lit then too, since the Mobs feature now says pointing at a mob lights its spawn points.
- `map.png`: the ring's card now opens on a close-up of the portal's spot, and shows from a Hidden Portal's row too, and the Map Preview dots every spawn point. Retake it with the pointer resting on the dashed ring once the close-up has settled, and update its alt text in `site/index.html`.
- `hero.png`, `game-window.png` and `navigation.png`: the Map Preview now dots every spawn point in red. Retaking `game-window.png` or `hero.png` means checking the titles' places at the top of `site/reel.ts`, as their row above says.
