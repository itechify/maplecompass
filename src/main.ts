import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

type PlacementMode = "inside" | "dockedRight" | "dockedLeft" | "detached";

/** Mirror of `compass_core::PerParty`. */
type PerParty = { solo: number; fullParty: number };

/** Mirror of `compass_core::Mob`. */
type Mob = {
  name: string;
  level: number;
  hp: number;
  exp: number;
  spawnCount: number;
  spawnSharePercent: number;
};

/** Mirror of `compass_core::MapSpot`: a cell of the minimap's 5x3 grid. */
type MapSpot = { column: number; row: number };

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

/** Mirror of `compass_core::LevelRange`. */
type LevelRange = { lowest: number; highest: number };

/** Mirror of `compass_core::MapData`. */
type MapData = {
  summary: { mobsPerHour: PerParty; expPerHour: PerParty } | null;
  mobs: Mob[];
  hiddenPortals: HiddenPortal[];
  listedMobs: Mob[];
  moreMobs: { count: number; levels: LevelRange } | null;
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
  mobLevels: LevelRange | null;
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
  editMode: boolean;
  gameFound: boolean;
  placementMode: PlacementMode;
  layer: "aboveGame" | "free" | "hidden";
  draggable: boolean;
  setup: { needed: "firstRun" | "resolutionChanged" | "requested" | null };
  hotkeys: {
    togglePanel: string;
    editMode: string;
    cyclePlacement: string;
    openMeowdbPage: string;
    setup: string;
    chooseCandidate: string;
  };
  unavailableHotkeys: string[];
  invalidHotkeys: string[];
  gameWindowTitle: string;
};

const byId = (id: string) => document.getElementById(id)!;
const panelEl = byId("panel");
const street = byId("street");
const map = byId("map");
const doubt = byId("doubt");
const mode = byId("mode");
const dataEl = byId("data");
const note = byId("note");
const candidatesEl = byId("candidates");
const candidatesNote = byId("candidates-note");
const candidateRows = byId("candidate-rows");
const hunting = byId("hunting");
const noMobs = byId("no-mobs");
const statusNote = byId("status-note");
const summary = byId("summary");
const mobsTable = byId("mobs");
const mobRows = byId("mob-rows");
const moreMobs = byId("more-mobs");
const portals = byId("portals");
const noPortals = byId("no-portals");
const portalRows = byId("portal-rows");
const footer = byId("footer");
const credit = byId("credit");
const openHint = byId("open-hint");
const hotkeyProblem = byId("hotkey-problem");
const iconCount = byId("icon-count");
const iconAlert = byId("icon-alert");
let draggable = false;

const PLACEMENT_NAMES: Record<PlacementMode, string> = {
  inside: "Inside",
  dockedRight: "Docked right",
  dockedLeft: "Docked left",
  detached: "Detached",
};

/** "super+alt+e" -> "Win+Alt+E" */
function keyLabel(accelerator: string): string {
  return accelerator
    .split("+")
    .map((k) => (k.toLowerCase() === "super" ? "Win" : k.charAt(0).toUpperCase() + k.slice(1)))
    .join("+");
}

/** Whether the shell couldn't register this hotkey, so it does nothing. */
const unavailable = (panel: PanelState, accelerator: string) =>
  panel.unavailableHotkeys.some((k) => k.toLowerCase() === accelerator.toLowerCase());

/**
 * "Win+Alt+E is taken by another app, so it does nothing here." and "Win+Alt+Banana isn't
 * a key combination.", then how to fix them.
 */
function hotkeyProblems(panel: PanelState): string {
  const invalid = panel.invalidHotkeys;
  const taken = panel.unavailableHotkeys.filter((k) => !invalid.includes(k));
  if (taken.length + invalid.length === 0) return "";
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
      invalid.length === 1
        ? `${list(invalid)} isn't a key combination.`
        : `${list(invalid)} aren't key combinations.`,
    );
  }
  sentences.push("Change the hotkeys in settings.json.");
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

/** "1 hidden portal · Lv 2–14 · Blue Snail, Stump, Orange Mushroom", then the dimmed map ID. */
function candidateRow(id: string, outline: Outline | null, current: boolean): HTMLLIElement {
  const row = document.createElement("li");
  row.classList.toggle("current", current);
  const parts: string[] = [];
  if (!outline) {
    // Its page is still on its way, or MeowDB can't give it.
    parts.push("No MeowDB data yet");
  } else {
    const { mobLevels, firstMobs, hiddenPortals } = outline;
    parts.push(`${hiddenPortals} hidden portal${hiddenPortals === 1 ? "" : "s"}`);
    if (mobLevels) parts.push(levels(mobLevels), firstMobs.join(", "));
    else parts.push("no mobs");
  }
  row.append(span("outline", parts.join(" · ")), span("id", id));
  return row;
}

/** "7 Oct 2026", in the player's time zone. */
const savedOn = (unixMs: number) =>
  new Date(unixMs).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" });

