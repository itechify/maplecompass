import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Mirror of `compass_core::TestRead`. */
type TestRead =
  | { status: "reading" }
  | { status: "recognised"; map: { id: string; streetName: string; mapName: string } }
  | { status: "failed" };

/** Mirror of `compass_core::ExpBarTestRead`. */
type ExpBarTestRead =
  | { status: "reading" }
  | { status: "read"; bar: { exp: number; percentHundredths: number } }
  | { status: "failed" };

type SetupReason = "firstRun" | "resolutionChanged" | "requested";

/** Mirror of the shell's `SetupView`. */
type SetupView = {
  setup: {
    needed: SetupReason | null;
    testRead: TestRead | null;
    credit: string | null;
    expBarTestRead: ExpBarTestRead | null;
    expBarKept: boolean;
  };
  unavailableHotkeys: string[];
  recognition: { status: "starting" | "ready" | "savedCopy" | "unavailable" };
  game: { width: number; height: number } | null;
  gameTitle: string;
  setupHotkey: string;
};

const STEPS = ["warning", "window", "requirements", "mark", "test", "exp", "expTest"] as const;
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
  // Back from EXP tracking after keeping the title goes to the title, not its test read.
  if (step === "exp" && by < 0 && !view?.setup.testRead) return show("mark");
  show(STEPS[Math.min(Math.max(STEPS.indexOf(step) + by, 0), STEPS.length - 1)]);
}

/** "super+alt+s" -> "Win+Alt+S" */
function keyLabel(accelerator: string): string {
  return accelerator
    .split("+")
    .map((k) => (k.toLowerCase() === "super" ? "Win" : k.charAt(0).toUpperCase() + k.slice(1)))
    .join("+");
}

const NOTICES: Record<SetupReason, string> = {
  firstRun: "",
  resolutionChanged:
    "The game window changed size since setup, so the minimap title and the EXP bar have probably moved. Mark them again.",
  requested:
    "Mark the minimap title again, or keep it and change EXP tracking. Until you finish, MapleCompass keeps reading the old spots.",
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
  for (const button of document.querySelectorAll<HTMLButtonElement>(".mark, .mark-exp")) {
    button.disabled = !game;
  }
  // On a requested setup the saved title still fits, and can be kept as it is.
  $("keep-title").hidden = openedFor !== "requested" || setup.testRead !== null;

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
  ($("title-next") as HTMLButtonElement).disabled = read?.status !== "recognised";
  $("test-credit").textContent = setup.credit ? `Map names from ${setup.credit}` : "";

  // EXP tracking: kept from before, it can be marked again, kept as it is or turned off.
  const kept = setup.expBarKept;
  $("exp-status").textContent = kept
    ? "EXP tracking is on. Mark the EXP bar again, keep it as it is, or turn tracking off."
    : "You can skip this and turn it on later.";
  $("exp-off").hidden = !kept;
  $("exp-skip").textContent = kept ? "Keep and finish" : "Skip";
  $("exp-test-skip").textContent = kept ? "Keep the one before" : "Skip";

  const expRead = setup.expBarTestRead;
  const expBox = $("exp-test-read");
  expBox.className = `test-read ${expRead?.status === "read" ? "recognised" : (expRead?.status ?? "reading")}`;
  if (!expRead || expRead.status === "reading") {
    expBox.textContent = "Reading the EXP bar…";
  } else if (expRead.status === "read") {
    const { exp, percentHundredths } = expRead.bar;
    const line = document.createElement("p");
    line.className = "map";
    line.textContent = `EXP ${exp.toLocaleString("en-US")} · ${(percentHundredths / 100).toFixed(2)}% ✓`;
    expBox.replaceChildren(line);
  } else {
    expBox.textContent =
      "Couldn't read the EXP figures in the marked box. Check that the status bar is showing, then mark a box around the EXP number and the percentage in brackets.";
  }
  ($("finish") as HTMLButtonElement).disabled = expRead?.status !== "read";

  $("resetup-hint").textContent =
    `To change setup later, press ${keyLabel(view.setupHotkey)} while you play.`;
  const taken = view.unavailableHotkeys.map(keyLabel);
  $("hotkey-problem").hidden = taken.length === 0;
  $("hotkey-problem").textContent =
    `${taken.join(", ")} ${taken.length === 1 ? "is" : "are"} taken by another app (or invalid), so ` +
    `${taken.length === 1 ? "that hotkey does" : "those hotkeys do"} nothing. Close the app that uses ` +
    `${taken.length === 1 ? "it" : "them"}, or change MapleCompass's hotkeys in settings.json, then restart MapleCompass.`;
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
listen("exp-bar-marked", () => show("expTest"));

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
    invoke("start_marking", { target: "title" }).catch((e) => alert(e));
  });
}

for (const button of document.querySelectorAll<HTMLButtonElement>(".mark-exp")) {
  button.addEventListener("click", () => {
    invoke("start_marking", { target: "expBar" }).catch((e) => alert(e));
  });
}

$("keep-title").addEventListener("click", () => show("exp"));
$("title-next").addEventListener("click", () => show("exp"));

/** Finish without a new EXP Bar Region: the one kept before (if any) stays. */
async function finishWithoutExpMark() {
  await invoke("discard_exp_bar_mark");
  await invoke("finish_setup");
}

$("exp-skip").addEventListener("click", finishWithoutExpMark);
$("exp-test-skip").addEventListener("click", finishWithoutExpMark);
$("exp-off").addEventListener("click", async () => {
  await invoke("clear_exp_bar_region");
  await invoke("finish_setup");
});
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
