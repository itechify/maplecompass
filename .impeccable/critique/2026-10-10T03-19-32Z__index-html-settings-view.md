---
target: settings
total_score: 26
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 2
target_identity: "file:C:\\Users\\itech\\.t3\\worktrees\\maplecompass\\t3code-8f9506a3\\index.html#settings-view"
timestamp: 2026-10-10T03-19-32Z
slug: index-html-settings-view
closed: true
---
Method: dual-agent (A: design review · B: detector + browser scan)

# Critique: Settings view and its gear button

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Waiting state has four cues; a successful change and Restore defaults are silent; nothing says the hotkeys are released while a row waits. |
| 2 | Match System / Real World | 2 | "Next Candidate" and "Item Lookup" are internal terms; "Taken" blames "another app" when it may be Windows; no framing of which combinations are safe to take from the game. |
| 3 | User Control and Freedom | 2 | Esc, Cancel and Back work; Restore defaults resets all eight with no confirm or undo; no per-row reset; no way to switch a hotkey off. |
| 4 | Consistency and Standards | 3 | Faithful to the Search View; the same 8 actions have three sets of names (toolbar tooltips, Settings, setup guide); Edit Blue used as a text colour. |
| 5 | Error Prevention | 2 | Catches no-modifier, duplicates and unusable keys; nothing warns about combinations that eat game input or belong to Windows; the hint offers Win as freely as Ctrl. |
| 6 | Recognition Rather Than Recall | 3 | All bindings visible with names; a conflict makes the player carry the combination through a cancel-and-rebind loop; Taken/Invalid reason only in a hover title. |
| 7 | Flexibility and Efficiency | 2 | Click Change, press keys is quick; no keyboard route in, no swap on conflict, no per-row reset, no "off". |
| 8 | Aesthetic and Minimalist Design | 3 | Clean and dense; eight identical "Change" buttons are the loudest column; "Settings" and "Hotkeys" title one section twice. |
| 9 | Error Recovery | 3 | Messages name the combination and the reason, with dot, box and words; the note sits below the whole list, up to 8 rows from its row; the unusable-key message doesn't name the key. |
| 10 | Help and Documentation | 3 | README, setup guide, gear tooltip and problem notes all point to the gear; no in-view explanation of Taken/Invalid. |
| **Total** | | **26/40** | **Acceptable** |

## Design Specificity Verdict

