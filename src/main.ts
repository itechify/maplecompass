import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { dayText } from "./dates";
import type { ExpTracking } from "./exp-tracking";
import { initMapPreview, renderMapPreview, renderPortalRows, type HiddenPortal, type MapPreview } from "./map-preview";
import {
  HOTKEY_NAMES,
  KEY_ONLY,
  codeLabel,
  hotkeyAvailable,
  keyLabel,
  sameHotkey,
  type Hotkey,
  type Hotkeys,
  type RebindProblem,
  type SettingsNotice,
  type SettingsView,
} from "./hotkeys";
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
  | { status: "noMap" }
  | ({ status: "ready" } & MapData)
  | ({ status: "savedCopy"; savedAtUnixMs: number } & MapData);

/** Mirror of `compass_core::MobDetails`: a figure shown as "—" is null. */
type MobDetails = {
  elements: { weak: string[]; strong: string[]; immune: string[] };
  touchDamage: { min: number; max: number } | null;
  mp: number | null;
  accuracy: number | null;
  avoid: number | null;
  physicalDefense: number | null;
  magicDefense: number | null;
  speed: string;
  hpPerExp: { hundredths: number; rank: number; ranked: number } | null;
};

/** Mirror of `compass_core::MobCard`: one listed mob's Mob Card. */
type MobCard = { id: string; url: string; image: string | null } & MobDetails;

/** Mirror of `compass_core::Recognition`. */
type Recognition =
  | { status: "starting" | "ready" | "unavailable" | "formatChanged" }
  | { status: "savedCopy"; savedAtUnixMs: number };

/** Mirror of `compass_core::Outline`. */
type Outline = {
  mobLevels: { lowest: number; highest: number } | null;
  firstMobs: string[];
  hiddenPortals: number;
};

/** Mirror of `compass_core::Candidates`. */
type Candidates = {
  maps: { id: string; outline: Outline }[];
  shown: number;
  chosen: boolean;
};

/** Mirror of `compass_core::ExpBar`. */
type ExpBar = { exp: number; percentHundredths: number };

