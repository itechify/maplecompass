import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hotkeyAvailable, keyLabel, type Hotkeys } from "./hotkeys";
import {
  TIER_NAMES,
  crawlLine,
  density,
  distanceText,
  distanceWithTravel,
  nearestLine,
  resultRow,
  resultsHead,
  resultsNote,
  routeStops,
  type HuntingTarget,
  type SearchState,
} from "./search-state";

type PlacementMode = "inside" | "dockedRight" | "dockedLeft" | "detached";

/** Mirror of `compass_core::PerParty`. */
type PerParty = { solo: number; fullParty: number };

/** Mirror of `compass_core::Mob`. */
type Mob = {
  id: string;
  name: string;
  level: number;
  hp: number;
  exp: number;
  spawnCount: number;
  spawnSharePercent: number;
};

/** Mirror of `compass_core::HiddenPortal`. */
type HiddenPortal = {
  name: string;
  toMapId: string;
  toMapName: string;
  toStreetName: string;
  toHiddenStreet: boolean;
  quest: boolean;
  position: string;
};

/** Mirror of `compass_core::MapData`. */
type MapData = {
  summary: { mobsPerHour: PerParty; expPerHour: PerParty } | null;
  mobs: Mob[];
  hiddenPortals: HiddenPortal[];
};

/** Mirror of `compass_core::MapDataStatus`. */
type MapDataStatus =
  | { status: "noMap" | "loading" | "unavailable" | "formatChanged" }
  | ({ status: "ready" } & MapData)
  | ({ status: "savedCopy"; savedAtUnixMs: number } & MapData);

/** Mirror of `compass_core::Recognition`. */
type Recognition =
  | { status: "starting" | "ready" | "unavailable" }
  | { status: "savedCopy"; savedAtUnixMs: number };

/** Mirror of `compass_core::Outline`. */
type Outline = {
  mobLevels: { lowest: number; highest: number } | null;
  firstMobs: string[];
  hiddenPortals: number;
};

/** Mirror of `compass_core::Candidates`. */
type Candidates = {
  maps: { id: string; outline: Outline | null }[];
  shown: number;
  chosen: boolean;
};

/** Mirror of `compass_core::PanelState`. */
type PanelState = {
  recognition: Recognition;
  map: { id: string; streetName: string; mapName: string; url: string } | null;
  confidence: number;
  notSure: boolean;
  candidates: Candidates | null;
  data: MapDataStatus;
  credit: string | null;
  expanded: boolean;
  gameFound: boolean;
  placementMode: PlacementMode;
  layer: "aboveGame" | "free" | "hidden";
  /** The player can drag the panel by its header (inside or detached). */
  draggable: boolean;
  setup: { needed: "firstRun" | "resolutionChanged" | "requested" | null };
  hotkeys: Hotkeys;
  unavailableHotkeys: string[];
  /** The shell has the hotkeys registered: the game or one of our windows is in front. */
  hotkeysLive: boolean;
  gameWindowTitle: string;
  search: SearchState;
};

const byId = (id: string) => document.getElementById(id)!;
const panelEl = byId("panel");
const street = byId("street");
const map = byId("map");
const dataEl = byId("data");
const note = byId("note");
const candidatesEl = byId("candidates");
const candidatesNote = byId("candidates-note");
const candidateRows = byId("candidate-rows");
const summary = byId("summary");
const mobsTable = byId("mobs");
const mobRows = byId("mob-rows");
const portals = byId("portals");
const noPortals = byId("no-portals");
const portalRows = byId("portal-rows");
const footer = byId("footer");
const credit = byId("credit");
const openHint = byId("open-hint");
const hotkeyProblem = byId("hotkey-problem");
const recovery = byId("recovery");
const helpHint = byId("help-hint");
const uncertainty = byId("uncertainty");
const crawlProgress = byId("crawl-progress");
const normalView = byId("normal-view");
const openSearch = byId("open-search");
const hunting = byId("hunting");
const huntingTitle = byId("hunting-title");
const huntingReopen = byId("hunting-reopen");
const huntingClear = byId("hunting-clear");
const huntingRouteTitle = byId("hunting-route-title");
const huntingRoute = byId("hunting-route");
const huntingRouteNone = byId("hunting-route-none");
const huntingCredit = byId("hunting-credit");
const searchView = byId("search-view");
const searchBack = byId("search-back");
const searchTitle = byId("search-title");
const mobsStep = byId("mobs-step");
const mapsStep = byId("maps-step");
const query = byId("query") as HTMLInputElement;
const suggestions = byId("suggestions");
const noMatch = byId("no-match");
const taxis = byId("taxis") as HTMLInputElement;
const teleportScrolls = byId("teleport-scrolls") as HTMLInputElement;
const searchNearest = byId("search-nearest");
const searchNote = byId("search-note");
const resultsBox = byId("results-box");
const resultsTable = byId("search-results") as HTMLTableElement;
const resultRows = byId("search-rows");
const searchCrawl = byId("search-crawl");
const searchCredit = byId("search-credit");
resultsTable.prepend(resultsHead());
let draggable = false;
/** The search view's open step, as last rendered. */
let step: SearchState["view"] = null;