Design review: the materials belong to MapleCompass but the structure is generic. Every material comes from the system (grey window bar with the game-blue back button, inset box on near-opaque lavender, Selection Periwinkle on the waiting row, light-steel text buttons, the orange-dot problem note, Taken/Invalid tags in the "paused" tag's cream/amber/brown without orange). The layout is the standard keybinding table (VS Code, Discord, OBS): name · keys · Change, Restore defaults at the bottom. It never speaks the game's language: the gear is a desktop metaphor, combinations are plain bold text, and the one place game knowledge matters (which keys are safe to take mid-fight; MapleStory's defaults put attack on Ctrl and jump on Alt) gets a neutral "Hold Ctrl, Alt or Win".

Detector: CLI `detect --json index.html` found 1 issue, low-contrast on the pre-existing "Not sure" chip (white on #ffd25a, 1.4:1), a false positive (its label is outlined with text-shadow). Browser injection on 5 harness states found 13-15 per state: ai-color-palette x11 on the pinned Game Button Blue (false positive), flat-type-hierarchy 12/13/14 px (false positive, pinned scale), cramped-padding on the waiting row (false positive, 3px clearance measured), low-contrast on the bottom hint 2.9:1 (harness artefact: glass over #222; a caveat over dark scenery), and low-contrast on "Press keys…" (Edit Blue #2f63c8 on #d3def3, 4.1:1), a true positive the design review found independently. Both assessments also saw the narrow-width truncation.

Visual overlays: injected and visible in the [Human] tab; the live server was stopped afterwards.

## Overall Impression

The waiting state and the error copy are the best part; the structure is a familiar settings table any app could ship. The biggest opportunity is to stop being neutral about keys: say which combinations are safe while fighting, and make the result visible (keycap-like combinations, a "saved" moment, an undo).

## What's Working

- The waiting state is unmistakable without motion: the row selected with its edge ring, "Press keys…" replacing the combination, Change turning into Cancel at the same 54px width, and the hint switching. Four cues, no layout shift, true to the no-motion rule.
- Errors teach in the player's terms: "Ctrl+Shift+F is already the “Search a mob” hotkey. Press another, or change that one first." and "…Shift+K would never reach the game, not even its chat". Never colour alone; announced with role="alert".
- The way to the fix connects: panel problem note, setup guide, README and the gear tooltip all lead to the gear; the keyboard is only taken while needed; rows on near-opaque lavender read over any scene (about 12:1).

## Priority Issues

1. [P1] Narrow widths lose the row's name when the row is broken. At 320px "Map's MeowDB pa…" truncates with default bindings; a Taken or Invalid tag cuts rows to "Next Candi…" and "Ite…"; a long combination like Ctrl+Alt+Shift+Win+Num Enter leaves the name 0px wide and pushes Change past the box edge. Cause: the combination and the tag are flex: none, so the name alone shrinks. Fix: name min-width ~7em, combination wraps at "+", Taken/Invalid on a second line under the name with its reason, and a container query below ~360px stacking the combination under the name. Suggested: /impeccable adapt
2. [P1] Keyboard-only and screen-reader players are shut out. The overlay can't be activated and the gear has no hotkey, so Settings can't be opened from the keyboard; the waiting state isn't in a live region (announces only "Settings, region"); success focus lands on "Change the Search a mob hotkey" without the new keys; Restore defaults is silent; the Taken reason is a title only; "Press keys…" is 4.1:1. Fix: a "Change hotkeys" button in setup's Overlay guide (keyboard-reachable via Ctrl+Shift+S) that opens Settings with focus; one aria-live="polite" status; current keys and state in each button's accessible name; "Press keys…" in Panel Ink. Suggested: /impeccable harden
3. [P2] Silent on the real risk, taking keys from the game or from Windows. The hint treats Ctrl, Alt and Win as equals; nothing says a hotkey swallows that press in the game whenever MapleCompass is in front; "Taken" always blames "another app". AltGr arrives as Ctrl+Alt on European layouts; many Win combinations belong to Windows. Fix: steer to Ctrl+Shift ("Hold Ctrl+Shift and press a letter you don't use in the game · Esc cancels"); "Windows or another app has it"; test Win+letter, Win+L and Alt+F4 while waiting in the real app. Suggested: /impeccable clarify
4. [P2] Saving and Restore defaults give no feedback, and Restore defaults has no way back. Nothing says "saved"; Restore defaults resets all eight with no confirm or undo, stays enabled when nothing would change, silently cancels a waiting row, and moves 38px under the pointer when a problem note grows the panel. Fix: the Grind Tracker's "Session reset." + Undo pattern ("Hotkeys back to their defaults." with Undo until the view closes), grey the button when all are default, a short "Saved" status after each change. Suggested: /impeccable polish
5. [P2] Conflicts force a loop, and the error sits far from its row. Cancel, rebind the other hotkey, come back and press the remembered combination again; the note appears under the whole list while the row still says "Press keys…". Fix: show the refused combination in the row in the problem palette ("Ctrl+Shift+F · in use"), put the note under that row inside the box, offer "Swap with Search a mob". Suggested: /impeccable layout

## Persona Red Flags

Alex (impatient power user): no keyboard route into Settings; no swap on conflict, no per-row reset; no way to switch off an unused hotkey, so all eight combinations stay swallowed while the game is in front; Restore defaults is all-or-nothing.

Sam (keyboard-only / screen reader): can't open Settings without a mouse; inside, the waiting state, success and Restore defaults are unannounced; the Taken reason is hover-only; "Press keys…" is 4.1:1.

The average MapleStory Classic player, mid-hunt: the gear sits between placement and "?", and a misclick hands the game's keyboard to the overlay until they click back; no warning that a combination built on the attack (Ctrl) or jump (Alt) key will eat presses mid-fight; "Next Candidate" and "Item Lookup" are internal terms (the toolbar says "Next map with this title"); "Taken" doesn't say by what (for a streamer, likely OBS or Discord).

## Minor Observations

- Names differ across surfaces: "Placement" / "Change placement" / "Move the panel…"; "Map's MeowDB page" / "Open MeowDB page". HOTKEY_NAMES claims the guide's order but not its names.
- The "Hotkeys" heading repeats "Settings" while it is the only section, and is the only section heading without a legend glyph.
- The no-modifier note ("Hold Ctrl, Alt or Win too…") sits directly above the hint saying the same.
- "That key can't be part of a hotkey" doesn't name the key, unlike the other two messages.
- The resting hint wraps with "front · Esc goes back" stranded at 380px.
- The gear icon (6 spokes round a ring) reads at real size as a sun or nut more than a gear.
- DESIGN.md is out of date: no gear in its toolbar list, still says only the search view takes the keyboard, no Settings View spec, Taken/Invalid tags and Edit Blue as text undocumented.

## Questions to Consider

- Why does rebinding live in the overlay at all? Setup's Overlay guide already lists all eight bindings in a keyboard-reachable desktop window; rebinding there would drop the gear and keep "the overlay never takes the keyboard" almost intact.
- Should the guidance start from the game's keymap: recommend Ctrl+Shift, warn when a combination rests on Ctrl or Alt alone?
- Should a hotkey be able to be switched off?
- Could combinations drawn as keycaps, in the look of the game's own key-binding window, be this view's signature move, and fix the width problem too?
