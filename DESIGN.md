---
name: MapleCompass
description: The in-game overlay panel that names the map, shows its Hidden Portals and judges its hunting, with its setup and title-marking windows, all drawn in MapleStory's own window language.
colors:
  ink: "#1b2648"
  ink-2: "#44527a"
  ink-3: "#505c7d"
  body: "rgba(234, 232, 245, 0.95)"
  body-solid: "#eae8f5"
  frame: "#3e527c"
  bar-hairline: "#2e4170"
  street: "#1d2b52"
  outline: "#22325a"
  bar-top: "#dfe8f7"
  bar-mid: "#b4c6e6"
  bar-base: "#94abd6"
  portal: "#1f63d6"
  portal-fill: "#eef5ff"
  portal-inner: "#8db3ee"
  mob: "#d8342a"
  mob-edge: "#8e1c15"
  select: "#d3def3"
  select-edge: "#98afda"
  orange: "#f08a1c"
  orange-deep: "#b85e00"
  problem-bg: "#fff1dc"
  problem-edge: "#e5a04f"
  problem-ink: "#6b3500"
  edit: "#2f63c8"
  mode-bg: "#dfe8f8"
  mode-ink: "#1f3f80"
  tag-bg: "#dae3f3"
  tag-edge: "#a3b6d8"
  tag-ink: "#284a86"
  quest-bg: "#dff0d8"
  quest-edge: "#98c58a"
  quest-ink: "#2b6a20"
  minimap-paper: "#eef3fb"
  minimap-frame: "#50648c"
  minimap-grid: "#b8c7e1"
  minimap-cell: "#a9c3ec"
  button-top: "#4673c6"
  button-base: "#2a52a6"
  button-edge: "#1c3672"
  button-light-top: "#f8fafe"
  button-light-base: "#d5dff0"
  disabled-bg: "#d3d9e5"
  disabled-edge: "#aeb8cc"
  disabled-ink: "#6b7591"
  veil: "rgba(18, 28, 56, 0.3)"
typography:
  headline:
    fontFamily: "Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 700
    lineHeight: "23px"
    letterSpacing: "0.01em"
  street:
    fontFamily: "Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 700
    lineHeight: "15px"
  title:
    fontFamily: "Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 700
    lineHeight: "17px"
  body-reading:
    fontFamily: "Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: "20px"
  body:
    fontFamily: "Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: "17px"
    fontFeature: "\"tnum\" 1"
  body-small:
    fontFamily: "Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: "15px"
  button:
    fontFamily: "Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 700
    lineHeight: "18px"
  label:
    fontFamily: "Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 700
    lineHeight: "15px"
    letterSpacing: "0.04em"
rounded:
  chip: "3px"
  notice: "4px"
  window: "6px"
spacing:
  inset-x: "11px"
  section: "10px"
  stack: "9px"
  row: "6px"
  list: "2px"
