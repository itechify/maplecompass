import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hotkeyAvailable, keyLabel, type Hotkeys } from "./hotkeys";

/** Mirror of `compass_core::TestRead`. */
type TestRead =
  | { status: "reading" }
  | { status: "recognised"; map: { id: string; streetName: string; mapName: string } }
  | { status: "failed" };

type SetupReason = "firstRun" | "resolutionChanged" | "requested";

/** Mirror of the shell's `SetupView`. */
type SetupView = {
  setup: { needed: SetupReason | null; testRead: TestRead | null; credit: string | null };
  unavailableHotkeys: string[];
  recognition: { status: "starting" | "ready" | "savedCopy" | "unavailable" };
  game: { width: number; height: number } | null;
  gameTitle: string;
  hotkeys: Hotkeys;
  settingsPath: string;
};

const STEPS = ["warning", "window", "requirements", "mark", "test"] as const;
type Step = (typeof STEPS)[number] | "guide";

const $ = (id: string) => document.getElementById(id)!;
let step: Step = "warning";
let view: SetupView | null = null;
/** Why setup was last opened; a new reason restarts the walk-through. */
let openedFor: SetupReason | null = null;
let picking = false;
let returnStep: Step = "window";
let renderedRead = "";

function show(next: Step) {
  if (next === "guide" && step !== "guide") returnStep = step;
  step = next;
  for (const section of document.querySelectorAll<HTMLElement>("[data-step]")) {
    section.hidden = section.dataset.step !== step;
  }
  // A re-setup skips the warning, so it has one step fewer.
  const steps = openedFor === "firstRun" ? STEPS : STEPS.slice(1);
  $("progress").textContent = step === "guide" ? "Guide" : `Step ${steps.indexOf(step) + 1} of ${steps.length}`;
  render();
  document.querySelector<HTMLElement>(`[data-step="${step}"] h2`)?.focus();
}

function move(by: number) {
  if (step === "guide") return;
  show(STEPS[Math.min(Math.max(STEPS.indexOf(step) + by, 0), STEPS.length - 1)]);
}

function shortcut(name: keyof Hotkeys): string {
  if (!view) return "";
  const binding = view.hotkeys[name];
  return keyLabel(binding) + (hotkeyAvailable(binding, view.unavailableHotkeys) ? "" : " (unavailable)");
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

  const windowStatus = game
    ? `Found the “${gameTitle}” window (${game.width}×${game.height}).`
    : `Can't find a window titled “${gameTitle}”. Start MapleStory, or pick its window by hand.`;
  if ($("window-status").textContent !== windowStatus) $("window-status").textContent = windowStatus;
  ($("window-next") as HTMLButtonElement).disabled = !game;
  ($("pick") as HTMLButtonElement).disabled = picking;
  for (const button of document.querySelectorAll<HTMLButtonElement>(".mark")) {
    button.disabled = !game;
  }

  const read = setup.testRead;
  const box = $("test-read");
  const readKey = JSON.stringify([read, recognition.status]);
  if (readKey !== renderedRead) {
    renderedRead = readKey;
    box.className = `test-read ${read?.status ?? "reading"}`;
    if (!read || read.status === "reading") {
      box.textContent = "Reading the title…";
    } else if (read.status === "recognised") {
      box.replaceChildren();
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
  }
  ($("finish") as HTMLButtonElement).disabled = read?.status !== "recognised";
  $("test-credit").textContent = setup.credit ? `Map names from ${setup.credit}` : "";
  $("handoff").hidden = read?.status !== "recognised";
  $("resetup-hint").textContent = hotkeyAvailable(view.hotkeys.setup, view.unavailableHotkeys)
    ? `${shortcut("setup")} reopens setup and this guide while you play.`
    : "The setup shortcut is unavailable. Resolve the shortcut warning before closing this window.";
  for (const label of document.querySelectorAll<HTMLElement>("[data-shortcut]")) {
    label.textContent = shortcut(label.dataset.shortcut as keyof Hotkeys);
  }
  const taken = view.unavailableHotkeys.map(keyLabel);
  $("hotkey-problem").hidden = taken.length === 0;
  $("hotkey-problem").textContent =
    `${taken.join(", ")} ${taken.length === 1 ? "is" : "are"} taken by another app (or invalid), so ` +
    `${taken.length === 1 ? "that hotkey does" : "those hotkeys do"} nothing. Close the app that uses ` +
    `${taken.length === 1 ? "it" : "them"}, or edit the hotkeys in ${view.settingsPath}, then restart MapleCompass.`;
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
listen("marking-cancelled", () => document.querySelector<HTMLElement>(`[data-step="${step}"] .mark`)?.focus());
listen<string>("marking-error", (e) => actionError(e.payload));
$("guide-open").addEventListener("click", () => show("guide"));
$("guide-back").addEventListener("click", () => show(returnStep));

function actionError(error: unknown) {
  $("action-error").textContent = `Couldn't complete that action: ${String(error)}`;
  $("action-error").hidden = false;
}

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
    $("action-error").hidden = true;
    invoke("start_marking").catch(actionError);
  });
}

$("finish").addEventListener("click", () => invoke("finish_setup").catch(actionError));

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
