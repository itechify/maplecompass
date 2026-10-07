import { listen } from "@tauri-apps/api/event";

/** Mirror of `compass_core::PanelState`. */
type PanelState = {
  recognition: "starting" | "ready" | "unavailable";
  map: { id: string; streetName: string; mapName: string } | null;
  confidence: number;
  notSure: boolean;
};

/** Mirror of the shell's `OverlayState`. */
type OverlayState = {
  gameFound: boolean;
  panel: PanelState;
};

const street = document.getElementById("street")!;
const map = document.getElementById("map")!;

function render(state: OverlayState) {
  const { recognition, map: current, notSure } = state.panel;
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
}

listen<OverlayState>("overlay-state", (e) => render(e.payload));
