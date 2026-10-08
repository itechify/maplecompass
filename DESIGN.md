---
name: MapleCompass
description: The in-game overlay panel that names the map, shows its Hidden Portals, judges its mobs, tracks EXP and leads to a Hunting Target, with its setup and title-marking windows, all drawn in MapleStory's own window language.
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
  rule: "rgba(62, 82, 124, 0.2)"
  inset-bg: "rgba(255, 255, 255, 0.55)"
  inset-edge: "rgba(62, 82, 124, 0.3)"
  portal: "#1f63d6"
  portal-fill: "#eef5ff"
  portal-inner: "#8db3ee"
  mob: "#d8342a"
  mob-edge: "#8e1c15"
  you: "#ffd21f"
  you-light: "#fff6c2"
  you-edge: "#7a5a00"
  select: "#d3def3"
  select-edge: "#98afda"
  orange: "#f08a1c"
  orange-deep: "#b85e00"
  problem-bg: "#fff1dc"
  problem-edge: "#e5a04f"
  problem-ink: "#6b3500"
  edit: "#2f63c8"
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
  tag-paused:
    backgroundColor: "{colors.problem-bg}"
    textColor: "{colors.problem-ink}"
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
  mini-minimap:
    backgroundColor: "{colors.minimap-paper}"
    width: "47px"
    height: "29px"
  icon-button:
    backgroundColor: "{colors.button-base}"
    textColor: "#ffffff"
    rounded: "{rounded.chip}"
    size: "22px"
  icon-button-light:
    backgroundColor: "{colors.button-light-base}"
    textColor: "{colors.ink}"
    rounded: "{rounded.chip}"
    size: "22px"
  search-input:
    backgroundColor: "#ffffff"
    textColor: "{colors.ink}"
    rounded: "{rounded.notice}"
    padding: "6px 10px"
  inset-box:
    backgroundColor: "{colors.inset-bg}"
    textColor: "{colors.ink}"
    typography: "{typography.body-small}"
    rounded: "{rounded.notice}"
    padding: "4px 10px"
  suggestion-row-current:
    backgroundColor: "{colors.select}"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.chip}"
    padding: "2px 8px"
  collapsed-icon:
    backgroundColor: "{colors.bar-mid}"
    textColor: "{colors.ink}"
    rounded: "{rounded.window}"
    size: "44px"
  level-badge:
    backgroundColor: "{colors.ink}"
    textColor: "#ffffff"
    rounded: "{rounded.chip}"
    padding: "0 2px"
    height: "12px"
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
  button-guide:
    backgroundColor: "{colors.button-light-base}"
    textColor: "{colors.ink}"
    rounded: "{rounded.notice}"
    padding: "3px 9px 4px"
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
---

# Design System: MapleCompass

## Overview

**Creative North Star: "The Window the Game Forgot to Ship"**

The overlay sits beside the game's own windows (the Quest Helper, the minimap) and is made from their materials: a translucent lavender-grey body, a steel-periwinkle title bar with a white top highlight and a dark-navy hairline, the map name in white bold lettering outlined in navy the way the minimap draws it, the minimap's legend of blue portal rings and red mob dots, and the game's small steel window buttons. The game supplies the materials and one signature move. Organisation, hierarchy and density come from a working tool: EXP pace first when it is tracked, then hidden exits, then the mobs, then the way to the Hunting Target, then the credit. It is not a screenshot of a game window. The setup window and the title-marking window are built from the same materials, so every MapleCompass window reads as one more game window.

It is light because the game's windows are light, and dense because it is read in glances between attacks. Every state is drawn in the same window. Doubt and problems get the game's orange, and nothing else does. The confirmed rejection is the generic dark dev-tools card with a stats table on top, which this world replaced.

Every window takes the pointer. The overlay is always clickable, but a click never takes the keyboard from the game except in the search view, where the query field needs it. The panel is dragged by its title bar when it sits inside the game or is detached. Its buttons (deep steel in the title bar, light steel in the body) and setup's buttons carry the game's hover, pressed and keyboard-focus states; list rows in the search view take a faint hover tint. Its height follows its content. Nothing in any window moves: there is no transition or animation anywhere.

