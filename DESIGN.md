---
name: MapleCompass
description: The in-game overlay panel that names the map, shows its Hidden Portals, judges its mobs, tracks EXP and leads to a Hunting Target, with its setup and title-marking windows, all drawn in MapleStory's own window language.
colors:
  panel-ink: "#16191f"
  panel-ink-2: "#222938"
  panel-ink-3: "#343d51"
  glass: "rgba(178, 194, 232, 0.62)"
  window-edge: "#6f95ba"
  window-bar-top: "#ececec"
  window-bar-mid: "#dfdfdf"
  window-bar-base: "#d6d6d6"
  window-bar-ink: "#3d4048"
  title-panel: "#a4c8da"
  title-ink: "#eef3f7"
  title-outline: "#4f7499"
  game-button-top: "#27b5fe"
  game-button-mid: "#44c4ff"
  game-button-base: "#66d4ff"
  game-button-edge: "#2f7595"
  panel-rule: "rgba(30, 45, 90, 0.25)"
  inset-bg: "rgba(255, 255, 255, 0.5)"
  inset-edge: "rgba(30, 45, 90, 0.3)"
  results-head: "#c5d0ec"
  ink: "#1b2648"
  ink-2: "#44527a"
  ink-3: "#505c7d"
  body: "rgba(234, 232, 245, 0.95)"
  body-solid: "#eae8f5"
  frame: "#3e527c"
  rule: "rgba(62, 82, 124, 0.2)"
  portal: "#1f63d6"
  portal-fill: "#eef5ff"
  teleport: "#1e9e3a"
  unknown: "#7a8396"
  route: "#c8189a"
  mob: "#d8342a"
  mob-edge: "#8e1c15"
  you: "#ffd21f"
  you-light: "#fff6c2"
  you-edge: "#7a5a00"
  select: "#d3def3"
  select-edge: "#98afda"
  orange: "#f08a1c"
  orange-deep: "#b85e00"
  doubt-top: "#ffd25a"
  doubt-base: "#f0a715"
  problem-bg: "#fff1dc"
  problem-edge: "#e5a04f"
  problem-ink: "#6b3500"
  edit: "#2f63c8"
  tag-bg: "#dae3f3"
  tag-edge: "#a3b6d8"
  tag-ink: "#284a86"
  quest-bg: "#ece4f7"
  quest-edge: "#b9a5dc"
  quest-ink: "#55308f"
  minimap-paper: "rgba(238, 243, 251, 0.45)"
  button-top: "#4673c6"
  button-base: "#2a52a6"
  button-edge: "#1c3672"
  button-light-top: "#f8fafe"
  button-light-base: "#d5dff0"
  disabled-bg: "#d3d9e5"
  disabled-edge: "#aeb8cc"
  disabled-ink: "#6b7591"
  veil: "rgba(18, 28, 56, 0.3)"
  exp-top: "#effb76"
  exp-mid: "#e5f916"
  exp-base: "#cfe400"
  exp-track: "#cfcfcf"
  exp-frame: "#676d70"
typography:
  headline:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 700
    lineHeight: "23px"
    letterSpacing: "0.01em"
  street:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 700
    lineHeight: "18px"
  section-heading:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 700
    lineHeight: "18px"
  status:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 700
    lineHeight: "23px"
  window-name:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 700
    lineHeight: "16px"
  title:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 700
    lineHeight: "17px"
  body-reading:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: "20px"
  body:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: "17px"
    fontFeature: "\"tnum\" 1"
  body-small:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: "15px"
  button:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 700
    lineHeight: "18px"
  label:
    fontFamily: "Maplestory, Arial, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 700
    lineHeight: "15px"
rounded:
  game-button: "2px"
  chip: "3px"
  window: "4px"
  hint: "6px"
  pill: "8px"
spacing:
  inset-x: "11px"
  section: "10px"
  stack: "9px"
  row: "6px"
  list: "2px"
  window-bottom: "16px"
components:
  panel:
    backgroundColor: "{colors.glass}"
    textColor: "{colors.panel-ink}"
    rounded: "{rounded.window}"
    width: "380px"
  window-bar:
    backgroundColor: "{colors.window-bar-mid}"
    textColor: "{colors.window-bar-ink}"
    typography: "{typography.window-name}"
    padding: "2px 3px 2px 5px"
    height: "22px"
  title-panel:
    backgroundColor: "{colors.title-panel}"
    textColor: "{colors.title-ink}"
    typography: "{typography.headline}"
    padding: "6px 10px 8px 11px"
  icon-button:
    backgroundColor: "{colors.game-button-mid}"
    textColor: "#ffffff"
    rounded: "{rounded.game-button}"
    size: "19px"
  icon-button-light:
    backgroundColor: "{colors.button-light-base}"
    textColor: "{colors.panel-ink}"
    rounded: "{rounded.game-button}"
    size: "19px"
  doubt-chip:
    backgroundColor: "{colors.doubt-base}"
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
    textColor: "{colors.panel-ink-2}"
    typography: "{typography.body-small}"
    rounded: "{rounded.chip}"
    padding: "3px 7px"
  candidate-row-current:
    backgroundColor: "{colors.select}"
    textColor: "{colors.panel-ink}"
    rounded: "{rounded.chip}"
    padding: "3px 7px"
  note-problem:
    backgroundColor: "{colors.problem-bg}"
    textColor: "{colors.problem-ink}"
    typography: "{typography.body}"
    rounded: "{rounded.window}"
    padding: "4px 8px 5px 24px"
  map-preview:
    backgroundColor: "{colors.minimap-paper}"
    textColor: "{colors.panel-ink-2}"
    typography: "{typography.body-small}"
    rounded: "{rounded.window}"
    padding: "5px"
  inset-box:
    backgroundColor: "{colors.inset-bg}"
    textColor: "{colors.panel-ink}"
    typography: "{typography.body-small}"
    rounded: "{rounded.window}"
    padding: "4px 10px"
  arrival:
    backgroundColor: "{colors.inset-bg}"
    textColor: "{colors.panel-ink}"
    typography: "{typography.title}"
    rounded: "{rounded.window}"
    padding: "5px 10px 6px"
  mob-card:
    backgroundColor: "{colors.body-solid}"
    textColor: "{colors.panel-ink}"
    typography: "{typography.body-small}"
    rounded: "{rounded.window}"
    padding: "6px 10px 8px 6px"
  mark-card:
    backgroundColor: "{colors.body-solid}"
    textColor: "{colors.panel-ink}"
    typography: "{typography.body-small}"
    rounded: "{rounded.window}"
    padding: "4px 8px 6px"
  search-input:
    backgroundColor: "#ffffff"
    textColor: "{colors.panel-ink}"
    rounded: "{rounded.window}"
    padding: "6px 10px"
  suggestion-row-current:
    backgroundColor: "{colors.select}"
    textColor: "{colors.panel-ink}"
    typography: "{typography.body}"
    rounded: "{rounded.chip}"
    padding: "2px 8px"
  collapsed-icon:
    size: "44px"
  count-chip:
    backgroundColor: "{colors.ink}"
    textColor: "#ffffff"
    rounded: "{rounded.pill}"
    padding: "0 3px"
    height: "13px"
  level-badge:
    backgroundColor: "{colors.ink}"
    textColor: "#ffffff"
    rounded: "{rounded.chip}"
    padding: "0 2px"
    height: "12px"
  level-badge-held:
    backgroundColor: "{colors.ink-2}"
    textColor: "#ffffff"
    rounded: "{rounded.chip}"
    padding: "0 2px"
    height: "12px"
  text-button:
    backgroundColor: "{colors.button-light-base}"
    textColor: "{colors.panel-ink}"
    rounded: "{rounded.game-button}"
    padding: "0 7px"
    height: "19px"
  text-button-disabled:
    backgroundColor: "{colors.disabled-bg}"
    textColor: "{colors.disabled-ink}"
    rounded: "{rounded.game-button}"
    padding: "0 7px"
    height: "19px"
  hotkey-row-waiting:
    backgroundColor: "{colors.select}"
    textColor: "{colors.panel-ink}"
    typography: "{typography.body}"
    rounded: "{rounded.chip}"
    padding: "2px 2px 4px 8px"
  keys-refused:
    backgroundColor: "{colors.problem-bg}"
    textColor: "{colors.problem-ink}"
    typography: "{typography.title}"
    rounded: "{rounded.chip}"
    padding: "0 5px"
  tracker-figures:
    backgroundColor: "{colors.body}"
    textColor: "{colors.panel-ink}"
    typography: "{typography.body-small}"
    rounded: "{rounded.window}"
    padding: "4px 8px 6px 9px"
  exp-gauge:
    backgroundColor: "{colors.exp-track}"
    textColor: "{colors.panel-ink}"
    rounded: "{rounded.game-button}"
    height: "10px"
  exp-gauge-fill:
    backgroundColor: "{colors.exp-mid}"
  setup-window:
    backgroundColor: "{colors.body-solid}"
    textColor: "{colors.ink}"
    typography: "{typography.body-reading}"
    width: "520px"
    height: "560px"
  setup-bar:
    backgroundColor: "{colors.window-bar-mid}"
    textColor: "{colors.window-bar-ink}"
    padding: "9px 20px 10px"
  mark-icon:
    size: "28px"
  button-primary:
    backgroundColor: "{colors.button-base}"
    textColor: "#ffffff"
    typography: "{typography.button}"
    rounded: "{rounded.window}"
    padding: "6px 16px 7px"
  button-secondary:
    backgroundColor: "{colors.button-light-base}"
    textColor: "{colors.ink}"
    typography: "{typography.button}"
    rounded: "{rounded.window}"
    padding: "6px 16px 7px"
  button-guide:
    backgroundColor: "{colors.button-light-base}"
    textColor: "{colors.ink}"
    rounded: "{rounded.window}"
    padding: "3px 9px 4px"
  button-disabled:
    backgroundColor: "{colors.disabled-bg}"
    textColor: "{colors.disabled-ink}"
    typography: "{typography.button}"
    rounded: "{rounded.window}"
    padding: "6px 16px 7px"
  test-read-recognised:
    backgroundColor: "{colors.title-panel}"
    textColor: "{colors.title-ink}"
    rounded: "{rounded.window}"
    padding: "8px 12px 9px"
  region-hint:
    backgroundColor: "{colors.body}"
    textColor: "{colors.ink}"
    rounded: "{rounded.hint}"
    padding: "9px 14px 10px"
