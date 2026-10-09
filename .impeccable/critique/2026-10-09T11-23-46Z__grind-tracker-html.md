---
target: grind tracker
total_score: 23
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 2
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\maplecompass\\t3code-224931a9\\grind-tracker.html"
target_fingerprint: "sha256:a02c612e179881a14ed463f866816175052d724e2ee42200dc5afafbe685ad8f"
target_path: "C:\\Users\\itech\\.t3\\worktrees\\maplecompass\\t3code-224931a9\\grind-tracker.html"
timestamp: 2026-10-09T11-23-46Z
slug: grind-tracker-html
closed: true
---
Method: dual-agent (A: design-review agent · B: detector/browser agent)

# Critique: Grind Tracker (`grind-tracker.html`)

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 2 | The window looks the same whether the EXP bar is being read or not. It never shows the last reading, and the Session never shows how long it has run. |
| 2 | Match System / Real World | 3 | "Rolling 5 min" means 5 minutes of active play, not clock time. "Solo ceiling" is a term the panel itself never uses. |
| 3 | User Control and Freedom | 2 | Reset can't be undone. A reset caused by a character switch can't be recovered. A dragged tracker never follows the panel again. |
| 4 | Consistency and Standards | 3 | EXP/h reads "61.2k" here and "297,499" in the panel. The warming up and paused figures turn Panel Ink Steel, a change of about 1.2:1 that nobody can see. |
| 5 | Error Prevention | 2 | The one destructive button, reset, looks exactly like × and sits 2px from it, with no confirmation. |
| 6 | Recognition Rather Than Recall | 3 | The Session column doesn't say how long the Session is. The ceiling share refers to a figure in another window, under a different label. |
| 7 | Flexibility and Efficiency | 2 | Show/hide has a hotkey and the position is remembered. Reset needs the mouse, there is no compact mode, and a dragged window can't re-attach to the panel. |
| 8 | Aesthetic and Minimalist Design | 3 | Compact and clean. But it shows one rate in three units, and the Session column repeats the Rolling column for the first 5 minutes. |
| 9 | Error Recovery | 1 | When the EXP bar can't be read, the window says the player is on a break. |
| 10 | Help and Documentation | 2 | The README explains it. In the app, the guide only lists the hotkey; nothing explains Rolling vs Session, ceiling, warming up or paused. |
| **Total** | | **23/40** | **Acceptable** |

## Design Specificity Verdict

**Design review:** the window frame is clearly MapleCompass's own; the body is generic.
- **Frame:** the Mini Map's grey window bar, bevelled game-blue buttons, the tally-board icon, Quest Helper glass and the Maplestory typeface. These are shared with the panel through `src/styles.css`, so it is a true sibling of the panel and sits naturally beside the real Quest Helper.
- **Body:** a label with a figure, a 2×2 rates table and a footnote sentence. A RuneLite XP tracker could use that layout unchanged.
- The panel has its signature move (Hidden Portals drawn on mini-minimaps); this window has none. It is the weakest expression of "the window the game forgot to ship" among the app's windows.

**Detector:** the file scan is clean: 0 findings on `grind-tracker.html` and `src/grind-tracker.ts`. Running it on the rendered page found 2 to 4 per state:
- **`ai-color-palette` on `#reset-session` and `#close` (all six states): false positive.** It flags them as a "cyan gradient", but they are the game's own window buttons, deliberately specified as Game Button Blue in DESIGN.md.
- **`low-contrast`, 3.9:1 (dark ink on glass) on `#tracking-extra` and `#tracking-waiting`: true, but only over dark scenes.** The same ink-on-glass pair on the "Time to Level" label, the column headers and the held figures was not flagged. The detector only checked paragraph elements, so it undercounts.
- **`low-contrast`, 2.9:1 (lightest ink on glass) on `#tracking-switched`: true.** This is the strongest finding, and both assessments found it independently.

**Caveat on contrast:** the review harness places the window over its flat `#222` margin, not over the game screenshot. So both measurements are a dark-scene worst case, and the detector ignores the light text edge DESIGN.md specifies for this glass. Measured over different backgrounds (text without that edge):

| Behind the glass | Panel Ink | Panel Ink Steel | Panel Ink Slate |
|---|---|---|---|
| black | 3.98 | 3.29 | 2.46 |
| `#222` (harness) | 4.76 | 3.94 | 2.94 |
| mid grey | 7.47 | 6.18 | 4.61 |
| white | 12.45 | 10.29 | 7.68 |

**Visual overlays:** the detector's overlay was injected in the preview browser's **[Human] Grind Tracker critique** tab, on the `active` state with 3 markers. It should be visible there now. It won't survive a reload, because the dev server has been stopped.

## Overall Impression

