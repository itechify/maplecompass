---
version: 1
slug: "index-html"
primary_target: "index.html"
related_targets: ["src/main.ts","src/styles.css"]
---

# Overlay panel

Scope: the in-game overlay panel (index.html, src/main.ts, src/styles.css), every state it can show. Mode: Operate. A glanceable, click-through tool read mid-hunt.

Job: on each map change, tell the player which map this is, where its Hidden Portals are, and whether its mobs are worth grinding, in one glance. Constraints: fixed 380 px width, height fits the content (44–900 px), 44 px collapsed square, no hover or pointer interaction except dragging in edit mode, credit "NiaMeowDB (meowdb.com)" wherever MeowDB data shows.

Scene: a player at a desk, the bright, saturated 2D game filling the screen, glancing at the right edge between attacks. The game's own windows there (Quest Helper, minimap) are light, translucent periwinkle and lavender, so the panel is light.

## Direction contract

THESIS: The panel is the window the game forgot to ship. It sits under the Quest Helper in the game's own window language, and puts hidden exits first. It refuses the generic dark dev-tools card with a stats table on top. Boundary: the game lends materials (body, chrome, lettering, minimap glyphs) and one signature move. Organisation, hierarchy and density are a working tool's, never a screenshot of a game window.

OWN-WORLD: Translucent lavender-grey body as in the Quest Helper. Steel-periwinkle title bar with white top highlight and dark-navy hairline. The Map Name in white bold lettering with a dark-navy outline, as the minimap draws it. Navy text, steel secondary text. The minimap's colour vocabulary: blue portal rings, red mob dots. Game-orange is reserved for doubt and problems. Arial-family sans, because the game uses one (a pinned-world override of the system-display-face rule). Street Name above Map Name stays, because it is the minimap's own idiom (a pinned-world override of the eyebrow ban).

STORY: The player learns the map, sees each hidden exit's spot and where it leads, checks EXP/h, and gets back to hunting. When unsure, the panel says so before anything else.

FIRST VIEWPORT: Title bar with the street name and the outlined map name, full width. Under it, Hidden Portals: each row is a 30×18 mini-minimap with a blue ring at the portal's cell, then the destination and its tags, then the position in words. Then one summary line ("EXP/h 6,180 solo · 27,900 6+"), then up to 5 mobs (red dot, name, Lv, HP, EXP, share) and a "+N more · Lv x–y" line, then the credit footer. Signature move: Hidden Portals drawn on miniature minimaps with the game's portal ring.

FORM: user-pinned, no roll (the user chose "lean into MapleStory" and confirmed this direction). Code-led: no image generation is available.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