components:
  panel:
    backgroundColor: "{colors.body}"
    textColor: "{colors.ink}"
    rounded: "{rounded.window}"
    width: "380px"
  section-body:
    padding: "8px 11px"
  doubt-chip:
    backgroundColor: "{colors.orange}"
    textColor: "#ffffff"
    typography: "{typography.label}"
    rounded: "{rounded.chip}"
    padding: "2px 7px 3px"
  tag:
    backgroundColor: "{colors.tag-bg}"
    textColor: "{colors.tag-ink}"
    typography: "{typography.label}"
    rounded: "{rounded.chip}"
    padding: "0 5px"
  tag-quest:
    backgroundColor: "{colors.quest-bg}"
    textColor: "{colors.quest-ink}"
    typography: "{typography.label}"
    rounded: "{rounded.chip}"
    padding: "0 5px"
  candidate-row:
    textColor: "{colors.ink-2}"
    typography: "{typography.body-small}"
    rounded: "{rounded.chip}"
    padding: "3px 7px"
  candidate-row-current:
    backgroundColor: "{colors.select}"
    textColor: "{colors.ink}"
    rounded: "{rounded.chip}"
    padding: "3px 7px"
  note-problem:
    backgroundColor: "{colors.problem-bg}"
    textColor: "{colors.problem-ink}"
    typography: "{typography.body}"
    rounded: "{rounded.notice}"
    padding: "4px 8px 5px 24px"
  mode-notice:
    backgroundColor: "{colors.mode-bg}"
    textColor: "{colors.mode-ink}"
    typography: "{typography.title}"
    rounded: "{rounded.notice}"
    padding: "4px 8px 5px"
  mini-minimap:
    backgroundColor: "{colors.minimap-paper}"
    width: "47px"
    height: "29px"
  collapsed-icon:
    backgroundColor: "{colors.bar-mid}"
    textColor: "{colors.ink}"
    rounded: "{rounded.window}"
    size: "44px"
  setup-window:
    backgroundColor: "{colors.body-solid}"
    textColor: "{colors.ink}"
    typography: "{typography.body-reading}"
    width: "520px"
    height: "560px"
  mark-icon:
    backgroundColor: "{colors.bar-mid}"
    rounded: "{rounded.window}"
    size: "28px"
  button-primary:
    backgroundColor: "{colors.button-base}"
    textColor: "#ffffff"
    typography: "{typography.button}"
    rounded: "{rounded.notice}"
    padding: "6px 16px 7px"
  button-secondary:
    backgroundColor: "{colors.button-light-base}"
    textColor: "{colors.ink}"
    typography: "{typography.button}"
    rounded: "{rounded.notice}"
    padding: "6px 16px 7px"
  button-disabled:
    backgroundColor: "{colors.disabled-bg}"
    textColor: "{colors.disabled-ink}"
    typography: "{typography.button}"
    rounded: "{rounded.notice}"
    padding: "6px 16px 7px"
  region-hint:
    backgroundColor: "{colors.body}"
    textColor: "{colors.ink}"
    rounded: "{rounded.window}"
    padding: "9px 14px 10px"
  key-cap:
    backgroundColor: "{colors.tag-bg}"
    textColor: "{colors.tag-ink}"
    rounded: "{rounded.chip}"
    padding: "0 6px"
---

# Design System: MapleCompass

## Overview

**Creative North Star: "The Window the Game Forgot to Ship"**

The overlay sits beside the game's own windows (the Quest Helper, the minimap) and is made from their materials: a translucent lavender-grey body, a steel-periwinkle title bar with a white top highlight and a dark-navy hairline, the map name in white bold lettering outlined in navy the way the minimap draws it, and the minimap's legend of blue portal rings and red mob dots. The game supplies the materials and one signature move. Organisation, hierarchy and density come from a working tool: hidden exits first, then the hunting numbers, then the credit. It is not a screenshot of a game window. The setup window and the title-marking window are built from the same materials, so every MapleCompass window reads as one more game window.

It is light because the game's windows are light, and dense because it is read in glances between attacks. Every state is drawn in the same window. Doubt and problems get the game's orange, and nothing else does. The confirmed rejection is the generic dark dev-tools card with a stats table on top, which this world replaced.

The overlay has no pointer interaction, so it has no hover or focus states. It is click-through except for dragging in edit mode, and its height follows its content. The setup window is clicked and tabbed through, so its buttons, and only they, carry hover, pressed, focus and disabled states. Nothing in any window moves: there is no motion.

**Key Characteristics:**
- Light window body in the game's lavender (translucent over the game, opaque on the desktop), under a gradient steel-blue title bar.
- Map name in outlined white lettering, with the Street Name above it as on the minimap.
- Minimap glyphs (portal ring, mob dot) are the only icon vocabulary, drawn in CSS and SVG.
- Game orange appears only for doubt and problems.
- One plain bold sans (Arial) throughout; tabular figures in the overlay.
- No hover or focus on the click-through overlay; interactive windows give their buttons the game's button states. No motion anywhere.

## Colors

Cool lavender and periwinkle neutrals with navy ink, two minimap accents (portal blue, mob red), a reserved game orange, and a deep steel for the one button that moves setup forward.

### Primary
- **Portal Ring Blue** (portal): the minimap's portal ring. Section mark for Hidden Portals, the ring and core in each mini-minimap, and the ring in the collapsed icon and in setup's mark icon, always over Portal Ring Fill (portal-fill) with Portal Inner Ring (portal-inner) as the icon's second ring.

