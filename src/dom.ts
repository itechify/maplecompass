/** Small DOM helpers the windows share, and where the panel's floating cards go. */

/** The element with this ID, which the page always has. */
export const byId = (id: string) => document.getElementById(id)!;

/** A span with this class and text. */
export function span(className: string, text: string): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = className;
  el.textContent = text;
  return el;
}

/**
 * Where a card `height` tall floats beside `anchor` in `panel`: its top, in CSS px from the
 * panel's top. On the side it `prefer`s if it fits there, otherwise on the other, `gap`
 * clear of the anchor, the panel's edges and the window bar `bar`. Where it fits on
 * neither side, it takes the roomier one. Never outside the panel, or over the bar.
 */
export function cardTop(
  panel: HTMLElement,
  bar: Element | null | undefined,
  anchor: DOMRect,
  height: number,
  gap: number,
  prefer: "above" | "below",
): number {
  const frame = panel.getBoundingClientRect();
  const highest = (bar ? bar.getBoundingClientRect().bottom - frame.top : 0) + gap;
  const lowest = panel.clientHeight - gap - height;
  const above = anchor.top - frame.top - gap - height;
  const below = anchor.bottom - frame.top + gap;
  const fitsAbove = above >= highest;
  const fitsBelow = below <= lowest;
  let top: number;
  if (fitsAbove && (prefer === "above" || !fitsBelow)) top = above;
  else if (fitsBelow) top = below;
  else {
    const roomBelow = panel.clientHeight - gap - below;
    const roomAbove = anchor.top - frame.top - gap - highest;
    top = roomBelow >= roomAbove ? below : above;
  }
  return Math.max(highest, Math.min(top, lowest));
}
