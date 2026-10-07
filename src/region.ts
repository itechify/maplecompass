import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// The shell lays this window exactly over the game's client area, so a point here in
// CSS px times the device pixel ratio is the same point in client px.

const box = document.getElementById("box")!;
const hint = document.getElementById("marking-hint")!;
const status = document.getElementById("selection-status")!;
type Rect = { x: number; y: number; width: number; height: number };
let selection: Rect | null = null;
let start: { x: number; y: number } | null = null;

function reset() {
  start = null;
  box.hidden = true;
}

function update(next: Rect | null) {
  const beginning = !selection && !!next;
  selection = next;
  if (start) return;
  if (!next) { reset(); return; }
  const scale = window.devicePixelRatio;
  draw({ left: next.x / scale, top: next.y / scale, width: next.width / scale, height: next.height / scale });
  status.textContent = `Box: left ${next.x}, top ${next.y}, width ${next.width}, height ${next.height} pixels.`;
  if (beginning) hint.focus();
}

function draw(r: { left: number; top: number; width: number; height: number }) {
  Object.assign(box.style, { left: `${r.left}px`, top: `${r.top}px`, width: `${r.width}px`, height: `${r.height}px` });
  box.hidden = false;
}

function actionError(error: unknown) {
  start = null;
  update(selection);
  status.textContent = `Couldn't update the Title Region: ${String(error)}. Press Esc to return to setup and try again.`;
}

function rectTo(x: number, y: number) {
  const s = start!;
  return {
    left: Math.max(Math.min(s.x, x), 0),
    top: Math.max(Math.min(s.y, y), 0),
    width: Math.abs(x - s.x),
    height: Math.abs(y - s.y),
  };
}

document.addEventListener("mousedown", (e) => {
  if (e.button !== 0) return;
  start = { x: e.clientX, y: e.clientY };
  draw({ left: e.clientX, top: e.clientY, width: 0, height: 0 });
});

document.addEventListener("mousemove", (e) => {
  if (!start) return;
  const r = rectTo(e.clientX, e.clientY);
  draw(r);
});

document.addEventListener("mouseup", (e) => {
  if (!start || e.button !== 0) return;
  const r = rectTo(e.clientX, e.clientY);
  reset();
  // A click, not a drag.
  if (r.width < 8 || r.height < 8) { update(selection); return; }
  const px = (v: number) => Math.round(v * window.devicePixelRatio);
  invoke("mark_title_region", {
    x: px(r.left),
    y: px(r.top),
    width: px(r.width),
    height: px(r.height),
  }).catch(actionError);
});

document.addEventListener("keydown", (e) => {
  if (e.isComposing || e.ctrlKey || e.metaKey) return;
  if (e.key === "Escape") {
    e.preventDefault();
    reset();
    invoke("cancel_marking").catch(actionError);
  } else if (selection && e.key === "Enter") {
    e.preventDefault();
    reset();
    invoke("confirm_title_selection").catch(actionError);
  } else if (selection && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
    e.preventDefault();
    start = null;
    invoke("adjust_title_selection", {
      direction: e.key.slice(5).toLowerCase(), resize: e.altKey, coarse: e.shiftKey,
    }).catch(actionError);
  }
});

listen<Rect | null>("selection-state", (e) => update(e.payload));
window.addEventListener("resize", () => update(selection));
invoke<{ setup: { selection: Rect | null } } | null>("setup_view").then((view) => {
  if (view && !selection) update(view.setup.selection);
}).catch(actionError);
