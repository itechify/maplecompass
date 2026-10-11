import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hotkeyAvailable, keyLabel, type Hotkeys } from "./hotkeys";
import { mapMark, type MapMarks } from "./map-marks";
import { LOADING_MAP_LIST, recognitionProblem, type Recognition } from "./recognition";

/** Mirror of `compass_core::TestRead`. */
type TestRead =
  | { status: "reading" }
  | { status: "recognised"; map: { id: string; streetName: string; mapName: string; mark: string | null } }
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
  /** Those of them that don't name a key combination; the rest another app has. */
  invalidHotkeys: string[];
  recognition: Recognition;
  game: { width: number; height: number } | null;
  gameTitle: string;
  hotkeys: Hotkeys;
  /** Every Map Mark, by OSMS's name for it, which a recognised map names its own by. */
  mapMarks: MapMarks;
};

const STEPS = ["warning", "window", "requirements", "mark", "test", "exp", "expTest"] as const;
/** Pages opened from the window bar, outside the walk-through: back returns to the step left. */
const ASIDES = ["guide", "about"] as const;
type Aside = (typeof ASIDES)[number];
type Step = (typeof STEPS)[number] | Aside;
const isAside = (s: Step): s is Aside => (ASIDES as readonly Step[]).includes(s);

const $ = (id: string) => document.getElementById(id)!;
let step: Step = "warning";
let view: SetupView | null = null;
/** Why setup was last opened; a new reason restarts the walk-through. */
let openedFor: SetupReason | null = null;
let picking = false;
let returnStep: Step = "window";
let renderedRead = "";

function show(next: Step) {
  if (isAside(next) && !isAside(step)) returnStep = step;
  step = next;
  for (const section of document.querySelectorAll<HTMLElement>("[data-step]")) {
    section.hidden = section.dataset.step !== step;
  }
  // A re-setup skips the warning, so it has one step fewer.
  const steps = openedFor === "firstRun" ? STEPS : STEPS.slice(1);
  $("progress").textContent =
    step === "guide" ? "Guide" : step === "about" ? "About & Credits" : `Step ${steps.indexOf(step) + 1} of ${steps.length}`;
  // On its own page, its button would only repeat the bar's label (and wrap the bar).
  $("about-open").hidden = step === "about";
  render();
  document.querySelector<HTMLElement>(`[data-step="${step}"] h2`)?.focus();
}

function move(by: number) {
  if (isAside(step)) return;
  // Back from the Grind Tracker after keeping the title goes to the title, not its test read.
  if (step === "exp" && by < 0 && !view?.setup.testRead) return show("mark");
  show(STEPS[Math.min(Math.max(STEPS.indexOf(step) + by, 0), STEPS.length - 1)]);
}

function shortcut(name: keyof Hotkeys): string {
  if (!view) return "";
  const binding = view.hotkeys[name];
  if (binding === "") return "Off";
  return keyLabel(binding) + (hotkeyAvailable(binding, view.unavailableHotkeys) ? "" : " (unavailable)");
}

const NOTICES: Record<SetupReason, string> = {
  firstRun: "",
  resolutionChanged:
    "The game window changed size since setup, so the minimap title and the EXP bar have probably moved. Mark them again.",
  requested:
    "Mark the minimap title again, or keep it and change the Grind Tracker. Until you finish, MapleCompass keeps reading the old spots.",
};

