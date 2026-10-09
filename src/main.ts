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
  resultGroup,
  resultsHead,
  resultsNote,
  routeStops,
  tripLine,
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
  spot: MapSpot | null;
};

/** Mirror of `compass_core::MapSpot`: a cell of the minimap's 5x3 grid. */
type MapSpot = { column: number; row: number };

/** Mirror of `compass_core::LevelRange`. */
type LevelRange = { lowest: number; highest: number };

/** Mirror of `compass_core::MapData`. */
type MapData = {
  summary: { mobsPerHour: PerParty; expPerHour: PerParty } | null;
  mobs: Mob[];
  hiddenPortals: HiddenPortal[];
  /** The mobs the panel lists: the five with the largest Spawn Share, by level. */
  listedMobs: Mob[];
  moreMobs: { count: number; levels: LevelRange } | null;
};

/** Mirror of `compass_core::Section`: a section the player can collapse to its heading. */
type Section = "hiddenPortals" | "mobs";

/** Mirror of `compass_core::MapDataStatus`. */
type MapDataStatus =
  | { status: "noMap" | "loading" | "unavailable" | "formatChanged" }
  | ({ status: "ready" } & MapData)
  | ({ status: "savedCopy"; savedAtUnixMs: number } & MapData);

/** Mirror of `compass_core::MobDetails`: a figure MeowDB shows as "—" is null. */
type MobDetails = {
  elements: { weak: string[]; strong: string[]; immune: string[] };
  touchDamage: { min: number; max: number } | null;
  mp: number | null;
  accuracy: number | null;
  avoid: number | null;
  physicalDefense: number | null;
  magicDefense: number | null;
  speed: string;
  movement: string;
  hpPerExp: { hundredths: number; rank: number; ranked: number } | null;
};

/** Mirror of `compass_core::MobCard`: one listed mob's Mob Card. */
type MobCard = { id: string; url: string; image: string | null } & (
  | { status: "loading" | "unavailable" | "formatChanged" }
  | ({ status: "ready" } & MobDetails)
  | ({ status: "savedCopy"; savedAtUnixMs: number } & MobDetails)
);

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

/** Mirror of `compass_core::ExpBar`. */
type ExpBar = { exp: number; percentHundredths: number };

/** Mirror of `compass_core::ExpRate`. */
type ExpRate = { expPerHour: number; percentPerHourHundredths: number | null };

