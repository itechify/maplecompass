/// <reference types="vite/client" />
// The site's behaviours: the tab of the part being read is raised, as the game raises the
// open tab, and on a wide screen the manual's minimap shows where you are in it
// (minimap.ts), both following the scroll; the hero films how MapleCompass reads the map
// you're on as the page scrolls past it (reel.ts). Without the script the page still reads
// whole, the hero is its still shot, and the tabs still jump to their parts.

import { minimap } from "./minimap.ts";
import { reel } from "./reel.ts";

const head = document.querySelector<HTMLElement>(".window-head")!;
const strip = document.querySelector<HTMLElement>(".tabs")!;
const tabs = [...strip.querySelectorAll<HTMLAnchorElement>("a.tab")];

/** What sticks at the top: the window's head, or on a phone the tabs alone (site.css). */
const stuck = () => (getComputedStyle(head).display === "contents" ? strip : head);

// A jump to a heading lands below what sticks, however tall it wraps.
const heads = new ResizeObserver(() => {
  document.documentElement.style.setProperty("--head", `${stuck().offsetHeight}px`);
});
heads.observe(head);
heads.observe(strip);

// The hero's film, run by the scroll while the hero stays put under what sticks.
const film = reel(() => stuck().offsetHeight);
// A heading is reached 48px below what sticks.
const map = minimap(tabs, () => stuck().offsetHeight + 48);
let raised = -1;

/** Run the hero's film, raise the tab of the part being read, and put the reader on the minimap. */
function follow() {
  film.draw(scrollY);
  const where = map.at(scrollY);
  map.draw(where);
  // The credits are read under the last tab.
  const current = Math.min(where.part, tabs.length - 1);
  if (current === raised) return;
  raised = current;
  tabs.forEach((tab, i) => {
    if (i === current) tab.setAttribute("aria-current", "location");
    else tab.removeAttribute("aria-current");
  });
  // On a narrow screen the strip scrolls sideways: keep the raised tab in it.
  const tab = tabs[current];
  if (tab.offsetLeft < strip.scrollLeft) strip.scrollLeft = tab.offsetLeft - 12;
  else if (tab.offsetLeft + tab.offsetWidth > strip.scrollLeft + strip.clientWidth) {
    strip.scrollLeft = tab.offsetLeft + tab.offsetWidth - strip.clientWidth + 12;
  }
}

/**
 * Fit the film to the screen, and read where every heading lies again, after the page's
 * layout changes: the film's stretch of scroll moves every heading after it.
 */
function remeasure() {
  film.measure();
  map.measure();
  follow();
}

addEventListener("scroll", follow, { passive: true });
addEventListener("resize", remeasure);
new ResizeObserver(remeasure).observe(document.querySelector(".window")!);
document.fonts.ready.then(remeasure);
remeasure();

// A screenshot that doesn't load leaves a dashed box saying what belongs there. While
// developing, that is a shot not taken yet; the build refuses to publish without it.
for (const img of document.querySelectorAll<HTMLImageElement>(".shot img")) {
  const figure = img.closest("figure")!;
  const missing = () => {
    figure.dataset.missing = import.meta.env.DEV
      ? `Not taken yet: site/${img.getAttribute("src")?.replace(/^\.\//, "")}. See site/screenshots/README.md.\n\n${img.alt}`
      : img.alt;
    figure.classList.add("missing");
  };
  if (img.complete && img.naturalWidth === 0) missing();
  else img.addEventListener("error", missing);
}