/** "Ctrl+Shift+N is taken by another app, so it does nothing here" */
function takenHotkeys(hotkeys: string[]): string {
  if (hotkeys.length === 0) return "";
  const one = hotkeys.length === 1;
  return `${hotkeys.map(keyLabel).join(", ")} ${one ? "is" : "are"} taken by another app (or invalid), so ${one ? "it does" : "they do"} nothing here`;
}

const number = (n: number) => n.toLocaleString("en-US");

function cell(text: string): HTMLTableCellElement {
  const td = document.createElement("td");
  td.textContent = text;
  return td;
}

function span(className: string, text: string): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = className;
  el.textContent = text;
  return el;
}

/** "Northern Top of Construction Site [Hidden Street]" over "right, upper". */
function portalRow(portal: HiddenPortal): HTMLLIElement {
  const row = document.createElement("li");
  const to = document.createElement("div");
  to.append(span("to", portal.toMapName));
  if (portal.toHiddenStreet) to.append(span("tag", "Hidden Street"));
  if (portal.quest) to.append(span("tag quest", "Quest"));
  row.append(to, span("where", portal.position));
  return row;
}

/** Keep player-recognizable clues ahead of the secondary map ID. */
function candidateRow(id: string, outline: Outline | null): HTMLLIElement {
  const row = document.createElement("li");
  const facts = document.createElement("div");
  facts.className = "candidate-facts";
  if (!outline) {
    row.append(span("candidate-mobs", "Comparison data unavailable"));
    facts.append(span("candidate-id", `Map ${id}`));
    row.append(facts);
    return row;
  }
  const { mobLevels, firstMobs, hiddenPortals } = outline;
  row.append(span("candidate-mobs", firstMobs.length ? firstMobs.join(", ") : "No mobs"));
  if (mobLevels) {
    facts.append(span("candidate-levels",
      mobLevels.lowest === mobLevels.highest
        ? `Lv ${mobLevels.lowest}`
        : `Lv ${mobLevels.lowest}–${mobLevels.highest}`,
    ));
  }
  facts.append(span("candidate-portals", `${hiddenPortals} hidden portal${hiddenPortals === 1 ? "" : "s"}`));
  facts.append(span("candidate-id", `Map ${id}`));
  row.append(facts);
  return row;
}

/** "7 Oct 2026", in the player's time zone. */
const savedOn = (unixMs: number) =>
  new Date(unixMs).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });

/** The note shown in place of (or above) the map's data. */
function dataNote(data: MapDataStatus, recognition: Recognition, sourceHint: string): string {
  const noMobs = (data: MapData) => (data.mobs.length === 0 ? " · No mobs here" : "");
  switch (data.status) {
    case "loading":
      return "Loading MeowDB data…";
    case "unavailable":
      return `Couldn't reach MeowDB. Check your connection, then restart MapleCompass to retry.${sourceHint}`;
    case "formatChanged":
      return `MeowDB's page format has changed. MapleCompass needs an update to read it.${sourceHint}`;
    case "savedCopy":
      return `MeowDB unreachable — showing a saved copy from ${savedOn(data.savedAtUnixMs)}${noMobs(data)}`;
    case "ready":
      if (recognition.status === "savedCopy") {
        return `MeowDB unreachable — map list from a saved copy from ${savedOn(recognition.savedAtUnixMs)}${noMobs(data)}`;
      }
      return data.mobs.length === 0 ? "No mobs here" : "";
    case "noMap":
      return "";
  }
}

