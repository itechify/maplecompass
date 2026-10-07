import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hotkeyAvailable, keyLabel, type Hotkeys } from "./hotkeys";
import { crawlLine, resultRow, showPicked, type PickedSection, type SearchState } from "./search-state";

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
  editMode: boolean;
  gameFound: boolean;
  placementMode: PlacementMode;
  layer: "aboveGame" | "free" | "hidden";
  draggable: boolean;
  setup: { needed: "firstRun" | "resolutionChanged" | "requested" | null };
  hotkeys: Hotkeys;
  unavailableHotkeys: string[];
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
const editHint = byId("edit-hint");
const helpHint = byId("help-hint");
const uncertainty = byId("uncertainty");
const crawlProgress = byId("crawl-progress");
const mobSearch = byId("mob-search");
const searchSection: PickedSection = {
  title: byId("search-title"),
  nearest: byId("search-nearest"),
  note: byId("search-note"),
  table: byId("search-results") as HTMLTableElement,
  rows: byId("search-rows"),
};
const searchMore = byId("search-more");
const searchCredit = byId("search-credit");
let draggable = false;

const PLACEMENT_NAMES: Record<PlacementMode, string> = {
  inside: "Inside",
  dockedRight: "Docked right",
  dockedLeft: "Docked left",
  detached: "Detached",
};

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

/** The picked mob's top results, below the current map's info, until the search is cleared. */
function renderSearch(panel: PanelState) {
  const { search } = panel;
  mobSearch.hidden = !panel.gameFound || !search.picked;
  if (!search.picked) return;
  showPicked(searchSection, search, search.picked, search.topResults.map(resultRow));
  const more = search.moreResults;
  const open = hotkeyAvailable(panel.hotkeys.search, panel.unavailableHotkeys)
    ? ` · ${keyLabel(panel.hotkeys.search)} shows all` : "";
  searchMore.textContent = more > 0 ? `${more} more map${more === 1 ? "" : "s"}${open}` : "";
  searchMore.hidden = searchMore.textContent === "";
  // The footer credits MeowDB whenever a map is shown; otherwise the results do here.
  searchCredit.textContent = search.credit ?? "";
  searchCredit.hidden = !search.credit || !!panel.credit;
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
  editHint.hidden = !panel.editMode;
  if (panel.editMode) {
    street.textContent = PLACEMENT_NAMES[panel.placementMode];
    const hints = [panel.draggable ? "Drag to move" : "Docked panel stays beside the game"];
    if (hotkeyAvailable(hotkeys.cyclePlacement, panel.unavailableHotkeys)) hints.push(`${keyLabel(hotkeys.cyclePlacement)}: placement`);
    if (hotkeyAvailable(hotkeys.editMode, panel.unavailableHotkeys)) hints.push(`${keyLabel(hotkeys.editMode)}: done`);
    editHint.textContent = hints.join(" · ");
  }
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
  renderSearch(panel);
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