/** Mirror of `compass_core::ExpTracking`. */
type ExpTracking = {
  state: "off" | "waiting" | "warmingUp" | "active" | "paused";
  timeToLevelS: number | null;
  timeToLevelBadge: string | null;
  rolling: ExpRate | null;
  sessionAverage: ExpRate | null;
  ceilingShareHundredths: number | null;
  deaths: { count: number; expLost: number; percentLostHundredths: number };
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
  /** The Mob Card of each mob the panel lists, in its order. */
  mobCards: MobCard[];
  expanded: boolean;
  /** Mirror of `compass_core::CollapsedSections`: the sections collapsed to their heading. */
  collapsedSections: Record<Section, boolean>;
  gameFound: boolean;
  placementMode: PlacementMode;
  layer: "aboveGame" | "hidden";
  /** The player can drag the panel by its header (inside or detached). */
  draggable: boolean;
  setup: { needed: "firstRun" | "resolutionChanged" | "requested" | null };
  hotkeys: Hotkeys;
  unavailableHotkeys: string[];
  /** Those of them that don't name a key combination; the rest another app has. */
  invalidHotkeys: string[];
  /** The shell has the hotkeys registered: the game or one of our windows is in front. */
  hotkeysLive: boolean;
  gameWindowTitle: string;
  expBarRegion: { x: number; y: number; width: number; height: number } | null;
  expBar: ExpBar | null;
  expTracking: ExpTracking;
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
const hotkeyProblem = byId("hotkey-problem");
const trackingEl = byId("tracking");
const trackingWaiting = byId("tracking-waiting");
const trackingFigures = byId("tracking-figures");
const timeToLevel = byId("time-to-level");
const trackingTag = byId("tracking-tag");
const rollingExp = byId("rolling-exp");
const sessionExp = byId("session-exp");
const rollingPercent = byId("rolling-percent");
const sessionPercent = byId("session-percent");
const trackingExtra = byId("tracking-extra");
const resetSession = byId("reset-session");
const badge = byId("badge");
const mapMobs = byId("map-mobs");
const noMobs = byId("no-mobs");
const moreMobs = byId("more-mobs");
const portalsToggle = byId("portals-toggle") as HTMLButtonElement;
const mobsToggle = byId("mobs-toggle") as HTMLButtonElement;
const mobCardEl = byId("mob-card");
const iconCount = byId("icon-count");
const iconAlert = byId("icon-alert");
const recovery = byId("recovery");
const uncertainty = byId("uncertainty");
const crawlProgress = byId("crawl-progress");
const normalView = byId("normal-view");
const toolbar = byId("toolbar");
const icon = document.querySelector<HTMLElement>(".icon")!;
const nextCandidate = byId("next-candidate");
const openMeowdb = byId("open-meowdb");
const hunting = byId("hunting");
const huntingTitle = byId("hunting-title");
const huntingReopen = byId("hunting-reopen");
const huntingClear = byId("hunting-clear");
const huntingRouteTitle = byId("hunting-route-title");
const huntingRoute = byId("hunting-route");
const huntingRouteNone = byId("hunting-route-none");
const huntingCredit = byId("hunting-credit");
const huntingArrival = byId("hunting-arrival");
const huntingTrip = byId("hunting-trip");
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
const searchCrawl = byId("search-crawl");
const searchCredit = byId("search-credit");
const resultsHeader = resultsHead();
resultsTable.append(resultsHeader);
let draggable = false;
/** The search view's open step, as last rendered. */
let step: SearchState["view"] = null;

/**
 * "Ctrl+Shift+N is taken by another app, so it does nothing here." and "Ctrl+Shift+Banana
 * isn't a key combination.": each hotkey that does nothing, with its real cause.
 */
function hotkeyCauses(panel: PanelState): string {
  const invalid = panel.invalidHotkeys;
  const taken = panel.unavailableHotkeys.filter((k) => !invalid.includes(k));
  const list = (keys: string[]) => keys.map(keyLabel).join(", ");
  const sentences = [];
  if (taken.length > 0) {
    sentences.push(
      taken.length === 1
        ? `${list(taken)} is taken by another app, so it does nothing here.`
        : `${list(taken)} are taken by another app, so they do nothing here.`,
    );
  }
  if (invalid.length > 0) {
    sentences.push(
      invalid.length === 1 ? `${list(invalid)} isn't a key combination.` : `${list(invalid)} aren't key combinations.`,
    );
  }
  return sentences.join(" ");
}

const number = (n: number) => n.toLocaleString("en-US");

const levels = ({ lowest, highest }: LevelRange) =>
  lowest === highest ? `Lv ${lowest}` : `Lv ${lowest}–${highest}`;

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

const SVG = "http://www.w3.org/2000/svg";

function svg<K extends keyof SVGElementTagNameMap>(
  tag: K,
  attributes: Record<string, string | number>,
): SVGElementTagNameMap[K] {
  const el = document.createElementNS(SVG, tag);
  for (const [name, value] of Object.entries(attributes)) el.setAttribute(name, String(value));
  return el;
}

/** The minimap's 5x3 grid in miniature, with the game's portal ring in the portal's cell. */
const CELL = 9;
function miniMap(spot: MapSpot | null): SVGSVGElement {
  const [width, height] = [5 * CELL, 3 * CELL];
  const el = svg("svg", { class: "minimap", viewBox: `-1 -1 ${width + 2} ${height + 2}` });
  el.setAttribute("aria-hidden", "true");
  el.classList.toggle("unknown", !spot);
  // Field, lit cell, grid, then the frame over them, so no edge cell covers its border.
  el.append(svg("rect", { class: "field", x: 0, y: 0, width, height, rx: 1.5 }));
  if (spot) {
    el.append(
      svg("rect", { class: "cell", x: spot.column * CELL, y: spot.row * CELL, width: CELL, height: CELL }),
    );
  }
  for (let c = 1; c < 5; c++) {
    el.append(svg("line", { class: "grid", x1: c * CELL, y1: 0, x2: c * CELL, y2: height }));
  }
  for (let r = 1; r < 3; r++) {
    el.append(svg("line", { class: "grid", x1: 0, y1: r * CELL, x2: width, y2: r * CELL }));
  }
  el.append(svg("rect", { class: "frame", x: 0, y: 0, width, height, rx: 1.5 }));
  if (spot) {
    const [cx, cy] = [(spot.column + 0.5) * CELL, (spot.row + 0.5) * CELL];
    el.append(
      svg("circle", { class: "ring", cx, cy, r: 3.3 }),
      svg("circle", { class: "core", cx, cy, r: 1.5 }),
    );
  }
  return el;
}

/** A tiny minimap, then "Northern Top of Construction Site [Hidden Street]" over "Right, upper". */
function portalRow(portal: HiddenPortal): HTMLLIElement {
  const row = document.createElement("li");
  const text = document.createElement("div");
  const to = document.createElement("div");
  to.className = "dest";
  to.append(span("to", portal.toMapName));
  if (portal.toHiddenStreet) to.append(span("tag", "Hidden Street"));
  if (portal.quest) to.append(span("tag quest", "Quest"));
  text.append(to, span("where", portal.position));
  row.append(miniMap(portal.spot), text);
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
  new Date(unixMs).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" });

/** The note shown in place of (or above) the map's data. */
function dataNote(data: MapDataStatus, recognition: Recognition, sourceHint: string): string {
  // A map without mobs says so in its Mobs section, not here.
  switch (data.status) {
    case "loading":
      return "Loading MeowDB data…";
    case "unavailable":
      return `Couldn't reach MeowDB. Check your connection, then restart MapleCompass to retry.${sourceHint}`;
    case "formatChanged":
      return `MeowDB's page format has changed. MapleCompass needs an update to read it.${sourceHint}`;
    case "savedCopy":
      return `MeowDB unreachable — showing a saved copy from ${savedOn(data.savedAtUnixMs)}`;
    case "ready":
      if (recognition.status === "savedCopy") {
        return `MeowDB unreachable — map list from a saved copy from ${savedOn(recognition.savedAtUnixMs)}`;
      }
      return "";
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

  // Hidden Portals first: what the game won't show the player.
  portals.hidden = !ready;
  noPortals.hidden = !ready || ready.hiddenPortals.length > 0;
  portalRows.replaceChildren(...(ready?.hiddenPortals ?? []).map(portalRow));

  mapMobs.hidden = !ready;
  noMobs.hidden = !ready || ready.listedMobs.length > 0;
  summary.hidden = !ready?.summary;
  if (ready?.summary) {
    const { mobsPerHour, expPerHour } = ready.summary;
    byId("mobs-solo").textContent = number(mobsPerHour.solo);
    byId("mobs-party").textContent = number(mobsPerHour.fullParty);
    byId("exp-solo").textContent = number(expPerHour.solo);
    byId("exp-party").textContent = number(expPerHour.fullParty);
  }
  mobsTable.hidden = !ready || ready.listedMobs.length === 0;
  mobRows.replaceChildren(
    ...(ready?.listedMobs ?? []).map((mob) => {
      const row = document.createElement("tr");
      // The name opens the mob's MeowDB page, and resting on it shows its Mob Card.
      const name = document.createElement("td");
      name.className = "mob";
      const button = document.createElement("button");
      button.type = "button";
      button.className = "mob-name";
      button.textContent = mob.name;
      button.dataset.id = mob.id;
      button.setAttribute("aria-describedby", "mob-card");
      button.classList.toggle("hovered", mob.id === hoveredMob);
      name.append(button);
      row.append(
        name,
        cell(String(mob.level)),
        cell(number(mob.hp)),
        cell(number(mob.exp)),
        cell(`${mob.spawnSharePercent}%`),
      );
      ["Mob", "Lv", "HP", "EXP", "Share"].forEach((label, i) => {
        row.cells[i].dataset.label = label;
      });
      return row;
    }),
  );
  const more = ready?.moreMobs;
  moreMobs.hidden = !more;
  if (more) moreMobs.textContent = `+${more.count} more · ${levels(more.levels)} · all on MeowDB`;

  // Each section collapses to its heading, which then says how many it holds. One with
  // nothing to list has nothing to collapse: its heading and "None" show as ever.
  const mobCount = (ready?.listedMobs.length ?? 0) + (ready?.moreMobs?.count ?? 0);
  foldSection(portals, portalsToggle, panel.collapsedSections.hiddenPortals, ready?.hiddenPortals.length ?? 0);
  foldSection(mapMobs, mobsToggle, panel.collapsedSections.mobs, mobCount);

  credit.textContent = panel.credit ?? "";
}

/** A section collapsed to its heading, or open; `count` is how many it lists. */
function foldSection(section: HTMLElement, toggle: HTMLButtonElement, collapsed: boolean, count: number) {
  const folded = collapsed && count > 0;
  section.classList.toggle("collapsed", folded);
  toggle.disabled = count === 0;
  toggle.setAttribute("aria-expanded", String(!folded));
  toggle.title = count === 0 ? "" : folded ? "Show this section" : "Collapse to the heading";
  const countEl = byId(`${toggle.id}-count`);
  countEl.hidden = !folded;
  countEl.textContent = String(count);
}

/** Which hotkey each icon at the top of the panel stands for. */
const CONTROL_HOTKEYS: Record<string, keyof Hotkeys> = {
  search: "search",
  chooseCandidate: "chooseCandidate",
  openMeowdbPage: "openMeowdbPage",
  cyclePlacement: "cyclePlacement",
  setup: "setup",
  togglePanel: "togglePanel",
  resetSession: "resetSession",
};

/**
 * The icons at the top of the panel, one per hotkey action. Each tooltip names its
 * hotkey (unless another app has it). The MeowDB icon shows only while a map is shown,
 * the next-candidate icon only while maps share its title.
 */
function renderToolbar(panel: PanelState) {
  for (const button of toolbar.querySelectorAll<HTMLButtonElement>("button[data-control]")) {
    const label = button.getAttribute("aria-label") ?? "";
    const hotkey = panel.hotkeys[CONTROL_HOTKEYS[button.dataset.control ?? ""]];
    button.title = hotkey && hotkeyAvailable(hotkey, panel.unavailableHotkeys) ? `${label} (${keyLabel(hotkey)})` : label;
  }
  openMeowdb.hidden = !panel.map;
  nextCandidate.hidden = !panel.candidates;
  icon.title = hotkeyAvailable(panel.hotkeys.togglePanel, panel.unavailableHotkeys)
    ? `Click to expand (${keyLabel(panel.hotkeys.togglePanel)})` : "Click to expand";
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
  // The mob on the first line, the map on the next.
  const mobLine = document.createElement("span");
  mobLine.className = "hunting-mob";
  mobLine.textContent = mob.name;
  const mapLine = document.createElement("span");
  mapLine.className = "hunting-map";
  mapLine.textContent = map.mapName;
  huntingTitle.replaceChildren(mobLine, mapLine);
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

  // The route from the map you're on (its first stop); without one, why not. Once you've
  // arrived the route has done its job: in its place, that you're here and the way you
  // came, until you leave the map.
  const { arrived, trip } = target;
  const routed = !arrived && map.route.length > 0;
  huntingRouteTitle.textContent = routed ? `Route from ${map.route[0].mapName}` : "";
  huntingRouteTitle.hidden = !routed;
  huntingRoute.replaceChildren(...(routed ? routeStops(map) : []));
  huntingRoute.hidden = !routed;
  huntingRouteNone.textContent = routed || arrived ? "" : distanceText(map.distance).title;
  huntingRouteNone.hidden = routed || arrived;
  huntingArrival.hidden = !arrived;
  huntingTrip.textContent = trip ? tripLine(trip) : "";
  huntingTrip.hidden = !trip;
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
  const groups = search.results.map((result, i) => {
    const group = resultGroup(result);
    const current = i === search.highlightedResult;
    group.classList.toggle("current", current);
    group.setAttribute("aria-selected", String(current));
    return group;
  });
  resultsBox.hidden = groups.length === 0;
  resultsTable.replaceChildren(resultsHeader, ...groups);
  const current = resultsTable.querySelector<HTMLElement>("tbody.current");
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

/** "−" (U+2212) for a negative figure, as the deaths counter writes it. */
const signed = (text: string) => text.replace(/^-/, "−");

/** 1234 hundredths -> "12.3%" with one decimal, "12%" with none: no finer than the
 *  figure can be trusted, so it doesn't flicker. */
const percent = (hundredths: number, decimals = 1) =>
  signed(`${(hundredths / 100).toFixed(decimals)}%`);

/** An EXP rate to 3 significant figures: 840, 6,120, 61.2k, 1.23M. */
function compact(n: number): string {
  const abs = Math.abs(n);
  if (abs < 10_000) return signed(Number(n.toPrecision(3)).toLocaleString("en-US"));
  // 999,999 rounds to 1,000k: write it 1M.
  const [value, unit] = Number((abs / 1_000).toPrecision(3)) < 1_000 ? [n / 1_000, "k"] : [n / 1_000_000, "M"];
  return signed(`${Number(value.toPrecision(3))}${unit}`);
}

const TRACKING_TAGS: Partial<Record<ExpTracking["state"], string>> = {
  warmingUp: "warming up",
  paused: "paused",
};

/** The Grind Tracker section: nothing while tracking is off, a quiet line until the
 *  Session starts, then Time to Level, the rates, and the ceiling and deaths when the
 *  state carries them. "—" for any figure not there yet. */
function renderTracking(panel: PanelState) {
  const tracking = panel.expTracking;
  trackingEl.hidden = tracking.state === "off";
  const waiting = tracking.state === "waiting";
  trackingWaiting.hidden = !waiting;
  trackingFigures.hidden = waiting;
  // The toolbar's reset icon, while there is a Session to reset.
  resetSession.hidden = tracking.state === "off" || waiting;

  timeToLevel.textContent = tracking.timeToLevelBadge ?? "—";
  const tag = TRACKING_TAGS[tracking.state];
  trackingTag.hidden = !tag;
  trackingTag.textContent = tag ?? "";
  trackingTag.className = `tag ${tracking.state}`;

  const exp = (rate: ExpRate | null) => (rate ? compact(rate.expPerHour) : "—");
  const share = (rate: ExpRate | null) =>
    rate?.percentPerHourHundredths != null ? percent(rate.percentPerHourHundredths) : "—";
  rollingExp.textContent = exp(tracking.rolling);
  sessionExp.textContent = exp(tracking.sessionAverage);
  rollingPercent.textContent = share(tracking.rolling);
  sessionPercent.textContent = share(tracking.sessionAverage);

  const extra = [];
  if (tracking.ceilingShareHundredths !== null) {
    extra.push(`${percent(tracking.ceilingShareHundredths, 0)} of the map's solo ceiling`);
  }
  const { deaths } = tracking;
  if (deaths.count > 0) {
    // No-break spaces: on a narrow panel, where the line wraps, the deaths read as one.
    extra.push(`deaths: ${deaths.count} (${percent(-deaths.percentLostHundredths)})`);
  }
  trackingExtra.textContent = extra.join(" · ");
  trackingExtra.hidden = extra.length === 0;
}

/** Time to Level on the collapsed icon, while there is one. */
function renderBadge(panel: PanelState) {
  const text = panel.expTracking.timeToLevelBadge;
  badge.hidden = !text;
  badge.textContent = text ?? "";
}

/** How long the pointer rests on a mob's name before its Mob Card shows. */
const MOB_CARD_DELAY_MS = 250;
/** The listed mob whose name the pointer is on, if any, and whether its card is due. */
let hoveredMob: string | null = null;
let mobCardDue = false;
let mobCardTimer: number | undefined;
/** The panel state last rendered, for the card to follow. */
let shownPanel: PanelState | null = null;
/** What the card last showed, so a render that changes nothing on it doesn't rebuild it. */
let mobCardShown = "";

function hoverMob(id: string | null) {
  if (id === hoveredMob) return;
  hoveredMob = id;
  mobCardDue = false;
  window.clearTimeout(mobCardTimer);
  for (const button of mobRows.querySelectorAll<HTMLElement>(".mob-name")) {
    button.classList.toggle("hovered", button.dataset.id === id);
  }
  if (id) {
    mobCardTimer = window.setTimeout(() => {
      mobCardDue = true;
      renderMobCard();
    }, MOB_CARD_DELAY_MS);
  }
  renderMobCard();
}

/** "—" for a figure the mob doesn't have. */
const figure = (n: number | null) => (n === null ? "—" : number(n));

/** A tag per element: "Weak to Fire", "Resists Ice", "Immune to Poison". */
function elementTags({ weak, strong, immune }: MobDetails["elements"]): HTMLElement {
  const tags = document.createElement("span");
  tags.className = "mob-card-elements";
  const all = [
    ...weak.map((e) => `Weak to ${e}`),
    ...strong.map((e) => `Resists ${e}`),
    ...immune.map((e) => `Immune to ${e}`),
  ];
  tags.append(...(all.length ? all.map((text) => span("tag", text)) : [span("none", "No element")]));
  return tags;
}

/** Touch and MP, ACC and AVOID, the defences in pairs, then HP per EXP and its rank. */
function mobFigures(details: MobDetails): HTMLDListElement {
  const list = document.createElement("dl");
  list.className = "mob-card-figures";
  const add = (label: string, ...value: (string | Node)[]) => {
    const pair = document.createElement("div");
    const dt = document.createElement("dt");
    dt.textContent = label;
    const dd = document.createElement("dd");
    dd.append(...value);
    pair.append(dt, dd);
    list.append(pair);
    return pair;
  };
  const touch = details.touchDamage;
  add("Touch", touch ? `${number(touch.min)}–${number(touch.max)}` : "—");
  add("MP", figure(details.mp));
  add("ACC", figure(details.accuracy));
  add("AVOID", figure(details.avoid));
  add("P.DEF", figure(details.physicalDefense));
  add("M.DEF", figure(details.magicDefense));
  const perExp = details.hpPerExp;
  const hpPerExp = perExp
    ? [(perExp.hundredths / 100).toFixed(2), span("rank", ` · #${perExp.rank} of ${perExp.ranked}`)]
    : ["—"];
  add("HP per EXP", ...hpPerExp).className = "wide";
  return list;
}

/** The Mob Card's content: the mob, then its figures or why there are none. */
function mobCardContent(mob: Mob, card: MobCard): Node[] {
  const sprite = document.createElement("div");
  sprite.className = "mob-card-sprite";
  if (card.image) {
    const img = document.createElement("img");
    img.alt = "";
    // Shrunk to fit, a sprite is smoothed: pixel doubling only works upward.
    img.addEventListener("load", () => img.classList.toggle("scaled", img.naturalWidth > 64 || img.naturalHeight > 64));
    // No saved image after all: the box stays empty.
    img.addEventListener("error", () => img.remove());
    img.src = card.image;
    sprite.append(img);
  }
  const ready = card.status === "ready" || card.status === "savedCopy" ? card : null;
  const who = document.createElement("div");
  who.className = "mob-card-who";
  who.append(
    span("mob-card-name", mob.name),
    span("mob-card-gait", ready ? `Lv ${mob.level} · ${ready.speed} · ${ready.movement.toLowerCase()}` : `Lv ${mob.level}`),
  );
  if (ready) who.append(elementTags(ready.elements));
  const head = document.createElement("div");
  head.className = "mob-card-head";
  head.append(sprite, who);
  const note = (text: string, problem = false) => {
    const p = document.createElement("p");
    p.className = problem ? "note problem" : "note";
    p.textContent = text;
    return p;
  };
  switch (card.status) {
    case "loading":
      return [head, note("Loading from MeowDB…")];
    case "unavailable":
      return [head, note("Couldn't reach MeowDB. MapleCompass tries again in a few minutes.", true)];
    case "formatChanged":
      return [head, note("MeowDB's page for this mob has changed. Click its name to read it there.", true)];
    case "ready":
      return [head, mobFigures(card)];
    case "savedCopy":
      return [head, mobFigures(card), note(`MeowDB unreachable — saved copy from ${savedOn(card.savedAtUnixMs)}`, true)];
  }
}

/** The space kept between the card and the hovered row, and the panel's edges. */
const MOB_CARD_GAP = 4;

/**
 * The hovered mob's Mob Card, once the pointer has rested on its name: below its row if it
 * fits in the panel, otherwise above it, never over the window bar. Where it fits on
 * neither side, it takes the roomier one. Only over the normal view of the expanded
 * panel, and gone once the mob isn't listed.
 */
function renderMobCard() {
  const panel = shownPanel;
  const button = hoveredMob
    ? mobRows.querySelector<HTMLElement>(`.mob-name[data-id="${CSS.escape(hoveredMob)}"]`)
    : null;
  const ready = panel?.data.status === "ready" || panel?.data.status === "savedCopy" ? panel.data : null;
  const mob = ready?.listedMobs.find((m) => m.id === hoveredMob);
  const card = panel?.mobCards.find((c) => c.id === hoveredMob);
  if (!mobCardDue || !panel?.expanded || step !== null || !button || !mob || !card) {
    mobCardEl.hidden = true;
    mobCardShown = "";
    return;
  }
  const shown = JSON.stringify([mob, card]);
  if (shown !== mobCardShown) {
    mobCardEl.replaceChildren(...mobCardContent(mob, card));
    mobCardShown = shown;
  }
  mobCardEl.hidden = false;
  const frame = panelEl.getBoundingClientRect();
  const row = (button.closest("tr") ?? button).getBoundingClientRect();
  const bar = normalView.querySelector(".window-bar")?.getBoundingClientRect();
  const height = mobCardEl.offsetHeight;
  const highest = (bar ? bar.bottom - frame.top : 0) + MOB_CARD_GAP;
  const lowest = panelEl.clientHeight - MOB_CARD_GAP - height;
  const below = row.bottom - frame.top + MOB_CARD_GAP;
  const above = row.top - frame.top - MOB_CARD_GAP - height;
  const roomBelow = panelEl.clientHeight - MOB_CARD_GAP - below;
  const roomAbove = row.top - frame.top - MOB_CARD_GAP - highest;
  let top: number;
  if (below <= lowest) top = below;
  else if (above >= highest) top = above;
  else top = roomBelow >= roomAbove ? below : above;
  mobCardEl.style.top = `${Math.max(highest, Math.min(top, lowest))}px`;
}

// Resting on a listed mob's name shows its Mob Card; clicking it opens its MeowDB page.
mobRows.addEventListener("mouseover", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLElement>(".mob-name") : null;
  hoverMob(button?.dataset.id ?? null);
});
mobRows.addEventListener("mouseleave", () => hoverMob(null));
mobRows.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLElement>(".mob-name") : null;
  if (!button?.dataset.id) return;
  invoke("open_mob_page", { id: button.dataset.id });
  hoverMob(null);
});

/** The collapsed icon: the shown map's Hidden Portal count, and a dot when something needs a look. */
function renderIcon(panel: PanelState) {
  const data = panel.data.status === "ready" || panel.data.status === "savedCopy" ? panel.data : null;
  iconCount.textContent = panel.map && data ? String(data.hiddenPortals.length) : "";
  iconAlert.hidden = !(
    panel.notSure ||
    (panel.candidates && !panel.candidates.chosen) ||
    ["unavailable", "formatChanged", "savedCopy"].includes(panel.data.status) ||
    panel.setup.needed === "firstRun" ||
    panel.setup.needed === "resolutionChanged"
  );
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
  // Not sure of the map, or of which map sharing its title: the facts shown are a guess.
  const doubtful = !!current && (notSure || (!!panel.candidates && !panel.candidates.chosen));
  document.body.classList.toggle("not-sure", doubtful);
  uncertainty.hidden = !doubtful;
  const setupHint = hotkeyAvailable(hotkeys.setup, panel.unavailableHotkeys)
    ? `Re-mark with ${keyLabel(hotkeys.setup)}.`
    : "Setup shortcut unavailable. Close the app using it, or change hotkeys in %APPDATA%\com.itechify.maplecompass\settings.json, then restart MapleCompass.";
  // Without a map, the title bar holds the status and the note under it what to do; the
  // Street Name slot stays the Street Name's.
  recovery.textContent = "";
  let problem = false;
  street.textContent = "";
  if (!panel.gameFound) {
    map.textContent = `Looking for ${panel.gameWindowTitle}…`;
  } else if (panel.setup.needed === "firstRun" || panel.setup.needed === "resolutionChanged") {
    // Nothing is read until the Title Region is marked (again).
    map.textContent =
      panel.setup.needed === "firstRun" ? "Finish setup to start" : "The game window changed size";
    recovery.textContent = `Mark the minimap title. ${setupHint}`;
    problem = panel.setup.needed === "resolutionChanged";
  } else if (recognition.status === "unavailable") {
    map.textContent = "Map recognition unavailable";
    recovery.textContent = "Couldn't load the map list from MeowDB. Check your connection. The Map list retries every minute.";
    problem = true;
  } else if (current) {
    street.textContent = current.streetName;
    map.textContent = current.mapName;
    if (notSure && !panel.candidates) {
      recovery.textContent = `Title not confirmed. Keep both minimap title lines visible. ${setupHint}`;
    }
  } else {
    map.textContent = "Reading the minimap title…";
    recovery.textContent = recognition.status === "starting"
      ? "Loading MeowDB's Map list…"
      : `Keep the minimap open with both title lines visible. If this persists, ${setupHint.charAt(0).toLowerCase() + setupHint.slice(1)}`;
  }
  recovery.hidden = recovery.textContent === "";
  recovery.classList.toggle("problem", problem);
  renderToolbar(panel);
  // Each hotkey that does nothing, its real cause, then one way to fix it.
  const causes = hotkeyCauses(panel);
  hotkeyProblem.textContent = causes && `${causes} ${
    hotkeyAvailable(hotkeys.setup, panel.unavailableHotkeys)
      ? `${keyLabel(hotkeys.setup)} opens setup; select Guide for shortcut help.`
      : "Close the other app, or edit %APPDATA%\com.itechify.maplecompass\settings.json, then restart MapleCompass."
  }`;
  hotkeyProblem.hidden = hotkeyProblem.textContent === "";
  renderTracking(panel);
  // The world crawl, while it runs, with or without a search: "Mapping the world: 312
  // of 422 maps known". Search can only find mobs on the maps known so far.
  const crawl = crawlLine(panel.search);
  crawlProgress.textContent = crawl && `Mapping the world: ${crawl}`;
  crawlProgress.hidden = !panel.gameFound || !crawl;
  renderData(panel);
  renderHunting(panel);
  renderSearchView(panel);
  shownPanel = panel;
  renderMobCard();
  renderBadge(panel);
  renderIcon(panel);
  expanded = panel.expanded;
  reportHeight();
}

// The shell sizes the window to the expanded panel's content, and the core keeps it
// within the space where the panel is placed. The panel is then no taller than the
// window, and its long part (the route, the suggestions, the map list) scrolls. So the
// core always learns the content's full height, borders and bottom padding included,
// the panel is measured without the window's limit: measured within it, the cut-off
// padding would not count, and the window would shrink a little with every report.
// Lifting the limit scrolls the long parts back to the top, so they are put back.
let expanded = true;
let reportedHeight = 0;
function reportHeight() {
  if (!expanded) return;
  const scrolls = [...panelEl.querySelectorAll<HTMLElement>(".scroll")];
  const scrolledTo = scrolls.map((el) => el.scrollTop);
  panelEl.classList.add("measuring");
  const height = Math.ceil(panelEl.getBoundingClientRect().height);
  panelEl.classList.remove("measuring");
  scrolls.forEach((el, i) => (el.scrollTop = scrolledTo[i]));
  if (height > 0 && height !== reportedHeight) {
    reportedHeight = height;
    invoke("panel_height", { height });
  }
}
window.addEventListener("resize", reportHeight);

// The panel is dragged by its header, or collapsed by its icon (inside the game, or
// detached), never by its buttons. A click anywhere leaves the keyboard with the game:
// the overlay can't be activated, except while the search view is open.
function startDrag() {
  getCurrentWindow().startDragging();
}

document.addEventListener("mousedown", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const onHeader = !!target?.closest(".drag-handle") && !target?.closest("button, input");
  if (draggable && e.button === 0 && onHeader) startDrag();
});

