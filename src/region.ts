import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// The shell lays this window exactly over the game's client area, so a point here in
// CSS px times the device pixel ratio is the same point in client px.

const box = document.getElementById("box")!;
let start: { x: number; y: number } | null = null;

function reset() {
  start = null;
  box.hidden = true;
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
});

document.addEventListener("mousemove", (e) => {
  if (!start) return;
  const r = rectTo(e.clientX, e.clientY);
  Object.assign(box.style, {
    left: `${r.left}px`,
    top: `${r.top}px`,
    width: `${r.width}px`,
    height: `${r.height}px`,
  });
  box.hidden = false;
});

document.addEventListener("mouseup", (e) => {
  if (!start || e.button !== 0) return;
  const r = rectTo(e.clientX, e.clientY);
  reset();
  // A click, not a drag.
  if (r.width < 8 || r.height < 8) return;
  const px = (v: number) => Math.round(v * window.devicePixelRatio);
  invoke("mark_title_region", {
    x: px(r.left),
    y: px(r.top),
    width: px(r.width),
    height: px(r.height),
  });
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    reset();
    invoke("cancel_marking");
  }
});

listen("start-marking", reset);