function renderData(panel: PanelState) {
  const { data } = panel;
  const shown = panel.gameFound && !!panel.map;
  dataEl.hidden = !shown;
  footer.hidden = !shown;
  if (!shown) return;

  const { candidates } = panel;
  candidatesEl.hidden = !candidates;
  if (candidates) {
    const next = hotkeyAvailable(panel.hotkeys.chooseCandidate, panel.unavailableHotkeys)
      ? `${keyLabel(panel.hotkeys.chooseCandidate)} for the next`
      : "Candidate shortcut unavailable; check shortcut settings";
    candidatesNote.textContent = candidates.chosen
      ? `Your pick of ${candidates.maps.length} maps with this title · ${next}`
      : `${candidates.maps.length} maps share this title; not sure which you're on · ${next}`;
    candidatesNote.classList.toggle("problem", !candidates.chosen);
    candidateRows.replaceChildren(
      ...candidates.maps.map((candidate, i) => {
        const row = candidateRow(candidate.id, candidate.outline);
        row.classList.toggle("current", i === candidates.shown);
        if (i === candidates.shown) row.setAttribute("aria-current", "true");
        return row;
      }),
    );
  }

  const sourceAvailable = hotkeyAvailable(panel.hotkeys.openMeowdbPage, panel.unavailableHotkeys);
  const sourceHint = sourceAvailable ? ` ${keyLabel(panel.hotkeys.openMeowdbPage)} opens the source page.` : "";
  note.textContent = dataNote(data, panel.recognition, sourceHint);
  note.hidden = note.textContent === "";
  const problem = ["unavailable", "formatChanged", "savedCopy"].includes(data.status);
  note.classList.toggle("problem", problem || panel.recognition.status === "savedCopy");

  const ready = data.status === "ready" || data.status === "savedCopy" ? data : null;
  summary.hidden = !ready?.summary;
  if (ready?.summary) {
    const { mobsPerHour, expPerHour } = ready.summary;
    byId("mobs-solo").textContent = number(mobsPerHour.solo);
    byId("mobs-party").textContent = number(mobsPerHour.fullParty);
    byId("exp-solo").textContent = number(expPerHour.solo);
    byId("exp-party").textContent = number(expPerHour.fullParty);
  }
  mobsTable.hidden = !ready || ready.mobs.length === 0;
  mobRows.replaceChildren(
    ...(ready?.mobs ?? []).map((mob) => {
      const row = document.createElement("tr");
      row.append(
        cell(mob.name),
        cell(String(mob.level)),
        cell(number(mob.hp)),
        cell(number(mob.exp)),
        cell(String(mob.spawnCount)),
        cell(`${mob.spawnSharePercent}%`),
      );
      ["Mob", "Lv", "HP", "EXP", "Count", "Share"].forEach((label, i) => {
        row.cells[i].dataset.label = label;
      });
      return row;
    }),
  );

  portals.hidden = !ready;
  noPortals.hidden = !ready || ready.hiddenPortals.length > 0;
  portalRows.replaceChildren(...(ready?.hiddenPortals ?? []).map(portalRow));

  credit.textContent = panel.credit ?? "";
  openHint.textContent = sourceAvailable ? `${keyLabel(panel.hotkeys.openMeowdbPage)}: source page` : "";
  helpHint.textContent = hotkeyAvailable(panel.hotkeys.setup, panel.unavailableHotkeys)
    ? `${keyLabel(panel.hotkeys.setup)}: help & setup` : "";
  helpHint.hidden = helpHint.textContent === "";
}

/**
 * The Hunting section, at the bottom of the normal view: the Hunting Target's mob and
 * map, the map's figures, and the route there from the map you're on, which scrolls
 * when the panel runs out of room. Its 🔍 reopens the mob's map list; its × clears it.
 */