The window looks like it belongs, and its copy is restrained. But players can't fully trust what it says:
- It can't show that it is reading.
- It calls a failed read a break.
- The one destructive button sits next to the most-pressed one.
- It moves every time the panel changes height.

The biggest opportunity is to make the tracker prove it is reading. That fixes the trust problem and gives the window the signature move it lacks.

## What's Working

1. **Figures are no more precise than they can be trusted, and states are named in words.**
   - EXP/h is rounded to 3 significant figures (`src/grind-tracker.ts:33`) and %/h to one decimal. Time to Level gets coarser the further off it is, and the figures hold still for 30s at a time, so nothing flickers.
   - "warming up", "paused" and "New Session: switched character" are tags or notes with words, not colour alone. This is PRODUCT.md principle 3 done right for the figures themselves.
2. **It is a real sibling of the panel.** The bar, buttons, glass, type and inks come from the same stylesheet, so it can't drift away from the panel's look.
3. **It respects play and holds up under stress.**
   - The window can't take keyboard focus, so a click never takes the keyboard from the game.
   - It stays clear of the Title Region and the EXP bar region.
   - × hides the window without stopping the Session.
   - Extreme figures (12.3M EXP/h, −988k, 2,777h, 128 deaths) all fit in 280px without overflowing or truncating.

## Priority Issues

### 1. [P1] An unreadable EXP bar shows as "paused" (needs core + UI changes)
- **What:**
  - When a frame doesn't read, the core only advances the clock (`crates/compass-core/src/lib.rs:411-420`).
  - After 2 minutes (`IDLE_MS`, `crates/compass-core/src/session.rs:104`), the window says "paused" while the player is still hunting.
  - Before the first EXP gain, "Waiting for EXP" never says whether anything is being read.
  - When readings come back, the whole gain lands at once while the gap was counted as a break, so the rates jump.
  - `PanelState.exp_bar` holds the last reading, but nothing shows it.
- **Why it matters:** PRODUCT.md principle 3 asks to make recognition uncertainty clear. The panel already says when it can't read the Title. For a tool that reads the screen, proof that it is reading is what trust rests on.
- **Fix:**
  - In the core, keep "not reading" separate from "not gaining", and don't count an unreadable stretch as a break.
  - Expose the reading state in `ExpTracking`.
  - In the UI, add a quiet line showing the last reading ("EXP 160 [3.81%]").
  - After N seconds without a read, show a problem note: "Can't read the EXP bar. Is something covering it?"
  - Make Waiting say what starts it: "Starts at your next EXP gain."
- **Suggested command:** /impeccable harden (then /impeccable clarify for the copy)

### 2. [P1] Reset sits beside × and can't be undone
- **What:**
  - Reset and close are identical 19px game-blue buttons, 2px apart (`grind-tracker.html:18`, `grind-tracker.html:21`).
  - There is no confirmation, so one mis-aimed click wipes a Session of several hours.
  - The README's reason for not giving reset a hotkey shows the risk is known, but the mouse side is unguarded.
- **Fix:**
  - Leave only × on the window bar.
  - Move reset into the body as a light-steel icon button, the way the Hunting Target's buttons sit in the panel's body, for example beside the "Session" header.
  - In the core, keep the previous figures until the next EXP gain and show "Session reset · Undo" until then. (Press-and-hold is out: it needs animation, which DESIGN.md forbids.)
- **Suggested command:** /impeccable harden

### 3. [P2] The tracker moves whenever the panel changes height (core change)
- **What:**
  - Until the player drags it, the tracker sits 8px below the panel, or above it when there's no room below (`crates/compass-core/src/grind_tracker.rs:57-67`).
  - The panel's height changes on every map: portals, mobs, route, crawl note. So the tracker moves up and down with each map change, and can flip from below the panel to above it.
  - When the panel is collapsed, the tracker is still placed against the expanded panel (`crates/compass-core/src/lib.rs:864`), so it floats under a panel that isn't there.
  - Once dragged, it can never follow the panel again: nothing ever clears the saved position (`grind_tracker.at`).
- **Why it matters:** a tool read at a glance has to be where the eye left it.
- **Fix:**
  - Anchor the default to something that doesn't move with content: the panel's fixed top edge, or the bottom of its maximum height.
  - Sit beside the icon when the panel is collapsed.
  - Re-attach the tracker when a drag ends near the panel.
- **Suggested command:** /impeccable layout

### 4. [P2] One rate in three units, while the useful comparison is buried (core + UI)
- **What:**
  - Time to Level, rolling EXP/h and rolling %/h all express the same rolling rate.
  - The Session column is identical to Rolling for the first 5 active minutes. The harness mock hides this by giving them different values.
  - The Session shows no length or total.
  - The ceiling share is the only "is this spot worth it?" signal, yet it is the smallest, palest line, run together with deaths.
  - Right after a map change, the ceiling share compares a rolling rate earned partly on the old map with the new map's ceiling.