---

# Design System: MapleCompass

## Overview

**Creative North Star: "The Window the Game Forgot to Ship"**

The overlay is one more of the game's windows laid over the screen, built from the two it sits beside. From the Mini Map it takes the light grey window bar with the window's name and small bright blue buttons, and the sky-blue title panel with the Street Name over the Map Name in light lettering outlined in blue. From the Quest Helper it takes the see-through periwinkle body with near-black text. The minimap's legend supplies the glyphs: blue portal rings, red mob dots, and the player's yellow dot. The pixel-art gold compass, the app icon, is the window's mark. The game supplies the materials and one signature move; organisation, hierarchy and density come from a working tool: EXP pace first when it is tracked, then the map and its hidden exits, then the way to the Hunting Target, then the mobs, then the credit. It is not a screenshot of a game window.

It is light because the game's windows are light, see-through because the game's windows over the screen are, and dense because it is read in glances between attacks. Every state is drawn in the same window. Doubt and problems get the game's orange, and nothing else does. The confirmed rejection is the generic dark dev-tools card with a stats table on top, which this world replaced. Setup is a window on the desktop rather than over the game: it keeps an opaque lavender body, navy ink and steel buttons under the same grey window bar, and shows a recognised map exactly as the overlay's title panel will. The title-marking window's hint is a small translucent lavender window.

Every window takes the pointer. The overlay is always clickable, but a click never takes the keyboard from the game except in the search view, where the query field needs it, and the settings view, where a hotkey's new keys are pressed. The panel is dragged by its window bar or title panel when it sits inside the game or is detached, and resized by its sides or bottom corners, width only. Its buttons and setup's buttons carry the game's hover, pressed and keyboard-focus states; list rows in the search view take a faint hover tint. Its height follows its content. There is no transition or animation anywhere: the Grind Tracker's EXP gauge changes with each reading, as the game's own does, without easing.

**Key Characteristics:**
- The Mini Map's light grey window bar and sky-blue title panel over the Quest Helper's see-through periwinkle body, in a 4px-cornered frame.
- Street Name over Map Name in light lettering outlined in blue, as the Mini Map draws its title.
- Three icon vocabularies only: minimap glyphs (portal ring, mob dot, you dot and the you yellow star) and the Map Preview's rings, route rings and teleport arrows in CSS and SVG, drawn line icons on the game's small blue buttons, and the pixel-art compass as the window's mark.
- Near-black ink with a faint light edge on the see-through body; navy ink on setup's opaque lavender.
- Game orange appears only for doubt and problems.
- NEXON's Maplestory typeface throughout, Light and Bold; numeric columns right-aligned.
- Buttons in every window have the game's hover, pressed and focus states. No motion anywhere.

## Colors

Cool periwinkles, sky blue and light grey from the game's own windows, near-black and navy inks, three minimap accents (portal blue, mob red, you yellow), two more ring colours for the Map Preview (teleport green, unknown grey), a route magenta for the way to the Hunting Target, a reserved game orange, the game's bright blue for overlay buttons, a deep steel for setup's, and the game's EXP lime for its EXP gauge alone.

### Primary
- **Portal Ring Blue** (portal): the minimap's portal ring. The section mark for the Map (over Portal Ring Fill, portal-fill), and the ring of every portal to another map, Doors included, on the Map Preview and on its Hidden Portal's row.

### Secondary
- **Game Orange** (orange, with orange-deep for edges and text outlines): the game's small orange button. The dot in problem notes (overlay, setup and the Grind Tracker's "Can't read the EXP bar"), in the settings view's line under keys it refused, and the collapsed icon's alert dot. The "Not sure" chip is the game's orange button itself, a gradient from Doubt Gold (doubt-top) to Doubt Amber (doubt-base). Problem notes sit on Problem Cream (problem-bg), edged in Problem Amber (problem-edge), with Problem Brown text (problem-ink). The "paused" tag in the Grind Tracker borrows that cream, amber and brown, without the orange, because held figures are a state to notice; so do the settings view's "Taken" and "Invalid" tags, and the keys it refused, set in a cream chip until others are pressed.

### Tertiary
- **Mob Dot Red** (mob, edged by mob-edge): the minimap's mob dot. The section mark for Mobs and the bullet on each mob row.
- **You Yellow** (you, lit by you-light, edged by you-edge): the minimap's dot for the player's own character. It marks the Hunting Target's arrival, "You're here", as a 10px dot with a you-light highlight up and to the left (and the map the player is on in the Personal Bests page the same way), and one other thing about the player, their best: the Grind Tracker's star on Last 5 min while it is a new best on the map (and the Personal Bests page's star on their best of all maps), an 11px inline SVG star filled You Yellow, lit by a you-light highlight up and to the left, and ringed in you-edge. Kept clear of Game Orange: a pale yellow with a dark gold edge, never a chip or a note.
- **Teleport Green** (teleport): the Map Preview's green for a teleport within the map, OSMS Data Explorer's meaning in a deeper shade. The ring of a portal that leads elsewhere on the same map, its dashed arrow to where it lands, and the row ring of such a Hidden Portal. The panel's one true green, kept apart from EXP Lime.
- **Unknown Grey** (unknown): OSMS's grey, taken from the panel's cool inks: the Map Preview's ring for a portal to a map not in the data, and that Hidden Portal's row ring.
- **Route Magenta** (route): the way to the Hunting Target. The Map Preview's wide ring around each portal the route takes next, and the Navigation section's mark, the same ring, so the section is the ring's legend. Magenta because nothing else in the panel holds it: its hue sits midway between Mob Dot Red and Quest Violet, far from blue, green, grey and the game's orange, and from You Yellow, which would say where the player stands (MapleCompass never knows). Never a fill, a tag or text.
- **EXP Lime** (exp-top, exp-mid, exp-base), on **EXP Track** grey (exp-track) under an **EXP Frame** (exp-frame): the game's own EXP gauge, sampled from its status bar (a lime fill on a light grey track, a dark grey frame with a white line beneath it). The Grind Tracker's EXP gauge, and nothing else. A yellow-green, kept apart from You Yellow's gold.