function renderHunting(panel: PanelState) {
  const target: HuntingTarget | null = panel.search.hunting;
  hunting.hidden = !target;
  if (!target) return;
  const { mob, map } = target;
  huntingTitle.textContent = `${mob.name} · ${map.mapName}`;
  huntingTitle.title = `${mob.name} (Lv ${mob.level}) on ${map.streetName}: ${map.mapName}`;
  const tier = byId("hunting-tier");
  tier.textContent = TIER_NAMES[map.tier];
  tier.className = `tier ${map.tier}`;
  byId("hunting-density").textContent = density(map.densityHundredths);
  byId("hunting-share").textContent = `${map.spawnSharePercent}%`;
  const distance = distanceWithTravel(map);
  const distanceEl = byId("hunting-distance");
  distanceEl.textContent = distance.text;
  distanceEl.title = distance.title;
  distanceEl.className = typeof map.distance === "string" ? "distance none" : "distance";

  // The route from the map you're on (its first stop); without one, why not.
  const routed = map.route.length > 0;
  huntingRouteTitle.textContent = routed ? `Route from ${map.route[0].mapName}` : "";
  huntingRouteTitle.hidden = !routed;
  huntingRoute.replaceChildren(...routeStops(map));
  huntingRoute.hidden = !routed;
  huntingRouteNone.textContent = routed ? "" : distanceText(map.distance).title;
  huntingRouteNone.hidden = routed;
  // The footer credits MeowDB whenever a map is shown; otherwise the section does.
  huntingCredit.textContent = panel.search.credit ?? "";
  huntingCredit.hidden = !panel.search.credit || !!panel.credit;
}

/**
 * The search view, which takes over the whole panel: first the query box and the mobs
 * it suggests, then the picked mob's ranked map list. Picking a map makes it the Hunting
 * Target; the back arrow or Esc returns to the normal view with nothing changed.
 */
function renderSearchView(panel: PanelState) {
  const { search } = panel;
  const view = search.view;
  normalView.hidden = view !== null;
  searchView.hidden = view === null;
  const opened = view !== step;
  step = view;
  if (view === null) return;
  mobsStep.hidden = view !== "mobs";
  mapsStep.hidden = view !== "maps";
  // Before the lists, which scroll their highlight into what room is left.
  // The world crawl, while it runs: search only knows the mobs on the maps known so far.
  const crawl = crawlLine(search);
  searchCrawl.textContent = crawl && `Mapping the world: ${crawl}`;
  searchCrawl.hidden = !crawl;
  searchCredit.textContent = search.credit ?? "";
  searchCredit.hidden = !search.credit;
  if (view === "mobs") {
    searchTitle.textContent = "Search a mob";
    renderMobs(search);
    // A fresh query box, and the typing goes to it.
    if (opened) {
      query.value = search.query;
      query.focus({ preventScroll: true });
    }
  } else {
    searchTitle.textContent = search.picked?.name ?? "";
    if (search.picked) searchTitle.append(span("level", `Lv ${search.picked.level}`));
    renderMaps(search);
    if (opened) resultsBox.focus({ preventScroll: true });
  }
}

/** The first step: the suggestions (the highlighted one is what Enter picks). */
function renderMobs(search: SearchState) {
  suggestions.replaceChildren(
    ...search.suggestions.map((mob, i) => {
      const row = document.createElement("li");
      row.id = `mob-${mob.id}`;
      row.dataset.id = mob.id;
      row.setAttribute("role", "option");
      row.append(span("name", mob.name), span("level", `Lv ${mob.level}`));
      const current = i === search.highlighted;
      row.classList.toggle("current", current);
      row.setAttribute("aria-selected", String(current));
      return row;
    }),
  );
  const listed = search.suggestions.length > 0;
  suggestions.hidden = !listed;
  query.setAttribute("aria-expanded", String(listed));
  const highlighted = search.highlighted === null ? null : search.suggestions[search.highlighted];
  if (highlighted) {
    query.setAttribute("aria-activedescendant", `mob-${highlighted.id}`);
    scrollWithin(suggestions, byId(`mob-${highlighted.id}`));
  } else {
    query.removeAttribute("aria-activedescendant");
  }
  noMatch.textContent = search.crawl.total === 0 ? "Loading MeowDB's Map list…" : "No known mob has that in its name.";
  noMatch.hidden = listed || search.query.trim() === "";
}