- **Fix:**
  - Time to Level as the headline.
  - One rolling line: "61.2k EXP/h · 12.9%/h, last 5 min".
  - A Session line with its length and total. This needs the active time and EXP gained in `ExpTracking`; show it only once it differs from Rolling.
  - The ceiling share on its own line, in plainer words ("21% of MeowDB's solo EXP/h here"), shown only once the rolling window lies entirely on this map.
  - Deaths on their own line.
- **Suggested command:** /impeccable distill

### 5. [P2] Text over dark scenes is hard to read, and the quiet states don't look quieter
- **What:**
  - Over dark scenes, the lightest ink (the switch note) falls to 2.5–2.9:1, and the secondary ink (labels, headers, the extra line, held figures) to 3.3–3.9:1.
  - Game text shows through the glass: an NPC name tag sat directly under "Rolling 5 min" in the review screenshots.
  - The colour change for warming up and paused can't be seen.
  - The explanation for a character switch is the quietest text in the window, quieter than the "Waiting for EXP" it explains.
- **Fix:**
  - Put the figures on the near-opaque lavender inset. DESIGN.md already does this for the search results, "the panel's densest reading, over the busiest scenes".
  - Make the switch note a standard 13px note in Panel Ink Steel.
  - Show held figures in Light weight instead of a colour shift.
- **Suggested command:** /impeccable typeset

## Persona Red Flags

**Alex (power user):**
- Reset needs the mouse.
- There's no Time-to-Level-only compact mode, and no way to re-attach a dragged window.
- Comparing two spots takes 60s of warming up, then figures that move only every 30s.
- "61.2k" has to be converted mentally to compare it with the panel's "297,499 solo".
- After a map change, the ceiling share is wrong for up to 5 minutes.
- There's no Session length or total to note down ("+38% in 2h 10m").

**Sam (accessibility):**
- Reset and close can't be reached from the keyboard. That follows from the rule that the window never takes the keyboard, but it means reset has no keyboard path at all.
- Only the switch note is a live region: "paused", "warming up" and new deaths change silently.
- A bare "—" is read aloud as "em dash".
- The toolbar and the section are both labelled "Grind Tracker", and the table's corner header cell is empty.
- Contrast is 3.3:1 and 2.5:1 over dark scenes.

**Mina (an ordinary Classic player, a level 35 Hunter mid-grind, who has never heard of MeowDB):**
- "21% of the map's solo ceiling": what ceiling? 21% reads like a failing grade.
- "Rolling 5 min" or "Session": which one should she trust?
- Two quick deaths turn Time to Level into a bare "—" with no words, and she thinks it broke.
- A potion run to town turns the window cream and brown ("paused"), which reads as a warning.
- She switches to her mule to drop off loot and back: two "New Session" notes, and her main's two-hour Session is gone.
- She closes the tracker with × and doesn't know the rising-line button in the panel's toolbar brings it back. That icon doesn't match the window's tally board.

## Minor Observations

- The " · " between the ceiling share and deaths isn't non-breaking. At 280px the active state ends its first line with a dangling "·" (`src/grind-tracker.ts:75-82`).
- `.rates td { min-width: 72px }` has no effect. The table stretches to 256px with columns of 43/107/107px, so "61.2k" sits about 150px from "EXP/h".
- The "—" comes from Arial at 18px bold and reads like a divider or a strikethrough beside "Time to Level".
- `compact()` switches from "9,870" to "10k" at 10,000, so one column can mix "−3,400" with "21.8k".
- The window's height changes across states: 67, 87, 134, 154, then 170px. When the tracker sits above the panel, its top edge moves with every state change.
- The collapsed icon's badge shows Time to Level with no qualifier while the tracker is paused or warming up.
- The `@container (max-width: 300px)` query always matches in a 280px window, and its `.extra` rule duplicates `src/grind-tracker.css:18`.
- The committed PR screenshots (`.impeccable/review/pr/grind-tracker-*.png`) show the old compass icon, so they are stale as review evidence.

## Questions to Consider

- If Time to Level, EXP/h and %/h are one number in three units, which one does a Classic player say out loud? Could the window be one headline and one line?
- Should the tracker take the game's own EXP gauge as its signature move: a slim, still gauge of the last reading that proves it is reading?
- Is "paused" really a problem? DESIGN.md chose the problem palette on purpose, but a town run is normal play.
- Should a Session belong to a character, so a quick switch to a mule doesn't erase the main's grind?
- Is "collapsed panel + Grind Tracker" the real grinding setup? If so, should it be designed for, rather than left floating where the expanded panel used to end?