function render() {
  if (!view) return;
  const { setup, game, gameTitle, recognition } = view;
  // About & Credits is about the app, not this setup: no notice over it.
  const notice = openedFor && step !== "about" ? NOTICES[openedFor] : "";
  $("notice").hidden = !notice;
  $("notice").textContent = notice;
  // A changed window size is a problem to fix; a requested re-setup is just information.
  $("notice").classList.toggle("problem", openedFor === "resolutionChanged");

  const windowStatus = game
    ? `Found the “${gameTitle}” window (${game.width}×${game.height}).`
    : `Can't find a window titled “${gameTitle}”. Start MapleStory, or pick its window by hand.`;
  if ($("window-status").textContent !== windowStatus) $("window-status").textContent = windowStatus;
  ($("window-next") as HTMLButtonElement).disabled = !game;
  ($("pick") as HTMLButtonElement).disabled = picking;
  for (const button of document.querySelectorAll<HTMLButtonElement>(".mark, .mark-exp")) {
    button.disabled = !game;
  }
  // On a requested setup the saved title still fits, and can be kept as it is.
  $("keep-title").hidden = openedFor !== "requested" || setup.testRead !== null;

  const read = setup.testRead;
  const box = $("test-read");
  const shownMark = read?.status === "recognised" && read.map.mark ? view.mapMarks[read.map.mark] : undefined;
  const readKey = JSON.stringify([read, recognition.status, shownMark]);
  if (readKey !== renderedRead) {
    renderedRead = readKey;
    // Recognised, the map shows as the overlay's title bar will show it.
    let kind: "reading" | "recognised" | "problem" = "reading";
    if (!read || read.status === "reading") {
      box.textContent = "Reading the title…";
    } else if (read.status === "recognised") {
      kind = "recognised";
      const title = document.createElement("div");
      title.className = "title";
      const street = document.createElement("p");
      street.className = "street";
      street.textContent = read.map.streetName;
      const map = document.createElement("p");
      map.className = "map";
      map.textContent = read.map.mapName;
      title.append(street, map);
      // Its Map Mark beside both lines, as the overlay's title bar draws it; with none, no
      // room for it.
      const mark = read.map.mark && view.mapMarks[read.map.mark] ? [mapMark(view.mapMarks, read.map.mark)] : [];
      box.replaceChildren(...mark, title, tag("Recognised"));
    } else if (recognition.status === "starting") {
      box.textContent = LOADING_MAP_LIST;
    } else if (recognitionProblem(recognition)) {
      kind = "problem";
      box.textContent = recognitionProblem(recognition);
    } else {
      kind = "problem";
      box.textContent =
        "No map recognised in the marked box. Check that the minimap title is showing, then mark a box around both lines with a little of the blue panel around them.";
    }
    box.className = `test-read ${kind}`;
  }
  ($("title-next") as HTMLButtonElement).disabled = read?.status !== "recognised";
  $("test-credit").textContent = setup.credit ?? "";

  // The Grind Tracker: kept from before, it can be marked again, kept as it is or turned off.
  const kept = setup.expBarKept;
  $("exp-status").textContent = kept
    ? "The Grind Tracker is on. Mark the EXP bar again, keep it as it is, or turn it off."
    : "You can skip this and turn it on later.";
  $("exp-off").hidden = !kept;
  $("exp-skip").textContent = kept ? "Keep and finish" : "Skip";
  $("exp-test-skip").textContent = kept ? "Keep the one before" : "Skip";

  const expRead = setup.expBarTestRead;
  const expBox = $("exp-test-read");
  expBox.className = `test-read ${expRead?.status === "read" ? "recognised" : expRead?.status === "failed" ? "problem" : "reading"}`;
  if (!expRead || expRead.status === "reading") {
    expBox.textContent = "Reading the EXP bar…";
  } else if (expRead.status === "read") {
    const { exp, percentHundredths } = expRead.bar;
    const title = document.createElement("div");
    title.className = "title";
    const line = document.createElement("p");
    line.className = "figures";
    line.textContent = `EXP ${exp.toLocaleString("en-US")} [${(percentHundredths / 100).toFixed(2)}%]`;
    title.append(line);
    expBox.replaceChildren(title, tag("Read"));
  } else {
    expBox.textContent =
      "Couldn't read the EXP figures in the marked box. Check that the EXP bar is showing, then mark a box around the EXP number and the percentage in brackets.";
  }
  ($("finish") as HTMLButtonElement).disabled = expRead?.status !== "read";

  $("handoff").hidden = read?.status !== "recognised";
  $("resetup-hint").textContent = hotkeyAvailable(view.hotkeys.setup, view.unavailableHotkeys)
    ? `${shortcut("setup")} reopens setup and this guide while you play.`
    : view.hotkeys.setup === ""
      ? "Help & Setup, the ? at the top of the panel, reopens setup and this guide while you play."
      : "The setup shortcut is unavailable. Resolve the shortcut warning before closing this window.";
  for (const label of document.querySelectorAll<HTMLElement>("[data-shortcut]")) {
    label.textContent = shortcut(label.dataset.shortcut as keyof Hotkeys);
  }
  // The hotkeys are changed on the panel, which shows only with the game.
  ($("change-hotkeys") as HTMLButtonElement).disabled = !game;
  $("change-hotkeys-note").textContent = game ? "" : "Start the game first: the hotkeys are changed on its panel.";
  $("change-hotkeys-note").hidden = !!game;
  $("hotkey-problem").textContent = hotkeyProblems(view);
  $("hotkey-problem").hidden = $("hotkey-problem").textContent === "";
}