### Neutral
- **Quest Helper Glass** (glass): the overlay's see-through periwinkle body, at 62% so the game shows through as it does through the Quest Helper. Over white, the ground the Personal Bests page's window stands on.
- **Window Edge** (window-edge): the 1px frame of the panel, the rules under its window bar and title panel, the light icon buttons' border, and setup's window bar and recognised test read.
- **Window Bar Grey** (window-bar-top, window-bar-mid, window-bar-base): the three stops of the Mini Map's and Quest Helper's light grey bar, one custom property `--window-bar` (`linear-gradient(180deg, #ececec 0%, #dfdfdf 55%, #d6d6d6 100%)`), shared by the overlay's, the search view's and the Grind Tracker's window bars, setup's and the Personal Bests page's. **Window Bar Ink** (window-bar-ink): the window's name, the search view's title and setup's window name and step count on that bar.
- **Mini Map Sky** (title-panel): the title panel under the window bar, and setup's recognised test read. **Title Lettering** (title-ink, outlined in title-outline): the Street Name and Map Name on it.
- **Game Button Blue** (game-button-top, game-button-mid, game-button-base, edged by game-button-edge): the game's bright blue window buttons, the overlay's and the Grind Tracker's toolbars and the search view's back button. Hover and pressed gradients are one step lighter and one step deeper.
- **Panel Ink** (panel-ink), **Panel Ink Steel** (panel-ink-2) and **Panel Ink Slate** (panel-ink-3): the Quest Helper's near-black text on the glass, for primary text and values; secondary text such as notes, positions, units, labels and table headers; and tertiary text and glyphs such as the footer's credit icon, candidate map IDs, key hints and placeholders.
- **Panel Rule** (panel-rule): the hairline above the footer, and above the Mob Card's figures.
- **Inset** (inset-bg, inset-edge): white at 50% edged in navy at 30%, for a list or box set into the panel's body: the search suggestions. Inset Edge alone frames the Map Preview's paper. The Grind Tracker's figures, the Navigation section's route and arrival, and the search results sit on near-opaque Window Lavender instead, edged the same. The Personal Bests page's table is the Inset tint on opaque lavender.
- **Results Header** (results-head): the opaque tone of the glass behind the search results' sticky header, so rows scrolled under it don't show through.
- **Ink** (ink), **Ink Steel** (ink-2) and **Ink Slate** (ink-3): navy text on lavender, in setup, the title-marking hint and the Personal Bests page; Ink is also the ground of the collapsed icon's count chip and Time to Level badge; Ink Steel is the badge's ground while its figure is held.
- **Window Lavender** (body) and **Window Lavender, Opaque** (body-solid): the title-marking hint's translucent body, and setup's and the Personal Bests page's opaque one; at 95%, the ground of the Grind Tracker's figures. **Frame Navy** (frame): the hint's border, setup's secondary buttons and the search field. **Window Rule** (rule): the hairline above setup's button row.
- **Selection Periwinkle** (select, edged by select-edge): the selected row wherever there is one: the shown candidate, the suggestion Enter picks, the result Enter makes the Hunting Target, the hotkey waiting for its new keys, the Hidden Portal row whose ring is lit; and the Personal Bests page's text selection. select-edge also colours every scrollbar thumb.
- **Tag Periwinkle** (tag-bg, tag-edge, tag-ink): the "Hidden Street" tag, the mark card's "Hidden", "Door", "Within the map", "Not in the map list" and "Next on your route" tags, the route's ride tags ("Taxi"), setup's "Recognised" and "Read" tags, and (tag-bg alone) the Personal Bests page's table header. **Quest Violet** (quest-bg, quest-edge, quest-ink): the "Quest" tag, on a Hidden Portal's row and on the mark card. Violet because no other meaning holds it: green now means a teleport within the map, and cream and amber tags already mean a state to notice (the Orange Means Doubt Rule keeps the game's orange for problems).
- **Edit Blue** (edit): the 2px keyboard-focus ring in every window, the dashed drag box when marking, the checked travel checkboxes (`accent-color`), and, at 8%, the hover tint on suggestion and result rows and the Personal Bests page's rows.
- **Minimap Paper** (minimap-paper): the Map Preview's paper under OSMS's see-through picture of the map, itself see-through at 45%: the game shows through the picture's empty parts, paler than through the glass beside it, as it shows through every window over the game (the Over the Game, See-Through Rule). Inset Edge still frames the box, and every ring keeps its white halo, so the marks hold over the game.
- **Button Steel** (button-top, button-base, edged by button-edge): the deep-steel gradient of setup's primary button, deep enough that white holds 4.5:1 even at its light top. **Light Steel** (button-light-top, button-light-base): setup's secondary buttons and the overlay's light icon and text buttons in the body. Hover gradients sit one step darker (deep) or lighter (light).
- **Greyed** (disabled-bg, disabled-edge, disabled-ink): a button the window won't take, flat with no highlight.
- **Navy Veil** (veil): the title-marking window's whole surface, so the game reads as paused behind it. It is non-zero on purpose: a fully transparent pixel would not take the mouse.

### Named Rules
**The Orange Means Doubt Rule.** Game orange marks only uncertainty and problems: the "Not sure" chip, problem notes, the collapsed alert dot. A confirmed state never uses it, so orange can't become decoration. Search tiers are told apart by weight and ink, never by orange.

**The Minimap Legend Rule.** Portal blue, mob red and you yellow come from the minimap's legend and keep that meaning. Blue is for portals, red is for mobs and yellow is for where the player is, with no other uses but one: you yellow also draws the star for a best (the Grind Tracker's new best, the Personal Bests page's best of all maps), since both are about the player. The Map Preview adds two ring colours from OSMS Data Explorer, each with one meaning: Teleport Green for a portal within the map and its arrow, Unknown Grey for a portal to a map not in the data. Blue stays the ring of a portal to another map, Doors included; green is never a tag. Route Magenta is the panel's own and means one thing, the way to the Hunting Target: the wide ring around a portal the route takes next, and the Navigation section's mark.

**The EXP Lime Rule.** The lime of the game's EXP gauge draws the EXP bar's reading in the Grind Tracker and nothing else: not progress in general, and never a highlight.

**The Over the Game, See-Through Rule.** A window laid over the game is see-through: the overlay on the Quest Helper's glass, the title-marking hint on translucent Window Lavender. A window on the desktop (setup) is opaque. Same family, different footing.

**The Two Inks Rule.** Text on the see-through glass is the Quest Helper's near-black Panel Ink; text on lavender is navy Ink. Never mix them in one window. The panel sets its own `color`: inheriting the body's would carry navy into every figure that doesn't name its ink.

## Typography

