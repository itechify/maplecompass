---
target: Map Leaderboard
total_score: 25
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 2
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3code-2d470cb1\\site\\leaderboards\\map\\index.html"
target_fingerprint: "sha256:bc3f01086b8c069ebce8c5aaf1150f1fc3f94c8ccb7596c7cb2128123832696a"
target_path: "C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3code-2d470cb1\\site\\leaderboards\\map\\index.html"
timestamp: 2026-10-15T05-01-51Z
slug: site-leaderboards-map-index-html
---
Method: dual-agent (A: design review · B: detector + browser)

## Map Leaderboard — Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 2 | On a deep link the rank line scrolls out of view; the table stops at 50 without saying 60 exist (map.ts:72) |
| 2 | Match System / Real World | 3 | Game terms throughout; "Its name", "Set", and "characters"/"entries" for the same count wobble |
| 3 | User Control and Freedom | 3 | Band links keep `name`; Esc and the veil close the dialog; focus returns to Report |
| 4 | Consistency and Standards | 3 | Lists show the street only, the map page street and region; the "Home" link colour is outside DESIGN.md |
| 5 | Error Prevention | 2 | Search offers maps with no entries; Send report stays visible after the report is taken |
| 6 | Recognition Rather Than Recall | 2 | You must know the map's exact name; Enter opens the first result with nothing marking it |
| 7 | Flexibility and Efficiency | 2 | Deep links are the only accelerator: no job filter, no name lookup, no arrow keys in results |
| 8 | Aesthetic and Minimalist Design | 3 | Calm, but 50–51 underlined Report links per page; body copy at 105–112 characters a line |
| 9 | Error Recovery | 2 | A bad link has a way out; empty band and empty map pages are dead ends |
| 10 | Help and Documentation | 3 | The FAQ answers sit on the page itself |
| **Total** | | **25/40** | **Acceptable** |

## Design Specificity Verdict
The frame is authored and the board is generic. The game-window bar, raised tabs, Maplestory type, the panel-style title (street · region over the Map Name), "Lv 20–29", the game's job names and the orange-free empty states are this product's. The board itself, a search box, pill filters and a ranked table with Report links, could be any game's leaderboard. Nothing draws EXP the way the game or the Personal Bests window does.

Detector: CLI 3 findings. `cramped-padding` on `.lb-dialog` is a false positive: the padding is on `.lb-dialog-body`, and the open dialog didn't repeat it. `design-system-color` flags the site-wide "Home" link (advisory). Browser: real `line-length` on the body paragraphs (105–112 characters, footer 176). `nested-cards` (the window bar) and `ai-color-palette` (the minimap's game-blue fold button) are false positives from the site's game chrome.

## Priority Issues
- **[P1] Shown-or-hidden bugs.** `.button`'s display beats `[hidden]`, so "Send report" stays beside Close after a report is taken. Under 640px `.lb-table{display:block}` (site.css:2166) beats `hidden`, leaving a stray bordered strip under an empty band. `send.disabled = true` (map.ts:137) drops focus to `<body>` inside the modal. Fix: `[hidden]{display:none !important}` and move focus to Close. → /impeccable harden
- **[P1] The deep link's payoff is weak.** The linked row is a tint about 1.24:1 against its neighbours (site.css:1831), and the "#60 of 60" line is far above it. Fix: repeat the rank at the row ("10 more · Egbert is #60 of 60"), add context ("#1 gets 157,000 EXP/h; #50 gets 73,400"), and mark the row beyond colour (the You Yellow star, per the Legend Rule). → /impeccable clarify
- **[P2] Search leads to dead ends.** Results carry no entry counts (search.ts:179), so a typed name can lead to "No one has a best… yet." Fix: show counts or "No entries yet", list maps with entries first, and mark the result Enter opens. Needs `/api/maps` to return more than 50 maps (ADR 0015). → /impeccable clarify
- **[P2] The cut at 50 is silent.** Fix: a caption "Top 50 of 60 at Lv 20–29"; rename the "Set" column to "Date". → /impeccable clarify
- **[P2] Report noise, phone layout and line length.** 79 tab stops on a deep link, mostly Report; the date wraps to "12 / Oct" at 375px; body copy runs past 100 characters. Fix: `nowrap` dates, quieter Report (DESIGN.md change), cap prose at about 70ch. → /impeccable distill, then /impeccable adapt

## Persona Red Flags
- **Alex (power user):** no job filter, no name lookup on a map's page, no cross-map view at their level, arrow keys dead in search results.
- **Sam (accessibility):** own row marked by colour alone; the phone table turns `display:block`/`grid` (site.css:2166–2179) and may lose table semantics; "⋯" read as "horizontal ellipsis"; 51 Report tab stops; focus lost after Send.
- **Casey (mobile):** rank line scrolls away; Report opens straight into the textarea and pops the keyboard; dates wrap; stray strip under an empty band.
- **An ordinary Classic player:** must type the map's name as the game writes it; can't tell which maps have data; ranked last with no explanation; the empty map page doesn't say "upload yours from Minerva".

## Minor Observations
- The search placeholder "Henesys Hunting Ground I" reads as a typed value.
- With no map in the link, "Map Leaderboard" shows twice (map/index.html:178–179).
- Empty states leave about 100px of blank space above their one line.
- Every row, the player's own included, offers Report.

## Questions to Consider
- A player really asks "where do I grind at Lv 23?" Should the front door rank maps for a level rather than ask for a map's name?
- Should the linked row carry the You Yellow star instead of the generic selection tint?
- Is raw EXP/h across all jobs a comparison players will trust, or should rank within the job be the default?