/** The second step: the picked mob's maps, the highlighted one first to Enter, and the travel toggles. */
function renderMaps(search: SearchState) {
  // The core keeps the toggles (and saves them); the checkboxes follow it.
  taxis.checked = search.travel.taxis;
  teleportScrolls.checked = search.travel.teleportScrolls;
  searchNearest.textContent = nearestLine(search);
  searchNearest.hidden = searchNearest.textContent === "";
  searchNote.textContent = resultsNote(search);
  searchNote.hidden = searchNote.textContent === "";
  const rows = search.results.map((result, i) => {
    const row = resultRow(result);
    const current = i === search.highlightedResult;
    row.classList.toggle("current", current);
    row.setAttribute("aria-selected", String(current));
    return row;
  });
  resultsBox.hidden = rows.length === 0;
  resultRows.replaceChildren(...rows);
  const current = resultRows.querySelector<HTMLElement>("tr.current");
  if (current) scrollWithin(resultsBox, current);
}

/** Scroll `box` just enough to show `item`. Not `scrollIntoView`, which would scroll the overlay's page too. */
function scrollWithin(box: HTMLElement, item: HTMLElement) {
  const outer = box.getBoundingClientRect();
  const inner = item.getBoundingClientRect();
  // Below a sticky table header, if there is one.
  const top = outer.top + (box.querySelector("thead")?.getBoundingClientRect().height ?? 0);
  if (inner.top < top) box.scrollTop -= top - inner.top;
  else if (inner.bottom > outer.bottom) box.scrollTop += inner.bottom - outer.bottom;
}

function render(panel: PanelState) {
  const { hotkeys } = panel;
  const { recognition, map: current, notSure } = panel;
  draggable = panel.draggable;
  // Hidden while the game is minimised or gone: the window stays, but draws nothing.
  document.body.classList.toggle("hidden", panel.layer === "hidden");
  document.body.classList.toggle("collapsed", !panel.expanded);
  document.body.classList.toggle("draggable", panel.draggable);
  document.body.classList.toggle("unreadable", !current);
  document.body.classList.toggle("not-sure", !!current && notSure);
  uncertainty.hidden = !current || !notSure;
  const setupHint = hotkeyAvailable(hotkeys.setup, panel.unavailableHotkeys)
    ? `Re-mark with ${keyLabel(hotkeys.setup)}.`
    : "Setup shortcut unavailable. Close the app using it, or change hotkeys in %APPDATA%\\com.itechify.maplecompass\\settings.json, then restart MapleCompass.";
  recovery.textContent = "";
  if (!panel.gameFound) {
    street.textContent = "";
    map.textContent = `Looking for ${panel.gameWindowTitle}…`;
  } else if (panel.setup.needed === "firstRun" || panel.setup.needed === "resolutionChanged") {
    // Nothing is read until the Title Region is marked (again).
    street.textContent =
      panel.setup.needed === "firstRun" ? "Finish setup to start" : "The game window changed size";
    map.textContent = "Mark the minimap title";
    recovery.textContent = setupHint;
  } else if (recognition.status === "unavailable") {
    street.textContent = "Couldn't load the map list from MeowDB";
    map.textContent = "Map recognition unavailable";
    recovery.textContent = "Check your connection. The Map list retries every minute.";
  } else if (current) {
    street.textContent = current.streetName;
    map.textContent = current.mapName;
    if (notSure && !panel.candidates) {
      recovery.textContent = `Title not confirmed. Keep both minimap title lines visible. ${setupHint}`;
    }
  } else {
    street.textContent = "";
    map.textContent = "Reading the minimap title…";
    recovery.textContent = recognition.status === "starting"
      ? "Loading MeowDB's Map list…"
      : `Keep the minimap open with both title lines visible. If this persists, ${setupHint.charAt(0).toLowerCase() + setupHint.slice(1)}`;
  }
  recovery.hidden = recovery.textContent === "";
  openSearch.title = hotkeyAvailable(hotkeys.search, panel.unavailableHotkeys)
    ? `Search a mob (${keyLabel(hotkeys.search)})` : "Search a mob";
  hotkeyProblem.textContent = takenHotkeys(panel.unavailableHotkeys);
  if (hotkeyProblem.textContent) {
    hotkeyProblem.textContent += hotkeyAvailable(hotkeys.setup, panel.unavailableHotkeys)
      ? `. ${keyLabel(hotkeys.setup)} opens setup; select Overlay guide for shortcut help.`
      : ". Close the app using these shortcuts, or edit %APPDATA%\\com.itechify.maplecompass\\settings.json, then restart MapleCompass.";
  }
  hotkeyProblem.hidden = hotkeyProblem.textContent === "";
  // The world crawl, while it runs, with or without a search: "Mapping the world: 312
  // of 422 maps known". Search can only find mobs on the maps known so far.
  const crawl = crawlLine(panel.search);
  crawlProgress.textContent = crawl && `Mapping the world: ${crawl}`;
  crawlProgress.hidden = !panel.gameFound || !crawl;
  renderData(panel);
  renderHunting(panel);
  renderSearchView(panel);
  expanded = panel.expanded;
  reportHeight();
}

