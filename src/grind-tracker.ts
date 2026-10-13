import "./errors";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { byId, span } from "./dom";
import type { ExpRate, ExpTracking, PacePoint, PaceTrace } from "./exp-tracking";
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
const expGaugeLost = byId("exp-gauge-lost");
const forecast = byId("forecast");
const pace = byId("pace");
const paceName = byId("pace-name");
const paceAheadRect = byId("pace-ahead-rect");
const paceBehindRect = byId("pace-behind-rect");
const paceArea = byId("pace-area");
const paceLead = byId("pace-lead");
const paceArrived = byId("pace-arrived");
const paceBest = byId("pace-best");
const paceTrace = byId("pace-trace");
const paceBestName = byId("pace-best-name");
const paceSplit = byId("pace-split");
const paceYou = byId("pace-you");

/** A Session's length: "7m", "1h 12m". */
function length(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  const [h, m] = [Math.floor(minutes / 60), minutes % 60];
  return h === 0 ? `${m}m` : m === 0 ? `${h}h` : `${h}h ${m}m`;
}

/** The view last rendered, to draw again at another width. */
let shown: TrackerView | null = null;

/** "16.2%" from hundredths of a percent, as the bar would show it to one decimal. */
const barPercent = (hundredths: number) => `${(hundredths / 100).toFixed(1)}%`;

/** A forecast's reach: "5m", "30m", "1h", "24h". */
const reach = (seconds: number) => (seconds < 3600 ? `${seconds / 60}m` : `${seconds / 3600}h`);

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
function render(view: TrackerView) {
  shown = view;
  const { tracking: t, personalBests, personalBestsOpen } = view;
  // Not showing, the window draws nothing (the shell lets clicks through it).
  document.body.classList.toggle("hidden", !view.shown);
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
    const lost = t.lostFromHundredths;
    const before = lost === null ? "" : `, down from ${barPercent(lost)} before a death`;
    expGauge.setAttribute("aria-valuetext", `EXP ${reading}${before}`);
    // The fill and the ground lost both measure from the gauge's own --filled.
    expGauge.style.setProperty("--filled", String(Math.min(t.bar.percentHundredths, 10_000) / 10_000));
    expGaugeLost.hidden = lost === null;
    if (lost !== null) expGaugeLost.style.setProperty("--lost", String(Math.min(lost, 10_000) / 10_000));
  }
  drawForecast(t);
  drawPace(t.pace);
  reportHeight();
}

/** The forecast under the gauge: Time to Level at its end, where the bar is full, then a
 *  notch where the bar will be in so long, clear of the fill, of each other and of the
 *  end, three at most. Where two crowd, the further reach takes the nearer's place, so a
 *  slow pace reads in hours rather than skipping one. */
function drawForecast(t: ExpTracking) {
  forecast.replaceChildren();
  forecast.hidden = !t.bar || (t.timeToLevelBadge === null && t.forecast.length === 0);
  if (!t.bar || forecast.hidden) return;
  const box = forecast.getBoundingClientRect();
  const filled = (Math.min(t.bar.percentHundredths, 10_000) / 10_000) * box.width;
  const notch = (at: number, text: string, className: string) => {
    const el = span(className, text);
    el.style.setProperty("--at", String(at));
    forecast.append(el);
    const r = el.getBoundingClientRect();
    return { el, left: r.left - box.left, right: r.right - box.left };
  };
  const end = t.timeToLevelBadge === null ? null : notch(1, t.timeToLevelBadge, "notch end");
  const room = end ? end.left - 6 : box.width;
  const kept: { el: HTMLElement; left: number; right: number; said: string }[] = [];
  for (const f of t.forecast) {
    const at = f.percentHundredths / 10_000;
    // A notch on the fill's edge says nothing the fill doesn't.
    if (at * box.width - filled < 12) continue;
    const n = { ...notch(at, reach(f.afterS), "notch"), said: `${barPercent(f.percentHundredths)} in ${reach(f.afterS)}` };
    const crowds = () => kept.length > 0 && n.left < kept[kept.length - 1].right + 7;
    if (n.left < 0 || n.right > room || (!crowds() && kept.length === 3)) {
      n.el.remove();
      // Each further reach is further right: none of them fits either.
      if (n.right > room || kept.length === 3) break;
      continue;
    }
    while (crowds()) kept.pop()!.el.remove();
    kept.push(n);
  }
  const said = kept.map((n) => n.said);
  const level = t.timeToLevelBadge === null ? "" : `level up in ${t.timeToLevelBadge}`;
  forecast.setAttribute("aria-label", `At this pace: ${[...said, level].filter(Boolean).join(", ")}`);
}

/** The Pace Trace's height in CSS px; its width is the figures box's. */
const PACE_HEIGHT = 40;
/** Room at the top and the right end for the player's dot. */
const DOT_ROOM = 5;

function set(el: Element, attributes: Record<string, string | number>) {
  for (const [name, value] of Object.entries(attributes)) el.setAttribute(name, String(value));
}

/** To the half pixel, so the 1px lines sit on the pixel grid at 1x. */
const half = (n: number) => Math.round(n * 2) / 2;

