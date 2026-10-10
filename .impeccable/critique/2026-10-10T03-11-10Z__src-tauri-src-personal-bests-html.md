---
target: personal bests
total_score: 25
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 1
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\maplecompass\\t3code-90a3eff2\\src-tauri\\src\\personal-bests.html"
target_fingerprint: "sha256:8db055f4406e40985c1f8711ed8816fdcc705dbfa1d0611a390209de36018aaf"
target_path: "C:\\Users\\itech\\.t3\\worktrees\\maplecompass\\t3code-90a3eff2\\src-tauri\\src\\personal-bests.html"
timestamp: 2026-10-10T03-11-10Z
slug: src-tauri-src-personal-bests-html
---
Method: dual-agent (A: design review sub-agent · B: detector + browser sub-agent)

# Critique: Personal Bests page (src-tauri/src/personal-bests.html)

Reviewed on a realistic sample page (9 maps, including the one you're on, one with no share, one the map list doesn't have, a 2025 date, a name with angle brackets) at 1280px and 420px wide.

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | "As of" line makes the snapshot clear; empty cells say nothing about why |
| 2 | Match System / Real World | 3 | Game terms right; "% of solo EXP/h" only explained by a hover tooltip |
| 3 | User Control and Freedom | 2 | Can't hide or remove a bugged best here; must travel to the map and use the Grind Tracker's × |
| 4 | Consistency and Standards | 3 | Figures match the Grind Tracker; map links lack the Edit Blue focus ring; full-width bar differs from setup's window |
| 5 | Error Prevention | 3 | Read-only, but the share sort puts a Lv 1–3 map first and invites the wrong conclusion |
| 6 | Recognition Rather Than Recall | 2 | Must keep your character's level in mind and know what "solo EXP/h" means |
| 7 | Flexibility and Efficiency | 2 | Sorting only; no level filter; sort resets every open |
| 8 | Aesthetic and Minimalist Design | 3 | Calm and on-brand; Region repeats; gap after the Map column |
| 9 | Error Recovery | 2 | Unknown map shows as a bold "999999999" with five empty cells and no explanation |
| 10 | Help and Documentation | 2 | One tooltip; nothing says how a best gets set or why cells are empty |
| **Total** | | **25/40** | **Acceptable** |

## Design Specificity Verdict
Design review: the colours, font and parts come from the design system (window bar, opaque lavender, Maplestory, inset box, selection colour, yellow "You're here" dot, Grind Tracker figures). The layout around them is generic: a full-width header over a centred 980px table, which reads as a web page rather than a game window (setup reads as a window because bar and body share one width). Swap the font and drop the yellow dot and it's any app's data table. The yellow star already means "best" in the Grind Tracker, but nothing marks your top map, and the loudest row is "where you are", not "your best". Verdict: right colours and parts, generic layout.

Automated scan: the template file is clean. The static scan of the filled-in page flags one cramped-padding warning on the table's box, a false positive (10px inset at the sides, 5–6px top and bottom, the dense row spacing DESIGN.md specifies). The injected in-page detector reported "No anti-patterns found" at both widths, so there are no overlays. The detector missed the narrow-window overflow, which the browser measurements caught.

## Overall Impression
Legible, honest about being a snapshot, careful with its numbers, but it ranks every best on one list while bests don't record which character or level set them, so for any one character most of the list is noise, and the "my best map" moment never lands.

## What's Working
1. An honest header line: snapshot and all-characters in one line; the time is left out rather than faked when unknown.
2. Numbers that match the Grind Tracker (three significant figures, the share rule, "Lv 2–14", year only for other years), right-aligned; each column first sorts the way it reads best; empty cells always go last.
3. Careful about where you are, and hard to break: the row you're on is marked only when the panel is certain; names go in as plain text and the data is escaped; the file needs nothing else to load; every text colour passes contrast (lowest is the footer at 5.48:1).

## Priority Issues
- [P1] The page doesn't help you pick a map for the character you're on. Bests don't record level, so the default list mixes Lv 1–3 to 46–50 maps, and the share sort puts a Lv 1–3 map first at 88%. Fix: a remembered "Your level" box that dims or hides maps outside your bracket, with "Your best map for Lv 32: …" at the top; or group rows into level bands and compare shares within a band. Command: /impeccable shape
- [P2] Narrow windows push the table out of its box: about 581px minimum, so below roughly 628px of window width the last two columns spill past the box and the page scrolls sideways (both reviewers measured this). It contradicts DESIGN.md's "a narrow window never pushes the table out of its box". Fix: below about 640px fold Region into the street line, let the date wrap, shorten the share header; as a backstop let the table scroll sideways inside its box, keeping the header pinned. Command: /impeccable adapt
- [P2] Missing data is silent: "999999999" with five empty cells; a blank share. Fix: "Not in MeowDB's map list" in the street line in the pale ink; a pale "—" with a screen-reader reason in the levels and share cells. Command: /impeccable harden
- [P2] A flat list where "you're here" outshines "your best", and it isn't framed as a game window. Fix: one centred window frame (bar as wide as the body, 1px edge in the window colour, 4px corners); the yellow star on the top map; calm the "you're here" row to its dot and edges; optionally a thin blue share gauge. Command: /impeccable bolder
- [P2] The podium button reads as a bar chart at 13px, appears without introduction, and the Best row doesn't lead to the page. Fix: a white outline star (matching the Grind Tracker's best star) or a trophy, and let the Best row's "Best" label open the page too. Command: /impeccable clarify

## Persona Red Flags
- Alex: the sort resets every open; no way to hide or remove a bugged best from the page; the map's ceiling is never shown, only the share; refreshing the tab never shows newer bests and nothing says so; "Date set" has no time of day.
- Sam: no table caption or accessible name; a change of sort isn't announced; empty cells read as "blank"; the share explanation is hover-only; map links use the browser's default focus ring, not Edit Blue; the table spills out of its box at 200% zoom; links open a new tab without saying so.
- Mina: with a Lv 32 character she discards most rows by eye every visit; "% of solo EXP/h" means nothing without hovering; "999999999" looks broken; the share sort calls a Lv 1–3 field her best; the overlays vanish while the browser is in front, with nothing pointing her back to the game.

## Minor Observations
- Sorting shifts the figures 16px (the triangle appears only on the sorted header); reserve its space on every header.
- The yellow dot pushes that row's name 16px right; hang it in the cell's left padding.
- The whole row tints on hover though only the name is a link.
- No favicon (needs the shell to fill @ICON@ everywhere, not just the first occurrence).
- Pinned header borders and corners untested with a long list (20+ maps).
- Region mostly repeats at low levels; could fold into the street line.

## Questions to Consider
- Should the page ask "Your level?" once and lead with your best map for that level?
- Is share of the solo ceiling the right second figure, or would the map's ceiling itself compare maps better?
- Is the browser the right home, or would an app window in setup's style (current, keeps its sort, can remove a best, keeps the overlays visible) serve better?
- What would make opening this feel like the game's own record book, like the Monster Book, rather than a spreadsheet?
