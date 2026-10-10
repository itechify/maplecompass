---
status: superseded by ADR-0008 (Mob Card figures and images come from OSMS Data Explorer's files)
---

# Mob Cards: listed mobs' MeowDB pages, and their images from maplestory.io

Hovering a mob's name in the panel shows its Mob Card: the mob's image and the figures its row doesn't show (element, Touch DMG, ACC, AVOID, defences, MP, how it moves, HP per EXP). Map pages don't have these, so MapleCompass now fetches mob pages too. When the shown map's page is in, MapleCompass fetches the pages of the mobs the panel lists (at most five), one at a time. They go ahead of the world crawl and share its pace of one MeowDB request every 2 seconds. It caches each page on disk and refetches it once its cached copy is a week old. A mob already fetched this run isn't fetched again for another map. This amends ADR 0004, which said search needs no monster pages. Search still doesn't; the Mob Cards do.

The images come from maplestory.io's MapleStory Classic World data (`/api/MCW/CBT2/mob/<id>/render/stand`), whose mob IDs are MeowDB's. MapleCompass fetches them the same way, one at a time on maplestory.io's own 2-second pace, and keeps them on disk for 30 days. The overlay loads them from those saved files through the app's own `mobimage` URL scheme, never from maplestory.io directly, so every request carries our honest user agent and goes through the cache.

## Considered options

- **Fetch a mob page when the player hovers.** It's the fewest requests, but the first hover on each mob would wait about a second, mid-hunt.
- **Add every mob page to the world crawl** (about 184 more). Every mob would have a card, even offline, but it's the most scraping, for mobs the player may never meet.
- **Read the stats from maplestory.io too.** Its data is structured, so it would survive a MeowDB redesign. But it has no elements, which are the card's most useful line, and it may lag the live game.
- **Animate the image** from maplestory.io's walking frames. Its stand render is a single still frame, and its frame timings aren't given.

## Consequences

- MeowDB's mob pages carry their figures only as rendered text, so they're read by their labels. The reader is all or nothing: a missing label means the card says MeowDB's page has changed, never a card with figures missing. Only MP may be absent (some bosses list none).
- The images are Nexon's assets served by a fan site. **The rest of ADR 0003 and ADR 0004 stands: this is personal use, and a public release needs MeowDB's written permission and the image source sorted out too.** The panel credits both: "NiaMeowDB (meowdb.com) · images: maplestory.io".
- If maplestory.io is down or lacks a mob, its card shows without an image; there is no fallback to MeowDB's own sprites.