/** The note shown in place of (or above) the map's data. */
function dataNote(data: MapDataStatus, recognition: Recognition): string {
  switch (data.status) {
    case "loading":
      return "Loading MeowDB data…";
    case "unavailable":
      return "MeowDB data unavailable";
    case "formatChanged":
      return "MeowDB's page format has changed, so its data isn't shown";
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
    const count = candidates.maps.length;
    const next = `${keyLabel(panel.hotkeys.chooseCandidate)}: next`;
    candidatesNote.textContent = candidates.chosen
      ? `Your pick, ${candidates.shown + 1} of ${count} maps with this title · ${next}`
      : `Not sure which of ${count} maps with this title you're on · ${next}`;
    candidatesNote.classList.toggle("problem", !candidates.chosen);
    candidateRows.replaceChildren(
      ...candidates.maps.map((c, i) => candidateRow(c.id, c.outline, i === candidates.shown)),
    );
  }

  note.textContent = dataNote(data, panel.recognition);
  note.hidden = note.textContent === "";
  const problem = ["unavailable", "formatChanged", "savedCopy"].includes(data.status);
  note.classList.toggle("problem", problem || panel.recognition.status === "savedCopy");

  const ready = data.status === "ready" || data.status === "savedCopy" ? data : null;

  portals.hidden = !ready;
  noPortals.hidden = !ready || ready.hiddenPortals.length > 0;
  portalRows.replaceChildren(...(ready?.hiddenPortals ?? []).map(portalRow));

  hunting.hidden = !ready;
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
      const name = cell(mob.name);
      name.className = "mob";
      row.append(
        name,
        cell(String(mob.level)),
        cell(number(mob.hp)),
        cell(number(mob.exp)),
        cell(`${mob.spawnSharePercent}%`),
      );
      return row;
    }),
  );
  const more = ready?.moreMobs;
  moreMobs.hidden = !more;
  if (more) moreMobs.textContent = `+${more.count} more · ${levels(more.levels)} · all on MeowDB`;

  credit.textContent = panel.credit ?? "";
  const open = panel.hotkeys.openMeowdbPage;
  // Never offer a hotkey that does nothing.
  openHint.hidden = unavailable(panel, open);
  openHint.textContent = `${keyLabel(open)}: MeowDB page`;
}

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
  document.body.classList.toggle("editing", panel.editMode);
  document.body.classList.toggle("draggable", panel.draggable);
  document.body.classList.toggle("unreadable", !current);
  // Not sure of the map, or of which map sharing its title: the facts shown are a guess.
  const doubtful = !!current && (notSure || (!!panel.candidates && !panel.candidates.chosen));
  document.body.classList.toggle("not-sure", doubtful);
  doubt.hidden = !doubtful;
  // Without a map, the title bar holds the status and a note under it says what to do.
  let status = "";
  let problem = false;
  street.textContent = "";
  if (!panel.gameFound) {
    map.textContent = `Looking for ${panel.gameWindowTitle}…`;
    status = "Run the game windowed or borderless.";
  } else if (panel.setup.needed === "firstRun" || panel.setup.needed === "resolutionChanged") {
    // Nothing is read until the Title Region is marked (again).
    map.textContent =
      panel.setup.needed === "firstRun" ? "Finish setup to start" : "The game window changed size";
    status = `${keyLabel(hotkeys.setup)} opens setup, to mark the minimap title${panel.setup.needed === "firstRun" ? "" : " again"}.`;
    problem = panel.setup.needed === "resolutionChanged";
  } else if (recognition.status === "unavailable") {
    map.textContent = "Map recognition unavailable";
    status = "Couldn't load the map list from MeowDB.";
    problem = true;
  } else if (current) {
    street.textContent = current.streetName;
    map.textContent = current.mapName;
  } else if (recognition.status === "starting") {
    map.textContent = "Loading the map list…";
  } else {
    map.textContent = "Reading the minimap title…";
    status = "Keep the minimap open.";
  }
  statusNote.textContent = status;
  statusNote.hidden = status === "";
  statusNote.classList.toggle("problem", problem);
  mode.hidden = !panel.editMode;
  if (panel.editMode) {
    mode.textContent =
      panel.placementMode === "inside"
        ? `Drag to move · ${keyLabel(hotkeys.editMode)} when done`
        : `${PLACEMENT_NAMES[panel.placementMode]} · ${keyLabel(hotkeys.cyclePlacement)} to change`;
  }
  hotkeyProblem.textContent = hotkeyProblems(panel);
  hotkeyProblem.hidden = hotkeyProblem.textContent === "";
  renderData(panel);
  renderIcon(panel);
  reportHeight(panel.expanded);
}

// The shell sizes the window to the expanded panel's content.
let reportedHeight = 0;
function reportHeight(expanded: boolean) {
  if (!expanded) return;
  const height = Math.ceil(panelEl.getBoundingClientRect().height);
  if (height > 0 && height !== reportedHeight) {
    reportedHeight = height;
    invoke("panel_height", { height });
  }
}

// A draggable overlay takes the mouse (the shell turns click-through off): in edit mode
// inside the game, and always when detached. Otherwise it never sees a click.
document.addEventListener("mousedown", (e) => {
  if (draggable && e.button === 0) {
    getCurrentWindow().startDragging();
  }
});

listen<PanelState>("overlay-state", (e) => render(e.payload));