/** "Recognised" or "Read", in the overlay's tag vocabulary. */
function tag(text: string): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = "tag";
  el.textContent = text;
  return el;
}

/** Each hotkey that does nothing, with its real cause, then how to fix it. */
function hotkeyProblems(view: SetupView): string {
  const invalid = view.invalidHotkeys;
  const taken = view.unavailableHotkeys.filter((k) => !invalid.includes(k));
  if (taken.length + invalid.length === 0) return "";
  const list = (keys: string[]) => keys.map(keyLabel).join(", ");
  const sentences = [];
  if (taken.length > 0) {
    sentences.push(
      taken.length === 1
        ? `${list(taken)} is taken by another app, so it does nothing.`
        : `${list(taken)} are taken by another app, so they do nothing.`,
    );
  }
  if (invalid.length > 0) {
    sentences.push(
      invalid.length === 1 ? `${list(invalid)} isn't a key combination.` : `${list(invalid)} aren't key combinations.`,
    );
  }
  // One way to recover, in one sentence.
  const them = taken.length + invalid.length === 1 ? "it" : "them";
  sentences.push(
    `${taken.length > 0 ? "Close the other app, or change" : "Change"} ${them} in the panel's Settings (the gear at its top).`,
  );
  return sentences.join(" ");
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
listen("marking-cancelled", () => document.querySelector<HTMLElement>(`[data-step="${step}"] .mark, [data-step="${step}"] .mark-exp`)?.focus());
listen<string>("marking-error", (e) => actionError(e.payload));
$("guide-open").addEventListener("click", () => show("guide"));
// The panel's settings view comes in front with the keyboard, its first Change focused.
$("change-hotkeys").addEventListener("click", () => {
  $("action-error").hidden = true;
  invoke("open_hotkey_settings").catch(actionError);
});
$("guide-back").addEventListener("click", () => show(returnStep));
$("about-open").addEventListener("click", () => show("about"));
$("about-back").addEventListener("click", () => show(returnStep));
$("site-open").addEventListener("click", () => {
  $("action-error").hidden = true;
  invoke("open_site").catch(actionError);
});
getVersion().then((version) => ($("app-version").textContent = version));

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
    invoke("start_marking", { target: "title" }).catch(actionError);
  });
}

for (const button of document.querySelectorAll<HTMLButtonElement>(".mark-exp")) {
  button.addEventListener("click", () => {
    $("action-error").hidden = true;
    invoke("start_marking", { target: "expBar" }).catch(actionError);
  });
}

$("keep-title").addEventListener("click", () => show("exp"));
$("title-next").addEventListener("click", () => show("exp"));

/** Finish without a new EXP Bar Region: the one kept before (if any) stays. */
async function finishWithoutExpMark() {
  await invoke("discard_exp_bar_mark");
  await invoke("finish_setup");
}

$("exp-skip").addEventListener("click", () => finishWithoutExpMark().catch(actionError));
$("exp-test-skip").addEventListener("click", () => finishWithoutExpMark().catch(actionError));
$("exp-off").addEventListener("click", async () => {
  try {
    await invoke("clear_exp_bar_region");
    await invoke("finish_setup");
  } catch (error) {
    actionError(error);
  }
});
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
