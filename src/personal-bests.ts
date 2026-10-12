import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { dayText } from "./dates";
import { byId, span } from "./dom";
import { type LevelRange, levels, rate, sharePercent, whole } from "./figures";
import { mapMark, type MapMarks } from "./map-marks";

/** Mirror of `minerva_core::PersonalBestsVisit`: one of a map's Visits. */
type BestsVisit = {
  expPerHour: number;
  setAtUnixMs: number | null;
  ceilingShareHundredths: number | null;
  /** The Visit under way's best: not in the Visit History until the Visit ends, so it
   *  can't be removed yet. */
  thisVisit: boolean;
};

/** Mirror of `minerva_core::PersonalBestsMap`. */
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
  /** Its Visit History, highest first, the Visit under way's best among them. */
  visits: BestsVisit[];
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
const footer = byId("footer");
const credit = byId("credit");
const announce = byId("announce");
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
const NO_DATE = "Set before Minerva knew the date";

/** A map's name as the window writes it. */
const mapName = (m: BestsMap) => m.mapName || `Map ${m.id}`;

/** Why a map's share isn't known. */
function noShare(m: BestsMap) {
  if (m.mapName === "") return NOT_LISTED;
  return m.mobLevels ? "No solo EXP/h for this map" : NO_MOBS;
}

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
  name.append(mapName(m), span("sr-only", " (opens MeowDB in your browser)"));
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
    missing(share, noShare(m));
  }
  tr.append(share);

  const date = cell("Date set", "date");
  if (m.setAtUnixMs !== null) date.textContent = day(m.setAtUnixMs);
  else missing(date, NO_DATE);
  tr.append(date);

  // Its Visit History opens under the row, and the count says how many it holds.
  const controls = cell(null, "controls");
  const open = opened.has(m.id);
  const toggle = document.createElement("button");
  toggle.type = "button";
  toggle.className = "visits-toggle";
  toggle.dataset.id = m.id;
  toggle.setAttribute("aria-expanded", String(open));
  toggle.title = open ? "Hide its visits" : "Show its visits";
  const count = m.visits.length === 1 ? "1 visit" : `${m.visits.length} visits`;
  toggle.append(count, span("sr-only", ` on ${mapName(m)}`), chevron());
  controls.append(toggle);
  tr.append(controls);
  return tr;
}

/** The disclosure's chevron: pointing right while the map is closed, down while open. */
function chevron() {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("class", "chevron");
  svg.setAttribute("viewBox", "0 0 8 8");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(ns, "path");
  path.setAttribute("d", "M3 1.5L5.5 4L3 6.5");
  svg.append(path);
  return svg;
}

/** Each of a map's Visits by a key of its own: its map, its best and, for Visits whose
 *  bests are the same, which of them it is. `null` for the Visit under way's, which has no
 *  ×. */
function visitKeys(m: BestsMap): (string | null)[] {
  const seen = new Map<string, number>();
  return m.visits.map((v) => {
    if (v.thisVisit) return null;
    const best = `${m.id} ${v.expPerHour} ${v.setAtUnixMs ?? "-"}`;
    const n = seen.get(best) ?? 0;
    seen.set(best, n + 1);
    return `${best} ${n}`;
  });
}

/** One of an opened map's Visits, under its row: its best's EXP/h, share and date under
 *  the row's, then its × (or, for the Visit under way's, "This visit"). */
function visitRow(m: BestsMap, v: BestsVisit, key: string | null, place: number, last: boolean) {
  const tr = document.createElement("tr");
  tr.className = last ? "visit last" : "visit";

  const first = cell(null, "text blank");
  first.append(span("sr-only", `${mapName(m)}, visit ${place} of ${m.visits.length}`));
  tr.append(first, cell(null, "text region-cell blank"), cell(null, "blank"));

  const exp = cell(null, "visit-exp");
  exp.textContent = rate(v.expPerHour);
  tr.append(exp);

  const share = cell(null, "visit-share");
  if (v.ceilingShareHundredths !== null) share.textContent = sharePercent(v.ceilingShareHundredths);
  else missing(share, noShare(m));
  tr.append(share);

  const date = cell(null, "date visit-date");
  if (v.setAtUnixMs !== null) date.textContent = day(v.setAtUnixMs);
  else missing(date, NO_DATE);
  tr.append(date);

  const controls = cell(null, "controls");
  if (key === null) controls.append(span("this-visit", "This visit"));
  else controls.append(removeButton(m, v, key));
  tr.append(controls);
  return tr;
}