### Secondary
- **Game Orange** (orange, with orange-deep for edges and text outlines): the game's small orange button. Used for the "Not sure" chip, the dot in problem notes (overlay and setup), and the collapsed icon's alert dot. Problem notes sit on Problem Cream (problem-bg), edged in Problem Amber (problem-edge), with Problem Brown text (problem-ink).

### Tertiary
- **Mob Dot Red** (mob, edged by mob-edge): the minimap's mob dot. Section mark for Hunting and the bullet on each mob row.

### Neutral
- **Window Lavender** (body): the translucent body of every window laid over the game (the overlay panel and the title-marking hint), at 95% opacity so it reads like the game's own and stays legible over busy scenes.
- **Window Lavender, Opaque** (body-solid): the same lavender with no translucency, for windows that sit on the desktop rather than over the game (the setup window).
- **Title Bar Steel** (bar-top, bar-mid, bar-base): the three stops of the vertical title-bar gradient. In code the gradient is one custom property, `--bar` (`linear-gradient(180deg, #dfe8f7 0%, #b4c6e6 48%, #94abd6 100%)`), shared by the overlay title bar, the collapsed icon, setup's title bar and mark icon, and the recognised test read. Reuse `--bar` rather than restating the stops.
- **Frame Navy** (frame) and **Bar Hairline** (bar-hairline): the 1px window border and the rule under the title bar. Frame Navy also edges the secondary button.
- **Street Navy** (street): the Street Name, and the setup title bar's window name and step count.
- **Lettering Outline** (outline): the navy outline around the white map name.
- **Ink** (ink): primary text, values, mob names, destinations, setup headings and body.
- **Ink Steel** (ink-2): secondary text such as notes, positions, units and table headers; setup's muted lines and neutral notices.
- **Ink Slate** (ink-3): tertiary text such as the footer and candidate map IDs; list markers in setup.
- **Selection Periwinkle** (select, edged by select-edge): the selected candidate row.
- **Tag Periwinkle** (tag-bg, tag-edge, tag-ink): the "Hidden Street" tag, setup's "Recognised" tag, and the Esc key cap. **NPC Green** (quest-bg, quest-edge, quest-ink): the "Quest" tag, taken from the minimap's NPC green.
- **Edit Blue** (edit, with mode-bg and mode-ink): the dashed edit-mode outline, the dashed edit-mode notice, the dashed drag box when marking the title, and the solid focus ring on setup's buttons.
- **Minimap Paper** (minimap-paper, minimap-frame, minimap-grid, minimap-cell): the mini-minimap's ground, frame, 5x3 grid lines and lit cell.
- **Button Steel** (button-top, button-base, edged by button-edge): the primary button's gradient, deeper than the title bar so white text holds 4.5:1 even at its light top. **Light Steel** (button-light-top, button-light-base): the secondary button's gradient. Hover gradients sit one step darker (primary) or lighter (secondary).
- **Greyed** (disabled-bg, disabled-edge, disabled-ink): a button the window won't take, flat with no highlight.
- **Navy Veil** (veil): the title-marking window's whole surface, so the game reads as paused behind it. It is non-zero on purpose: a fully transparent pixel would not take the mouse.

### Named Rules
**The Orange Means Doubt Rule.** Game orange marks only uncertainty and problems: the "Not sure" chip, problem notes, the collapsed alert dot. A confirmed state never uses it, so orange can't become decoration.

**The Minimap Legend Rule.** Portal blue and mob red come from the minimap's legend and keep that meaning. Blue is for portals and red is for mobs, with no other uses.

**The Over the Game, Translucent Rule.** A window laid over the game uses the translucent Window Lavender (body); a window on the desktop uses the opaque one (body-solid). Same colour, different footing.

## Typography

**Display Font:** Arial (with "Segoe UI", system-ui, sans-serif), as the shared `--font`
**Body Font:** the same family
**Label/Mono Font:** none distinct; overlay figures use tabular numerals

**Character:** The game sets its windows in a plain bold sans, so every window uses one face everywhere. Arial is a pinned exception to the usual system-face caution, because it is the game's own idiom. Weight (700 against 400) and colour carry the hierarchy, not a second family.

