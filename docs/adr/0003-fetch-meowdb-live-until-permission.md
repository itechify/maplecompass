---
status: accepted
---

# Fetch map data live from MeowDB; get permission before public release

MeowDB is the only source with everything MapleCompass needs (connected maps with portal types, per-mob spawn counts, and its own map summary figures), but it has no API and no data download. For now we fetch a map's MeowDB page when the player enters that map and read the figures from it. Fetched pages may be cached on disk so revisits don't refetch. We fetch only on a confirmed map change, never ahead of time, and identify ourselves with an honest user agent. We chose this over shipping a bundled snapshot, because the snapshot needs MeowDB's permission and we didn't want to wait for it.

**This is a personal-use arrangement, and MapleCompass must not be released publicly while it stands.** MeowDB's terms (§4) forbid "automated scripts, bots, or scrapers to access or collect data from the Site without permission", and live fetching very likely falls under that. Before any public release, we need written permission from MeowDB (contact@meowdb.com, see meowdb.com/reuse), or we revisit this decision. Every screen that shows MeowDB data credits it as "NiaMeowDB (meowdb.com)".

## Consequences

- The app needs a network connection, and it breaks when MeowDB changes its page structure or URLs. MeowDB plans to rename itself to NiaDB in late October 2026.
- We still need an index of map names and IDs, both to turn a recognised title into a page URL and to snap OCR output to a real name. That index also comes from MeowDB.
