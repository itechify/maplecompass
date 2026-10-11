---
target: github pages website
total_score: 28
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 2
target_identity: "file:/home/jjd/.t3/worktrees/maplecompass/t3code-fe939ad0/site/index.html"
target_fingerprint: "sha256:2cc87afb7fc992334881e447b17b3cb9baed07c473edf1264894d2bac07f1700"
target_path: /home/jjd/.t3/worktrees/maplecompass/t3code-fe939ad0/site/index.html
timestamp: 2026-10-11T01-51-56Z
slug: site-index-html
---
Method: dual-agent (A: design-review sub-agent · B: detector sub-agent)

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Raised tab follows the scroll with aria-current; nothing says what the Download gives you or what Windows will say next |
| 2 | Match System / Real World | 3 | "Hunting Target" used at index.html:251, :335 before :418 defines it; Density Tier / Spawn Share (:406-407) unexplained; "notification area", "Overlay Guide" |
| 3 | User Control and Freedom | 3 | Anchors give Back history; sticky head reaches any part |
| 4 | Consistency and Standards | 3 | Tabs look like panes but are scroll anchors; setup screenshot is the reopened setup ("Step 3 of 6", "Keep the current title") under "On first run" (:199) |
| 5 | Error Prevention | 2 | SmartScreen "Run anyway" and windowed/borderless only in Install (:173-178), after the Download; phones get the .exe silently |
| 6 | Recognition Rather Than Recall | 3 | Drawn legend/rings/tags strong; search view and Personal Bests only in words; hero panel illegible |
| 7 | Flexibility and Efficiency | 3 | Sticky tabs, in-tab rows, stable ids, skip link, Keys table; no copyable heading links |
| 8 | Aesthetic and Minimalist Design | 3 | Restrained chrome; edge cases (.bad rename, single instance, Loading map…, stroke counting) at full weight |
| 9 | Error Recovery | 2 | No "when it doesn't work" topic; no report-a-problem link |
| 10 | Help and Documentation | 3 | Ordered by the reader's questions, one-page Ctrl+F; long, no troubleshooting |
| **Total** | | **28/40** | **Good (bottom of band)** |

## Design Specificity Verdict

LLM: the frame is unmistakably MapleCompass (grey bar + compass, raised Light Steel tabs, steel Download, toolbar legend in the panel's game-blue buttons, ring keys/tags/Not sure chip drawn as the panel draws them). Inside, it is a generic docs article: a 38em column with a figure at right and an empty right half where there is none (How it stays safe, Search a mob, Map Summary, Personal Bests). Tabs are scroll anchors on a 9.6k px page (16.5k on phones). The hero shows the panel at ~100x280px in a 594px game window, illegible: it sells MapleStory, not MapleCompass.

Deterministic: detect site/index.html: 0 findings, 1 advisory (flattened tab color-mix, FP). detect site/: 11 (3 broken-image regex literals in vite.config.ts FP; 4 font-size advisories all documented in DESIGN.md typography prose, FP/sidecar gap; 4 color advisories: 3 inset :active press shadows documented in Shadow Vocabulary prose but untokenised, plus the flattened tab colour). Rendered 1280 and 390: 6 (5 nested-cards on the deliberate window frame, FP; 1 low-contrast on span.chip-doubt 1.6:1, real, also caught by review).

Overlays injected in a [Human] preview tab; same 6 findings at every scroll position and both widths.

## Priority Issues

- [P1] Hero doesn't show the product. hero.png at 31%, panel illegible (index.html:84-92, site.css:325-327); same image is og:image. Fix: hero = 1:1 crop of minimap title + panel from the existing hero.png; whole-window shot moves to The panel/Placement; crop used for the card. Command: /impeccable layout
- [P1] Download friction not addressed at the Download. SmartScreen + windowed/borderless only in Install (:173-178); public source only in foot fine print (:646-648). Fix: one fine-print line under the hero Download; "source is public" bullet in How it stays safe; problem note stays word for word. Command: /impeccable clarify
- [P2] Spec-dense reading path; terms before definitions (Hunting Target :251/:335 vs :418; Density Tier/Spawn Share :406-407; edge cases :162-165, :219-222, :329-333, :378-382; %LOCALAPPDATA% before Install). Fix: lead with see/do, exceptions into asides, define at first use, crops for Search and Personal Bests. Command: /impeccable distill
- [P2] Phone visitors: 2.9MB hero PNG fetchpriority=high at 352px, 520KB fonts, .exe Download with no "on your PC" note, 32px tabs, 16/14px link rows, 130px sticky head. Fix: srcset WebP/AVIF, phone install note, 44px tabs, 32px link rows. Command: /impeccable adapt, /impeccable optimize
- [P2] No recovery path. Fix: "When it doesn't work" topic after Setup mapping symptoms to fixes + GitHub Issues link. Command: /impeccable harden

## Persona Red Flags

- Jordan: Hunting Target before definition; setup shot shows reopened setup (Step 3 of 6, Keep the current title) vs 7 first-run steps (src/setup.ts:40) and a 6-item list; Overlay Guide / notification area; SmartScreen after the click.
- Casey: phone Download fetches an .exe; ~3.4MB up front; 15% sticky head; 32/31/16/14px targets; 16.5k px page.
- Sam: Not sure chip 1.4-2.0:1 at 11px; phone open tab 1.18:1 vs siblings (site.css:808 vs :819); link underlines 1.83:1; font swap moves Download ~28px.
- Min (cautious Classic World main): told to bypass SmartScreen with no counterweight; public source only in fine print; "at your own risk" x3 with no "how to check"; no maintainer named; no link to Nexon's terms.

## Minor Observations

- h1 balanced break splits at the seam ("The map you're / on, at a glance").
- Problem note orphans "risk."; hero fine print orphans "What's new" (text-wrap: pretty).
- "How it stays safe" twice within ~150px (:76, :97); 8 links in the hero column.
- Items: opener + single topic.
- "Next:" buttons point at content directly below (~77px per part).
- item-lookup.png at 76%, alert text ~8px.
- setup.png lower half empty lavender.
- Bar's version/platform repeats the hero fine print.
- NEXON vs Nexon (:71 vs :105) is inherited from the app's two verbatim texts (setup.html:26, :159).

## Questions to Consider

- Why do raised tabs scroll a 16.5k px page instead of showing one pane, with the long page as no-script fallback?
- Who is the hero picture for, when every player knows Henesys?
- "At your own risk" x3, "Run anyway" x1, strongest trust signal at 13px at the bottom: what does a cautious player need before clicking?
- Does safety deserve its own tab?
