---
version: 1
slug: "site-index-html"
primary_target: "site/index.html"
related_targets: ["site/site.css","site/site.ts"]
---

# Surface: the MapleCompass site

## Scope and mode
One page for players at itechify.github.io/maplecompass, built from `site/index.html`, `site/site.css` and `site/site.ts`. Mode: **Read** (a player manual), with a Download in its head. Spec: GitHub issue #64.

## Audience, job and constraints
- The average MapleStory Classic player: deciding whether to install MapleCompass, then learning what each part of the panel does.
- The reader's questions, in order: what is it, is it safe for my account, how do I install and set it up, what does each part show, what are the keys.
- Content comes from the app as it ships (the release the page describes), in the panel's own words and GLOSSARY.md's terms. The setup warning and the NEXON disclaimer are word for word.
- Real screenshots only, taken by hand (`site/screenshots/README.md`); the build fails while one is missing.
- No analytics, cookies or third-party requests; fonts served from the site.

## Direction contract
THESIS: The manual is one of the game's own windows, opened wide: its window bar carries the name and Download, and the game's raised window tabs file the manual as a player meets the app (Get started, Where you are, Hunting, Grinding, Items, Keys). It refuses the docs-site default of a sidebar tree on white.
OWN-WORLD: Setup's opaque Window Lavender body under the Mini Map's grey window bar, framed in Window Edge, standing on Quest Helper Glass over white as the Personal Bests page does. Raised tabs in Light Steel, the open one joined to the lavender. Steel primary and light-steel secondary buttons, Tag Periwinkle tags, the problem note only for the account risk, the minimap legend marks on the Map, Navigation and Mobs headings. Maplestory throughout; no motion.
STORY: A player learns what MapleCompass shows, believes it only looks at the screen, downloads it, and later returns to a tab to look up one feature or key.
FIRST VIEWPORT: The window fills the width (up to 1200px), 28px down. Bar: 32px compass, "MapleCompass", version and platform, Download (steel) at the right. Under it the six tabs, "Get started" raised. Body: left, the title, a two-paragraph lede, the risk note and Download beside "How it stays safe"; right, the hero shot framed as the game window, caption under it.
FORM: Inventory tabs, position 5 of 7 on the ordered list; seed key c9497a89. Signature interaction: the bar and tabs stick as the window's head, and the tab of the group you are reading is raised as the game raises the open tab, following the scroll; a tab jumps to its group.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Unresolved
- The screenshots: taken by the owner before the first deploy.