/** A Visit's ×, as the Grind Tracker's Best row has; once clicked, "Remove?", which
 *  removes it with a second click. */
function removeButton(m: BestsMap, v: BestsVisit, key: string) {
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.visit = key;
  button.dataset.id = m.id;
  button.dataset.exp = String(v.expPerHour);
  if (v.setAtUnixMs !== null) button.dataset.set = String(v.setAtUnixMs);
  if (armed === key) {
    button.className = "text-button remove armed";
    button.textContent = "Remove?";
    button.title = "Click again to remove this visit from your history";
  } else {
    button.className = "icon-button light remove";
    button.setAttribute("aria-label", "Remove this visit from your history");
    button.title = "Remove this visit from your history";
    button.innerHTML = '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" /></svg>';
  }
  return button;
}

/** The rows as last sent, and how the player sorted them: kept until Minerva closes,
 *  so the window opens again as they left it. */
let maps: BestsMap[] = [];
let marks: MapMarks = {};
let sort: { column: Column; way: Way } = { column: "exp", way: "descending" };
/** The maps opened to their Visit History, by ID, kept as the sort is. */
const opened = new Set<string>();
/** The × clicked once, by its Visit's key, until the second click removes it, a click
 *  anywhere else, the window hiding or the Visit going: one at a time. */
let armed: string | null = null;

function renderRows() {
  // The best of all maps, which the star marks; none with only one map to compare.
  const best = maps.length > 1 ? maps[0] : null;
  const keys = new Map(maps.map((m) => [m.id, visitKeys(m)]));
  if (armed !== null && ![...keys.values()].some((k) => k.includes(armed))) armed = null;
  rows.replaceChildren(
    ...sorted(maps, sort).flatMap((m) => {
      if (!opened.has(m.id)) return [row(m, best, marks)];
      const mapKeys = keys.get(m.id)!;
      const visits = m.visits.map((v, i) => visitRow(m, v, mapKeys[i], i + 1, i === m.visits.length - 1));
      return [row(m, best, marks), ...visits];
    }),
  );
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
  // Hidden, nothing stays armed. Closed, the window has no rows, and the maps opened stay
  // opened for when it opens again; open, a map whose row went is no longer opened.
  if (!view.shown) armed = null;
  if (maps.length > 0) {
    for (const id of opened) if (!maps.some((m) => m.id === id)) opened.delete(id);
  }
  count.textContent = `${maps.length === 1 ? "1 map" : `${maps.length} maps`} · shared by all your characters`;
  // The credit as the main window's info icon, its words in its tooltip.
  footer.hidden = !view.credit;
  credit.title = view.credit;
  credit.setAttribute("aria-label", view.credit);
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
  if (e.button === 0 && onHeader) invoke("start_drag");
});

// Its ×, through the core like the panel's icons; a map's name, to its MeowDB page; a
// map's count, opening or closing its Visit History; a Visit's ×, arming it, and again,
// removing it through the core. Any other click disarms it.
document.addEventListener("click", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const control = target?.closest<HTMLButtonElement>("button[data-control]");
  if (control) invoke("panel_press", { control: control.dataset.control });
  const map = target?.closest<HTMLButtonElement>("button.row-map");
  if (map) invoke("personal_bests_open_map", { id: map.dataset.id });
  const toggle = target?.closest<HTMLButtonElement>("button.visits-toggle");
  const remove = target?.closest<HTMLButtonElement>("button.remove");
  const before = armed;
  armed = null;
  if (remove && remove.dataset.visit === before) {
    const set = remove.dataset.set;
    invoke("personal_bests_remove_visit", {
      map: remove.dataset.id,
      expPerHour: Number(remove.dataset.exp),
      setAtUnixMs: set === undefined ? null : Number(set),
    });
  } else if (remove) {
    armed = remove.dataset.visit ?? null;
    announce.textContent = "Click Remove? again to remove this visit from your history.";
  }
  if (toggle) {
    const id = toggle.dataset.id!;
    if (!opened.delete(id)) opened.add(id);
  }
  if (toggle || armed !== before) {
    renderRows();
    reportHeight();
  }
});

listen<BestsView>("personal-bests-state", (e) => render(e.payload));
// The rows may have been sent before the page was listening.
invoke<BestsView | null>("personal_bests_view").then((view) => {
  if (view) render(view);
});
reportHeight();
