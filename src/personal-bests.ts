import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { dayText } from "./dates";
import { byId, span } from "./dom";
import { type LevelRange, levels, rate, sharePercent, whole } from "./figures";
import { mapMark, type MapMarks } from "./map-marks";

/** Mirror of `compass_core::PersonalBestsMap`. */
type BestsMap = {
  id: string;
  streetName: string;
  mapName: string;
  region: string;
  /** Its Map Mark, by OSMS's name for it. */
  mark: string | null;
  url: string;
  mobLevels: LevelRange | null;
  expPerHour: number;
  soloExpCeiling: number | null;
  ceilingShareHundredths: number | null;
  setAtUnixMs: number | null;
  here: boolean;
};

/** What the shell sends: whether the window shows at all, the core's rows, highest EXP/h
 *  first, and every Map Mark, which each row's map names its own by. */
type BestsView = { shown: boolean; maps: BestsMap[]; credit: string; mapMarks: MapMarks };

type Column = "map" | "region" | "levels" | "exp" | "share" | "set";
type Way = "ascending" | "descending";
type Key = string | number | [number, number] | null;

const panelEl = byId("panel");
const count = byId("count");
const rows = byId("rows");
const noteMissing = byId("note-missing");
const credit = byId("credit");
const announce = byId("sorted");
const headers = [...document.querySelectorAll<HTMLTableCellElement>("th[data-column]")];

/** What each column sorts by, which way it reads best first, and how a sort by it is
 *  said each way. A map without the figure goes last either way. */
const COLUMNS: Record<Column, { name: string; key: (m: BestsMap) => Key; first: Way; ways: [string, string] }> = {
  map: { name: "map", key: (m) => m.mapName.toLowerCase() || null, first: "ascending", ways: ["A to Z", "Z to A"] },
  region: { name: "region", key: (m) => m.region.toLowerCase() || null, first: "ascending", ways: ["A to Z", "Z to A"] },
  levels: {
    name: "mob levels",
    key: (m) => (m.mobLevels ? [m.mobLevels.lowest, m.mobLevels.highest] : null),
    first: "ascending",
    ways: ["lowest first", "highest first"],
  },
  exp: { name: "best EXP/h", key: (m) => m.expPerHour, first: "descending", ways: ["lowest first", "highest first"] },
  share: {
    name: "share of solo EXP/h",
    key: (m) => m.ceilingShareHundredths,
    first: "descending",
    ways: ["lowest first", "highest first"],
  },
  set: { name: "date set", key: (m) => m.setAtUnixMs, first: "descending", ways: ["oldest first", "newest first"] },
};

function compare(a: Key, b: Key): number {
  if (Array.isArray(a) && Array.isArray(b)) return compare(a[0], b[0]) || compare(a[1], b[1]);
  return a! < b! ? -1 : a! > b! ? 1 : 0;
}

/** The maps sorted by `column`, `way`. Ties keep the core's order: highest EXP/h. */
function sorted(maps: BestsMap[], { column, way }: { column: Column; way: Way }) {
  const { key } = COLUMNS[column];
  const sign = way === "ascending" ? 1 : -1;
  return [...maps].sort((a, b) => {
    const [x, y] = [key(a), key(b)];
    if (x === null || y === null) return Number(x === null) - Number(y === null);
    return sign * compare(x, y);
  });
}

/** A day by the calendar in the player's local time: "3 Oct" this year, "3 Oct 2025"
 *  another. */
function day(unixMs: number) {
  const date = new Date(unixMs);
  return dayText(date, date.getFullYear() !== new Date().getFullYear());
}

/** A cell, labelled for the narrowest layout, where each figure stands under its label. */
function cell(label: string | null, className?: string) {
  const td = document.createElement("td");
  if (label) td.dataset.label = label;
  if (className) td.className = className;
  return td;
}

/** A figure that isn't known: a dash, and why. */
function missing(td: HTMLTableCellElement, why: string) {
  td.classList.add("none");
  td.title = why;
  const dash = span("", "—");
  dash.setAttribute("aria-hidden", "true");
  td.append(dash, span("sr-only", why));
  return td;
}

/** Why a map's figures aren't known: it isn't in the map list, or, for its levels and
 *  share, no mobs spawn there. */
const NOT_LISTED = "Not in the map list";
const NO_MOBS = "No mobs spawn on this map";

/** The star on the best of all maps: the Grind Tracker's new-best star. */
function star() {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("class", "best-star");
  svg.setAttribute("viewBox", "0 0 16 16");
  svg.setAttribute("role", "img");
  svg.setAttribute("aria-label", "Your best of all maps");
  const title = document.createElementNS(ns, "title");
  title.textContent = "Your best of all maps";
  const path = document.createElementNS(ns, "path");
  path.setAttribute("d", "M8 1.5L9.76 6.07L14.66 6.34L10.85 9.43L12.11 14.16L8 11.5L3.89 14.16L5.15 9.43L1.34 6.34L6.24 6.07Z");
  const lit = document.createElementNS(ns, "ellipse");
  lit.setAttribute("class", "lit");
  for (const [k, v] of Object.entries({ cx: 7.1, cy: 6.9, rx: 1.5, ry: 1.1 })) lit.setAttribute(k, String(v));
  svg.append(title, path, lit);
  return svg;
}