// The shell sizes the window to the expanded panel's content, and the core keeps it
// within the space where the panel is placed. The panel is then no taller than the
// window, and its long part (the route, the suggestions, the map list) scrolls. What
// is scrolled out of sight counts too, so the core always learns the content's full
// height.
let expanded = true;
let reportedHeight = 0;
function reportHeight() {
  if (!expanded) return;
  const scrolledAway = [...panelEl.querySelectorAll<HTMLElement>(".scroll")]
    .filter((el) => el.getClientRects().length > 0)
    .reduce((sum, el) => sum + el.scrollHeight - el.clientHeight, 0);
  const height = Math.ceil(panelEl.scrollHeight + scrolledAway);
  if (height > 0 && height !== reportedHeight) {
    reportedHeight = height;
    invoke("panel_height", { height });
  }
}
window.addEventListener("resize", reportHeight);

// The panel is dragged by its header (inside the game, or detached), never by its
// buttons. A click anywhere leaves the keyboard with the game: the overlay can't be
// activated, except while the search view is open.
document.addEventListener("mousedown", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const onHeader = !!target?.closest(".drag-handle") && !target?.closest("button, input");
  if (draggable && e.button === 0 && onHeader) {
    getCurrentWindow().startDragging();
  }
});

openSearch.addEventListener("click", () => invoke("search_open"));
searchBack.addEventListener("click", () => invoke("search_escape"));
huntingReopen.addEventListener("click", () => invoke("hunting_reopen"));
huntingClear.addEventListener("click", () => invoke("hunting_clear"));

// A click on a suggested mob or a listed map picks it, like Enter on it.
function pickOnClick(list: HTMLElement, item: string) {
  list.addEventListener("click", (e) => {
    const picked = e.target instanceof Element ? e.target.closest<HTMLElement>(item) : null;
    if (picked?.dataset.id) invoke("search_pick", { id: picked.dataset.id });
  });
}
pickOnClick(suggestions, "li[data-id]");
pickOnClick(resultRows, "tr[data-id]");

/** The search view's keys: the arrows move the highlight, Enter picks, Esc goes back. */
function searchKey(e: KeyboardEvent): boolean {
  switch (e.key) {
    case "ArrowDown":
    case "ArrowUp":
      invoke("search_move", { down: e.key === "ArrowDown" });
      break;
    case "Enter":
      invoke("search_enter");
      break;
    case "Escape":
      invoke("search_escape");
      break;
    default:
      return false;
  }
  e.preventDefault();
  return true;
}
query.addEventListener("input", () => invoke("search_query", { query: query.value }));
query.addEventListener("keydown", (e) => {
  if (!e.isComposing && searchKey(e)) e.stopPropagation();
});
// The map list has no text box: its keys come from wherever the focus is in the view.
document.addEventListener("keydown", (e) => {
  if (step === null || e.isComposing) return;
  // Buttons and checkboxes keep their own Enter.
  if (e.key === "Enter" && e.target instanceof Element && e.target.closest("button, input")) return;
  searchKey(e);
});
// After a click on a checkbox, the keys go back to the map list.
taxis.addEventListener("change", () => {
  invoke("search_toggle_taxis");
  resultsBox.focus({ preventScroll: true });
});
teleportScrolls.addEventListener("change", () => {
  invoke("search_toggle_teleport_scrolls");
  resultsBox.focus({ preventScroll: true });
});
// Whenever the overlay comes in front with the search view open, the keys go to it.
window.addEventListener("focus", () => {
  if (step === "mobs") query.focus({ preventScroll: true });
  else if (step === "maps") resultsBox.focus({ preventScroll: true });
});

listen<PanelState>("overlay-state", (e) => render(e.payload));

