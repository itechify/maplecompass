---
version: 1
slug: "personal-bests-html"
primary_target: "personal-bests.html"
related_targets: ["src/personal-bests.ts","src/personal-bests.css"]
---

# Personal Bests window

Scope: the Personal Bests window (personal-bests.html, src/personal-bests.ts, src/personal-bests.css), every map's Personal Best over the game. Mode: Operate. Read at a pause, to choose where to grind next and to check a map's history.

Job: compare maps at a glance, and see whether a map's best was the usual or one lucky Visit. Constraints: the panel's window opened wide, on its glass, 920px or the game's client width, up to 600px tall; the table in a near-opaque inset; the You Yellow star on the best of all maps.

## Lanes (overdrive, 12 Oct 2026)

Chosen by the user over re-ranking rows in motion and a whole-map card beside a resting row. The core already sends every map's Visits, highest first, the Visit under way's among them; nothing new is asked of it.

- **What.** A lane in each row, after Best EXP/h: the best drawn as the Grind Tracker draws the EXP bar, lime in its grey frame, from 0 to the best of all maps, which runs the whole lane. Read down the window, the lanes are a chart. A notch in a bar is another Visit; You Yellow's dot is the Visit under way's best, stepping with the EXP, never eased. An opened Visit History draws each Visit's own thinner bar on the same scale.
- **The scale.** Linear, shared by every map, so a low-level map's best reads as the sliver it is beside a high one's. The header is a ruler at round steps, three to five, its figures thinned where the lane is short.
- **Room.** Region joins the street line at every width (the narrow layout's fold, made the default), so the Region sort goes; Map takes 28% of the table, the lane 23%. Under 840px the lane goes under its figure; in the cards under 540px it runs the card's width.
- **The EXP Lime Rule** now lets the lime draw a best's EXP/h on its lane: EXP, and nothing else.
- **No new motion.** The lanes change at once, as the game changes them.
- **Checked** headless at 920 (1× and 2×), 800, 700, 600 and 520 px, on made-up bests: nine maps with an opened history, the Visit under way and an armed "Remove?", bests in the millions, and a single map.

## Simpler comparison chrome (15 Oct 2026)

Manage shares the Character picker's row. The map count, top-level sharing status and attribution footer are removed; upload status remains under Manage and problems remain visible. The sharing switch says "Share to the Map Leaderboards", and an opened Visit History links to "Map Leaderboard". The share header's ? explains the actual solo ceiling calculation, using the same hover/focus tooltip as the panel and Grind Tracker.
