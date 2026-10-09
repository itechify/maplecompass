import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ExpRate, ExpTracking } from "./exp-tracking";

const byId = (id: string) => document.getElementById(id)!;
const panelEl = byId("panel");
const toolbar = byId("toolbar");
const resetSession = byId("reset-session");
const trackingSwitched = byId("tracking-switched");
const trackingWaiting = byId("tracking-waiting");
const trackingFigures = byId("tracking-figures");
const timeToLevel = byId("time-to-level");
const trackingTag = byId("tracking-tag");
const rollingExp = byId("rolling-exp");
const sessionExp = byId("session-exp");
const rollingPercent = byId("rolling-percent");
const sessionPercent = byId("session-percent");
const trackingExtra = byId("tracking-extra");

/** "−" (U+2212) for a negative figure, as the deaths counter writes it. */
const signed = (text: string) => text.replace(/^-/, "−");

/** 1234 hundredths -> "12.3%" with one decimal, "12%" with none: no finer than the
 *  figure can be trusted, so it doesn't flicker. */
const percent = (hundredths: number, decimals = 1) =>
  signed(`${(hundredths / 100).toFixed(decimals)}%`);

/** An EXP rate to 3 significant figures: 840, 6,120, 61.2k, 1.23M. */
function compact(n: number): string {
  const abs = Math.abs(n);
  if (abs < 10_000) return signed(Number(n.toPrecision(3)).toLocaleString("en-US"));
  // 999,999 rounds to 1,000k: write it 1M.
  const [value, unit] = Number((abs / 1_000).toPrecision(3)) < 1_000 ? [n / 1_000, "k"] : [n / 1_000_000, "M"];
  return signed(`${Number(value.toPrecision(3))}${unit}`);
}

const TRACKING_TAGS: Partial<Record<ExpTracking["state"], string>> = {
  warmingUp: "warming up",
  paused: "paused",
};

/** The Grind Tracker: a quiet line until the Session starts, then Time to Level, the
 *  rates, and the ceiling and deaths when the state carries them. "—" for any figure not
 *  there yet. Above them, when the Session started afresh by itself, why. */
function render(tracking: ExpTracking) {
  const waiting = tracking.state === "waiting" || tracking.state === "off";
  trackingWaiting.hidden = !waiting;
  trackingFigures.hidden = waiting;
  trackingSwitched.hidden = !tracking.characterSwitched;
  // The reset icon, while there is a Session to reset.
  resetSession.hidden = waiting;

  timeToLevel.textContent = tracking.timeToLevelBadge ?? "—";
  const tag = TRACKING_TAGS[tracking.state];
  trackingTag.hidden = !tag;
  trackingTag.textContent = tag ?? "";
  trackingTag.className = `tag ${tracking.state}`;

  const exp = (rate: ExpRate | null) => (rate ? compact(rate.expPerHour) : "—");
  const share = (rate: ExpRate | null) =>
    rate?.percentPerHourHundredths != null ? percent(rate.percentPerHourHundredths) : "—";
  rollingExp.textContent = exp(tracking.rolling);
  sessionExp.textContent = exp(tracking.sessionAverage);
  rollingPercent.textContent = share(tracking.rolling);
  sessionPercent.textContent = share(tracking.sessionAverage);

  const extra = [];
  if (tracking.ceilingShareHundredths !== null) {
    extra.push(`${percent(tracking.ceilingShareHundredths, 0)} of the map's solo ceiling`);
  }
  const { deaths } = tracking;
  if (deaths.count > 0) {
    // No-break spaces: where the line wraps, the deaths read as one.
    extra.push(`deaths: ${deaths.count} (${percent(-deaths.percentLostHundredths)})`);
  }
  trackingExtra.textContent = extra.join(" · ");
  trackingExtra.hidden = extra.length === 0;
  reportHeight();
}

// The shell sizes the window to the content, as the overlay does the panel.
let reportedHeight = 0;
function reportHeight() {
  const height = Math.ceil(panelEl.getBoundingClientRect().height);
  if (height > 0 && height !== reportedHeight) {
    reportedHeight = height;
    invoke("grind_tracker_height", { height });
  }
}
window.addEventListener("resize", reportHeight);
// The height depends on the font: measure again once it has loaded.
document.fonts.ready.then(reportHeight);

// Dragged by its header, never by its buttons. A click leaves the keyboard with the game:
// the window can't be activated.
document.addEventListener("mousedown", (e) => {
  const target = e.target instanceof Element ? e.target : null;
  const onHeader = !!target?.closest(".drag-handle") && !target?.closest("button");
  if (e.button === 0 && onHeader) getCurrentWindow().startDragging();
});

// Reset and close, through the core like the panel's icons.
toolbar.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLButtonElement>("button[data-control]") : null;
  if (button) invoke("panel_press", { control: button.dataset.control });
});

listen<ExpTracking>("grind-tracker-state", (e) => render(e.payload));
// The figures may have been sent before the page was listening.
invoke<ExpTracking | null>("grind_tracker_view").then((view) => {
  if (view) render(view);
});
reportHeight();
