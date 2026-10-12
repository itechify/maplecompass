---
status: superseded by ADR-0008 (there is no crawl: OSMS Data Explorer's files hold the whole world)
---

# Crawl every MeowDB map page for personal use

Mob search tells the player which maps have a mob, how dense each one is and how far away it is. Distance means the fewest map changes from the player's map, so it needs the portal graph of the whole world. No single MeowDB page has it: each map's page lists only that map's own portals, mobs and Mob Rate. So Minerva now fetches all of MeowDB's map pages (about 420) in the background, about one every 2 seconds. It caches them on disk and refetches a page once its cached copy is a week old. This supersedes ADR 0003's rule that a map's page is fetched only on a confirmed map change and never ahead of time.

The crawled pages give the mob list, every map's mobs, spawn counts and Mob Rate, and every map's portals and nearest town. Search therefore needs no monster pages. Until the crawl is done, search works on the maps known so far and says how many those are.

## Considered options

- **Coarse nearness from the map index** (same street, then same region). Allowed under ADR 0003, but it can't tell a map 1 portal away from one 6 portals away on the same street.
- **A graph built only from maps the player has visited.** It never breaks the old rule, but it often can't give a Distance at all.
- **A bundled snapshot of the graph.** It's the right shape for a public release, but like the crawl it needs MeowDB's permission.

## Consequences

- **Minerva is now plainly a scraper, and must not be released publicly until MeowDB gives written permission** (contact@meowdb.com, see meowdb.com/reuse). The rest of ADR 0003 stands: an honest user agent, the "NiaMeowDB (meowdb.com)" credit, and saved copies when MeowDB can't be reached. With permission, a bundled snapshot should replace the crawl.
- What the crawl learns is kept between launches as extracted data (a few MB), each map dated by when MeowDB served its page. Re-reading every cached page at each launch made the app lag until it was done. A map whose data is over a week old still counts as known; it is refreshed in the background, oldest first and no more than one every 30 seconds, so the maps' dates spread out instead of all falling due the same day. Only maps never seen are fetched at the 2-second pace, and only they show the "Mapping the world" progress. The cached pages stay, for saved copies and for building the data again when the reader changes.
- Ships, taxis and Teleport Scrolls aren't portals, so MeowDB's pages don't show them. The links for ships, taxis and named-town Teleport Scrolls are kept by hand in the code. A nearest-town Teleport Scroll goes to the map's `returnMap` from its page.
