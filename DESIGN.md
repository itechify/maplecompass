---
name: MapleCompass
description: "The Scout’s Notes: quiet, practical map context beside the game."
colors:
  accent: "#f5c542"
  bg: "#121a24"
  text: "#f2f5f8"
  muted: "#9fb6cc"
  ok: "#7fd18b"
  bad: "#f08a7a"
  panel: "rgba(16, 24, 36, 0.82)"
  inset: "rgba(16, 24, 36, 0.9)"
  secondary-text: "#c3ccd6"
  warning: "#f0c674"
  tag-text: "#c3d4e4"
  tag-fill: "rgba(159, 182, 204, 0.2)"
  quest-fill: "rgba(240, 198, 116, 0.2)"
  notice-fill: "rgba(245, 197, 66, 0.08)"
  selection-fill: "rgba(245, 197, 66, 0.12)"
  scrim: "rgba(8, 14, 22, 0.35)"
  secondary-border: "#3a4a5c"
  inset-border: "#2a3846"
  accent-ink: "#1b1405"
  icon: "#0891b2"
typography:
  headline:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "19px"
    fontWeight: 600
    lineHeight: 1.5
  window-title:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.5
  map-title:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "17px"
    fontWeight: 600
  test-map-title:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "18px"
    fontWeight: 600
    lineHeight: 1.5
  body:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  button:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "14px"
    fontWeight: 600
    lineHeight: 1.5
  marking-hint:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.4
  note:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "13px"
    fontWeight: 400
  data:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "12px"
    fontWeight: 400
  street:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "12px"
    fontWeight: 400
    letterSpacing: "0.02em"
  footer:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "12px"
    fontWeight: 400
  tag:
    fontFamily: '"Segoe UI", system-ui, sans-serif'
    fontSize: "11px"
    fontWeight: 400
rounded:
  tag: "3px"
  button: "6px"
  inset: "8px"
  panel: "10px"
spacing:
  compact: "4px"
  group: "8px"
  control-gap: "10px"
  panel-inline: "14px"
  control-inline: "16px"
  list-inset: "20px"
  setup-inline: "24px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-ink}"
    typography: "{typography.button}"
    rounded: "{rounded.button}"
    padding: "7px 16px"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    rounded: "{rounded.button}"
    padding: "7px 16px"
  overlay-panel:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    rounded: "{rounded.panel}"
    padding: "10px 14px"
    width: "380px"
  test-read:
    backgroundColor: "{colors.inset}"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    rounded: "{rounded.inset}"
    padding: "14px 16px"
  notice:
    backgroundColor: "{colors.notice-fill}"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    padding: "8px 12px"
  tag-hidden-street:
    backgroundColor: "{colors.tag-fill}"
    textColor: "{colors.tag-text}"
    typography: "{typography.tag}"
    rounded: "{rounded.tag}"
    padding: "0px 5px"
  tag-quest:
    backgroundColor: "{colors.quest-fill}"
    textColor: "{colors.warning}"
    typography: "{typography.tag}"
    rounded: "{rounded.tag}"
    padding: "0px 5px"
  map-table:
    textColor: "{colors.text}"
    typography: "{typography.data}"
  candidate-list:
    textColor: "{colors.muted}"
    typography: "{typography.data}"
  marking-hint:
    backgroundColor: "{colors.inset}"
    textColor: "{colors.text}"
    typography: "{typography.marking-hint}"
    rounded: "{rounded.inset}"
    padding: "10px 16px"
  text-field:
    backgroundColor: "{colors.inset}"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    rounded: "{rounded.button}"
    padding: "7px 10px"
---

# Design System: MapleCompass

## Overview

**Creative North Star: "The Scout’s Notes"**

MapleCompass is quiet, practical, and compact. Its interface reads like a small set of useful notes beside the game: a clear Map Name, tightly grouped facts, and brief signals when something needs attention. The game keeps the player's attention; the companion stays easy to consult.

Familiar, straightforward controls sit on Midnight Slate surfaces. Compass Gold identifies setup actions and marking feedback, while cool supporting text gives the Map Name and selected information room to lead. Rounded containers and translucent fills keep the system approachable without introducing ornamental depth.

**Key Characteristics:**

