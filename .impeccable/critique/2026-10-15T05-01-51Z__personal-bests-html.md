---
target: Personal Bests window
total_score: 23
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 2
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3code-2d470cb1\\personal-bests.html"
target_fingerprint: "sha256:13d21a0371057f014dea1a90d87507b2372e888ca856858dec97476bb459cba6"
target_path: "C:\\Users\\itech\\.t3\\worktrees\\minerva\\t3code-2d470cb1\\personal-bests.html"
timestamp: 2026-10-15T05-01-51Z
slug: personal-bests-html
---
Method: dual-agent (A: design review · B: detector + browser)

## Personal Bests window — Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | The switch stays on while claimed, held or retrying (personal-bests.ts:687-692) |
| 2 | Match System / Real World | 3 | "Merge into…" never says it is for a renamed Character; the held copy is technical (ts:650) |
| 3 | User Control and Freedom | 2 | No undo anywhere; Move only from Unassigned (ts:436) |
| 4 | Consistency and Standards | 3 | The armed switch is styled; armed text buttons aren't |
| 5 | Error Prevention | 1 | A double-click runs an irreversible remove (verified) |
| 6 | Recognition Rather Than Recall | 3 | With one Character the picker hides (ts:586) and nothing names whose bests these are |
| 7 | Flexibility and Efficiency | 2 | No bulk assign, no level filter, pointer only |
| 8 | Aesthetic and Minimalist Design | 2 | "Map Leaderboard" on every row; five layers above the table |
| 9 | Error Recovery | 2 | The rejected line is cut short; held's remedy is "turn off and on" |
| 10 | Help and Documentation | 2 | Nothing says uploading is public under the Character's name, or what Merge is for |
| **Total** | | **23/40** | **Acceptable** |

## Design Specificity Verdict
Specific core, generic management chrome. The lanes are the EXP gauge, the ruler hangs like the forecast, the notches, You Yellow dots and star are the game's: only Minerva could draw them. The new layer above them (a pill toggle switch, the system's first; rows of identical light text buttons; status sentences) reads as a settings page stacked on a game window. The per-Character picker partly answers the 10 Oct critique's P1 (bests from every level mixed in one list); the new controls cost more than they add.

Detector: CLI 1 finding, browser 6. `cramped-padding` on `.box` is a false positive (the cells carry the inset; only `.sr-only` text sits at the edge). `ai-color-palette` on the close button is the game's blue button, intentional. `body-text-viewport-edge` ×4 is mostly the 1280px stub, but it points at a real problem: `.notes` has `max-width: none`, so notes and the rejected line run past 120 characters in a wide window.

## Priority Issues
- **[P1] A double-click confirms.** The armed button takes the trigger's place (ts:466-470, 617), so the second click of a double-click acts (ts:841-850). Verified for removing a Visit and a Character, which can't be undone. Armed text buttons look idle: no `.armed` style (styles.css:329-351). Fix: ignore a second click within about 400ms of arming; draw armed buttons in Problem Cream with a Problem Amber edge. → /impeccable harden
- **[P1] Remove and Merge quietly take the Character off the site** (lib.rs:1228-1229, 1241-1243), and their copy doesn't say so (ts:618, 625). The upload prompt doesn't say it's public (ts:695). Fix: "Remove Egbert, their bests on 6 maps and their Map Leaderboard entries?"; "Show Egbert's bests on 14 maps publicly on owlofminerva.app?" → /impeccable clarify
- **[P2] "Map Leaderboard" on every row** (ts:321-327). At 600px it wraps and rows run 4–7 lines; at 920×600 the controls take 158px and only 4 maps show. It even shows on "Not in the map list" rows and on Unassigned. Fix: move it into the opened Visit History or after the visit count; drop it on unknown maps. → /impeccable distill
- **[P2] Upload problems look like success.** Claimed, held and retrying are plain Ink Steel under a switch that's on (css:399-404), against the Orange Means Doubt Rule. Held's remedy uses a one-click off that deletes. "Up to date" shows the current time when there is no upload time (ts:645). The rejected line is cut short with no way to read it (html:30, styles.css:69-74). Fix: the problem-note pattern with its orange dot, a "Send again" button, a wrapping line. → /impeccable harden
- **[P2] Unassigned is triaged one Visit at a time** (ts:496-511), about 4 clicks each. Fix: "Assign all to…" on its count line, and Move on every Visit. → /impeccable shape

## Persona Red Flags
- **Alex (power user):** dozens of Unassigned Visits assigned by hand; no level-band filter; no undo; pointer only.
- **Sam (accessibility):** the window never takes focus, so focus rings are out of keyboard reach; the armed switch reads "switch, off, Upload…?"; truncated rejected reasons can't be read; the management layer is all 11–12px.
- **Mina (ordinary player, 3 Characters):** after updating, most bests are Unassigned, some with no level to tell whose; "Merge into…" reads as combining alts; a double-click habit costs her a Character.

## Minor Observations
- The credit is an info icon (html:73-77); DESIGN.md:711 asks for the credit's words.
- Three yellow dots (picker, gutter, lane) mean three things.
- The switch's on-fill is Portal Blue (css:385), stretching the Minimap Legend Rule; the CSS comment says Periwinkle.
- A Character with no maps is offered "Upload… on 0 maps".
- Unassigned gets the "Your best of all maps" star (ts:558).
- In the 520px cards a Visit line runs its level into its figure.

## Questions to Consider
- Should Character management move to the settings view, leaving the window over the game read-only?
- Is Unassigned a one-time "Whose were these?" step rather than a fourth tab?
- Should the leaderboard link go to the Character's page on the site rather than to each map's?
