/** Display labels and availability for the bindings supplied by the core. */
export type Hotkeys = {
  togglePanel: string;
  cyclePlacement: string;
  openMeowdbPage: string;
  setup: string;
  chooseCandidate: string;
  search: string;
  grindTracker: string;
  itemLookup: string;
};

export function keyLabel(accelerator: string): string {
  return accelerator.split("+").map((part) => {
    const key = part.trim();
    return key.toLowerCase() === "super" ? "Win" : key.charAt(0).toUpperCase() + key.slice(1);
  }).join("+");
}

export function hotkeyAvailable(binding: string, unavailable: string[]): boolean {
  const normalize = (value: string) => value.split("+").map((key) => key.trim().toLowerCase()).sort().join("+");
  return !unavailable.some((key) => normalize(key) === normalize(binding));
}