**Display Font:** Maplestory (NEXON's typeface, bundled in `assets/fonts`; see its NOTICE), falling back to Arial, "Segoe UI", system-ui, sans-serif, as the shared `--font`
**Body Font:** the same family
**Label/Mono Font:** none distinct; Maplestory's figures are proportional (it has no tabular figures)

**Character:** Every window is set in Maplestory, NEXON's own rounded sans for the game (chosen by the user in October 2026), so the text reads as the game's. It comes in two weights, declared as one family in `theme.css`: Light answers weights up to 500, so regular text (400) is Light, and Bold answers 600 and up. Weight (Bold against Light) and colour carry the hierarchy, not a second family. It loads with `font-display: block`, bundled with the app, so text never draws in Arial first and changes width; the overlay measures its height again once it has loaded. It lacks the en dash, em dash and minus sign, which come from Arial. Light is thinner than the game draws its window text over a busy scene, so the panel gives all its text a 0.3px stroke in the text's own colour (`-webkit-text-stroke-width: var(--text-weight-stroke)`); more clogs the Bold figures. The outlined title lettering and the doubt chip, whose outlines are their weight, take none. Maplestory sets wider than Arial: a long Map Name takes its second line sooner, and long single-line facts truncate sooner.

### Hierarchy
- **Headline** (700, 18px/23px, 0.01em): the Map Name, in Title Lettering on the title panel, in the overlay and in setup's recognised test read. Wraps to at most two lines rather than losing its end. It wraps in balanced lines (`text-wrap: balance`), so a second line is never a stranded word. When no map is read, the status takes this slot at 15px/23px bold (Status) in plain Panel Ink with no outline, so it doesn't read as a map name. The Time to Level figure and setup's step headings use 18px bold in plain ink, without the outline. Held (warming up, paused, or with the EXP bar unread), the Time to Level figure and the rates drop to Light (the Best row's figure, which doesn't move, stays bold): a step back in weight, never a fainter colour or opacity, so held figures stay as legible over the game. With no figure to give, the slot says why in 13px Light Panel Ink Steel ("not yet", "no gain yet", "losing EXP"), never a bare dash. Setup's EXP test read sets its figures at 16px/22px bold.
- **Street** (700, 14px/18px): the Street Name above the Map Name, in the same Title Lettering, the Mini Map's own idiom.
- **Section Heading** (700, 14px/18px): the overlay's section headings (Map, Navigation, Mobs) and the Hunting Target's mob under Navigation, in Panel Ink. The Street's size, one step above the 13px bold rows they head, so a heading never reads as one more row.
- **Window Name** (700, 12px/16px): the window's name on the window bar ("MapleCompass", "Grind Tracker") in Window Bar Ink, and the search view's title there, the mob's level beside it in regular weight. Setup's step count is the same size and weight; setup's window name is 14px.
- **Title** (700, 13px/17px): portal destinations, summary and rate values, the Hunting Target's figures and "You're here".
- **Body Reading** (400, 14px/20px): setup's prose, the search field's text, and the title-marking hint's heading (at 700). Setup is read at rest, not in glances, so it reads one step larger than the overlay.
- **Body** (400, 13px/17px): overlay notes, the mob and rates tables, the suggestions, and any panel text without a role of its own (the panel sets 13px/17px). Mob names are 700. Setup's notices, test read and shortcuts use 13px at an 18px line; the title-marking hint's instructions are 13px.
- **Body Small** (400, 12px/15–17px; 15px on a single line, 16px where it wraps, 17px in a table row): labels, summary units, table headers, figure labels, the route, the arrival's trip, candidate rows, key hints, the search view's credit, setup's credit, the title-marking status line, the Map Preview's "Loading map…" and the mark card's street.
- **Button** (700, 13px/18px): setup's button labels, sentence case. The Overlay Guide button in setup's window bar is 12px.
- **Label** (700, 11px/15px): tags, in sentence case and untracked, so a tag never takes room from the destination beside it. The "Not sure" chip alone is uppercase, tracked 0.04em, at a 13px line. Candidate map IDs are 11px regular.

The collapsed icon carries the system's two sizes below the Label step, held there because they share a 44px square with the compass: the Hidden Portal count at 10px/13px bold and the Time to Level badge at 9px/12px bold, both white on Ink (the badge in Light on Ink Steel while its figure is held). They are exceptions, not scale steps.

### Named Rules
**The Right-Aligned Figures Rule.** Numeric columns are right-aligned so EXP, HP, rates and share line up at their ends from one glance to the next. Maplestory's figures are proportional and it has no tabular ones; the body still asks for `tabular-nums`, which the Arial fallback honours.

**The Outlined Lettering Rule.** Only a map's title, the Street Name and the Map Name, uses the Mini Map's light lettering outlined in blue (the shared `--title-lettering` text-shadow: a 1px title-outline stroke in eight directions plus one soft 0 2px 1px drop). It appears on the overlay's title panel and in setup's recognised test read, nowhere else. Everything else, Time to Level included, is ink.

**The Notes Wrap, Names Truncate Rule.** Notes and the Map Name wrap so they can be read in full; notes, key hints and the arrival's trip wrap with `text-wrap: pretty`, the Map Name with `balance`. Single-line facts (destinations, mob names, route stops, suggestion names, result maps) truncate with an ellipsis instead of growing the panel.

## Layout

A single column, 380px wide until the player drags a side or bottom corner (never under 320px), whose height fits its content (44–600px, or less where the panel is placed). The overlay measures its full height, borders and bottom padding included, and the window resizes to match; past that, the long parts (the route, the suggestions, the map list) scroll on Selection Edge scrollbars while the rest keeps its height. A route of three or more stops never shrinks below two and a half of them (6.5em), the cut third saying it scrolls; a shorter route never shrinks. Should the panel still not fit, its body scrolls as a whole under the window bar and title panel, which stay pinned at the top.

The window bar runs full width (at least 22px tall, padding 2px 3px 2px 5px, 5px gaps): the 16px compass, the window's name, and at the right the toolbar (2px between buttons). The title panel runs full width, flush beneath it (padding 6px 10px 8px 11px, 1px between lines): the Street Name, then the Map Name with the doubt chip at its right. The body below is a vertical stack with 11px side insets and 16px at the bottom, clear of the corner grips: 9px between top-level items (11px under a window bar with no title panel, as in the search view), 10px between sections, and 2px between portal rows and between candidate rows. The rhythm is tuned pixel by pixel to the game's windows, not taken from a modular scale.

Order is fixed: window bar, title panel, then any recovery or hotkey notes, then candidates when several maps share a title, then the status note, then the Map (the Map Preview, then the Hidden Portals), then Navigation while there is a Hunting Target (the target, its figures, then the route or the arrival), then Mobs (a two-row summary grid of EXP/h and Mobs/h, solo and 6+ party, then the top five mobs in a fixed-layout table and a "+N more" line), then a footer with the credit below a Panel Rule. Candidates, the status note and the three sections stand 10px apart; Navigation has no box of its own, so its parts sit in the body's column and its route gives way to everything else when the panel runs out of room. The Map Preview runs the column's full width, 7px above the rows. A Hidden Portal row is one line: its 12px ring, then the destination and its tags, padded 2px 5px and set out 5px at each side so its lit tint clears the text. The Map goes when it has nothing to show: no Map Preview, no note in its place and no Hidden Portals. When Mobs has nothing to list, its heading and "No mobs on this map" share one line. Empty containers collapse rather than leave strips.

The Grind Tracker has a window of its own, built like the panel without a title panel or corner grips: 280px wide, as tall as its content. Its window bar holds its own icon (the pixel-art gold tally board in `assets/grind-tracker.png`, supplied by the owner, in place of the compass), "Grind Tracker", and at the right the button that opens the Personal Bests page (only once some map has a Personal Best), then the × that closes it. Its body, inset 11px with 10px at the bottom, stacks 8px apart: the notes (6px apart), then the figures set into the body, then the EXP gauge at the foot, where the game keeps its own. Inside the figures (padding 4px 8px 6px 9px, 3px apart): the Time to Level row, with the reset button at its far end; the rates, a row for the last 5 minutes, once the Session has run longer than that a row for the Session with its length, and on a map with a Personal Best from earlier Visits a Best row with its ×; then a line for the ceiling share and one for deaths, each wrapping. Until the Session starts, the figures hold the Best row alone, when there is one, under the waiting note. Until the player drags it, it sits beside the panel (or the collapsed icon) 8px away with the tops level, so it never moves as the panel grows and shrinks with each map: towards the middle of the game when the panel is inside it or detached, away from the game when the panel is docked, and on the other side when that one would leave the monitor. Let go within an eighth of its width of that spot, it follows the panel again. The panel's toolbar has a button that shows and hides it, pressed while it shows.

The search view replaces the whole panel: its own window bar (7px gaps, the back button then the title), then a 6px-gapped step: the query field, suggestions and the key hint; or, once a mob is picked, the nearest map, the results table, the travel checkboxes and the key hint; then the credit.

The settings view replaces the whole panel the same way: its window bar ("Settings"), then the Hotkeys heading over its one-line intro, the hotkey rows in an inset box 6px under them, and the actions row (the last change's notice, then Undo and Restore defaults at the right). Its rows don't truncate: what a hotkey does wraps, keeping at least 8em, and its keys break only after a "+", so the narrowest panel loses no word of either.

Collapsed, the window becomes a 44px square holding the compass. One container query (`max-width: 300px`, for raised text scale) narrows the panel without losing alignment or ends: the summary puts each figure on its own line beside its party size (solo, then 6+ party under it) with the label beside the pair; mob rows become labelled cards, the name over its four figures side by side; an empty Mobs section's heading stays on one line and "No mobs on this map" goes under it if it must.

The setup window is a fixed 520×560 opaque window: a full-width window bar (padding 9px 20px 10px, 10px gaps, wrapping) holding the 28px compass, the window name, the step count ("Step N of 7" on first run, "of 6" when setup reopens, "Guide" in the Overlay Guide) and the Overlay Guide button at the right, then a column with 22px side insets and 18px bottom padding that scrolls if it must. Each step is a heading (18px above, 8px below), prose in 10px-spaced paragraphs, and a footer pinned to the bottom of the window above a Window Rule, buttons right-aligned with 10px between them, the step forward always last. Notices sit above the step, 14px below the window bar and 8px apart.

The title-marking window covers the game client exactly. Its hint is centred horizontally, 12% up from the bottom, or 12% down from the top while marking the EXP bar so it never covers what is being marked; it is as wide as its longest line, up to the window less 64px.

## Elevation & Depth

No window has a drop shadow. Depth comes from the game's window chrome: a faint white ring just inside the frame, bevels on the buttons (a white top highlight, and on the game's blue buttons a darker bottom edge), 1px ring strokes around glyphs, and the outlined lettering of a map's title. The panel sits flat on the game scene, and its see-through glass is what joins it to the game's windows. A pressed button sinks with an inset shadow rather than lifting; the search field is the one surface set into the body, with a soft inner shadow. While the map shown isn't confirmed, the facts recede to 70% opacity rather than moving.

### Shadow Vocabulary
- **Window inner ring** (`box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.35)`): just inside the panel's frame. The title-marking hint uses 0.65.
- **Bar top highlight** (`box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.9)`): the top edge of setup's window bar. The doubt chip uses the same at 0.6, setup's primary button at 0.3, and the light-steel buttons a solid `#fff`.
- **Game button bevel** (`box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.55), inset 0 -1px 0 rgba(0, 60, 110, 0.25)`): the game's blue buttons, whose white icon carries a `drop-shadow(0 1px 0 rgba(0, 50, 90, 0.45))`.
- **Pressed** (`box-shadow: inset 0 1px 2px rgba(0, 40, 80, 0.5)`): a held game-blue button. A held deep-steel button uses `inset 0 1px 3px rgba(10, 20, 50, 0.45)`, light steel the same at 0.25.
- **Field inset** (`box-shadow: inset 0 1px 2px rgba(20, 30, 60, 0.15)`): the search field only.
- **Glyph ring** (`box-shadow: 0 0 0 1px <edge>`): the dark edge on mob dots, the you dot and the orange dots. The alert dot adds a second white ring (`0 0 0 2px #fff`).
- **Title lettering** (`--title-lettering`: eight 1px `text-shadow` offsets in title-outline, plus `0 2px 1px rgba(30, 60, 100, 0.3)`): a map's title only. The doubt chip's white label is outlined the same way in orange-deep, in four directions; setup's primary button label carries a 1px button-edge drop (`0 1px 0`).
- **Light edge** (`text-shadow: 0 0 2px rgba(238, 242, 255, 0.85), 0 0 4px rgba(238, 242, 255, 0.6)`, kept off the glyph's own edge so it doesn't thin the stroke): every line of text on the panel's body, outside the window bar and title panel.

### Named Rules
**The Chrome, Not Lift Rule.** Depth is drawn the way the game draws it, with highlights, bevels and hairlines. Nothing floats on a blurred drop shadow.

**The Light Edge Rule.** Text on the see-through glass keeps a faint light edge so a busy, saturated scene behind it can't swallow it. It is a legibility edge, as the Quest Helper's text has, not a glow: never coloured, never on the bars.

## Shapes

Small, game-window corners: 4px on the panel, notices, setup's buttons, the search field, inset boxes and the recognised test read; 3px on chips, tags, the badge and selectable rows; 2px on the game's small blue buttons and the light icon buttons; 6px on the title-marking hint; a full 8px pill on the collapsed icon's count chip. Borders are 1px solid in a darker tone of the fill. Dashed lines mark three things: the drag box while marking, a place not yet fixed; the ring of a Hidden or Quest Portal, a way the game doesn't show plainly; and the Map Preview's arrow from a teleport to where it lands, as OSMS draws it. Circles are reserved for minimap glyphs and the Map Preview's rings. The panel's bottom corners carry a dotted grip in their triangle, the way the game's windows mark a resizable corner.

## Components

### Window Bar
The Mini Map's light grey strip: `--window-bar`, a Window Edge rule beneath, no text edge. The 16px pixel-art compass, the window's name in Window Name type, and the toolbar at the right. It and the title panel are the drag handle: inside the game or detached, they show a move cursor and drag the panel, except over the buttons. The search view's window bar holds the back button, then the mob's name in Window Name type with its level in regular weight. Setup's window bar runs the full window width with a white top highlight and holds the 28px compass, the window name (14px bold), the step count (12px bold), both in Window Bar Ink, and the Overlay Guide button.

### Title Panel
The Mini Map's sky blue (title-panel) under the window bar, edged beneath in Window Edge: the Street Name over the Map Name, both in Title Lettering, with the doubt chip at the Map Name's right end when the map isn't confirmed. Each line keeps 1px of room at its sides so the outline isn't cut where a long name ends in an ellipsis. While a map is shown the title is its MeowDB page's link (the open-page hotkey's action, so the toolbar has no button for it): a pointer cursor, both lines underlined 1px in Title Ink (3px offset) under the pointer, and a tooltip naming the hotkey. A press drags the panel once the mouse moves 4px, and opens the page if let go first, as the collapsed icon does.

