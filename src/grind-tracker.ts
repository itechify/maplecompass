import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { byId } from "./dom";
import type { ExpRate, ExpTracking } from "./exp-tracking";
import { percent, rate, sharePercent, whole } from "./figures";

/** What the shell sends: the figures, whether the window shows at all, whether some map
 *  has a Personal Best (the Personal Bests button shows), and whether the Personal Bests
 *  window is open (the button shows as on). */
type TrackerView = { shown: boolean; tracking: ExpTracking; personalBests: boolean; personalBestsOpen: boolean };

const panelEl = byId("panel");
const openBests = byId("open-bests");
const tracking = byId("tracking");
const unreadable = byId("tracking-unreadable");
const switched = byId("tracking-switched");
const resetNote = byId("tracking-reset");
const waiting = byId("tracking-waiting");
const figures = byId("tracking-figures");
const headline = byId("tracking-headline");
const timeToLevel = byId("time-to-level");
const tag = byId("tracking-tag");
const rollingRow = byId("rolling-row");
const rollingExp = byId("rolling-exp");
const rollingPercent = byId("rolling-percent");
const rollingStar = byId("rolling-star");
const sessionRow = byId("session-row");
const sessionLength = byId("session-length");
const sessionExp = byId("session-exp");
const sessionPercent = byId("session-percent");
const bestRow = byId("best-row");
const bestExp = byId("best-exp");
const share = byId("tracking-share");
const deaths = byId("tracking-deaths");
const exp = byId("tracking-exp");
const expFigures = byId("exp-figures");
const expGauge = byId("exp-gauge");
const expGaugeFill = byId("exp-gauge-fill");

/** A Session's length: "7m", "1h 12m". */
function length(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  const [h, m] = [Math.floor(minutes / 60), minutes % 60];
  return h === 0 ? `${m}m` : m === 0 ? `${h}h` : `${h}h ${m}m`;
}

/** Why there is no Time to Level, in words: never a bare dash beside the label. */
function noTimeToLevel(rolling: ExpRate | null) {
  if (rolling && rolling.expPerHour < 0) return "losing EXP";
  if (rolling && rolling.expPerHour === 0) return "no gain yet";
  return "not yet";
}

/** The Grind Tracker: a line until the Session starts, then Time to Level over the rates
 *  and what else the Session carries; the map's Best row from arriving, before the
 *  Session starts too; the EXP bar's last reading at the foot, from the first reading on.
 *  Above them, why the figures went or can't move. */
function render({ shown, tracking: t, personalBests, personalBestsOpen }: TrackerView) {
  // Not showing, the window draws nothing (the shell lets clicks through it).
  document.body.classList.toggle("hidden", !shown);
  // Hidden, not greyed, while no map has a best to show; pressed while their window is open.
  openBests.hidden = !personalBests;
  openBests.setAttribute("aria-pressed", String(personalBestsOpen));
  const started = t.rolling !== null || t.state === "warmingUp" || t.state === "active" || t.state === "paused";
  tracking.dataset.state = t.state;
  // Rates still settling, held while paused, or the bar out of sight: the figures step back.
  tracking.classList.toggle("held", t.state !== "active");

  unreadable.hidden = t.state !== "unreadable";
  switched.hidden = !t.characterSwitched;
  resetNote.hidden = !t.resetUndoable;
  waiting.hidden = started || t.state === "unreadable";

  // The Best row shows from arriving on the map: until the Session starts it is all the
  // figures hold, under the waiting note. Time to Level and the rates wait for a gain.
  figures.hidden = !started && t.personalBest === null;
  headline.hidden = !started;
  rollingRow.hidden = !started;
  timeToLevel.textContent = t.timeToLevelBadge ?? noTimeToLevel(t.rolling);
  timeToLevel.classList.toggle("words", !t.timeToLevelBadge);
  // The headline's one tag, "paused". Warming up has none: its figures in Light say it.
  tag.hidden = t.state !== "paused";

  const expPerHour = (r: ExpRate | null) => (r ? rate(r.expPerHour) : "–");
  const percentPerHour = (r: ExpRate | null) =>
    r?.percentPerHourHundredths != null ? percent(r.percentPerHourHundredths) : "–";
  rollingExp.textContent = expPerHour(t.rolling);
  rollingPercent.textContent = percentPerHour(t.rolling);
  // The core says when Last 5 min is a new best on the map. The star keeps its room in
  // the row while it's off, so the figures don't shift as it comes and goes.
  rollingStar.classList.toggle("on", t.newBest);
  // Only once the Session is longer than the Rolling Rate's window does its average say
  // something else.
  sessionRow.hidden = !started || t.activeS <= t.rollingS;
  sessionLength.textContent = length(t.activeS);
  sessionExp.textContent = expPerHour(t.sessionAverage);
  sessionPercent.textContent = percentPerHour(t.sessionAverage);
  // The map's best from its history, written as the rates are, with its ×.
  bestRow.hidden = t.personalBest === null;
  bestExp.textContent = t.personalBest === null ? "" : rate(t.personalBest);

  const shareOf = t.ceilingShareHundredths;
  share.hidden = shareOf === null || t.soloExpCeiling === null;
  if (shareOf !== null && t.soloExpCeiling !== null) {
    share.textContent = `${sharePercent(shareOf)} of this map's solo EXP/h (${whole(t.soloExpCeiling)})`;
  }
  const died = t.deaths.count;
  deaths.hidden = died === 0;
  // No-break spaces: where the line wraps, the loss reads as one.
  const lost = percent(-t.deaths.percentLostHundredths);
  deaths.textContent = `${died} ${died === 1 ? "death" : "deaths"} (${lost} of the level)`;

  exp.hidden = !t.bar;
  if (t.bar) {
    // As the bar writes it: "8,435 [41.72%]".
    const reading = `${whole(t.bar.exp)} [${(t.bar.percentHundredths / 100).toFixed(2)}%]`;
    expFigures.textContent = reading;
    expGauge.setAttribute("aria-valuenow", String(t.bar.percentHundredths / 100));
    expGauge.setAttribute("aria-valuetext", `EXP ${reading}`);
    expGaugeFill.style.setProperty("--filled", String(Math.min(t.bar.percentHundredths, 10_000) / 10_000));
  }
  reportHeight();
}

// The shell sizes the window to the content, as the overlay does the panel. The panel is
// no taller than the window, so it is measured without that limit, or the window could
// never grow.
let reportedHeight = 0;
function reportHeight() {
  panelEl.classList.add("measuring");
  const height = Math.ceil(panelEl.getBoundingClientRect().height);
  panelEl.classList.remove("measuring");
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

// Close, reset and its Undo, removing the best and opening or closing the Personal Bests
// window, through the core like the panel's icons.
panelEl.addEventListener("click", (e) => {
  const button = e.target instanceof Element ? e.target.closest<HTMLButtonElement>("button[data-control]") : null;
  if (button) invoke("panel_press", { control: button.dataset.control });
});

listen<TrackerView>("grind-tracker-state", (e) => render(e.payload));
// The figures may have been sent before the page was listening.
invoke<TrackerView | null>("grind_tracker_view").then((view) => {
  if (view) render(view);
});
reportHeight();
