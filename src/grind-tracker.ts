import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { dayText } from "./dates";
import type { ExpRate, ExpTracking, PersonalBestLine, VisitBest } from "./exp-tracking";

/** What the shell sends: the figures, and whether the window shows at all. */
type TrackerView = { shown: boolean; tracking: ExpTracking };

const byId = (id: string) => document.getElementById(id)!;
const panelEl = byId("panel");
const tracking = byId("tracking");
const unreadable = byId("tracking-unreadable");
const switched = byId("tracking-switched");
const resetNote = byId("tracking-reset");
const forgottenNote = byId("tracking-forgotten");
const waiting = byId("tracking-waiting");
const figures = byId("tracking-figures");
const timeToLevel = byId("time-to-level");
const tag = byId("tracking-tag");
const rollingExp = byId("rolling-exp");
const rollingPercent = byId("rolling-percent");
const sessionRow = byId("session-row");
const sessionLength = byId("session-length");
const sessionExp = byId("session-exp");
const sessionPercent = byId("session-percent");
const best = byId("tracking-best");
const bestText = byId("tracking-best-text");
const bestTag = byId("tracking-best-tag");
const bestOpen = byId("tracking-best-open");
const bestEarlier = byId("best-earlier");
const bestEarlierExp = byId("best-earlier-exp");
const bestEarlierWhen = byId("best-earlier-when");
const bestVisit = byId("best-visit");
const bestVisitExp = byId("best-visit-exp");
const bestVisitWhen = byId("best-visit-when");
const bestForget = byId("best-forget");
const share = byId("tracking-share");
const deaths = byId("tracking-deaths");
const exp = byId("tracking-exp");
const expFigures = byId("exp-figures");
const expGauge = byId("exp-gauge");
const expGaugeFill = byId("exp-gauge-fill");

/** "−" (U+2212) for a negative figure. */
const signed = (text: string) => text.replace(/^-/, "−");

/** A whole number as the game writes it: 8,435. */
const whole = (n: number) => signed(n.toLocaleString("en-US"));

/** 1234 hundredths -> "12.3%" with one decimal, "12%" with none: no finer than the
 *  figure can be trusted, so it doesn't flicker. From 100% on, whole percents. */
const percent = (hundredths: number, decimals = Math.abs(hundredths) >= 10_000 ? 0 : 1) =>
  signed(`${(hundredths / 100).toFixed(decimals)}%`);

/** A share of a rate (a map's solo ceiling, a Personal Best): "73%", and "7.3%" under
 *  10%, where one decimal still says something. */
const sharePercent = (hundredths: number) => percent(hundredths, hundredths < 1000 ? 1 : 0);

/** An EXP rate to 3 significant figures, written out as the panel writes its rates:
 *  840, 6,120, 61,200, 1,230,000. */
const rate = (n: number) => whole(Number(n.toPrecision(3)));

/** A Session's length: "7m", "1h 12m". */
function length(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  const [h, m] = [Math.floor(minutes / 60), minutes % 60];
  return h === 0 ? `${m}m` : m === 0 ? `${h}h` : `${h}h ${m}m`;
}

const TAGS: Partial<Record<ExpTracking["state"], string>> = {
  warmingUp: "warming up",
  paused: "paused",
};

/** The Personal Best line: the best named as the Rolling Rate's row shows its rates, so
 *  it reads as the figure the player saw there. A no-break space keeps the figure with the
 *  words before it where the line wraps. */
function personalBest(line: PersonalBestLine) {
  const named = rate(line.best.expPerHour);
  if (line.kind === "firstVisit") return `Your first best EXP/h here: ${named}`;
  const of = line.shareHundredths;
  if (of === null) return `Your best EXP/h here: ${named}`;
  return `${sharePercent(of)} of your best EXP/h here (${named})`;
}

/** Whether two moments fall on the same day in the player's local time. */
const sameDay = (a: Date, b: Date) =>
  a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();

/** When a best was set, by the calendar in the player's local time: "set today", "set
 *  yesterday", "set 3 Oct", and "set 3 Oct 2025" in another year. */
function setOn(unixMs: number, now = new Date()) {
  const set = new Date(unixMs);
  if (sameDay(set, now)) return "set today";
  if (sameDay(set, new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1))) return "set yesterday";
  return `set ${dayText(set, set.getFullYear() !== now.getFullYear())}`;
}

/** One best in the opened line: its figure as the Rolling Rate showed it, and when it was
 *  set. */
function showBest(row: HTMLElement, figure: HTMLElement, date: HTMLElement, visitBest: VisitBest | null) {
  row.hidden = !visitBest;
  if (!visitBest) return;
  figure.textContent = rate(visitBest.expPerHour);
  date.textContent = visitBest.setAtUnixMs === null ? "" : setOn(visitBest.setAtUnixMs);
}

