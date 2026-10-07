import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Mirror of `compass_core::TestRead`. */
type TestRead =
  | { status: "reading" }
  | { status: "recognised"; map: { id: string; streetName: string; mapName: string } }
  | { status: "failed" };

type SetupReason = "firstRun" | "resolutionChanged" | "requested";

/** Mirror of the shell's `SetupView`. */
type SetupView = {
  setup: { needed: SetupReason | null; testRead: TestRead | null };
  recognition: { status: "starting" | "ready" | "savedCopy" | "unavailable" };
  game: { width: number; height: number } | null;
  gameTitle: string;
  setupHotkey: string;
};

const STEPS = ["warning", "window", "requirements", "mark", "test"] as const;
type Step = (typeof STEPS)[number];

const $ = (id: string) => document.getElementById(id)!;
let step: Step = "warning";
let view: SetupView | null = null;
/** Why setup was last opened; a new reason restarts the walk-through. */
let openedFor: SetupReason | null = null;
let picking = false;

function show(next: Step) {
  step = next;
  for (const section of document.querySelectorAll<HTMLElement>("section")) {
    section.hidden = section.dataset.step !== step;
  }
  // A re-setup skips the warning, so it has one step fewer.
  const steps = openedFor === "firstRun" ? STEPS : STEPS.slice(1);
  $("progress").textContent = `Step ${steps.indexOf(step) + 1} of ${steps.length}`;
  render();
}

function move(by: number) {
  show(STEPS[Math.min(Math.max(STEPS.indexOf(step) + by, 0), STEPS.length - 1)]);
}

/** "ctrl+shift+s" -> "Ctrl+Shift+S" */
function keyLabel(accelerator: string): string {
  return accelerator
    .split("+")
    .map((k) => k.charAt(0).toUpperCase() + k.slice(1))
    .join("+");
}

const NOTICES: Record<SetupReason, string> = {
  firstRun: "",
  resolutionChanged:
    "The game window changed size since setup, so the minimap title has probably moved. Mark it again.",
  requested: "Mark the minimap title again. Until you finish, MapleCompass keeps reading the old spot.",
};

function render() {
  if (!view) return;
  const { setup, game, gameTitle, recognition } = view;
  const notice = openedFor ? NOTICES[openedFor] : "";
  $("notice").hidden = !notice;
  $("notice").textContent = notice;

  $("window-status").textContent = game
    ? `Found the “${gameTitle}” window (${game.width}×${game.height}).`
    : `Can't find a window titled “${gameTitle}”. Start MapleStory, or pick its window by hand.`;
  ($("window-next") as HTMLButtonElement).disabled = !game;
  ($("pick") as HTMLButtonElement).disabled = picking;
  for (const button of document.querySelectorAll<HTMLButtonElement>(".mark")) {
    button.disabled = !game;
  }

  const read = setup.testRead;
  const box = $("test-read");
  box.className = `test-read ${read?.status ?? "reading"}`;
  if (!read || read.status === "reading") {
    box.textContent = "Reading the title…";
  } else if (read.status === "recognised") {
    box.innerHTML = "";
    const street = document.createElement("p");
    street.className = "street";
    street.textContent = read.map.streetName;
    const map = document.createElement("p");
    map.className = "map";
    map.textContent = `${read.map.mapName} ✓`;
    box.append(street, map);
  } else if (recognition.status === "starting") {
    box.textContent = "Loading MeowDB's map list…";
  } else if (recognition.status === "unavailable") {
    box.textContent =
      "Couldn't load MeowDB's map list, so no map can be recognised yet. Check your connection; MapleCompass tries again every minute.";
  } else {
    box.textContent =
      "No map recognised in the marked box. Check that the minimap title is showing, then mark a box around both lines with a little of the blue panel around them.";
  }
  ($("finish") as HTMLButtonElement).disabled = read?.status !== "recognised";
  $("resetup-hint").textContent =
    `To mark the title again later, press ${keyLabel(view.setupHotkey)} while you play.`;
}

function update(next: SetupView) {
  view = next;
  const needed = view.setup.needed;
  if (needed && needed !== openedFor) {
    openedFor = needed;
    show(needed === "firstRun" ? "warning" : "window");
  } else {
    if (!needed) openedFor = null;
    render();
  }
}

listen<SetupView>("setup-state", (e) => update(e.payload));
// The shell may have sent the state before this page was listening.
invoke<SetupView | null>("setup_view").then((v) => {
  if (v && !view) update(v);
});

listen("title-marked", () => show("test"));

for (const button of document.querySelectorAll<HTMLButtonElement>("[data-action]")) {
  button.addEventListener("click", () => {
    const action = button.dataset.action;
    if (action === "next") move(1);
    else if (action === "back") move(-1);
    else if (action === "close") getCurrentWindow().close();
  });
}

for (const button of document.querySelectorAll<HTMLButtonElement>(".mark")) {
  button.addEventListener("click", () => {
    invoke("start_marking").catch((e) => alert(e));
  });
}

$("finish").addEventListener("click", () => invoke("finish_setup"));

// Picking by hand: the player has a few seconds to put the mouse over the game window.
// Nothing is clicked or sent to the game; only the window under the pointer is read.
$("pick").addEventListener("click", async () => {
  picking = true;
  render();
  const status = $("pick-status");
  for (let s = 3; s > 0; s--) {
    status.textContent = `Hover the mouse over the game window… ${s}`;
    await new Promise((r) => setTimeout(r, 1000));
  }
  try {
    const title = await invoke<string>("pick_game_window");
    status.textContent = `Picked “${title}”.`;
  } catch (e) {
    status.textContent = `Couldn't pick a window: ${e}. Try again.`;
  }
  picking = false;
  render();
});

show("warning");