function row(m: BestsMap, best: BestsMap | null, marks: MapMarks) {
  const tr = document.createElement("tr");
  const known = m.mapName !== "";

  const mapCell = cell(null, "text map-cell");
  if (m.here) {
    const dot = span("here-dot", "");
    dot.setAttribute("role", "img");
    dot.setAttribute("aria-label", "You're here");
    dot.title = "You're here";
    mapCell.append(dot);
  }
  // The name opens the map's MeowDB page in the browser, as a mob's name in the panel does.
  const name = document.createElement("button");
  name.type = "button";
  name.className = "row-map";
  name.dataset.id = m.id;
  name.title = "Open on MeowDB";
  name.append(known ? m.mapName : `Map ${m.id}`, span("sr-only", " (opens MeowDB in your browser)"));
  // The map's Map Mark before its name; the street under the name, not under the mark.
  const named = document.createElement("div");
  named.className = "row-name";
  named.append(mapMark(marks, m.mark), name);
  mapCell.append(named);
  const street = document.createElement("div");
  street.className = known ? "row-street" : "row-street unknown";
  if (known) street.append(m.streetName, span("region", ` · ${m.region}`));
  else street.textContent = NOT_LISTED;
  mapCell.append(street);
  tr.append(mapCell);

  const region = cell(null, "text region-cell");
  if (known) region.textContent = m.region;
  else missing(region, NOT_LISTED);
  tr.append(region);

  const lv = cell("Mob levels");
  if (m.mobLevels) lv.textContent = levels(m.mobLevels);
  else missing(lv, known ? NO_MOBS : NOT_LISTED);
  tr.append(lv);

  const exp = cell("Best EXP/h", "exp");
  if (m === best) exp.append(star());
  exp.append(rate(m.expPerHour));
  tr.append(exp);

  const share = cell("% of solo");
  if (m.ceilingShareHundredths !== null && m.soloExpCeiling !== null) {
    share.append(sharePercent(m.ceilingShareHundredths), span("ceiling", `of ${whole(m.soloExpCeiling)}`));
  } else {
    missing(share, !known ? NOT_LISTED : m.mobLevels ? "No solo EXP/h for this map" : NO_MOBS);
  }
  tr.append(share);

  const date = cell("Date set", "date");
  if (m.setAtUnixMs !== null) date.textContent = day(m.setAtUnixMs);
  else missing(date, "Set before MapleCompass knew the date");
  tr.append(date);
  return tr;
}

/** The rows as last sent, and how the player sorted them: kept until MapleCompass closes,
 *  so the window opens again as they left it. */
let maps: BestsMap[] = [];
let marks: MapMarks = {};
let sort: { column: Column; way: Way } = { column: "exp", way: "descending" };

function renderRows() {
  // The best of all maps, which the star marks; none with only one map to compare.
  const best = maps.length > 1 ? maps[0] : null;
  rows.replaceChildren(...sorted(maps, sort).map((m) => row(m, best, marks)));
  for (const th of headers) {
    if (th.dataset.column === sort.column) th.setAttribute("aria-sort", sort.way);
    else th.removeAttribute("aria-sort");
  }
}

function render(view: BestsView) {
  // Not showing, the window draws nothing (the shell lets clicks through it).
  document.body.classList.toggle("hidden", !view.shown);
  maps = view.maps;
  marks = view.mapMarks;
  count.textContent = `${maps.length === 1 ? "1 map" : `${maps.length} maps`} · shared by all your characters`;
  credit.textContent = view.credit;
  noteMissing.hidden = maps.every((m) => m.mobLevels !== null && m.ceilingShareHundredths !== null);
  renderRows();
  reportHeight();
}

for (const th of headers) {
  th.querySelector("button")!.addEventListener("click", () => {
    const column = th.dataset.column as Column;
    const flipped: Way = sort.way === "ascending" ? "descending" : "ascending";
    const way = column === sort.column ? flipped : COLUMNS[column].first;
    sort = { column, way };
    renderRows();
    announce.textContent = `Sorted by ${COLUMNS[column].name}, ${COLUMNS[column].ways[way === "ascending" ? 0 : 1]}.`;
  });
}

// The shell sizes the window to the content, as the Grind Tracker's is, up to its most;
// past that the table scrolls. The panel is no taller than the window, so it is measured
// without that limit, or the window could never grow.
let reportedHeight = 0;
function reportHeight() {
  panelEl.classList.add("measuring");
  const height = Math.ceil(panelEl.getBoundingClientRect().height);
  panelEl.classList.remove("measuring");
  if (height > 0 && height !== reportedHeight) {
    reportedHeight = height;
    invoke("personal_bests_height", { height });
  }
}
// Narrowed to a smaller game, the rows lay out again, as cards at the narrowest.
window.addEventListener("resize", reportHeight);
// The height depends on the font: measure again once it has loaded.
document.fonts.ready.then(reportHeight);

// Dragged by its header, never by its button. A click leaves the keyboard with the game:
// the window can't be activated.
document.addEventListener("mousedown", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const onHeader = !!target?.closest(".drag-handle") && !target?.closest("button");
  if (e.button === 0 && onHeader) getCurrentWindow().startDragging();
});

// Its ×, through the core like the panel's icons; a map's name, to its MeowDB page.
panelEl.addEventListener("click", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const control = target?.closest<HTMLButtonElement>("button[data-control]");
  if (control) invoke("panel_press", { control: control.dataset.control });
  const map = target?.closest<HTMLButtonElement>("button.row-map");
  if (map) invoke("personal_bests_open_map", { id: map.dataset.id });
});

listen<BestsView>("personal-bests-state", (e) => render(e.payload));
// The rows may have been sent before the page was listening.
invoke<BestsView | null>("personal_bests_view").then((view) => {
  if (view) render(view);
});
reportHeight();
