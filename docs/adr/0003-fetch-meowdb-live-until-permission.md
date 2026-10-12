---
status: superseded in part by ADR-0004 (map pages were crawled ahead of time), then by ADR-0008 for maps and mobs, which come from OSMS Data Explorer's files. ADR-0008 also lifts its ban on a public release. Saved Copies stay, and its honest user agent and credit still hold for Item Lookup's MeowDB search (ADR-0007)
---

# Fetch map data live from MeowDB; get permission before public release

MeowDB is the only source with everything Minerva needs (connected maps with portal types, per-mob spawn counts, and its own map summary figures), but it has no API and no data download. For now we fetch a map's MeowDB page when the player enters that map and read the figures from it. Fetched pages may be cached on disk so revisits don't refetch. We fetch only on a confirmed map change, never ahead of time, and identify ourselves with an honest user agent. We chose this over shipping a bundled snapshot, because the snapshot needs MeowDB's permission and we didn't want to wait for it.

**This is a personal-use arrangement, and Minerva must not be released publicly while it stands.** MeowDB's terms (§4) forbid "automated scripts, bots, or scrapers to access or collect data from the Site without permission", and live fetching very likely falls under that. Before any public release, we need written permission from MeowDB (contact@meowdb.com, see meowdb.com/reuse), or we revisit this decision. Every screen that shows MeowDB data credits it as "NiaMeowDB (meowdb.com)".

### Exception: maps that share a title

A few titles belong to more than one map (Maple Road / Mushroom Town, Hidden Street / Coconut Harvest). When the player is confirmed on such a title, and the map they came from doesn't settle which map it is, we fetch the pages of every map with that title, one at a time, not only the one we show. The product decision on identical titles (ticket #7) needs them: if all would show the same Map Summary, mobs and Hidden Portals the panel shows it without asking, and otherwise it lists the candidates with what sets each apart. This stays within the spirit of the rule above: it happens only on a confirmed title, never ahead of time, it covers at most a handful of pages, and a page cached less than a day ago is used without a request.

## Consequences

- The app needs a network connection, and it breaks when MeowDB changes its page structure or URLs. MeowDB plans to rename itself to NiaDB in late October 2026.
- When MeowDB can't be reached, a page or index cached earlier is still used, and the panel says it is a saved copy and from when. With nothing cached, the panel shows the map without MeowDB's data.
- We still need an index of map names and IDs, both to turn a recognised title into a page URL and to snap OCR output to a real name. That index also comes from MeowDB.