- Compact facts with a clear Map Name and aligned numeric columns.
- Midnight Slate surfaces, cool supporting text, and purposeful Compass Gold.
- Familiar, straightforward buttons, containers, and small descriptive tags.
- Flat tonal layers, subtle borders, and immediate state changes.

This is a record of the incumbent implementation in [src/styles.css](src/styles.css), [src/setup.css](src/setup.css), [index.html](index.html), [setup.html](setup.html), and [region.html](region.html), with the window geometry in [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json) and `src-tauri/src/lib.rs`. Static browser previews confirmed the CSS at the desktop window sizes; Tauri event delivery and game interactions were not exercised. The frontmatter follows the [DESIGN.md format](https://raw.githubusercontent.com/google-labs-code/design.md/main/docs/spec.md) and owns the exact token values. The sidecar extends it with previews and metadata.

## Colors

The palette combines Midnight Slate and cool light text with Compass Gold for controls and marking; status colors communicate recognition and data conditions.

### Primary

- **Compass Gold** (`accent`): primary setup buttons, the marking rectangle, and keyboard focus.
- **Gold Ink** (`accent-ink`): dark text on filled Compass Gold buttons.
- **Compass Gold washes** (`notice-fill`, `selection-fill`): the setup notice and the translucent interior of the Title Region selection.

### Secondary

- **Saved-copy Amber** (`warning`, `quest-fill`): problem notes, the uncertainty question mark, and the Quest tag. This is softer than the action accent.
- **Recognition Green** (`ok`): the recognised Map Name in the setup test read.
- **Retry Coral** (`bad`): the failed test-read border.

### Neutral

- **Midnight Slate** (`bg`): the opaque setup background. The overlay's related `panel` and `inset` fills preserve their distinct alpha values.
- **Clear Frost** (`text`): Map Names, primary text, and the selected Candidate.
- **Blue Grey** (`muted`): Street Names, table headers, progress, portal positions, and unselected Candidates.
- **Soft Frost** (`secondary-text`): ordinary notes, unreadable or uncertain Map Names, empty-state text, attribution, and footer hints.
- **Cool tag tones** (`tag-text`, `tag-fill`): descriptive Hidden Street tags.
- **Slate borders** (`secondary-border`, `inset-border`): outlined secondary buttons and the test-read container.
- **Marking scrim** (`scrim`): the dim layer across the game while marking the Title Region.

The existing icon uses `icon` turquoise and white artwork from [assets/icon.svg](assets/icon.svg). Turquoise belongs to that identity asset; it is not the setup action color.

**The Accent Has a Job Rule.** Use Compass Gold for the existing action and marking roles. Keep recognition, uncertainty, and Quest signals in their own established status tones.

## Typography

Segoe UI with system-ui and sans-serif fallbacks is the single interface family. Regular text carries supporting information; semibold text establishes Map Names, setup headings, primary buttons, selected Candidates, portal destinations, and summary values. There is no separate display or monospace family.

| Token | Use |
| --- | --- |
| `headline` | Instructional setup heading. |
| `window-title` | Setup window heading beside the icon. |
| `map-title` | Current Map Name in the overlay. |
| `test-map-title` | Recognised Map Name in the setup test read. |
| `body` | Setup instructions and secondary-button text. |
| `button` | Filled setup-button label. |
| `marking-hint` | Instruction over the game during Title Region marking. |
| `note` | Overlay status and problem explanations; the setup test's Street Name also uses this size. |
| `data` | Tables, Candidates, portal lists, and small section labels. |
| `street` | Overlay Street Name, with slight tracking. |
| `footer` | Attribution, source-page hotkey, and help/setup hint. |
| `tag` | Hidden Street and Quest labels. |

Setup inherits the body line-height; marking inherits its own hint line-height. The overlay uses the browser's normal line-height rather than a declared shared ratio. Do not impose the setup ratio on the overlay as an extracted fact. An unreadable overlay replaces the semibold Map Name with regular body-size text.

**The Facts Stay Aligned Rule.** Use tabular numerals in the tables, right-align numeric columns, and left-align the first descriptive column. Table headers remain regular; summary values are semibold.

## Layout

The spatial model is a compact desktop column, with separate overlay, setup, and marking windows. A container query reflows facts below 300px of available inline space. There are no web navigation layouts or documented mobile adaptations.

- **Overlay:** the shell maintains a fixed logical width (380px). Height follows the rendered content, bounded by the shell (44–900px); the initial expanded height is a starting value, not the final layout. The panel and data stack use the `group` gap, with `control-gap` vertical padding and `panel-inline` horizontal padding.
- **Setup:** fixed and non-resizable at 520 × 560 logical pixels, with 18px top, 24px side, and 20px bottom padding. The body scrolls when needed. Its wrapping header includes the icon, title, progress, and Overlay guide. One task or the guide shows at a time; actions stay at the bottom when space permits.
- **Marking:** the shell lays the window over the game client area. Instructions are centered, 12% from the bottom, and wrap within their inset surface. A drag or Arrow keys adjust the selection; coordinates appear beneath the controls.
- **Tables and lists:** tight table padding is 1px vertically, 10px before summary columns, and 6px before mob columns. Summary values have a 72px minimum width at normal scale; mob columns have fixed proportions. Portals retain a 4px item gap and a 6px destination/tag gap. Candidate items have two structured text rows with an 8px gap between items.
- **Overflow:** names clip where declared; Candidate clues, IDs, status notes, and footer hints wrap. The uncertainty marker stays outside the clipped Map Name. Below 300px of container width, mob facts reflow into labeled rows and portal destinations wrap. The overlay does not scroll; its native height is capped at 900 logical pixels, so forced text zoom with large data sets can exceed that limit.
- **Collapsed state:** the shell becomes a small square (44 × 44 logical pixels), containing only the icon. Window placement and screen fitting are managed by the core and shell, not CSS breakpoints.

## Elevation & Depth

Depth comes from flat tonal layers: the opaque setup background, translucent overlay and inset surfaces, a dim marking scrim, and small tinted labels. Test results and secondary buttons use subtle borders. There are no authored box shadows, text shadows, gradients, backdrop blurs, transitions, or animations. The overlay and marking windows explicitly disable window shadows; the setup window retains ordinary OS chrome.

**The Flat Layers Rule.** Preserve the current hierarchy with fill opacity, text emphasis, and restrained borders. Window stacking is a platform behavior, not a decorative elevation token.

## Shapes

Corners are gently rounded, with progressively smaller curves for panels, inset containers, buttons, and tags. Use the `rounded` tokens for those roles. The collapsed icon shares the panel corner treatment; the asset's own silhouette remains defined by its SVG.

The Title Region selection is a square-cornered, solid outline (2px). Setup notices have a left rule (3px) without a rounded outer container. These are distinct functional shapes, not interchangeable card variants.

## Components

### Buttons

Familiar, straightforward setup controls. The primary variant uses Compass Gold, Gold Ink, and semibold labels; the secondary variant uses a transparent fill, a Slate border (1px), and regular Clear Frost text. Both use the `button` corner token and the padding specified in their component tokens.

Disabled controls reduce opacity (0.45). Enabled buttons brighten slightly on hover. Keyboard focus uses a two-pixel Compass Gold outline with a three-pixel offset. There are no authored transitions. New setup steps focus their task heading; important results and status changes use live-status semantics.

### Overlay panel

The compact information container uses the translucent `panel` fill and its namesake corner token. A small Street Name precedes the semibold Map Name. Grouped data and a low-emphasis attribution footer follow beneath; credit and the source-page hint share a wrapping row, followed by the help/setup shortcut. The footer uses Soft Frost for contrast on bright game scenes.

An uncertain Map Name becomes Soft Frost and gains an amber question mark. When the panel can be dragged, its header uses a move cursor. Hidden and collapsed states switch the displayed content immediately.

### Map Summary and mob tables

Dense facts with aligned numbers. Headers use Blue Grey; summary values receive semibold emphasis. Mob rows remain regular, with the first column left-aligned and its long names clipped where needed. No row cards, striping, or decorative cell borders are established.

### Candidate list

A compact list of possible Maps and their distinguishing information. Mob names lead; level range, Hidden Portal count, and a secondary Map ID follow on a wrapping facts row. The shown Candidate is Clear Frost and semibold, prefixed by a triangular pointer; other rows are Blue Grey with equivalent leading space. The note above explains uncertainty or the player's choice and supplies the cycling hotkey. This is a keyboard-controlled list, not a set of clickable tabs.

### Hidden Portal list and tags

Semibold destinations sit above muted approximate positions. Small, noninteractive tags distinguish Hidden Street destinations and Quest Portals. Both tags use the `tag` corner token and padding from their component tokens; the Quest variant uses amber instead of the cool tag tones. Empty lists show plain supporting text.

### Setup notice

A tinted block with a Compass Gold left rule and body-size text. It communicates setup changes or unavailable hotkeys without adding a new modal or card hierarchy. Its spacing is defined by the `notice` component token, with a top margin (14px).

### Test read

An inset, bordered result container. Reading text uses Blue Grey. A recognised result displays its Street Name above a larger semibold green Map Name and a check mark. Failure switches the border to Retry Coral and supplies explanatory text. The Finish control is disabled until a Map is recognised.

### Title Region marking

A crosshair cursor over the marking scrim, a centered inset instruction, and a Compass Gold selection outline with a light interior wash. The hint and rectangle ignore pointer events, leaving the marking window to receive the drag. Arrow keys move the selection, Alt+Arrow resizes it, and Shift uses ten-pixel steps. Enter tests the box; Escape returns to setup. Selection bounds and adjustments belong to the core and act only on MapleCompass’s own window.

### Text field

The query box of the overlay's search view: the app's one text field. It fills its row on the `inset` fill, with a Slate border (1px), the `button` corner token and Clear Frost text; a small Blue Grey label sits above it, and its placeholder is Blue Grey. Keyboard focus uses the buttons' two-pixel Compass Gold outline with a three-pixel offset. Its suggestions drop below in an inset, bordered list with the `inset` corner token: rows are Blue Grey with the level right-aligned, and the highlighted row, the one Enter picks, is Clear Frost and semibold on the `selection-fill` wash. It is a keyboard-controlled combobox: the arrow keys move the highlight while the text box keeps the focus.

### Search view and Hunting section

The panel's header has a small 🔍 at its right that opens the search view: a small secondary icon button (22px square, transparent fill, Slate border, `button` corner). The whole panel is clickable, and the header is where it is dragged from.

The search view takes over the whole panel. A bar with a ← icon button (back, changing nothing) and a semibold note-size heading tops it. Its first step holds the text field and its suggestions; its second, headed by the picked mob (the level in Blue Grey), holds the nearest-map headline, every map in a results table with the highlighted one on the `selection-fill` wash and a sticky header, the travel checkboxes, and a key hint. Rows and suggestions take a click, with a faint Clear Frost wash on hover.

The Hunting section is its own section at the bottom of the normal view, divided from the current map's info by a Slate rule (1px) above. A semibold note-size heading names the Hunting Target ("Axe Stump · Deep Valley III"), with a 🔍 and a × icon button at its right. Below it sit Tier, Density, Share and Distance side by side (Blue Grey labels over the figures), then the route from the current map in an inset, bordered box.

The panel is never taller than the space where it is placed. When its content is longer, only its long part scrolls: the Hunting section's route, the suggestions, or the map list.

### App icon

Use the existing SVG and packaged icons. The setup header displays the icon at a small size (28 × 28px); collapsed mode fills its window and applies reduced opacity (0.85). The turquoise identity asset remains distinct from Compass Gold controls.

The app has no existing web navigation component, tooltip system, or custom animation vocabulary to extract.

## Do's and Don'ts

### Do:

- **Do** keep the Map Name prominent and supporting facts compact.
- **Do** preserve the distinct opaque setup, translucent panel, and inset surface roles.
- **Do** use the established status tones together with explanatory text, the uncertainty mark, or the recognised-result check mark.
- **Do** keep numeric columns aligned and status explanations able to wrap.
- **Do** retain familiar button variants, visible keyboard focus, and the visible disabled state.
- **Do** keep attribution and source-page hints legible within the low-emphasis footer.

### Don't:

- **Don't** substitute the icon's turquoise for the Compass Gold action accent.
- **Don't** turn descriptive portal tags or Candidate rows into controls without an explicit interaction change.
- **Don't** ellipsize status explanations using the name-row clipping treatment.
- **Don't** describe shadows, blur, hover animation, or mobile breakpoints as established parts of this system.
- **Don't** treat the overlay's starting height as a fixed content height.
- **Don't** turn preview-only tonal ramps or specimen data into product colors or factual map data.
