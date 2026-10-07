import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Mirror of `compass_core::PanelState`. */
type PanelState = {
  recognition: "starting" | "ready" | "unavailable";
  map: { id: string; streetName: string; mapName: string } | null;
  confidence: number;
  notSure: boolean;
  expanded: boolean;
  editMode: boolean;
};

/** Mirror of the shell's `OverlayState`. */
type OverlayState = {
  gameFound: boolean;
  panel: PanelState;
  hotkeys: { togglePanel: string; editMode: string };
};

const street = document.getElementById("street")!;
const map = document.getElementById("map")!;
let editMode = false;

/** "ctrl+shift+e" -> "Ctrl+Shift+E" */
function keyLabel(accelerator: string): string {
  return accelerator
    .split("+")
    .map((k) => k.charAt(0).toUpperCase() + k.slice(1))
    .join("+");
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
}

// In edit mode the overlay takes the mouse (the shell turns click-through off), so the
// player can drag it anywhere on it. Outside edit mode it never sees a click.
document.addEventListener("mousedown", (e) => {
  if (editMode && e.button === 0) {
    getCurrentWindow().startDragging();
  }
});

listen<OverlayState>("overlay-state", (e) => render(e.payload));