**Key Characteristics:**
- Light window body in the game's lavender (translucent over the game, opaque on the desktop), under a gradient steel-blue title bar.
- Map name in outlined white lettering, with the Street Name above it as on the minimap.
- Two icon vocabularies only: minimap glyphs (portal ring, mob dot, mini-minimap) in CSS and SVG, and drawn line icons on the game's small steel buttons.
- Game orange appears only for doubt and problems.
- One plain bold sans (Arial) throughout; tabular figures in the overlay.
- Buttons in every window have the game's hover, pressed and focus states. No motion anywhere.

## Colors

Cool lavender and periwinkle neutrals with navy ink, two minimap accents (portal blue, mob red), a reserved game orange, and a deep steel for buttons that act.

### Primary
- **Portal Ring Blue** (portal): the minimap's portal ring. Section mark for Hidden Portals, the ring and core in each mini-minimap, and the ring in the collapsed icon and in setup's mark icon, always over Portal Ring Fill (portal-fill) with Portal Inner Ring (portal-inner) as the icon's second ring.

### Secondary
- **Game Orange** (orange, with orange-deep for edges and text outlines): the game's small orange button. Used for the "Not sure" chip, the dot in problem notes (overlay and setup), and the collapsed icon's alert dot. Problem notes sit on Problem Cream (problem-bg), edged in Problem Amber (problem-edge), with Problem Brown text (problem-ink). The "paused" tag in EXP tracking borrows that cream, amber and brown, without the orange, because held figures are a state to notice.

### Tertiary
- **Mob Dot Red** (mob, edged by mob-edge): the minimap's mob dot. Section mark for Mobs and the bullet on each mob row.
- **You Yellow** (you, lit by you-light, edged by you-edge): the minimap's dot for the player's own character. It marks only the Hunting Target's arrival, "You're here", as a 10px dot with a you-light highlight up and to the left. Kept clear of Game Orange: a pale yellow with a dark gold edge, never a chip or a note.

### Neutral
- **Window Lavender** (body): the translucent body of every window laid over the game (the overlay panel and the title-marking hint), at 95% opacity so it reads like the game's own and stays legible over busy scenes.
- **Window Lavender, Opaque** (body-solid): the same lavender with no translucency, for windows that sit on the desktop rather than over the game (the setup window), and behind the sticky header of the search results so rows don't show through it.
- **Title Bar Steel** (bar-top, bar-mid, bar-base): the three stops of the vertical title-bar gradient. In code the gradient is one custom property, `--bar` (`linear-gradient(180deg, #dfe8f7 0%, #b4c6e6 48%, #94abd6 100%)`), shared by the overlay and search title bars, the collapsed icon, setup's title bar and mark icon, and setup's recognised test reads. Reuse `--bar` rather than restating the stops.
- **Frame Navy** (frame) and **Bar Hairline** (bar-hairline): the 1px window border and the rule under the title bar. Frame Navy also edges the secondary and light icon buttons and the search field.
- **Window Rule** (rule): Frame Navy at 20%, the hairline between parts of a window: above the overlay's Hunting Target and footer, and above setup's button row.
- **Inset** (inset-bg, inset-edge): white at 55% edged in Frame Navy at 30%, for a list set into the body: the Hunting Target's route and the search suggestions.
- **Street Navy** (street): the Street Name, the search view's title, and the setup title bar's window name and step count.
- **Lettering Outline** (outline): the navy outline around the white map name.
- **Ink** (ink): primary text, values, mob names, destinations, setup headings and body; also the ground of the Time to Level badge.
- **Ink Steel** (ink-2): secondary text such as notes, positions, units, labels and table headers; setup's muted lines and neutral notices.
- **Ink Slate** (ink-3): tertiary text such as the footer, candidate map IDs, key hints, the crawl note and placeholders; list markers in setup.
- **Selection Periwinkle** (select, edged by select-edge): the selected row wherever there is one: the shown candidate, the suggestion Enter picks, the result Enter makes the Hunting Target. select-edge also colours every scrollbar thumb.
- **Tag Periwinkle** (tag-bg, tag-edge, tag-ink): the "Hidden Street" and "warming up" tags, and setup's "Recognised" and "Read" tags. **NPC Green** (quest-bg, quest-edge, quest-ink): the "Quest" tag, taken from the minimap's NPC green.
- **Edit Blue** (edit): the 2px keyboard-focus ring in every window, the dashed drag box when marking, the checked travel checkboxes (`accent-color`), and, at 8%, the hover tint on suggestion and result rows.
- **Minimap Paper** (minimap-paper, minimap-frame, minimap-grid, minimap-cell): the mini-minimap's ground, frame, 5x3 grid lines and lit cell.
- **Button Steel** (button-top, button-base, edged by button-edge): the deep-steel gradient of setup's primary button and the overlay's title-bar icon buttons, deeper than the title bar so white holds 4.5:1 even at its light top. **Light Steel** (button-light-top, button-light-base): the secondary button's and the body icon buttons' gradient. Hover gradients sit one step darker (deep) or lighter (light).
- **Greyed** (disabled-bg, disabled-edge, disabled-ink): a button the window won't take, flat with no highlight.
- **Navy Veil** (veil): the title-marking window's whole surface, so the game reads as paused behind it. It is non-zero on purpose: a fully transparent pixel would not take the mouse.