/** The Pace Trace. With a Best row it is a race, drawn as split times against a ghost:
 *  the Best's pace is a flat line from where the race on the map begins, and the trace is
 *  how far ahead of it the player is, falling back as the Best's pace runs on while the
 *  EXP holds and stepping up at each gain; the lead lit, the gap behind shaded, the split
 *  written by the player's dot. Without one, it is the EXP gained, a step at each gain.
 *  The scale is the trace's own, so a lead of a few kills fills it as a race should. */
function drawPace(trace: PaceTrace | null) {
  // An <svg> has no `hidden` property: the attribute, which the page's [hidden] rule hides.
  const none = !trace || trace.points.length === 0;
  pace.toggleAttribute("hidden", none);
  if (!trace || none) return;
  const width = Math.round(pace.getBoundingClientRect().width);
  const height = PACE_HEIGHT;
  set(pace, { viewBox: `0 0 ${width} ${height}`, height });
  const { points, best, spanMs } = trace;

  // The corners of the trace, as [ms before the last change, EXP].
  const corners: [number, number][] = [];
  let perMs = 0;
  if (best) {
    const [from, to] = best;
    perMs = (to.exp - from.exp) / Math.max(1, from.agoMs - to.agoMs);
    const ahead = (p: PacePoint) => p.exp - from.exp - perMs * (from.agoMs - p.agoMs);
    corners.push([from.agoMs, 0]);
    for (const p of points.filter((p) => p.agoMs < from.agoMs)) {
      const [agoMs, lead] = corners[corners.length - 1];
      corners.push([p.agoMs, lead - perMs * (agoMs - p.agoMs)], [p.agoMs, ahead(p)]);
    }
  } else {
    corners.push([points[0].agoMs, points[0].exp]);
    for (const p of points.slice(1)) corners.push([p.agoMs, corners[corners.length - 1][1]], [p.agoMs, p.exp]);
  }

  const values = corners.map(([, v]) => v);
  let [lo, hi] = [Math.min(0, ...values), Math.max(0, ...values)];
  // In a race, never finer than a minute of the Best's pace from top to foot: a lead of a
  // few EXP is a few EXP, not the whole height.
  const least = Math.max(1, perMs * 60_000);
  if (hi - lo < least) [lo, hi] = [lo - (least - (hi - lo)) / 2, hi + (least - (hi - lo)) / 2];
  const [left, right, top, bottom] = [0.5, width - DOT_ROOM, DOT_ROOM, height - DOT_ROOM];
  const x = (agoMs: number) => half(right - (Math.min(agoMs, spanMs) / spanMs) * (right - left));
  const y = (value: number) => half(bottom - ((value - lo) / (hi - lo)) * (bottom - top));

  const line = corners.map(([agoMs, v], i) => `${i ? "L" : "M"}${x(agoMs)},${y(v)}`).join("");
  const start = x(corners[0][0]);
  const zero = y(0);
  paceTrace.setAttribute("d", line);
  // In a race, the gap between the trace and the Best's line: lit where ahead, shaded
  // where behind. Otherwise, the EXP gained, shaded down to the foot.
  paceArea.setAttribute("d", `${line}V${best ? zero : height}H${start}Z`);
  paceLead.setAttribute("d", `${line}V${zero}H${start}Z`);
  set(paceAheadRect, { x: 0, y: 0, width, height: zero });
  set(paceBehindRect, { x: 0, y: zero, width, height: height - zero });
  if (best) paceArea.setAttribute("clip-path", "url(#pace-behind)");
  else paceArea.removeAttribute("clip-path");

  paceArrived.toggleAttribute("hidden", trace.arrivedAgoMs === null);
  const arrived = x(trace.arrivedAgoMs ?? 0);
  set(paceArrived, { x1: arrived, x2: arrived, y1: 0, y2: height });

  const [, now] = corners[corners.length - 1];
  const dot = { x: x(0), y: y(now) };
  set(paceYou, { cx: dot.x, cy: dot.y });

  for (const el of [paceLead, paceBest, paceBestName, paceSplit]) el.toggleAttribute("hidden", !best);
  if (!best) {
    paceName.textContent = "Your EXP over the last 5 minutes of play, a step at each gain.";
    return;
  }
  set(paceBest, { x1: start, y1: zero, x2: dot.x, y2: zero });
  // "Best" at the start of its line, on the side the trace leaves clear there.
  const early = corners.filter(([agoMs]) => x(agoMs) <= start + 30).map(([, v]) => v);
  const aboveEarly = early.reduce((sum, v) => sum + v, 0) > 0;
  const below = (aboveEarly && zero + 11 <= height) || zero < 11;
  set(paceBestName, { x: start + 3, y: below ? zero + 10 : zero - 3 });
  // The split, by the dot: "+103" ahead, "−240" behind.
  const split = Math.round(now);
  paceSplit.textContent = split > 0 ? `+${whole(split)}` : whole(split);
  paceSplit.classList.toggle("behind", split < 0);
  set(paceSplit, { x: dot.x - 7, y: Math.max(9, Math.min(height - 1, dot.y + 4)) });
  const standing = split > 0 ? `${whole(split)} EXP ahead` : split < 0 ? `${whole(-split)} EXP behind` : "level with it";
  paceName.textContent = `Your last 5 minutes of play against your best's pace on this map: ${standing}.`;
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
// The trace and the forecast are drawn to the window's width.
window.addEventListener("resize", () => (shown ? render(shown) : reportHeight()));
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
