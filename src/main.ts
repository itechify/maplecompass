import { listen } from "@tauri-apps/api/event";

/** Mirror of `compass_core::PanelState`. */
type PanelState = {
  title: { streetName: string; mapName: string } | null;
};

/** Mirror of the shell's `OverlayState`. */
type OverlayState = {
  gameFound: boolean;
  panel: PanelState;
};

const street = document.getElementById("street")!;
const map = document.getElementById("map")!;

function render(state: OverlayState) {
  const title = state.panel.title;
  document.body.classList.toggle("unreadable", !title);
  if (!state.gameFound) {
    street.textContent = "";
    map.textContent = "Looking for MapleStory…";
  } else if (title) {
    street.textContent = title.streetName;
    map.textContent = title.mapName;
  } else {
    street.textContent = "";
    map.textContent = "Can't read the minimap title";
  }
}

listen<OverlayState>("overlay-state", (e) => render(e.payload));