### Named Rules
**The Orange Means Doubt Rule.** Game orange marks only uncertainty and problems: the "Not sure" chip, problem notes, the collapsed alert dot. A confirmed state never uses it, so orange can't become decoration. Search tiers are told apart by weight and ink, never by orange.

**The Minimap Legend Rule.** Portal blue, mob red and you yellow come from the minimap's legend and keep that meaning. Blue is for portals, red is for mobs and yellow is for where the player is, with no other uses.

**The Over the Game, Translucent Rule.** A window laid over the game uses the translucent Window Lavender (body); a window on the desktop uses the opaque one (body-solid). Same colour, different footing.

## Typography

**Display Font:** Arial (with "Segoe UI", system-ui, sans-serif), as the shared `--font`
**Body Font:** the same family
**Label/Mono Font:** none distinct; overlay figures use tabular numerals

**Character:** The game sets its windows in a plain bold sans, so every window uses one face everywhere. Arial is a pinned exception to the usual system-face caution, because it is the game's own idiom. Weight (700 against 400) and colour carry the hierarchy, not a second family.

### Hierarchy
- **Headline** (700, 18px/23px, 0.01em): the Map Name, in white with a navy outline, in the overlay and in setup's recognised test read. Wraps to at most two lines rather than losing its end. When no map is read, the status takes this slot at 15px in plain ink with no outline, so it doesn't read as a map name. The Time to Level figure and setup's step headings use 18px bold in plain ink, without the outline. Setup's EXP test read sets its figures at 16px/22px bold.
- **Street** (700, 12px/15px): the Street Name above the Map Name, the minimap's own idiom; also setup's step count in the title bar.
- **Title** (700, 13px/17px): section headings (Hidden Portals, Mobs, the Hunting Target's mob), portal destinations, summary and rate values, and the Hunting Target's figures. The search view's title and setup's window name are the same weight at 14px in Street Navy; setup's "While you play" subheading is 14px in ink.
- **Body Reading** (400, 14px/20px): setup's prose, the search field's text, and the title-marking hint's heading (at 700). Setup is read at rest, not in glances, so it reads one step larger than the overlay.
- **Body** (400, 13px/17px): overlay notes, the mob and rates tables, the suggestions. Mob names are 700. Setup's notices, test read and shortcuts use 13px at an 18px line; the title-marking hint's instructions are 13px.
- **Body Small** (400, 12px/15–17px): positions, labels, summary units, table headers, figure labels, the route, candidate rows, key hints, the crawl note, the footer, setup's MeowDB credit, the title-marking status line.
- **Button** (700, 13px/18px): setup's button labels, sentence case. The Overlay guide button in setup's title bar is 12px.
- **Label** (700, 11px, 0.04em; uppercase only on the doubt chip): tags and the "Not sure" chip. Candidate map IDs are 11px regular.

The Time to Level badge on the collapsed icon is 9px/12px bold white, the system's one size below the Label step, held to that because it shares a 44px square with the portal ring. It is an exception, not a scale step.

### Named Rules
**The Tabular Figures Rule.** In the overlay all figures are tabular (`font-variant-numeric: tabular-nums` on the body), and numeric columns are right-aligned so EXP, HP, rates and share line up from one glance to the next.

**The Outlined Lettering Rule.** Only a Map Name uses white lettering with a navy outline (the shared `--lettering` text-shadow: a 1px stroke in eight directions plus one soft 0 2px 2px drop). It appears in the overlay title bar and in setup's recognised test read, nowhere else. Everything else, Time to Level included, is ink on the lavender body.

**The Notes Wrap, Names Truncate Rule.** Notes and the Map Name wrap so they can be read in full. Single-line facts (destinations, mob names, route stops, suggestion names, result maps) truncate with an ellipsis instead of growing the panel.

## Layout

A single 380px-wide column whose height fits its content (44–600px, or less where the panel is placed). The overlay reports its height and the window resizes to match; past that, the long parts (the route, the suggestions, the map list) scroll on Selection Edge scrollbars while the rest keeps its height. The title bar runs full width (padding 5px 8px 7px 11px): a top row at least 22px tall with the Street Name and, at the right, the toolbar (3px between buttons), then the Map Name with the doubt chip at its right. The body below it is a vertical stack with 11px side insets: 9px between top-level items, 10px between sections, 6px between portal rows and 2px between candidate rows. The rhythm is tuned pixel by pixel to the game's windows, not taken from a modular scale.

Order is fixed: title bar, then any recovery or hotkey notes, then EXP tracking when on (Time to Level, a rates table of Rolling 5 min and Session, an extra line), then candidates when several maps share a title, then the status note, then Hidden Portals, then Mobs (a two-row summary grid of EXP/h and Mobs/h, solo and 6+ party, then the top five mobs in a fixed-layout table and a "+N more" line), then the Hunting Target below a Window Rule, then the crawl note, then a footer with the credit below a Window Rule. A portal row is a two-column grid: the 47px mini-minimap, then the destination with its tags over the position in words. When a section has nothing to list, its heading and its "None on this map" line share one line. Empty containers collapse rather than leave strips.

The search view replaces the whole panel: its own title bar (padding 6px 11px 7px 8px, back button then title), then a 6px-gapped step: the query field, suggestions and the key hint; or, once a mob is picked, the nearest map, the results table, the travel checkboxes and the key hint; then the crawl note and credit.

Collapsed, the window becomes a 44px square holding the portal ring, the Hidden Portal count and an optional alert dot, with the Time to Level badge along its bottom edge when EXP is tracked. The width is fixed; one container query (`max-width: 300px`, for raised text scale) folds the summary to three columns and turns mob rows into labelled cards.

The setup window is a fixed 520×560 opaque window: a full-width title bar (padding 9px 20px 10px, 10px gaps, wrapping) holding the mark icon, the window name, the step count ("Step N of 7" on first run, "of 6" when setup reopens, "Guide" in the overlay guide) and the Overlay guide button at the right, then a column with 22px side insets and 18px bottom padding that scrolls if it must. Each step is a heading (18px above, 8px below), prose in 10px-spaced paragraphs, and a footer pinned to the bottom of the window above a Window Rule, buttons right-aligned with 10px between them, the step forward always last. Notices sit above the step, 14px below the title bar and 8px apart.

The title-marking window covers the game client exactly. Its hint is centred horizontally, 12% up from the bottom, or 12% down from the top while marking the EXP bar so it never covers what is being marked; it is as wide as its longest line, up to the window less 64px.

## Elevation & Depth

No window has a drop shadow. Depth comes from the game's window chrome: inset white highlights (a 1px inner ring on the body, a 1px top highlight on the title bar, the doubt chip and the buttons), 1px ring strokes around glyphs (`0 0 0 1px`), and the outlined lettering of the Map Name. The panel sits flat on top of the game scene, and its translucency is what joins it to the game's windows. A pressed button sinks with an inset shadow rather than lifting; the search field is the one surface set into the body, with a soft inner shadow. While the map shown isn't confirmed, the facts recede to 55% opacity rather than moving.

### Shadow Vocabulary
- **Window inner highlight** (`box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.65)`): just inside the frame of the panel and the title-marking hint. The collapsed icon and setup's mark icon use 0.7.
- **Bar top highlight** (`box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.9)`): the top edge of every title bar and the recognised test read. The doubt chip uses the same at 0.55, deep-steel buttons (primary and icon) at 0.3, and light-steel buttons a solid `#fff`.
- **Pressed** (`box-shadow: inset 0 1px 3px rgba(10, 20, 50, 0.45)`): a held deep-steel button; 0.25 on light steel.
- **Field inset** (`box-shadow: inset 0 1px 2px rgba(20, 30, 60, 0.15)`): the search field only.
- **Glyph ring** (`box-shadow: 0 0 0 1px <edge>`): the dark edge on mob dots and the orange dots. The alert dot adds a second white ring (`0 0 0 2px #fff`).
- **Lettering outline** (`--lettering`: eight 1px `text-shadow` offsets in outline, plus `0 2px 2px rgba(20, 30, 60, 0.35)`): Map Names only. The primary button's white label carries a 1px button-edge drop instead (`0 1px 0`).

### Named Rules
**The Chrome, Not Lift Rule.** Depth is drawn the way the game draws it, with highlights and hairlines. Nothing floats on a blurred drop shadow.

## Shapes

Small, game-window corners: 6px on windows (the panel, the collapsed icon, the mark icon, the title-marking hint, the recognised test read), 4px on notices, buttons, the search field and inset boxes, 3px on icon buttons, chips, tags, the badge and selectable rows. Borders are 1px solid in a darker tone of the fill. Dashed lines mean a place not yet fixed: the dashed drag box while marking, and the dashed frame of a mini-minimap whose portal cell is unknown. Circles are reserved for minimap glyphs.

## Components

### Title Bar
The minimap's title bar: the shared `--bar` gradient (bar-top 0%, bar-mid 48%, bar-base 100%) with a white top highlight and a Bar Hairline rule underneath. In the overlay its top row holds the Street Name and the toolbar, and the outlined Map Name runs beneath with the doubt chip at its right end when the map isn't confirmed. It is the drag handle: inside the game or detached, it shows a move cursor and drags the panel, except over its buttons. The search view's title bar is one row: the back button, then the mob's name (14px bold, Street Navy) and its level in regular Ink Steel. In setup it runs the full window width and holds the 28px mark icon, the window name (14px bold, Street Navy), the step count (12px bold, Street Navy) and the Overlay guide button.

### Toolbar and Icon Buttons
The game's small window buttons, one per hotkey action, drawn at the right of the title bar's top row: search, next candidate, MeowDB page, reset Session, placement, help and setup, collapse. Buttons whose action isn't available are hidden, not greyed.
- **Shape:** 22px square, 3px radius, 1px border, no padding.
- **Icon:** a 14px inline SVG on a 16-unit grid, stroked in `currentColor` at 1.8 with round caps and joins, no fill (a single filled dot where a glyph needs one).
- **Deep steel** (title bars): the Button Steel gradient, button-edge border, white icon, 0.3 top highlight. The search view's back button is the same.
- **Light steel** (body): the Light Steel gradient, Frame Navy border, ink icon, solid white top highlight. The Hunting Target's reopen-search and clear buttons.
- **States:** hover shifts the gradient one step; pressed adds the Pressed shadow; keyboard focus draws the 2px Edit Blue ring offset 2px. The cursor is a pointer while the panel is draggable.

### Mark Icon
The collapsed icon in small: a 28px window square in `--bar`, 6px radius, Frame Navy border and a 0.7 inner highlight, holding a 20px portal ring (outer ring in portal over portal-fill, inner ring in portal-inner). It is setup's only icon.

### Doubt Chip ("Not sure")
The game's small orange button, used once. It has a gradient from #ffc469 to orange, a 1px orange-deep border, a 3px radius, and white uppercase 11px bold text outlined in orange-deep. It shows while the map is unconfirmed and while several maps share the title and none is chosen. It is static and never clickable.

### Buttons
The game's window buttons, solid and pressable, in setup.
- **Shape:** 4px radius, 1px border, padding 6px 16px 7px, 13px/18px bold sentence-case labels.
- **Primary:** the Button Steel gradient, edged in button-edge, white label with a 1px button-edge drop and a 0.3 white top highlight. One per step: the step forward (Next, I understand, continue, Mark the title, Mark the EXP bar, Finish, Back to setup), always the rightmost.
- **Secondary:** the Light Steel gradient, edged in Frame Navy, Ink label with no text shadow and a solid white top highlight. Back, Quit, Skip, Pick by hand, Mark again, Keep the current title, Turn off EXP tracking.
- **Guide:** the secondary button at 12px with padding 3px 9px 4px, at the right end of setup's title bar.
- **Hover:** the gradient shifts one step (primary darker, secondary lighter). No movement.
- **Pressed:** the hover gradient (primary) or rest gradient (secondary) with the inset Pressed shadow.
- **Focus:** a 2px solid Edit Blue outline, offset 2px, on keyboard focus only (`:focus-visible`).
- **Disabled:** greyed and flat: disabled-bg, disabled-edge border, disabled-ink label, no highlight or text shadow. Next on the test read and Finish stay disabled until a read succeeds.

### Section Heading with Legend Mark
A 13px bold heading led by a CSS minimap glyph: a 10px ring (2.5px portal border on portal-fill) for Hidden Portals, or an 8px red dot with a mob-edge ring for Mobs.

### Portal Row (signature)
Each Hidden Portal is drawn on a miniature minimap: a 47x29px SVG of the minimap's 5x3 grid (9-unit cells). It is drawn in this order: the Minimap Paper field, the portal's cell lit in minimap-cell, the grid lines, then the unfilled minimap-frame border on top so no edge cell covers it, and last the game's hollow ring (portal stroke, 1.7 wide) with a solid portal core. When the spot is unknown, the frame is dashed and no cell is lit. Beside it: the destination (13px bold, truncating), any tags, and the position in words (12px Ink Steel, first letter capitalised).

### Tags
Tags are 11px bold with a 3px radius and a 1px border. "Hidden Street" and "warming up" are in Tag Periwinkle, "Quest" in NPC Green, and "paused" in the problem palette. Padding follows the host line: 0 5px at a 15px line in the overlay, 0 6px at 16px in setup's "Recognised" and "Read" tags.

### Candidate Rows
When several maps share a title, each row leads with what sets the map apart ("1 hidden portal · Lv 2–8 · Blue Snail…") followed by its map ID in Ink Slate. The shown map's row uses Selection Periwinkle with an inset 1px select-edge ring and bold ink text.

### Notices
- **Note:** Ink Steel text that wraps, unboxed: 13px/17px in the overlay, 13px/18px in setup.
- **Crawl note:** "Mapping the world: N of M maps known" while the world list loads, 12px Ink Slate, quieter than a note.
- **Problem note:** Problem Cream, a 1px Problem Amber border, a 4px radius, Problem Brown text, and an 8px orange dot at the left. The state is carried by the dot, the box and the words, not by colour alone. In setup the same note is padded 7px 12px 8px 28px with the dot 11px in, for hotkey problems and failed actions alike.

### EXP Tracking
A quiet "waiting for EXP" note until a Session starts. Then a headline row: "Time to Level" (12px Ink Steel), the figure (18px bold ink) and an optional tag; a rates table with bold values at least 72px wide under Rolling 5 min and Session; and a 12px Ink Steel line for the ceiling share and deaths. While warming up or paused, the figures fall to Ink Steel. "—" stands for any figure not there yet.

### Summary and Mob Table
The summary is a five-column grid: label, value, unit, value, unit. Values are 13px bold ink, right-aligned with a 52px minimum width. The mob table runs full width with fixed column shares at 13px/17px, right-aligned numeric columns, 6px between columns and 12px Ink Steel headers. Each mob name is bold, carries a 6px red mob dot, and truncates. "+N more" follows in 12px Ink Steel.

### Hunting Target
Its own section at the bottom of the normal view, 8px below a Window Rule. A heading of two truncating lines (the mob in 13px bold, the map in regular Ink Steel) with the two light icon buttons at its right; a four-column figures grid (Tier, Density, Share, Distance: 12px Ink Steel labels over 13px values, bold except the tier); a 12px bold route title; and the route in an inset box (padding 4px 10px, 4px radius, 12px), each stop a right-aligned Ink Steel step beside the map with its street beneath. A step that isn't a portal (ship, taxi, teleport scroll) is set in bold ink, by emphasis alone. A Distance of 0 reads "here".

**Arrival.** Once the player reaches the target's map from another, the route has done its job and its inset box resolves to its last stop: the same inset box (padding 5px 10px 6px), the You Yellow dot, "You're here" in 13px bold ink, and beneath it the trip as it stood when the target was picked ("3 map changes from Construction Site North of Kerning City, by taxi") in 12px Ink Steel, wrapping. It is a status, static like everything else, and stays until the player leaves the map or presses ×; a target picked on the map the player is on shows no arrival until they leave and come back.

### Search View
- **Field:** a 12px Ink Steel label over the app's one text input: white, 1px Frame Navy border, 4px radius, padding 6px 10px, 14px ink, Ink Slate placeholder, the Field inset shadow, and the shared focus ring.
- **Suggestions:** an inset box (padding 3px, 13px) of rows (padding 2px 8px, 3px radius) in Ink Steel, name truncating and level at the right; the row Enter picks wears Selection Periwinkle with its inset ring and bold ink.
- **Results:** a fixed-layout table of map (bold when current) over street, density, share, tier and distance. The header sticks to the top on opaque lavender. The current row is Selection Periwinkle. Tiers read by weight and ink (high bold, medium Ink Steel, low Ink Slate); a missing distance is Ink Slate. The list takes the arrow keys, and its selection, not a focus ring, shows where they act.
- **Row hover:** suggestion and result rows take Edit Blue at 8% under the pointer.
- **Travel:** an Ink Steel legend and checkboxes in Edit Blue (`accent-color`), 12px.
- **Key hint:** one 12px Ink Slate line naming the keys.

### Collapsed Icon
A 44px window square in the shared `--bar` gradient, 6px radius, with a 30px portal ring (an outer ring in portal over portal-fill and an inner ring in portal-inner), the Hidden Portal count in 14px bold, and a 9px orange alert dot top-right when something needs a look. With EXP tracked, the Time to Level badge runs along its bottom edge (2px in from each side, 3px radius, Ink ground, white 9px/12px bold, "47m"), and the ring shrinks to 24px and the count to 12px to make room. Clicks and drags pass through the badge to the icon.

### Unconfirmed State
While the map shown isn't confirmed, or several maps share its title and none is chosen, the facts (portals and mobs) recede to 55% behind the doubt chip and the candidate list, so the panel states its doubt before anything else.

### Test Read (setup)
One box with three states. Reading: a plain unboxed note. Nothing read: the problem note (padded 9px 12px 10px 28px). Read: a 6px-radius `--bar` box with Frame Navy border and top highlight and a tag at the right in sentence case. For the title, it shows the map exactly as the overlay's title bar will, the Street Name over the outlined Map Name, tagged "Recognised"; for the EXP bar, the figures as the bar writes them ("EXP 160 [3.81%]", 16px bold ink), tagged "Read".

### Shortcuts List (setup)
A two-column definition grid at 13px: the action in Ink Steel, its keys right-aligned in bold ink (gap 6px 16px). A compact variant (gap 2px 12px) sits under "While you play" on the title test step; the full list is the overlay guide.

### Title-Marking Window
A transparent window over the game client, its whole surface the Navy Veil with a crosshair cursor.
- **Hint:** a small game window: translucent Window Lavender, 1px Frame Navy border, 6px radius, the window inner highlight, padding 9px 14px 10px, text centred. A 14px bold Ink heading ("Mark both lines of the minimap title"), two 13px Ink Steel lines of keyboard instructions, and a 12px Ink Slate status line. It ignores the pointer.
- **Drag box:** a 2px dashed Edit Blue outline laid over a 2px solid white band (95%), so it shows on the minimap's light blue and on dark scenery alike. Clear inside, so what it frames reads at full strength.

## Do's and Don'ts

### Do:
- **Do** build new states and windows from the game's window materials: the Window Lavender body, the Title Bar Steel gradient with its white highlight and hairline, and navy ink.
- **Do** use the translucent body for windows over the game and the opaque body for windows on the desktop.
- **Do** reserve game orange for doubt and problems, and pair it with words and a shape (chip, dot, bordered note) so the state never relies on colour alone.
- **Do** draw icons as minimap glyphs (portal ring, mob dot, mini-minimap) in CSS or inline SVG, or as 1.8-stroke line icons on the game's small steel buttons.
- **Do** keep overlay figures tabular and numeric columns right-aligned.
- **Do** let notes and the Map Name wrap, and truncate single-line facts with an ellipsis.
- **Do** keep the Street Name above the Map Name wherever a map is shown as the overlay shows it.
- **Do** give every button, in every window, hover, pressed and a 2px Edit Blue `:focus-visible` ring offset 2px, deep steel where it acts from a title bar or moves setup forward and light steel beside it or in a body.
- **Do** mark a selected row with Selection Periwinkle and its inset edge, and set a list into the body as an inset box.

### Don't:
- **Don't** return to the dark dev-tools card with a stats table on top.
- **Don't** use orange, or the outlined white lettering, for anything other than their single roles.
- **Don't** add small labels or kickers above content beyond the Street Name in the title bar; a label belongs only to the field or figure it names.
- **Don't** let a click on the overlay take the keyboard from the game outside the search view.
- **Don't** add motion (transitions or animations) to any window.
- **Don't** float any window on a blurred drop shadow; depth is inset highlights and hairlines.
- **Don't** introduce a second typeface or icon fonts.