### Hierarchy
- **Headline** (700, 18px/23px, 0.01em): the Map Name, in white with a navy outline, in the overlay and in setup's recognised test read. Wraps to at most two lines rather than losing its end. When no map is read, the status takes this slot at 15px in plain ink with no outline, so it doesn't read as a map name. Setup's step headings use the same metrics in plain ink, without the outline.
- **Street** (700, 12px/15px): the Street Name above the Map Name, the minimap's own idiom; also setup's step count in the title bar.
- **Title** (700, 13px/17px): section headings (Hidden Portals, Hunting), portal destinations, summary values, and the edit-mode notice. Setup's window name in the title bar is the same weight at 14px.
- **Body Reading** (400, 14px/20px): setup's prose and the title-marking hint (at 700). Setup is read at rest, not in glances, so it reads one step larger than the overlay.
- **Body** (400, 13px/17px): overlay notes and the mob table. Mob names are 700. Setup's notices and test read use 13px at an 18px line.
- **Body Small** (400, 12px/15–17px): positions, summary labels and units, table headers, candidate rows, the footer, setup's MeowDB credit.
- **Button** (700, 13px/18px): setup's button labels, sentence case.
- **Label** (700, 11px, 0.04em; uppercase only on the doubt chip): tags and the "Not sure" chip. The Esc key cap is 12px.

### Named Rules
**The Tabular Figures Rule.** In the overlay all figures are tabular (`font-variant-numeric: tabular-nums` on the body), and numeric columns are right-aligned so EXP, HP and share line up from one glance to the next.

**The Outlined Lettering Rule.** Only a Map Name uses white lettering with a navy outline (the shared `--lettering` text-shadow: a 1px stroke in eight directions plus one soft 0 2px 2px drop). It appears in the overlay title bar and in setup's recognised test read, nowhere else. Everything else is ink on the lavender body.

**The Notes Wrap, Names Truncate Rule.** Notes and the Map Name wrap so they can be read in full. Single-line facts (destinations, mob names, footer) truncate with an ellipsis instead of growing the panel.

## Layout

A single 380px-wide column whose height fits its content (44–900px). The overlay reports its height and the window resizes to match. The title bar runs full width (padding 6px 11px 7px). The body below it is a vertical stack with 11px side insets: 9px between top-level items, 10px between sections, 6px between portal rows and 2px between candidate rows. The rhythm is tuned pixel by pixel to the game's windows, not taken from a modular scale.

Order is fixed: title bar, then any status, mode or problem notes, then candidates when several maps share a title, then Hidden Portals, then Hunting (a two-row summary grid of EXP/h and Mobs/h, solo and 6+ party, then up to five mob rows and a "+N more" line), then a footer with the credit and the open-page hotkey. A portal row is a two-column grid: the 47px mini-minimap, then the destination with its tags over the position in words. When a section has nothing to list, its heading and "None on this map" share one line. Empty containers collapse rather than leave strips.

Collapsed, the window becomes a 44px square holding the portal ring, the Hidden Portal count and an optional alert dot. There are no breakpoints: the width is fixed.

The setup window is a fixed 520×560 opaque window: a full-width title bar (padding 9px 20px 10px, 10px gaps) holding the mark icon, the window name and "Step N of 5" at the right, then a column with 22px side insets and 18px bottom padding. Each step is a heading (18px above, 8px below), prose in 10px-spaced paragraphs, and a footer pinned to the bottom of the window above a faint 1px rule (Frame Navy at 20%), buttons right-aligned with 10px between them, the step forward always last. Notices sit above the step, 14px below the title bar.

The title-marking window covers the game client exactly. Its hint is centred horizontally, 12% up from the bottom, on one line.

## Elevation & Depth

No window has a drop shadow. Depth comes from the game's window chrome: inset white highlights (a 1px inner ring on the body, a 1px top highlight on the title bar, the doubt chip and the buttons), 1px ring strokes around glyphs (`0 0 0 1px`), and the outlined lettering of the Map Name. The panel sits flat on top of the game scene, and its translucency is what joins it to the game's windows. A pressed button sinks with an inset shadow rather than lifting.