/** Mirror of `compass_core::PanelState`. */
type PanelState = {
  recognition: Recognition;
  map: { id: string; streetName: string; mapName: string; url: string } | null;
  confidence: number;
  notSure: boolean;
  candidates: Candidates | null;
  data: MapDataStatus;
  /** The shown map's Map Preview; null while there is none, or it isn't settled which map it is. */
  mapPreview: MapPreview | null;
  credit: string | null;
  /** The Mob Card of each mob the panel lists, in its order. */
  mobCards: MobCard[];
  expanded: boolean;
  /** Mirror of `compass_core::CollapsedSections`: the sections collapsed to their heading. */
  collapsedSections: Record<Section, boolean>;
  gameFound: boolean;
  placementMode: PlacementMode;
  layer: "onTop" | "hidden";
  /** The player can drag the panel by its header (inside or detached). */
  draggable: boolean;
  setup: { needed: "firstRun" | "resolutionChanged" | "requested" | null };
  hotkeys: Hotkeys;
  unavailableHotkeys: string[];
  /** Those of them that don't name a key combination; the rest another app has. */
  invalidHotkeys: string[];
  /** The shell has the hotkeys registered: the game or one of our windows is in front. */
  hotkeysLive: boolean;
  /** The settings view, which takes over the panel; null while it is closed. */
  settingsView: SettingsView | null;
  gameWindowTitle: string;
  expBarRegion: { x: number; y: number; width: number; height: number } | null;
  expBar: ExpBar | null;
  expTracking: ExpTracking;
  /** Mirror of `compass_core::GrindTrackerWindow`: the player wants it open. */
  grindTracker: { open: boolean };
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
const footer = byId("footer");
const credit = byId("credit");
const hotkeyProblem = byId("hotkey-problem");
const grindTracker = byId("grind-tracker");
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
const titlebar = byId("titlebar");
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
const settingsView = byId("settings-view");
const settingsBack = byId("settings-back");
const hotkeyRows = byId("hotkey-rows");
const settingsNotice = byId("settings-notice");
const settingsAnnounce = byId("settings-announce");
const hotkeysUndo = byId("hotkeys-undo") as HTMLButtonElement;
const hotkeysReset = byId("hotkeys-reset") as HTMLButtonElement;
const resultsHeader = resultsHead();
resultsTable.append(resultsHeader);
let draggable = false;

/**
 * What each part of the panel was last rendered from. The panel state comes again
 * whenever anything in it changes (the world crawl's progress, say): a part whose own
 * inputs didn't change keeps its rows, and the scroll and hover on them.
 */
const renderedFrom = new Map<string, string>();
function changed(part: string, ...inputs: unknown[]): boolean {
  const key = JSON.stringify(inputs);
  if (renderedFrom.get(part) === key) return false;
  renderedFrom.set(part, key);
  return true;
}
/** The search view's open step, as last rendered. */
let step: SearchState["view"] = null;
/** The settings view, as last rendered: null while it is closed. */
let settings: SettingsView | null = null;

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

/** Keep player-recognizable clues ahead of the secondary map ID. */
function candidateRow(id: string, outline: Outline): HTMLLIElement {
  const row = document.createElement("li");
  const facts = document.createElement("div");
  facts.className = "candidate-facts";
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
const savedOn = (unixMs: number) => dayText(new Date(unixMs), true);

/** The note shown above the map's data: only that it comes from a saved copy over a week old. */
function dataNote(data: MapDataStatus): string {
  // A map without mobs says so in its Mobs section, not here.
  switch (data.status) {
    case "savedCopy":
      return `OSMS Data Explorer unreachable — showing a saved copy from ${savedOn(data.savedAtUnixMs)}`;
    case "ready":
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
  const inputs = [data, panel.mapPreview, panel.candidates, panel.collapsedSections, panel.credit, panel.recognition];
  if (!changed("data", ...inputs, panel.hotkeys, panel.unavailableHotkeys)) return;

  const { candidates } = panel;
  candidatesEl.hidden = !candidates;
  if (candidates) {
    const nextKey = panel.hotkeys.chooseCandidate;
    const next = hotkeyAvailable(nextKey, panel.unavailableHotkeys)
      ? `${keyLabel(nextKey)} for the next`
      : nextKey === ""
        ? "the two arrows at the top for the next"
        : "Candidate shortcut unavailable; change it in Settings";
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

  note.textContent = dataNote(data);
  note.hidden = note.textContent === "";
  note.classList.toggle("problem", data.status === "savedCopy");

  const ready = data.status === "ready" || data.status === "savedCopy" ? data : null;

  // Portals first: the map with every portal on it, then the Hidden Portals, what the game
  // won't show the player.
  portals.hidden = !ready;
  const hiddenPortals: HiddenPortal[] = ready?.hiddenPortals ?? [];
  const pictured = renderMapPreview(ready ? panel.mapPreview : null, panel.map?.mapName ?? "");
  noPortals.hidden = !ready || hiddenPortals.length > 0;
  renderPortalRows(hiddenPortals, panel.mapPreview?.marks ?? []);

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
  foldSection(portals, portalsToggle, panel.collapsedSections.hiddenPortals, hiddenPortals.length, pictured);
  foldSection(mapMobs, mobsToggle, panel.collapsedSections.mobs, mobCount);

  credit.textContent = panel.credit ?? "";
}

/**
 * A section collapsed to its heading, or open; `count` is how many it lists, and
 * `pictured` whether it shows more than its list (the Portals section's Map Preview).
 */
function foldSection(section: HTMLElement, toggle: HTMLButtonElement, collapsed: boolean, count: number, pictured = false) {
  const holds = count > 0 || pictured;
  const folded = collapsed && holds;
  section.classList.toggle("collapsed", folded);
  toggle.disabled = !holds;
  toggle.setAttribute("aria-expanded", String(!folded));
  toggle.title = !holds ? "" : folded ? "Show this section" : "Collapse to the heading";
  const countEl = byId(`${toggle.id}-count`);
  countEl.hidden = !folded;
  countEl.textContent = String(count);
}

/** Which hotkey each icon at the top of the panel stands for. */
const CONTROL_HOTKEYS: Record<string, keyof Hotkeys> = {
  search: "search",
  chooseCandidate: "chooseCandidate",
  cyclePlacement: "cyclePlacement",
  setup: "setup",
  togglePanel: "togglePanel",
  toggleGrindTracker: "grindTracker",
};

/**
 * The icons at the top of the panel, one per hotkey action. Each tooltip names its
 * hotkey (unless another app has it). The next-candidate icon shows only while maps share
 * its title, the Grind Tracker's only while
 * the EXP bar is tracked (pressed while its window is open).
 */
function renderToolbar(panel: PanelState) {
  for (const button of toolbar.querySelectorAll<HTMLButtonElement>("button[data-control]")) {
    const label = button.getAttribute("aria-label") ?? "";
    const hotkey = panel.hotkeys[CONTROL_HOTKEYS[button.dataset.control ?? ""]];
    button.title = hotkey && hotkeyAvailable(hotkey, panel.unavailableHotkeys) ? `${label} (${keyLabel(hotkey)})` : label;
  }
  // The map's title opens its MeowDB page, while a map is shown.
  titlebar.classList.toggle("linked", !!panel.map);
  const pageKey = panel.hotkeys.openMeowdbPage;
  titlebar.title = !panel.map
    ? ""
    : hotkeyAvailable(pageKey, panel.unavailableHotkeys)
      ? `Open this map on MeowDB (${keyLabel(pageKey)})`
      : "Open this map on MeowDB";
  nextCandidate.hidden = !panel.candidates;
  grindTracker.hidden = panel.expTracking.state === "off";
  const open = panel.grindTracker.open;
  grindTracker.setAttribute("aria-pressed", String(open));
  grindTracker.setAttribute("aria-label", open ? "Hide the Grind Tracker" : "Show the Grind Tracker");
  const trackerKey = panel.hotkeys.grindTracker;
  grindTracker.title = hotkeyAvailable(trackerKey, panel.unavailableHotkeys)
    ? `${grindTracker.getAttribute("aria-label")} (${keyLabel(trackerKey)})`
    : grindTracker.getAttribute("aria-label") ?? "";
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
  if (!target || !changed("hunting", target, panel.search.credit, panel.credit)) return;
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
  normalView.hidden = view !== null || panel.settingsView !== null;
  searchView.hidden = view === null;
  const opened = view !== step;
  step = view;
  if (view === null) return;
  mobsStep.hidden = view !== "mobs";
  mapsStep.hidden = view !== "maps";
  // Before the lists, which scroll their highlight into what room is left.
  // The world crawl, while it runs: search only knows the mobs on the maps known so far.
  renderCrawl(searchCrawl, search, true);
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

/** What a hotkey does, as the settings view and the setup guide name it. */
const hotkeyName = (hotkey: Hotkey) => HOTKEY_NAMES.find(([h]) => h === hotkey)?.[1] ?? hotkey;

/** A combination as read aloud or in a sentence: "Ctrl+Shift+F", or "off". */
const spokenKeys = (binding: string) => (binding === "" ? "off" : keyLabel(binding));

/**
 * A combination as the settings view draws it: "Ctrl+Shift+F", breaking only after a "+"
 * so a long one wraps rather than squeezing what the hotkey does; "Off" with none.
 */
function keysElement(binding: string, kind: "set" | "refused" = "set"): HTMLElement {
  const keys = document.createElement("kbd");
  keys.className = `keys ${kind}`;
  if (binding === "") {
    keys.classList.add("off");
    keys.textContent = "Off";
    return keys;
  }
  const parts = keyLabel(binding).split("+");
  parts.forEach((key, i) => {
    keys.append(key);
    if (i < parts.length - 1) keys.append("+", document.createElement("wbr"));
  });
  return keys;
}

/** "Ctrl+Shift+F is already “Search a mob”. …": why the combination pressed wasn't taken. */
function rebindProblemText(problem: RebindProblem): string {
  switch (problem.kind) {
    case "noModifier":
      return `Add Ctrl, Alt or Win: as a hotkey, ${keyLabel(problem.pressed)} would never reach the game, not even its chat.`;
    case "unsupportedKey":
      return `${codeLabel(problem.code)} can't be part of a hotkey. Press another key.`;
    case "inUse":
      return `${keyLabel(problem.pressed)} is already “${hotkeyName(problem.by)}”. Swap them, or press other keys.`;
  }
}

/** "Saved: Search a mob is now Ctrl+Alt+F.": what the player's last change came to. */
function settingsNoticeText(notice: SettingsNotice, hotkeys: Hotkeys): string {
  switch (notice.kind) {
    case "saved": {
      const now = (hotkey: Hotkey) => `${hotkeyName(hotkey)} is now ${spokenKeys(hotkeys[hotkey])}`;
      const swapped = notice.swappedWith ? `, and ${now(notice.swappedWith)}` : "";
      return `Saved: ${now(notice.hotkey)}${swapped}.`;
    }
    case "restored":
      return "Every hotkey is back to its default, and on.";
    case "restoreUndone":
      return "Saved: your hotkeys are back as they were.";
  }
}

function textButton(text: string, focusKey: string, label: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "text-button";
  button.textContent = text;
  button.dataset.focus = focusKey;
  button.setAttribute("aria-label", label);
  return button;
}

/** A row's second line: what to press while it waits, why its keys were refused, or why they do nothing. */
function rowNote(text: string, kind: "help" | "problem" | "broken", tag?: string, button?: HTMLButtonElement): HTMLElement {
  const note = document.createElement("p");
  note.className = `row-note ${kind}`;
  const words = span("words", text);
  if (tag) words.prepend(span("tag", tag), " ");
  note.append(words);
  if (button) note.append(button);
  return note;
}

/** One hotkey's row: what it does, its keys and Change; waiting, its instruction or problem under it. */
function hotkeyRow(hotkey: Hotkey, name: string, panel: PanelState, view: SettingsView): HTMLLIElement {
  const row = document.createElement("li");
  row.dataset.focus = `row:${hotkey}`;
  row.tabIndex = -1;
  const binding = panel.hotkeys[hotkey];
  const waiting = view.rebinding === hotkey;
  row.classList.toggle("rebinding", waiting);
  const { problem } = view;
  let keys: HTMLElement;
  if (!waiting) keys = keysElement(binding);
  else if (problem?.kind === "unsupportedKey") keys = keysElement(codeLabel(problem.code), "refused");
  else if (problem) keys = keysElement(problem.pressed, "refused");
  else keys = span("keys pending", "Press keys…");
  const change = textButton(
    waiting ? "Cancel" : "Change",
    `change:${hotkey}`,
    waiting ? `Cancel changing ${name}` : `Change ${name}, now ${spokenKeys(binding)}`,
  );
  change.dataset.hotkey = hotkey;
  row.append(span("action", name), keys, change);
  if (waiting && problem) {
    const swap =
      problem.kind === "inUse" ? textButton("Swap", `swap:${hotkey}`, `Swap with ${hotkeyName(problem.by)}`) : undefined;
    swap?.addEventListener("click", () => invoke("rebind_swap"));
    row.append(rowNote(rebindProblemText(problem), "problem", undefined, swap));
  } else if (waiting) {
    const turnOff =
      binding === "" ? undefined : textButton("Turn off", `turn-off:${hotkey}`, `Turn off ${name}`);
    if (turnOff) turnOff.title = "Turn off (Backspace)";
    turnOff?.addEventListener("click", () => invoke("rebind_turn_off"));
    row.append(rowNote("Hold Ctrl+Shift and press a key · Esc cancels", "help", undefined, turnOff));
  } else if (binding === "" && hotkey === KEY_ONLY) {
    row.append(rowNote("It has no button, so it can't be used while it is off.", "help"));
  } else if (binding !== "" && !hotkeyAvailable(binding, panel.unavailableHotkeys)) {
    const invalid = panel.invalidHotkeys.some((key) => sameHotkey(key, binding));
    const why = invalid ? "These aren't keys a hotkey can use." : "Windows or another app has these keys.";
    row.append(rowNote(why, "broken", invalid ? "Invalid" : "Taken"));
    change.setAttribute("aria-label", `Change ${name}, now ${spokenKeys(binding)}, which does nothing: ${why}`);
  }
  return row;
}

/** What to announce for the settings view's latest change: the problem, the wait, or the notice. */
function settingsAnnouncement(view: SettingsView, hotkeys: Hotkeys): string {
  if (view.problem) return rebindProblemText(view.problem);
  if (view.rebinding) {
    return `Press the new keys for ${hotkeyName(view.rebinding)}, holding Ctrl and Shift. Escape cancels; Backspace turns it off.`;
  }
  return view.notice ? settingsNoticeText(view.notice, hotkeys) : "";
}

/**
 * The settings view, in the search view's place: each hotkey with what it does, its keys and
 * a Change button. Changing one, its row waits for the new keys, which the keys pressed now
 * are (see the keydown handler below), with its instruction, or why the keys pressed were
 * refused, under it. Every change is saved at once, and the line by Restore defaults says
 * so. The rows are drawn anew on every change, so the focus is put back where it was.
 */
function renderSettingsView(panel: PanelState) {
  const view = panel.settingsView;
  settingsView.hidden = view === null;
  const was = settings;
  settings = view;
  if (view === null) return;
  const focused = (document.activeElement as HTMLElement | null)?.dataset?.focus;
  const { hotkeys, unavailableHotkeys, invalidHotkeys } = panel;
  const redrawn = changed("hotkey-rows", hotkeys, unavailableHotkeys, invalidHotkeys, view.rebinding, view.problem);
  if (redrawn) {
    hotkeyRows.replaceChildren(...HOTKEY_NAMES.map(([hotkey, name]) => hotkeyRow(hotkey, name, panel, view)));
  }
  settingsNotice.textContent = view.notice ? settingsNoticeText(view.notice, hotkeys) : "";
  hotkeysUndo.hidden = view.notice?.kind !== "restored";
  // Nothing to restore, or a row waiting: one change at a time.
  hotkeysReset.disabled = view.atDefaults || view.rebinding !== null;
  const announcement = settingsAnnouncement(view, hotkeys);
  if (changed("settings-announce", announcement)) settingsAnnounce.textContent = announcement;
  // Opened: the first Change. Waiting: its row, so no key presses a button. Done: that
  // row's Change. Restored: Undo, and undone: Restore defaults. Otherwise where it was.
  const target =
    was === null
      ? `change:${HOTKEY_NAMES[0][0]}`
      : view.rebinding && view.rebinding !== was.rebinding
        ? `row:${view.rebinding}`
        : !view.rebinding && was.rebinding
          ? `change:${was.rebinding}`
          : view.notice?.kind !== was.notice?.kind && view.notice?.kind === "restored"
            ? "undo"
            : view.notice?.kind !== was.notice?.kind && view.notice?.kind === "restoreUndone"
              ? "reset"
              : redrawn
                ? focused
                : undefined;
  if (target) settingsView.querySelector<HTMLElement>(`[data-focus="${target}"]`)?.focus({ preventScroll: true });
}

/** The first step: the suggestions (the highlighted one is what Enter picks). */
function renderMobs(search: SearchState) {
  if (!changed("suggestions", search.suggestions, search.highlighted, search.query, search.crawl.total)) return;
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
  noMatch.textContent = search.crawl.total === 0 ? "Loading the map list…" : "No known mob has that in its name.";
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
  if (!changed("results", search.results, search.highlightedResult)) return;
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

/**
 * The world crawl's line and gauge (`el`), while it runs and `shown`: "Mapping the world:
 * 312 of 422 maps known" over the game's gauge, filled to the share known. The fill eases
 * to each new count in CSS, through `--known`.
 */
function renderCrawl(el: HTMLElement, search: SearchState, shown: boolean) {
  const line = crawlLine(search);
  el.hidden = !shown || !line;
  if (el.hidden) return;
  el.querySelector(".crawl-text")!.textContent = `Mapping the world: ${line}`;
  const { known, total } = search.crawl;
  const gauge = el.querySelector<HTMLElement>(".gauge")!;
  gauge.setAttribute("aria-valuenow", String(known));
  gauge.setAttribute("aria-valuemax", String(total));
  gauge.setAttribute("aria-valuetext", line);
  gauge.style.setProperty("--known", String(total > 0 ? known / total : 0));
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

/** Time to Level on the collapsed icon, while there is one: in the light weight while it
 *  is still settling or held, as the Grind Tracker shows it. */
function renderBadge(panel: PanelState) {
  const text = panel.expTracking.timeToLevelBadge;
  badge.hidden = !text;
  badge.textContent = text ?? "";
  badge.classList.toggle("held", panel.expTracking.state !== "active");
}

// The Map Preview's card shows over the expanded panel's normal view only, and the panel
// is measured again once its picture is in.
initMapPreview({ cardAllowed: () => !!shownPanel?.expanded && step === null, laidOut: () => reportHeight() });

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

/** The Mob Card's content: the mob, then its figures. */
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
  const who = document.createElement("div");
  who.className = "mob-card-who";
  who.append(span("mob-card-name", mob.name), span("mob-card-gait", `Lv ${mob.level} · ${card.speed}`));
  who.append(elementTags(card.elements));
  const head = document.createElement("div");
  head.className = "mob-card-head";
  head.append(sprite, who);
  return [head, mobFigures(card)];
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
    panel.data.status === "savedCopy" ||
    panel.setup.needed === "firstRun" ||
    panel.setup.needed === "resolutionChanged" ||
    panel.expTracking.state === "unreadable"
  );
}

function render(panel: PanelState) {
  const { hotkeys } = panel;
  const { recognition, map: current, notSure } = panel;
  draggable = panel.draggable;
  // Hidden while the game is minimised or gone, or another app is in front: the window
  // stays, but draws nothing.
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
    : hotkeys.setup === ""
      ? "Re-mark from Help & Setup, the ? at the top of the panel."
      : "Setup shortcut unavailable. Close the app using it, or change it in Settings (the gear at the top of the panel).";
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
    recovery.textContent =
      "Couldn't load the map list from OSMS Data Explorer. Check your connection. The map list retries every minute.";
    problem = true;
  } else if (recognition.status === "formatChanged") {
    map.textContent = "Map recognition unavailable";
    recovery.textContent = "OSMS Data Explorer's data has changed format. MapleCompass needs an update to read it.";
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
      ? "Loading the map list from OSMS Data Explorer…"
      : `Keep the minimap open with both title lines visible. If this persists, ${setupHint.charAt(0).toLowerCase() + setupHint.slice(1)}`;
  }
  recovery.hidden = recovery.textContent === "";
  recovery.classList.toggle("problem", problem);
  renderToolbar(panel);
  // Each hotkey that does nothing, its real cause, then one way to fix it.
  const causes = hotkeyCauses(panel);
  const them = panel.unavailableHotkeys.length === 1 ? "it" : "them";
  const taken = panel.unavailableHotkeys.some((key) => !panel.invalidHotkeys.includes(key));
  hotkeyProblem.textContent = causes &&
    `${causes} ${taken ? "Close the other app, or change" : "Change"} ${them} in Settings (the gear at the top of the panel).`;
  hotkeyProblem.hidden = hotkeyProblem.textContent === "";
  // The world crawl, while it runs, with or without a search: "Mapping the world: 312
  // of 422 maps known". Search can only find mobs on the maps known so far.
  renderCrawl(crawlProgress, panel.search, panel.gameFound);
  renderData(panel);
  renderHunting(panel);
  renderSearchView(panel);
  renderSettingsView(panel);
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
// The panel's height depends on its font: measure again once it has loaded.
document.fonts.ready.then(reportHeight);

// The panel is dragged by its header, or collapsed by its icon (inside the game, or
// detached), never by its buttons. A click anywhere leaves the keyboard with the game:
// the overlay can't be activated, except while the search view is open.
function startDrag() {
  getCurrentWindow().startDragging();
}

document.addEventListener("mousedown", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const onHeader = !!target?.closest(".drag-handle") && !target?.closest("button, input");
  // The map's title is also a link: it drags only once the mouse moves (below).
  const onLink = !!target?.closest(".titlebar.linked");
  if (draggable && e.button === 0 && onHeader && !onLink) startDrag();
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

/** How far the mouse moves, pressed, before a press on something clickable drags. */
const DRAG_AFTER_PX = 4;

/**
 * `el` is clicked to `press` its control, and dragged to move the panel where it can be:
 * a press becomes a drag once the mouse moves (and never a click), and a click if it is let
 * go first. Only
 * while `clickable` says so; otherwise it is left to the header's own drag.
 */
function clickOrDrag(el: HTMLElement, control: string, clickable: () => boolean) {
  let pressed: { x: number; y: number } | null = null;
  el.addEventListener("mousedown", (e) => {
    pressed = e.button === 0 && clickable() ? { x: e.screenX, y: e.screenY } : null;
  });
  el.addEventListener("mousemove", (e) => {
    if (!pressed) return;
    // Moved: not a click, and a drag where the panel can be dragged.
    if (Math.abs(e.screenX - pressed.x) + Math.abs(e.screenY - pressed.y) >= DRAG_AFTER_PX) {
      pressed = null;
      if (draggable) startDrag();
    }
  });
  el.addEventListener("mouseup", (e) => {
    if (pressed && e.button === 0) invoke("panel_press", { control });
    pressed = null;
  });
  el.addEventListener("mouseleave", () => {
    pressed = null;
  });
}

// The collapsed icon expands the panel again (like the collapse hotkey); the map's title
// opens its MeowDB page (like Ctrl+Shift+M).
clickOrDrag(icon, "togglePanel", () => true);
clickOrDrag(titlebar, "openMeowdbPage", () => titlebar.classList.contains("linked"));

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
// Whenever the overlay comes in front with the search view or the settings view open, the
// keys go to it.
window.addEventListener("focus", () => {
  if (step === "mobs") query.focus({ preventScroll: true });
  else if (step === "maps") resultsBox.focus({ preventScroll: true });
  else if (settings && !settingsView.contains(document.activeElement)) {
    const target = settings.rebinding ? `row:${settings.rebinding}` : `change:${HOTKEY_NAMES[0][0]}`;
    settingsView.querySelector<HTMLElement>(`[data-focus="${target}"]`)?.focus({ preventScroll: true });
  }
});

settingsBack.addEventListener("click", () => invoke("settings_close"));
hotkeysReset.addEventListener("click", () => invoke("hotkeys_reset"));
hotkeysUndo.addEventListener("click", () => invoke("hotkeys_undo_reset"));
// A row's Change button waits for its new combination; while it waits, it cancels.
hotkeyRows.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLButtonElement>("button[data-hotkey]") : null;
  const hotkey = button?.dataset.hotkey as Hotkey | undefined;
  if (!hotkey) return;
  if (settings?.rebinding === hotkey) invoke("rebind_cancel");
  else invoke("rebind_begin", { hotkey });
});
// The settings view's keys. While a hotkey waits for its new combination, every key goes
// to the core as part of it (a modifier on its own waits for the key, Esc cancels, and
// Backspace turns it off), and none does what it would do here, but for two that no hotkey
// could be: Tab alone moves to the waiting row's buttons, and Enter or Space alone presses
// the one focused. Otherwise Esc goes back.
document.addEventListener("keydown", (e) => {
  if (!settings || e.isComposing) return;
  if (settings.rebinding) {
    const plain = !(e.ctrlKey || e.altKey || e.metaKey);
    const onButton = e.target instanceof HTMLButtonElement && (e.key === "Enter" || e.key === " ");
    if (plain && (e.key === "Tab" || onButton)) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.repeat) return;
    const press = { code: e.code, ctrl: e.ctrlKey, alt: e.altKey, shift: e.shiftKey, win: e.metaKey };
    invoke("rebind_key", { press });
  } else if (e.key === "Escape") {
    e.preventDefault();
    invoke("settings_close");
  }
});

listen<PanelState>("overlay-state", (e) => render(e.payload));

