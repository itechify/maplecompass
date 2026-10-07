import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

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

/** Mirror of `compass_core::MapDataStatus`. */
type MapDataStatus =
  | { status: "noMap" | "loading" | "unavailable" | "formatChanged" }
  | {
      status: "ready";
      summary: { mobsPerHour: PerParty; expPerHour: PerParty } | null;
      mobs: Mob[];
    };

/** Mirror of `compass_core::PanelState`. */
type PanelState = {
  recognition: "starting" | "ready" | "unavailable";
  map: { id: string; streetName: string; mapName: string; url: string } | null;
  confidence: number;
  notSure: boolean;
  data: MapDataStatus;
  credit: string | null;
  expanded: boolean;
  editMode: boolean;
};

/** Mirror of the shell's `OverlayState`. */
type OverlayState = {
  gameFound: boolean;
  panel: PanelState;
  hotkeys: { togglePanel: string; editMode: string; openMeowdbPage: string };
};

const byId = (id: string) => document.getElementById(id)!;
const panelEl = byId("panel");
const street = byId("street");
const map = byId("map");
const dataEl = byId("data");
const note = byId("note");
const summary = byId("summary");
const mobsTable = byId("mobs");
const mobRows = byId("mob-rows");
const footer = byId("footer");
const credit = byId("credit");
const openHint = byId("open-hint");
let editMode = false;

/** "ctrl+shift+e" -> "Ctrl+Shift+E" */
function keyLabel(accelerator: string): string {
  return accelerator
    .split("+")
    .map((k) => k.charAt(0).toUpperCase() + k.slice(1))
    .join("+");
}

const number = (n: number) => n.toLocaleString("en-US");

function cell(text: string): HTMLTableCellElement {
  const td = document.createElement("td");
  td.textContent = text;
  return td;
}

/** The note shown in place of (or above) the map's data. */
function dataNote(data: MapDataStatus): string {
  switch (data.status) {
    case "loading":
      return "Loading MeowDB data…";
    case "unavailable":
      return "MeowDB data unavailable";
    case "formatChanged":
      return "MeowDB's page format has changed, so its data isn't shown";
    case "ready":
      return data.mobs.length === 0 ? "No mobs here" : "";
    case "noMap":
      return "";
  }
}

function renderData(state: OverlayState) {
  const { data } = state.panel;
  const shown = state.gameFound && !!state.panel.map;
  dataEl.hidden = !shown;
  footer.hidden = !shown;
  if (!shown) return;

  note.textContent = dataNote(data);
  note.hidden = note.textContent === "";
  note.classList.toggle("problem", data.status === "unavailable" || data.status === "formatChanged");

  const ready = data.status === "ready" ? data : null;
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

  credit.textContent = state.panel.credit ?? "";
  openHint.textContent = `${keyLabel(state.hotkeys.openMeowdbPage)} opens the page`;
}

function render(state: OverlayState) {
  const { panel } = state;
  const { recognition, map: current, notSure } = panel;
  editMode = panel.editMode;
  document.body.classList.toggle("collapsed", !panel.expanded);
  document.body.classList.toggle("editing", panel.editMode);
  document.body.classList.toggle("unreadable", !current);
  document.body.classList.toggle("not-sure", !!current && notSure);
  if (!state.gameFound) {
    street.textContent = "";
    map.textContent = "Looking for MapleStory…";
  } else if (recognition === "unavailable") {
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
    street.textContent = `Drag to move · ${keyLabel(state.hotkeys.editMode)} when done`;
  }
  renderData(state);
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

// In edit mode the overlay takes the mouse (the shell turns click-through off), so the
// player can drag it anywhere on it. Outside edit mode it never sees a click.
document.addEventListener("mousedown", (e) => {
  if (editMode && e.button === 0) {
    getCurrentWindow().startDragging();
  }
});

listen<OverlayState>("overlay-state", (e) => render(e.payload));