### Shadow Vocabulary
- **Window inner highlight** (`box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.65)`): just inside the frame of the panel and the title-marking hint. The collapsed icon and setup's mark icon use 0.7.
- **Bar top highlight** (`box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.9)`): the top edge of every title bar and the recognised test read. The doubt chip uses the same at 0.55, the primary button at 0.3, and the secondary button a solid `#fff`.
- **Pressed** (`box-shadow: inset 0 1px 3px rgba(10, 20, 50, 0.45)`): a held primary button; 0.25 on the secondary.
- **Glyph ring** (`box-shadow: 0 0 0 1px <edge>`): the dark edge on mob dots and the orange dots. The alert dot adds a second white ring (`0 0 0 2px #fff`).
- **Lettering outline** (`--lettering`: eight 1px `text-shadow` offsets in outline, plus `0 2px 2px rgba(20, 30, 60, 0.35)`): Map Names only. The primary button's white label carries a 1px button-edge drop instead (`0 1px 0`).

### Named Rules
**The Chrome, Not Lift Rule.** Depth is drawn the way the game draws it, with highlights and hairlines. Nothing floats on a blurred drop shadow.

## Shapes

Small, game-window corners: 6px on windows (the panel, the collapsed icon, the mark icon, the title-marking hint, the recognised test read), 4px on notices (problem, edit mode) and buttons, 3px on chips, tags, key caps and candidate rows. Borders are 1px solid in a darker tone of the fill. Dashed lines mean editing or unknown: the 2px dashed Edit Blue outline in edit mode, the dashed edit-mode notice, the dashed drag box while marking the title, and the dashed frame of a mini-minimap whose portal cell is unknown. Circles are reserved for minimap glyphs.

## Components

### Title Bar
The minimap's title bar: the shared `--bar` gradient (bar-top 0%, bar-mid 48%, bar-base 100%) with a white top highlight and a Bar Hairline rule underneath. In the overlay it holds the Street Name over the outlined Map Name, with the doubt chip at the right end when the map isn't confirmed. In setup it runs the full window width and holds the 28px mark icon, the window name (14px bold, Street Navy) and the step count (12px bold, Street Navy) at the right.

### Mark Icon
The collapsed icon in small: a 28px window square in `--bar`, 6px radius, Frame Navy border and a 0.7 inner highlight, holding a 20px portal ring (outer ring in portal over portal-fill, inner ring in portal-inner). It is setup's only icon.

### Doubt Chip ("Not sure")
The game's small orange button, used once. It has a gradient from #ffc469 to orange, a 1px orange-deep border, a 3px radius, and white uppercase 11px bold text outlined in orange-deep. It is static and never clickable.

### Buttons (interactive windows only)
The game's window buttons, solid and pressable. They exist only where the window takes the mouse and keyboard (setup).
- **Shape:** 4px radius, 1px border, padding 6px 16px 7px, 13px/18px bold sentence-case labels.
- **Primary:** the Button Steel gradient, edged in button-edge, white label with a 1px button-edge drop and a 0.3 white top highlight. One per step: the step forward (Next, I understand, continue, Mark the title, Finish), always the rightmost.
- **Secondary:** the Light Steel gradient, edged in Frame Navy, Ink label with no text shadow and a solid white top highlight. Back, Quit, Pick by hand, Mark again.
- **Hover:** the gradient shifts one step (primary darker, secondary lighter). No movement.
- **Pressed:** the hover gradient (primary) or rest gradient (secondary) with the inset Pressed shadow.
- **Focus:** a 2px solid Edit Blue outline, offset 2px, on keyboard focus only (`:focus-visible`).
- **Disabled:** greyed and flat: disabled-bg, disabled-edge border, disabled-ink label, no highlight or text shadow. Finish stays disabled until a map is recognised.

### Section Heading with Legend Mark
A 13px bold heading led by a CSS minimap glyph: a 10px ring (2.5px portal border on portal-fill) for Hidden Portals, or an 8px red dot with a mob-edge ring for Hunting.

### Portal Row (signature)
Each Hidden Portal is drawn on a miniature minimap: a 47x29px SVG of the minimap's 5x3 grid (9-unit cells). It is drawn in this order: the Minimap Paper field, the portal's cell lit in minimap-cell, the grid lines, then the unfilled minimap-frame border on top so no edge cell covers it, and last the game's hollow ring (portal stroke, 1.7 wide) with a solid portal core. When the spot is unknown, the frame is dashed and no cell is lit. Beside it: the destination (13px bold, truncating), any tags, and the position in words (12px Ink Steel, first letter capitalised).

