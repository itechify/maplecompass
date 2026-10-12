/** Display labels and availability for the bindings supplied by the core. */
export type Hotkeys = {
  togglePanel: string;
  cyclePlacement: string;
  openMeowdbPage: string;
  setup: string;
  chooseCandidate: string;
  search: string;
  grindTracker: string;
  itemCard: string;
  itemLookup: string;
};

/** Mirror of `compass_core::Hotkey`: one of the hotkeys, named as in `Hotkeys`. */
export type Hotkey = keyof Hotkeys;

/**
 * What each hotkey does, named and ordered as the setup guide (setup.html) lists them, so
 * the panel and the guide never disagree.
 */
export const HOTKEY_NAMES: [Hotkey, string][] = [
  ["togglePanel", "Collapse / expand"],
  ["cyclePlacement", "Change placement"],
  ["chooseCandidate", "Next map with this title"],
  ["openMeowdbPage", "Open the map's MeowDB page"],
  ["search", "Search"],
  ["itemCard", "Show the hovered item's card"],
  ["itemLookup", "Open the hovered item's MeowDB page"],
  ["grindTracker", "Show / hide the Grind Tracker"],
  ["setup", "Help & Setup"],
];

/** The hotkeys whose actions have no button: turned off, they can't be used at all. */
export const KEY_ONLY: Hotkey[] = ["itemCard", "itemLookup"];

/** Mirror of `compass_core::RebindProblem`. */
export type RebindProblem =
  | { kind: "noModifier"; pressed: string }
  | { kind: "unsupportedKey"; code: string }
  | { kind: "inUse"; pressed: string; by: Hotkey };

/** Mirror of `compass_core::SettingsNotice`: what the player's last change came to. */
export type SettingsNotice =
  | { kind: "saved"; hotkey: Hotkey; swappedWith: Hotkey | null }
  | { kind: "restored" }
  | { kind: "restoreUndone" };

/** Mirror of `compass_core::SettingsView`. */
export type SettingsView = {
  rebinding: Hotkey | null;
  problem: RebindProblem | null;
  notice: SettingsNotice | null;
  /** Every hotkey has its default: Restore defaults would change nothing. */
  atDefaults: boolean;
};

/** Keys whose name in a combination isn't how a player would write them. */
const KEY_LABELS: Record<string, string> = {
  super: "Win",
  backquote: "`",
  backslash: "\\",
  bracketleft: "[",
  bracketright: "]",
  comma: ",",
  equal: "=",
  minus: "-",
  period: ".",
  quote: "'",
  semicolon: ";",
  slash: "/",
  arrowup: "Up",
  arrowdown: "Down",
  arrowleft: "Left",
  arrowright: "Right",
  pageup: "Page Up",
  pagedown: "Page Down",
  printscreen: "Print Screen",
  numpadadd: "Num Plus",
  numpadsubtract: "Num Minus",
  numpadmultiply: "Num *",
  numpaddivide: "Num /",
  numpaddecimal: "Num .",
  numpadenter: "Num Enter",
  numpadequal: "Num =",
};

export function keyLabel(accelerator: string): string {
  return accelerator.split("+").map((part) => {
    const key = part.trim().toLowerCase();
    const numpad = /^numpad(\d)$/.exec(key);
    if (numpad) return `Num ${numpad[1]}`;
    return KEY_LABELS[key] ?? part.trim().charAt(0).toUpperCase() + part.trim().slice(1);
  }).join("+");
}

/** A key that can't be part of a hotkey, by its `KeyboardEvent.code`: "CapsLock" is "Caps Lock". */
export function codeLabel(code: string): string {
  return code.replace(/([a-z])([A-Z0-9])/g, "$1 $2");
}

/** Whether the hotkey acts when pressed: it is on (not empty), and no other app has it. */
export function hotkeyAvailable(binding: string, unavailable: string[]): boolean {
  return binding !== "" && !unavailable.some((key) => sameHotkey(key, binding));
}

/** Whether two combinations name the same keys ("Ctrl+Shift+C", "shift + ctrl + c"). */
export function sameHotkey(a: string, b: string): boolean {
  const normalize = (value: string) => value.split("+").map((key) => key.trim().toLowerCase()).sort().join("+");
  return normalize(a) === normalize(b);
}