### Toolbar and Icon Buttons
The game's small window buttons, one per hotkey action, at the right of the window bar: search, next candidate, Grind Tracker (pressed while its window shows), placement, Settings, Help & Setup, collapse. Settings is the one with no hotkey (it is where the hotkeys are changed); its icon is a six-toothed cog with its hub, drawn as an outline, so at 13px it reads as a gear rather than a sun. The Grind Tracker's window has only its Personal Bests button and its close button on its bar, with no hotkeys; its reset is a light button in its body, away from the ×. Buttons whose action isn't available are hidden, not greyed.
- **Shape:** 19px square, 2px radius, 1px border, no padding.
- **Icon:** a 13px inline SVG on a 16-unit grid, stroked in `currentColor` at 2 with round caps and joins, no fill (a single filled dot where a glyph needs one).
- **Game blue** (window bars): the Game Button Blue gradient, game-button-edge border, white icon with its drop, and the game button bevel. The search view's back button is the same.
- **Light steel** (body): the Light Steel gradient, Window Edge border, Panel Ink icon with no drop, solid white top highlight. The Navigation section's reopen-search and clear buttons, and the Grind Tracker's reset and the × on its Best row. That × is always this light button, never the window bar's game-blue ×, so it can't be read as closing the window; its title and accessible name say "Remove this best from your history".
- **Text button** (body): the light steel button with a word where an icon couldn't say it, 19px tall, padding 0 7px, 12px/17px bold Panel Ink, no text shadow. The Grind Tracker's: the Undo in its reset note. The settings view's: Change and Cancel, Turn off and Swap (all 54px wide, one in another's place), Undo and Restore defaults. One the panel won't take now is greyed and flat, as setup greys its buttons: Restore defaults with every hotkey at its default, or while a hotkey waits for its keys.
- **States:** hover shifts the gradient one step lighter; pressed one step deeper with the Pressed shadow; keyboard focus draws the 2px Edit Blue ring offset 2px. The cursor is a pointer while the panel is draggable.

### Corner Grips
A 14px triangle at each bottom corner of the expanded panel, dotted (1px dots on a 4px grid in navy at 55%) at 80% opacity, full on hover, with a sideways resize cursor. Dragging one, or either side, sets the panel's width; the height always follows the content.

### Doubt Chip ("Not sure")
The game's small orange button, used once. It has a gradient from Doubt Gold to Doubt Amber, a 1px orange-deep border, a 3px radius, a 0.6 white top highlight, and white uppercase 11px bold text outlined in orange-deep. It shows while the map is unconfirmed and while several maps share the title and none is chosen. It is static and never clickable. Its tooltip says why, in place of a note under the title: "Title not confirmed. Keep both minimap title lines visible." and how to re-mark it, or "N maps share this title; not sure which you're on."

### Buttons
The game's window buttons, solid and pressable, in setup.
- **Shape:** 4px radius, 1px border, padding 6px 16px 7px, 13px/18px bold sentence-case labels.
- **Primary:** the Button Steel gradient, edged in button-edge, white label with a 1px button-edge drop and a 0.3 white top highlight. One per step: the step forward (Next, I understand, continue, Mark the title, Mark the EXP bar, Finish, Back to setup), always the rightmost.
- **Secondary:** the Light Steel gradient, edged in Frame Navy, Ink label with no text shadow and a solid white top highlight. Back, Quit, Skip, Pick by hand, Mark again, Keep the current title, Turn off the Grind Tracker.
- **Guide:** the secondary button at 12px with padding 3px 9px 4px, at the right end of setup's window bar.
- **Hover:** the gradient shifts one step (primary darker, secondary lighter). No movement.
- **Pressed:** the hover gradient (primary) or rest gradient (secondary) with the inset Pressed shadow.
- **Focus:** a 2px solid Edit Blue outline, offset 2px, on keyboard focus only (`:focus-visible`).
- **Disabled:** greyed and flat: disabled-bg, disabled-edge border, disabled-ink label, no highlight or text shadow. Next on the test read and Finish stay disabled until a read succeeds.

### Section Heading with Legend Mark
A 14px/18px bold heading (Section Heading) led by a CSS glyph from the legend: a 10px ring (2.5px portal border on portal-fill) for the Map; the same ring in Route Magenta on white for Navigation, the ring the Map Preview puts around a portal the route takes next; an 8px red dot with a mob-edge ring for Mobs.