### Tags
Tags are 11px bold with a 3px radius and a 1px border. "Hidden Street" is in Tag Periwinkle and "Quest" in NPC Green. Padding follows the host line: 0 5px at a 15px line in the overlay, 0 6px at 16px in setup's "Recognised" tag. The Esc key cap in the title-marking hint is the same vocabulary at 12px/18px.

### Candidate Rows
When several maps share a title, each row leads with what sets the map apart ("1 hidden portal · Lv 2–8 · Blue Snail…") followed by its map ID in Ink Slate. The shown map's row uses Selection Periwinkle with an inset 1px select-edge ring and bold ink text.

### Notices
- **Note:** Ink Steel text that wraps, unboxed: 13px/17px in the overlay, 13px/18px in setup.
- **Problem note:** Problem Cream, a 1px Problem Amber border, a 4px radius, Problem Brown text, and an 8px orange dot at the left. The state is carried by the dot, the box and the words, not by colour alone. In setup the same note is padded 7px 12px 8px 28px with the dot 11px in.
- **Edit-mode notice:** mode-bg with a 1px dashed Edit Blue border, 13px bold mode-ink text.

### Test Read (setup)
One box with three states. Reading: a plain unboxed note. Nothing recognised: the problem note (padded 9px 12px 10px 28px). Recognised: the map exactly as the overlay's title bar will show it, as a 6px-radius `--bar` box with Frame Navy border and top highlight, the Street Name over the outlined Map Name, and a "Recognised" tag at the right in sentence case.

### Summary and Mob Table
The summary is a five-column grid: label, value, unit, value, unit. Values are 13px bold ink, right-aligned with a 52px minimum width. The mob table runs full width at 13px/17px with right-aligned numeric columns and 12px Ink Steel headers. Each mob name is bold, carries a 6px red mob dot, and truncates at 160px.

### Collapsed Icon
A 44px window square in the shared `--bar` gradient, 6px radius, with a 30px portal ring (an outer ring in portal over portal-fill and an inner ring in portal-inner), the Hidden Portal count in 14px bold, and a 9px orange alert dot top-right when something needs a look.

### Unconfirmed State
While the map shown isn't confirmed, the facts (portals and hunting) visibly recede behind the doubt chip and the candidate list, so the panel states its doubt before anything else.

### Title-Marking Window
A transparent window over the game client, its whole surface the Navy Veil with a crosshair cursor.
- **Hint:** a small game window: translucent Window Lavender, 1px Frame Navy border, 6px radius, the window inner highlight, padding 9px 14px 10px. One line of 14px bold Ink ("Drag a box around the two lines of the minimap title"), then the Esc key cap and "to cancel" in regular Ink Steel. It ignores the pointer.
- **Drag box:** a 2px dashed Edit Blue outline laid over a 2px solid white band (95%), so it shows on the minimap's light blue and on dark scenery alike. Clear inside, so the title it frames reads at full strength.

## Do's and Don'ts

### Do:
- **Do** build new states and windows from the game's window materials: the Window Lavender body, the Title Bar Steel gradient with its white highlight and hairline, and navy ink.
- **Do** use the translucent body for windows over the game and the opaque body for windows on the desktop.
- **Do** reserve game orange for doubt and problems, and pair it with words and a shape (chip, dot, bordered note) so the state never relies on colour alone.
- **Do** draw icons as minimap glyphs (portal ring, mob dot, mini-minimap) in CSS or inline SVG.
- **Do** keep overlay figures tabular and numeric columns right-aligned.
- **Do** let notes and the Map Name wrap, and truncate single-line facts with an ellipsis.
- **Do** keep the Street Name above the Map Name wherever a map is shown as the overlay shows it.
- **Do** give every button in an interactive window hover, pressed, disabled and a 2px Edit Blue `:focus-visible` ring offset 2px, with one deep-steel primary per step.

### Don't:
- **Don't** return to the dark dev-tools card with a stats table on top.
- **Don't** use orange, or the outlined white lettering, for anything other than their single roles.
- **Don't** add small labels or kickers above content beyond the Street Name in the title bar.
- **Don't** add hover or focus states to the click-through overlay: it is read in glances. Don't add motion to any window.
- **Don't** float any window on a blurred drop shadow; depth is inset highlights and hairlines.
- **Don't** introduce a second typeface or icon fonts.