/** The pointer is on the Personal Best line: it shows opened. */
let bestHovered = false;
/** The Personal Best line last rendered, for the hover to open. */
let bestLine: PersonalBestLine | null = null;

/** The Personal Best line opened in place while the pointer is on it: the best it names
 *  and when it was set, and this Visit's best once it has beaten it; on a first Visit,
 *  this Visit's. Dated afresh each time, as days pass. Under them, Forget, labelled with
 *  the row it takes, as the core says: the higher of the two. */
function renderBestOpen() {
  const line = bestLine;
  bestOpen.hidden = !bestHovered || !line;
  best.classList.toggle("open", !bestOpen.hidden);
  if (!line || bestOpen.hidden) return;
  const [earlier, thisVisit] = line.kind === "earlierVisits" ? [line.best, line.newBest] : [null, line.best];
  showBest(bestEarlier, bestEarlierExp, bestEarlierWhen, earlier);
  showBest(bestVisit, bestVisitExp, bestVisitWhen, thisVisit);
  const forgetsThisVisit = line.forgets === "thisVisit";
  bestForget.textContent = forgetsThisVisit ? "Forget this Visit's best" : "Forget your best";
  bestForget.title = forgetsThisVisit
    ? "Forget this Visit's best: only play from now on counts towards it"
    : "Forget your best here: your next-best Visit takes over";
}

/** Why there is no Time to Level, in words: never a bare dash beside the label. */
function noTimeToLevel(rolling: ExpRate | null) {
  if (rolling && rolling.expPerHour < 0) return "losing EXP";
  if (rolling && rolling.expPerHour === 0) return "no gain yet";
  return "not yet";
}

/** The Grind Tracker: a line until the Session starts, then Time to Level over the rates
 *  and what else the Session carries; the EXP bar's last reading at the foot, from the
 *  first reading on. Above them, why the figures went or can't move. */
function render({ shown, tracking: t }: TrackerView) {
  // Not showing, the window draws nothing (the shell lets clicks through it).
  document.body.classList.toggle("hidden", !shown);
  const started = t.rolling !== null || t.state === "warmingUp" || t.state === "active" || t.state === "paused";
  tracking.dataset.state = t.state;
  // Rates still settling, held while paused, or the bar out of sight: the figures step back.
  tracking.classList.toggle("held", t.state !== "active");

  unreadable.hidden = t.state !== "unreadable";
  switched.hidden = !t.characterSwitched;
  resetNote.hidden = !t.resetUndoable;
  forgottenNote.hidden = !t.forgetUndoable;
  waiting.hidden = started || t.state === "unreadable";

  figures.hidden = !started;
  timeToLevel.textContent = t.timeToLevelBadge ?? noTimeToLevel(t.rolling);
  timeToLevel.classList.toggle("words", !t.timeToLevelBadge);
  const label = TAGS[t.state];
  tag.hidden = !label;
  tag.textContent = label ?? "";
  tag.className = `tag ${t.state}`;

  const expPerHour = (r: ExpRate | null) => (r ? rate(r.expPerHour) : "–");
  const percentPerHour = (r: ExpRate | null) =>
    r?.percentPerHourHundredths != null ? percent(r.percentPerHourHundredths) : "–";
  rollingExp.textContent = expPerHour(t.rolling);
  rollingPercent.textContent = percentPerHour(t.rolling);
  // Only once the Session is longer than the Rolling Rate's window does its average say
  // something else.
  sessionRow.hidden = t.activeS <= t.rollingS;
  sessionLength.textContent = length(t.activeS);
  sessionExp.textContent = expPerHour(t.sessionAverage);
  sessionPercent.textContent = percentPerHour(t.sessionAverage);

  // Just above the ceiling's line, in its style; the tag as the headline's. Gone from
  // under the pointer (another map, or the window hidden), it closes.
  best.hidden = !t.personalBest;
  bestLine = t.personalBest;
  if (!bestLine || !shown) bestHovered = false;
  if (t.personalBest) {
    bestText.textContent = personalBest(t.personalBest);
    bestTag.hidden = t.personalBest.kind !== "earlierVisits" || !t.personalBest.newBest;
  }
  renderBestOpen();
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

// The pointer on the Personal Best line opens it in place, and the window grows to fit;
// moving off closes it. The line and what it opens are one target, so the pointer stays
// on it as it opens, even where the window moves up to keep on the monitor.
function hoverBest(on: boolean) {
  bestHovered = on;
  renderBestOpen();
  reportHeight();
}
best.addEventListener("mouseenter", () => hoverBest(true));
best.addEventListener("mouseleave", () => hoverBest(false));

// Close, reset, Forget and their undos, through the core like the panel's icons.
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