The whole heading row is the section's toggle: a click anywhere on it collapses the section to the heading, or opens it again. At its right sits the game's small light button (19px, Light Steel, Window Edge border, solid white top highlight) with a 13px line icon: a dash while open, a plus while collapsed. Hovering anywhere on the row lightens the button, pressing sinks it, and keyboard focus rings the row. Collapsed, everything under the heading goes; Mobs alone adds how many it holds in regular Panel Ink Steel ("Mobs 6"), while "Map" and "Navigation" stay as they are. A Mobs section with nothing to show has nothing to collapse: its heading drops the button and stays beside "No mobs on this map", whatever was collapsed. The Map and Navigation always have something when they show: the Map shows only with a Map Preview (loading, drawn, or the note that it couldn't be loaded) or a Hidden Portal row, Navigation only with a Hunting Target. Collapsed sections are a setting: the same on every map, and kept between sessions.

### Map Preview (signature)
OSMS's picture of the whole map with every portal ringed where it is, then a row per Hidden Portal, so the list and the picture read as one legend. It heads the Map section on every map once the map is settled: none while several maps share the title and none is chosen, or when OSMS has no picture for it.
- **Box:** set into the body at full width, 7px above the rows: Minimap Paper, see-through at 45% under OSMS's see-through picture so the game shows through the picture's empty parts, paler than through the glass, inside a 1px Inset Edge with a 4px radius, with 5px of paper around the picture so a ring at its edge isn't cut. The picture is fitted to the width and centred, never taller than 168px, so the box stays within 180px; it is fitted again when the panel changes width or the section opens. To assistive tech it is one image, named with the map and its portals ("Map of Kerning City: 6 portals marked, 2 of them hidden", adding ", 1 next on your route" while there is one); the rows carry the Hidden Portals as text.
- **Loading:** a 140px box in the same frame and paper, "Loading map…" centred in 12px/16px regular Panel Ink Steel; then the box takes the picture's fitted height. **Failure:** the problem note "Couldn't load the map picture" in the box's place, 7px above the rows.
- **Rings:** an SVG over the picture, drawn in screen pixels whatever the picture's scale: radius 4 to the middle of a 1.75 stroke over a 22% fill of the ring's colour, on a white halo (3.75 stroke, 95% white) so it holds on dark art and pale paper alike. Portal Ring Blue to another map (Doors included), Teleport Green within the map, Unknown Grey to a map not in the data. A Hidden or Quest Portal's ring is dashed (2.6 1.6), its gaps showing the halo. A ring answers the pointer within 8px of its centre.
- **Arrows:** each teleport within the map draws a dashed Teleport Green line (1.5 stroke, dashes 3.5 2.5, round caps) over a white under-stroke (3.25, 85% white), from its ring's edge to the landing ring's edge, or to the landing point when nothing is ringed there, with a small white-edged Teleport Green head where it lands. A pair that land on each other is one line with a head at each end. The arrows lie under every ring.
- **Next on the route:** while there is a Hunting Target the player hasn't reached and the route goes on from the shown map through a portal, every portal there to the route's next map (never a Quest Portal, which no route takes) wears a wide Route Magenta ring around its own: radius 10 to the middle of a 2.25 stroke, solid whatever the portal's kind, on a white halo of its own (4.5 stroke, 95% white). It is always drawn, not only under the pointer, outside where the lit ring goes, over the arrows and under every ring, so it never hides one. A next step by ship, taxi or Teleport Scroll marks nothing. No other mark is that size or that colour.
- **Rows:** each Hidden Portal's row leads with its ring as the picture draws it, as a 12px SVG (blue and dashed when it has no mark on the picture), then the destination (13px bold, truncating) and its tags. A map with no Hidden Portals has no rows and no line saying so.
- **Lit:** the pointer on a ring, or on its row, lights both: the ring gains a second ring around it (radius 7.5, 1.5 stroke) in its own colour and comes over its neighbours, its arrow thickens to 2.25, and the row takes Selection Periwinkle with an inset 1px select-edge ring, its text where it was. A route ring stays as it is, the lit ring inside it. On a ring, its Mark Card shows too. The cursor stays the default, a click does nothing, and marks take no focus. Nothing moves.

### Mark Card
The card of the ring the pointer is on, shown at once (marks are small and sparse, so the Mob Card's wait would feel broken) and gone when the pointer leaves it; the pointer passes through it. Centred on the ring, 6px above it, or 6px below it when above would bring it within 6px of the window bar; kept 8px in from the panel's sides, never over the window bar or outside the panel, and never while the panel is collapsed or off its normal view. Opaque Body Solid under a 1px Window Edge with a 4px radius and the window's inner ring (0.65), no drop shadow; padding 4px 8px 6px, 12px/15px Panel Ink, at most the panel's width less 16px. The destination in 13px bold, then its street in Panel Ink Steel, both truncating, then 3px down a wrapping line of Tags saying its kind: "Hidden", "Quest", "Door", "Within the map", "Not in the map list", and "Next on your route" for a portal in a route ring. A visible portal to a known map, off the route, has none.

### Tags
Tags are 11px bold with a 3px radius and a 1px border. "Hidden Street", and the mark card's "Hidden", "Door", "Within the map" and "Not in the map list", are in Tag Periwinkle, "Quest" in Quest Violet, and "paused" in the problem palette. Padding follows the host line: 0 5px at a 15px line in the overlay, 0 6px at 16px in setup's "Recognised" and "Read" tags.

### Candidate Rows
When several maps share a title, each row leads with what sets the map apart ("1 hidden portal · Lv 2–8 · Blue Snail…") followed by its map ID in Panel Ink Slate. The shown map's row uses Selection Periwinkle with an inset 1px select-edge ring and bold Panel Ink text.

### Notices
- **Note:** Panel Ink Steel text that wraps, unboxed: 13px/17px in the overlay; Ink Steel at 13px/18px in setup.
- **Problem note:** Problem Cream, a 1px Problem Amber border, a 4px radius, Problem Brown text, and an 8px orange dot at the left. The state is carried by the dot, the box and the words, not by colour alone. In setup the same note is padded 7px 12px 8px 28px with the dot 11px in, for hotkey problems and failed actions alike.

### Grind Tracker
In its own window, read at a glance between attacks.
- **Notes,** on the glass in 13px Panel Ink, wrapping: "Waiting for EXP" until the Session starts; "You switched character, so this is a new Session." when the Session started afresh by itself, until it warms up; "Session reset." with the Undo text button, until the next EXP gain, when the reset can no longer be undone; and the problem note "Can't read the EXP bar. Move whatever covers it, or mark it again from Help & Setup." once the bar has gone 10 seconds without reading whole.
- **Figures:** a box set into the body on Window Lavender at 95%, edged in Inset Edge with a 4px radius, so the densest reading holds over any scene. A headline row: "Time to Level" (12px Panel Ink Steel), the figure (18px bold) and, while paused, the "paused" tag in the problem palette, with the light reset button at the far end. Warming up has no tag: its figures in Light say it. Then the rates, the panel's summary in miniature: a five-column grid at 12px Panel Ink Steel of period, value, "EXP/h", value, "/h" (the "/h" set against its percentage so "12.9%/h" reads as one), values 13px bold Panel Ink and right-aligned. "Last 5 min" always; "Session" with its length in Panel Ink Slate ("1h 12m") only once the Session is longer than the rolling window; then "Best" with the map's Personal Best from its history (the best of earlier Visits, as saved), its "EXP/h", and the light × in the percentages' column, which removes that best from the history at once, with no undo and no note, the next-best Visit's taking the row. The Best row shows from arriving on the map, with no wait: before the Session's first gain too, alone in the box under "Waiting for EXP". It never shows on a first Visit, a map without a Map Summary, or while Candidates wait for a choice. While Last 5 min beats it, over a full 5 minutes of play on the map, the You Yellow star follows "Last 5 min" (named "New best on this map"); the core says when, comparing the two figures as they are written, so the star never shows beside two that read the same. The star keeps its room while it's off, so the figures never shift as it comes and goes. EXP rates are written out at three significant figures as the panel writes its rates ("61,200"). Then "21% of this map's solo EXP/h (297,499)", counting only the play on the shown map once there is a minute of it, and "1 death (−2.1% of the level)", 12px Panel Ink Steel lines that wrap. Held, the figures drop to Light (see Typography), all but the Best row's: it doesn't move.
- **EXP gauge,** the window's signature: the game's status-bar EXP gauge in miniature. "EXP" in 12px bold and the reading as the bar writes it ("8,435 [41.72%]") in 12px Panel Ink, over a 10px gauge framed in EXP Frame with a 2px radius and a white top line: an EXP Lime fill on the EXP Track grey. It shows from the first reading, while waiting too, so the player sees the bar being read, and changes with every reading rather than holding with the figures. No easing, no sheen. It is a `progressbar` whose value text is the reading.
- **Personal Bests button,** on the window bar left of the ×: the game-blue icon button, an outlined star (the Best row's own mark for a best, drawn as the bar's icons are), named "Open your Personal Bests in the browser". Hidden, not greyed, until some map has a Personal Best (the Visit under way's counts). The Best row's "Best" opens the page too: plain at rest, so the row reads as before, Portal Blue and underlined under the pointer, as a mob's name in the panel. Either opens the Personal Bests page and leaves the window as it was.

### Personal Bests Page
Every map's Personal Best, in the player's browser rather than over the game: read at rest, to compare maps, so it is one of the game's windows in setup's opaque language and reading size, not the overlay's glass. One self-contained file, written afresh each time it opens, its fonts and icon inline (the icon is the tab's too).
- **Window:** one centred window up to 920px wide, 28px from the top, framed in a 1px Window Edge with a 4px radius and the window inner ring (0.35), no shadow, standing on Quest Helper Glass over white, as the game's windows stand on its scenes. Its bar is exactly as wide as its body: setup's `--window-bar` (padding 8px 18px 9px, white top highlight, Window Edge rule beneath) with the Grind Tracker's tally-board icon at 32px, scaled smoothly (its pixels don't divide evenly at that size), and "Personal Bests" (14px bold Window Bar Ink). Under 640px of browser the window meets the page's edges, unframed but for the rule under it.
- **Body:** Window Lavender, Opaque, 14px/20px Ink, 20px side insets. First a 13px Ink Steel line, "As of 9 Oct, 14:32 · 23 maps · shared by all your characters" (no time when the wall clock wasn't known). Under the table, 12px Ink Steel notes: what solo EXP/h is ("the map's ceiling, worked out as NiaMeowDB does"), what a dash means (only when there is one), and that the page shows the bests as they were, with the overlay's windows back when the player returns to the game. Then the credit, "Data: OSMS Data Explorer (osmsdataexplorer.com) · Pages: NiaMeowDB (meowdb.com)", in 12px Ink Slate below a Window Rule.
- **Table:** set into the body as an inset box (Inset tint, 1px Inset Edge, 4px radius), 13px/17px, 5px of padding above and below each row, rows parted by Window Rules. Columns: Map (its name in bold, a link to its MeowDB page in a new tab, the street under it in 12px Ink Steel; the name wraps), Region, Mob levels ("Lv 2–14"), Best EXP/h (bold), % of solo EXP/h ("21%", "5.6%" under 10%, with "of 297,499", the ceiling, under it in 12px Ink Steel), and Date set ("3 Oct", with its year when it isn't this one). Numbers right-aligned, written as the Grind Tracker writes them (EXP/h at three significant figures). The map's column keeps a 24px gutter at its left, so every name starts on one edge.
- **The best of all maps:** the Grind Tracker's new-best star (11px, You Yellow lit by you-light, ringed in you-edge), before the highest Best EXP/h, named "Your best of all maps", whatever the sort; none with only one map. It is the window's one emphasis.
- **Here:** the map the player is on, while the panel is certain of it, has only the You Yellow dot (named "You're here") hung in the gutter: where they stand never outweighs their best.
- **Missing figures:** a figure that isn't known is an Ink Slate "—", its reason under the pointer and for screen readers: the map not in the map list, no mobs spawning on it, no solo EXP/h for it, or a best set before MapleCompass knew the date. A map not in the map list reads "Map 999999999", with "Not in the map list" in Ink Slate where its street goes.
- **Sorting:** the sticky header on Tag Periwinkle (12px Ink Steel) is a row of buttons. Highest Best EXP/h first; a header sorts by its column the way it reads best first (lowest levels, highest share, newest, A to Z), and again the other way. The sorted header is bold Ink with a small triangle for the way (`aria-sort`), on the figures' side; every header keeps the room of its bold label and its triangle, so sorting never moves a figure. Ties keep highest EXP/h first; missing figures go last either way. Each sort is announced ("Sorted by mob levels, lowest first.").
- **Links and hover:** rows take Edit Blue at 8% under the pointer, a reading aid; only the name is a link, ink at rest, Portal Blue and underlined under the pointer, Frame Navy pressed, as a mob's name in the panel. Sort buttons and links take the Edit Blue focus ring.
- **Narrower:** the window is a size container. Under 700px Region joins the street line ("Victoria Road · Victoria Island"), the date may wrap, and the share's header says "% of solo". Under 540px each map becomes a card, as the panel's mob rows do: its name and street over its four figures side by side, each under its 12px Ink Steel label; the headers stay as a wrapping row of sort buttons. The table never leaves its box.
### Summary and Mob Table
The summary is a five-column grid at 12px: label, value, unit, value, unit. Values are 13px bold Panel Ink, right-aligned with a 52px minimum width. The mob table runs full width with fixed column shares at 13px/17px, right-aligned numeric columns, 6px between columns and 12px Panel Ink Steel headers. Each mob name is bold, carries a 6px red mob dot, and truncates. Plain at rest, the name turns Portal Blue and underlined (1px, 2px offset) under the pointer, with a pointer cursor, and Frame Navy while pressed; keyboard focus takes the Edit Blue ring: it opens the mob's MeowDB page, and resting on it shows its Mob Card. "+N more" follows in 12px Panel Ink Steel.

### Mob Card
Shown 250ms after the pointer rests on a listed mob's name, gone as soon as it leaves; the pointer passes through it. It floats over the panel's own content, 8px in from each side and 4px below the mob's row, or above it when there is no room below; where it fits on neither side it takes the roomier one. It never covers the window bar, never leaves the panel and never resizes it. Opaque Body Solid under a 1px Window Edge frame with a 4px radius and the window's inner ring (0.65), so the content it covers doesn't show through; no drop shadow. Padding 6px 10px 8px 6px, 12px/15px Panel Ink.

- **Head:** a 64px square holding OSMS's animation of the mob, standing on the square's floor at its own size (drawn pixelated), or scaled down smoothly to fit, never up; the square stays, empty, until the animation is in, or when there is none. Beside it, 8px over, the name in 13px bold, then "Lv N · Slow" in Panel Ink Steel (both truncate), then a Tag per element ("Weak to Fire", "Resists Ice", "Immune to Poison"), wrapping, or "No element" in Panel Ink Steel.
- **Figures:** 7px below the head under a Panel Rule, two columns 18px apart, as in the game's stat window: each 12px Panel Ink Steel label at the left of its 13px bold value, right-aligned so the figures line up. Touch and MP, ACC and AVOID, P.DEF and M.DEF in pairs, then HP per EXP across the full width with its rank among the mobs in the game ("· #34 of 92") in regular Panel Ink Steel. "—" stands for a figure the mob doesn't have. The figures come with the map's data, so the card has no loading or problem notes of its own.

### Navigation
The Hunting Target and the way there, a section between the Map and the Mobs that shows whenever there is a Hunting Target, arrived or not, whether or not a map is shown (with none, its Distance reads "?" and a note says why there is no route). Its Section Heading, "Navigation", collapses it like the others; no rule sets it apart. Its parts sit in the body's own column, 6px apart and 5px under the heading: first the target, a 40px square holding OSMS's animation of the mob (the Mob Card's, standing on the square's floor at its own size, drawn pixelated, or scaled down smoothly to fit, never up; the square stays, empty, until the animation is in, or when there is none), 8px clear of two truncating lines (the mob in 14px bold Section Heading, the map under it in 12px Panel Ink Steel, as a street sits under its map), with the two light icon buttons at its right; then a four-column figures grid (Tier, Density, Share, Distance: 12px Panel Ink Steel labels over 13px values, bold except the tier); a 12px bold route title; and the route in an inset box on the near-opaque Body lavender (padding 5px 10px 5px 8px, 4px radius, 13px/17px), drawn as the minimap draws a way: a 2px Portal Inner rail joins a glyph per stop, the You Yellow dot where the player starts, a portal ring (2.5px Portal Blue on Portal Fill) for each portal, a Panel Ink Steel ring for a ship, taxi or teleport scroll, and the red mob dot at the target, whose map is bold. Each stop is its map, with a Tag naming the ride when it isn't a portal ("Taxi"), and its street in 12px Panel Ink Steel only where the street changes. Squeezed, the route keeps about three and a half stops. A Distance of 0 reads "here". While the route goes on from the shown map through a portal, the Map Preview rings that portal in Route Magenta, the ring that leads the section's heading.

**Arrival.** Once the player reaches the target's map from another, the route has done its job and its inset box resolves to its last stop: the same near-opaque inset box (padding 5px 10px 6px), the You Yellow dot, "You're here" in 13px bold, and beneath it the trip as it stood when the target was picked ("3 map changes from Construction Site North of Kerning City, by taxi") in 12px Panel Ink Steel, wrapping. It is a status, static like everything else, and stays until the player leaves the map or presses ×; a target picked on the map the player is on shows no arrival until they leave and come back.

### Search View
- **Field:** a 12px Panel Ink Steel label over the app's one text input: white, 1px Frame Navy border, 4px radius, padding 6px 10px, 14px Panel Ink, Panel Ink Slate placeholder, the Field inset shadow, and the shared focus ring.
- **Suggestions:** an inset box (padding 3px, 13px) of rows (padding 2px 8px, 3px radius) in Panel Ink Steel, name truncating and level at the right; the row Enter picks wears Selection Periwinkle with its inset ring and bold Panel Ink.
- **Results:** a fixed-layout table set into the body as an inset box (1px Inset Edge, 4px radius) on the near-opaque Body lavender rather than the light inset tint: the panel's densest reading, over the busiest scenes. Each map is its own row group, parted from the next by a Panel Rule: its name in bold across the row with its Distance (bold, a number, or "here") at the right, then its street in 12px Panel Ink Steel under Map with density, share and tier under their headers (columns 30/17/13/19/21%). The header sticks to the top on the opaque Results Header tone. The current map is Selection Periwinkle framed by Selection Edge hairlines. Tiers read by weight and ink (high bold, medium Panel Ink Steel, low Panel Ink Slate); a missing distance is Panel Ink Slate. The list takes the arrow keys, and its selection, not a focus ring, shows where they act.
- **Row hover:** suggestion and result rows take Edit Blue at 8% under the pointer.
- **Travel:** a Panel Ink Steel legend and checkboxes in Edit Blue (`accent-color`), 12px.
- **Key hint:** one 12px/16px Panel Ink Slate line naming the keys, its arrows in text presentation (`font-variant-emoji: text`) so they stay ink rather than the emoji font's blue keycaps.

### Settings View
Opened by the gear, or by Change hotkeys in setup's Overlay Guide, it takes the keyboard as the search view does and closes when the player clicks back into the game. It changes the hotkeys, and every change is saved at once.
- **Head:** "Hotkeys" in Section Heading type, and under it, 2px down, one 12px/16px Panel Ink Steel line on what a hotkey costs the game: "While the game or MapleCompass is in front, these keys act here and never reach the game."
- **Rows:** an inset box on the near-opaque Body lavender (1px Inset Edge, 4px radius, padding 3px, rows 1px apart), as the map list is set. Each row is a three-column grid (padding 2px 2px 2px 8px, gaps 3px 8px, at least 23px tall, 3px radius): what the hotkey does in 13px Panel Ink Steel, wrapping, never under 8em; its keys in 13px bold Panel Ink, right-aligned, breaking only after a "+" ("Ctrl+Shift+F"; "Off" in Light Panel Ink Steel when it has none); and a 54px Change text button. The names are the setup guide's, word for word.
- **Waiting for keys:** the row wears Selection Periwinkle with its inset ring, its name turns Panel Ink, "Press keys…" in Light Panel Ink Steel takes the keys' place, and Change becomes Cancel. A second line spans the row: "Hold Ctrl+Shift and press a key · Esc cancels" in 12px Panel Ink Steel, with Turn off under Cancel (Backspace or Delete does the same).
- **Refused:** the keys pressed sit in a cream chip (Problem Cream, a 1px inset Problem Amber ring, Problem Brown, 3px radius, padding 0 5px) in the keys' place, and the second line says why in Problem Brown after a 7px orange dot: "Ctrl+Shift+F is already “Search a mob”. Swap them, or press other keys.", with Swap under Cancel; "Add Ctrl, Alt or Win: …"; "Caps Lock can't be part of a hotkey. Press another key."
- **Doing nothing:** under a row whose keys Windows or another app has, or that aren't keys at all, a "Taken" or "Invalid" tag in the problem palette (as "paused") leads a 12px Panel Ink Steel line saying why. Item Lookup, the one action with no button, says when it is off that it can't be used.
- **Actions:** the last change in 12px Panel Ink Steel ("Saved: Search a mob is now Ctrl+Alt+F."), then Undo while Restore defaults can be undone, then Restore defaults, right-aligned; the line takes a row of its own above them when it would be squeezed.
- **Focus and speech:** opening focuses the first Change; waiting focuses its row, so no key press lands on a button (plain Tab still reaches Cancel, Turn off and Swap, and Enter or Space presses them); done, its Change again. Restored, Undo; undone, Restore defaults. One polite live region speaks the wait, the refusal and the saved change, and each Change names its hotkey and keys.

### Footer
Whenever the panel credits anything, a 14px info glyph (a 1.5px ring in Panel Ink Slate, Panel Ink Steel under the pointer, a help cursor) at the right, 4px below a Panel Rule. Its tooltip is the credit, so the credit takes one short line rather than two or three: with a map shown, "Data: OSMS Data Explorer (osmsdataexplorer.com) · Pages: NiaMeowDB (meowdb.com)"; with no map shown but a Hunting Target, "Data: OSMS Data Explorer (osmsdataexplorer.com)", since Navigation opens no pages. The search view ends on that same line written out, in 12px Panel Ink Slate.

### Collapsed Icon
The app icon itself: the pixel-art gold compass filling the 44px square, with no frame. The Hidden Portal count sits on a chip at its bottom-right (an Ink pill with a 1px white border, at least 14px wide, 10px/13px bold white, hidden when there is no count), and a 9px orange alert dot with a white ring at its top-right when something needs a look. With EXP tracked, the Time to Level badge runs along its bottom edge (2px in from each side, 3px radius, Ink ground, white 9px/12px bold, "47m"; in Light on Ink Steel while the figure is held) and the count chip rises above it. The alert dot shows too while the EXP bar can't be read. Clicks and drags pass through the badge to the icon.

### Unconfirmed State
While the map shown isn't confirmed, or several maps share its title and none is chosen, the facts (the Map and the Mobs) recede to 70% behind the doubt chip and the candidate list, so the panel states its doubt before anything else. Navigation, the player's own plan, stays as it is.

### Test Read (setup)
One box with three states. Reading: a plain unboxed note. Nothing read: the problem note (padded 9px 12px 10px 28px). Read: the map as the overlay's title panel will show it, on Mini Map Sky with a Window Edge border and 4px radius (padding 8px 12px 9px), the Street Name over the Map Name in Title Lettering, tagged "Recognised" at the right in sentence case. For the EXP bar, the figures as the bar writes them ("EXP 160 [3.81%]", 16px bold ink), tagged "Read".

### Shortcuts List (setup)
A two-column definition grid at 13px: the action in Ink Steel, its keys right-aligned in bold ink (gap 6px 16px). A compact variant (gap 2px 12px) sits under "While you play" on the title test step; the full list is the Overlay Guide.

### Title-Marking Window
A transparent window over the game client, its whole surface the Navy Veil with a crosshair cursor.
- **Hint:** a small window on the lavender: translucent Window Lavender, 1px Frame Navy border, 6px radius, a 0.65 white inner ring, padding 9px 14px 10px, text centred. A 14px bold Ink heading ("Mark both lines of the minimap title"), two 13px Ink Steel lines of keyboard instructions, and a 12px Ink Slate status line. It ignores the pointer.
- **Drag box:** a 2px dashed Edit Blue outline laid over a 2px solid white band (95%), so it shows on the minimap's light blue and on dark scenery alike. Clear inside, so what it frames reads at full strength.

## Do's and Don'ts

### Do:
- **Do** build new overlay states from the game's window materials: the grey window bar, the Mini Map's sky-blue title panel, the Quest Helper's see-through glass with near-black text, and the game's bright blue buttons.
- **Do** keep setup on its opaque lavender with navy ink and steel buttons under the same grey window bar.
- **Do** use a see-through body for windows over the game and an opaque one for windows on the desktop.
- **Do** reserve game orange for doubt and problems, and pair it with words and a shape (chip, dot, bordered note) so the state never relies on colour alone.
- **Do** draw icons as minimap glyphs (portal ring, mob dot, you dot and the you yellow star) and the Map Preview's rings and arrows in CSS or inline SVG, or as 2-stroke line icons on the game's small buttons; the pixel-art compass is the only raster and marks the window.
- **Do** keep numeric columns right-aligned.
- **Do** let notes and the Map Name wrap, and truncate single-line facts with an ellipsis.
- **Do** keep the Street Name above the Map Name wherever a map is shown as the overlay shows it.
- **Do** draw the EXP bar's reading as the game's own EXP gauge, in EXP Lime, and nothing else in that lime.
- **Do** give every button, in every window, hover, pressed and a 2px Edit Blue `:focus-visible` ring offset 2px: game blue on the overlay's window bars, light steel in its body, deep steel for setup's step forward and light steel beside it.
- **Do** mark a selected row with Selection Periwinkle and its inset edge, and set a list into the body as an inset box.

### Don't:
- **Don't** return to the dark dev-tools card with a stats table on top.
- **Don't** use orange, or the outlined title lettering, for anything other than their single roles.
- **Don't** add small labels or kickers above content beyond the Street Name in the title panel; a label belongs only to the field or figure it names.
- **Don't** let a click on the overlay take the keyboard from the game outside the search and settings views.
- **Don't** add motion (transitions or animations) to any window.
- **Don't** float any window on a blurred drop shadow; depth is highlights, bevels and hairlines.
- **Don't** introduce a second typeface or icon fonts.