// The expanded panel's bottom corners resize it, whatever its placement, as its edges
// do: its width only, as the height follows the content. Once the player lets go the
// core keeps the width (never under its minimum).
for (const grip of document.querySelectorAll<HTMLElement>(".grip")) {
  grip.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    getCurrentWindow().startResizeDragging(grip.dataset.resize === "West" ? "West" : "East");
  });
}

// The collapsed icon: a click expands the panel again (like the collapse hotkey), and
// pressing on it and moving the mouse drags it, where the panel can be dragged.
const DRAG_AFTER_PX = 4;
let iconPress: { x: number; y: number } | null = null;
icon.addEventListener("mousedown", (e) => {
  if (e.button === 0) iconPress = { x: e.screenX, y: e.screenY };
});
icon.addEventListener("mousemove", (e) => {
  if (!iconPress || !draggable) return;
  if (Math.abs(e.screenX - iconPress.x) + Math.abs(e.screenY - iconPress.y) >= DRAG_AFTER_PX) {
    iconPress = null;
    startDrag();
  }
});
icon.addEventListener("mouseup", (e) => {
  if (iconPress && e.button === 0) invoke("panel_press", { control: "togglePanel" });
  iconPress = null;
});
icon.addEventListener("mouseleave", () => {
  iconPress = null;
});

// Each icon at the top of the panel does what its hotkey does.
toolbar.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLButtonElement>("button[data-control]") : null;
  if (button) invoke("panel_press", { control: button.dataset.control });
});
searchBack.addEventListener("click", () => invoke("search_escape"));
huntingReopen.addEventListener("click", () => invoke("hunting_reopen"));
huntingClear.addEventListener("click", () => invoke("hunting_clear"));
// A click on a section's heading collapses the section to it, or opens it again.
for (const toggle of [portalsToggle, mobsToggle]) {
  toggle.addEventListener("click", () => invoke("panel_toggle_section", { section: toggle.dataset.section }));
}

// A click on a suggested mob or a listed map picks it, like Enter on it.
function pickOnClick(list: HTMLElement, item: string) {
  list.addEventListener("click", (e) => {
    const picked = e.target instanceof Element ? e.target.closest<HTMLElement>(item) : null;
    if (picked?.dataset.id) invoke("search_pick", { id: picked.dataset.id });
  });
}
pickOnClick(suggestions, "li[data-id]");
pickOnClick(resultsTable, "tbody[data-id]");

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

