import { HOTKEY_NAMES, keyLabel, type Hotkey } from "../src/hotkeys.ts";

/**
 * The hotkeys a new install starts with, as `compass_core::Hotkeys::default()` sets them
 * (`crates/compass-core/src/settings.rs`). Written here by hand: when the defaults change,
 * this changes in the same PR. A hotkey added to `Hotkeys` without one here fails the
 * type check.
 */
const DEFAULTS: Record<Hotkey, string> = {
  togglePanel: "ctrl+shift+c",
  cyclePlacement: "ctrl+shift+p",
  openMeowdbPage: "ctrl+shift+m",
  setup: "ctrl+shift+s",
  chooseCandidate: "ctrl+shift+n",
  search: "ctrl+shift+f",
  grindTracker: "ctrl+shift+g",
  itemCard: "ctrl+shift+i",
  itemLookup: "ctrl+shift+o",
};

const escape = (text: string) =>
  text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** The hotkey table's rows: what each does, named and ordered as the app's guide lists them. */
export function hotkeyRows(): string {
  return HOTKEY_NAMES.map(
    ([hotkey, name]) => `<tr><th scope="row">${escape(name)}</th><td><kbd>${escape(keyLabel(DEFAULTS[hotkey]))}</kbd></td></tr>`,
  ).join("\n");
}
