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
  gameWindowTitle: string;
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

/** "Win+Alt+E is taken by another app, so it does nothing here" */
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

/** "090040200 · Lv 2–14 · Blue Snail, Stump, Orange Mushroom · 1 hidden portal" */
function candidateLine(id: string, outline: Outline | null): string {
  if (!outline) return `${id} · loading…`;
  const parts = [id];
  const { mobLevels, firstMobs, hiddenPortals } = outline;
  if (mobLevels) {
    parts.push(
      mobLevels.lowest === mobLevels.highest
        ? `Lv ${mobLevels.lowest}`
        : `Lv ${mobLevels.lowest}–${mobLevels.highest}`,
      firstMobs.join(", "),
    );
  } else {
    parts.push("no mobs");
  }
  parts.push(`${hiddenPortals} hidden portal${hiddenPortals === 1 ? "" : "s"}`);
  return parts.join(" · ");
}

/** "7 Oct 2026", in the player's time zone. */
const savedOn = (unixMs: number) =>
  new Date(unixMs).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });

/** The note shown in place of (or above) the map's data. */
function dataNote(data: MapDataStatus, recognition: Recognition): string {
  const noMobs = (data: MapData) => (data.mobs.length === 0 ? " · No mobs here" : "");
  switch (data.status) {
    case "loading":
      return "Loading MeowDB data…";
    case "unavailable":
      return "MeowDB data unavailable";
    case "formatChanged":
      return "MeowDB's page format has changed, so its data isn't shown";
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
    const next = `${keyLabel(panel.hotkeys.chooseCandidate)} for the next`;
    candidatesNote.textContent = candidates.chosen
      ? `Your pick of ${candidates.maps.length} maps with this title · ${next}`
      : `${candidates.maps.length} maps share this title; not sure which you're on · ${next}`;
    candidatesNote.classList.toggle("problem", !candidates.chosen);
    candidateRows.replaceChildren(
      ...candidates.maps.map((candidate, i) => {
        const row = document.createElement("li");
        row.textContent = candidateLine(candidate.id, candidate.outline);
        row.classList.toggle("current", i === candidates.shown);
        return row;
      }),
    );
  }

  note.textContent = dataNote(data, panel.recognition);
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
      return row;
    }),
  );

  portals.hidden = !ready;
  noPortals.hidden = !ready || ready.hiddenPortals.length > 0;
  portalRows.replaceChildren(...(ready?.hiddenPortals ?? []).map(portalRow));

  credit.textContent = panel.credit ?? "";
  openHint.textContent = `${keyLabel(panel.hotkeys.openMeowdbPage)} opens the page`;
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
  document.body.classList.toggle("not-sure", !!current && notSure);
  if (!panel.gameFound) {
    street.textContent = "";
    map.textContent = `Looking for ${panel.gameWindowTitle}…`;
  } else if (panel.setup.needed === "firstRun" || panel.setup.needed === "resolutionChanged") {
    // Nothing is read until the Title Region is marked (again).
    street.textContent =
      panel.setup.needed === "firstRun" ? "Finish setup to start" : "The game window changed size";
    map.textContent = `Set up: ${keyLabel(panel.hotkeys.setup)}`;
  } else if (recognition.status === "unavailable") {
    street.textContent = "Couldn't load the map list from MeowDB";
    map.textContent = "Map recognition unavailable";
  } else if (current) {
    street.textContent = current.streetName;
    map.textContent = current.mapName;
  } else {
    street.textContent = "";
    map.textContent = "Reading the minimap title…";
  }
  if (panel.editMode) {
    const place = `${PLACEMENT_NAMES[panel.placementMode]} · ${keyLabel(hotkeys.cyclePlacement)} to change`;
    street.textContent =
      panel.placementMode === "inside"
        ? `Drag to move · ${keyLabel(hotkeys.editMode)} when done`
        : place;
  }
  hotkeyProblem.textContent = takenHotkeys(panel.unavailableHotkeys);
  hotkeyProblem.hidden = hotkeyProblem.textContent === "";
  renderData(panel);
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
