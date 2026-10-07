import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

type PlacementMode = "inside" | "dockedRight" | "dockedLeft" | "detached";

/** Mirror of `compass_core::PanelState`. */
type PanelState = {
  recognition: "starting" | "ready" | "unavailable";
  map: { id: string; streetName: string; mapName: string } | null;
  confidence: number;
  notSure: boolean;
  expanded: boolean;
  editMode: boolean;
  gameFound: boolean;
  placementMode: PlacementMode;
  layer: "aboveGame" | "free" | "hidden";
  draggable: boolean;
};

/** Mirror of the shell's `OverlayState`. */
type OverlayState = {
  panel: PanelState;
  hotkeys: { togglePanel: string; editMode: string; cyclePlacement: string };
};

const street = document.getElementById("street")!;
const map = document.getElementById("map")!;
let draggable = false;

const PLACEMENT_NAMES: Record<PlacementMode, string> = {
  inside: "Inside",
  dockedRight: "Docked right",
  dockedLeft: "Docked left",
  detached: "Detached",
};

/** "ctrl+shift+e" -> "Ctrl+Shift+E" */
function keyLabel(accelerator: string): string {
  return accelerator
    .split("+")
    .map((k) => k.charAt(0).toUpperCase() + k.slice(1))
    .join("+");
}

function render(state: OverlayState) {
  const { panel, hotkeys } = state;
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
    const place = `${PLACEMENT_NAMES[panel.placementMode]} · ${keyLabel(hotkeys.cyclePlacement)} to change`;
    street.textContent =
      panel.placementMode === "inside"
        ? `Drag to move · ${keyLabel(hotkeys.editMode)} when done`
        : place;
  }
}

// A draggable overlay takes the mouse (the shell turns click-through off): in edit mode
// inside the game, and always when detached. Otherwise it never sees a click.
document.addEventListener("mousedown", (e) => {
  if (draggable && e.button === 0) {
    getCurrentWindow().startDragging();
  }
});

listen<OverlayState>("overlay-state", (e) => render(e.payload));
