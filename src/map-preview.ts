/**
 * The Map section: the Map Preview, OSMS's picture of the whole map with every portal
 * ringed where it is, then a row per Hidden Portal. Rings are drawn in screen pixels over
 * the fitted picture, whatever its scale: blue to another map (Doors included), green
 * within the map with a dashed arrow to where it lands, grey to a map not in the data;
 * dashed when hidden or a Quest Portal. A portal the Hunting Target's route takes next
 * wears a wide magenta ring around its own, always. Hovering a ring says where it leads
 * and lights its row; hovering a row lights its ring. The wheel zooms the picture inside
 * its box towards the pointer, from the whole map to the picture's own size, the rings
 * keeping theirs; zoomed in, a drag moves it, and a double-click shows the whole map
 * again. A new map shows whole. A click does nothing.
 */

import { byId, cardTop, span } from "./dom";
import { mapMark, type MapMarks } from "./map-marks";

/** Mirror of `minerva_core::HiddenPortal`. */
export type HiddenPortal = {
  name: string;
  toMapId: string;
  toMapName: string;
  toStreetName: string;
  /** The destination's Map Mark, by OSMS's name for it. */
  toMark: string | null;
  toHiddenStreet: boolean;
  quest: boolean;
};

/** Mirror of `minerva_core::Leads`. */
type Leads = "anotherMap" | "withinTheMap" | "mapNotInTheData";

/** Mirror of `minerva_core::PortalKind`. */
type PortalKind = "visible" | "hidden" | "door" | "quest";

/** Mirror of `minerva_core::PortalMark`: where it is in the picture's pixels. */
type PortalMark = {
  x: number;
  y: number;
  leads: Leads;
  kind: PortalKind;
  toMapId: string;
  toMapName: string;
  toStreetName: string;
  /** The destination's Map Mark, by OSMS's name for it; within the map, the map's own. */
  toMark: string | null;
  /**
   * A teleport within the map: where it lands, the mark there if it has one, and whether
   * that mark lands back here (a pair).
   */
  landsAt: { x: number; y: number; mark: number | null; bothWays: boolean } | null;
  /** Its row among the Hidden Portals. */
  hiddenPortal: number | null;
  /** The Hunting Target's route goes on from the shown map through this portal. */
  nextOnRoute: boolean;
};

/**
 * How the picture is laid out: its box's size, the most it may be zoomed (to the picture's
 * own size), how far it is moved in from its top left, and its screen pixels to a picture
 * pixel.
 */
type Frame = { width: number; height: number; most: number; left: number; top: number; sx: number; sy: number };

/** Mirror of `minerva_core::MapPreview`. */
export type MapPreview = {
  picture: string;
  image: { status: "loading" | "unavailable" } | { status: "ready"; url: string };
  marks: PortalMark[];
};

/** The tallest the picture is drawn, inside its paper's 5px margin: the box stays within 180px. */
const MAX_PICTURE_HEIGHT = 168;
/** The paper's margin around the picture, so a ring at its edge isn't cut. */
const PAPER = 5;
/** A ring's radius (to the middle of its stroke), in screen pixels. */
const RING = 4;
/** The lit ring's outer ring. */
const LIT_RING = 7.5;
/** The ring around a portal next on the Hunting Target's route, outside the lit ring. */
const ROUTE_RING = 10;
/** The space kept between the card and the ring, and the panel's edges. */
const CARD_GAP = 6;
/** How much a notch of the mouse wheel zooms in, or out. */
const ZOOM_PER_NOTCH = 1.25;
/** The wheel's delta for a notch, in CSS px; a touchpad sends smaller ones. */
const NOTCH = 100;

const SVG = "http://www.w3.org/2000/svg";

const preview = byId("map-preview");
const picture = byId("map-picture");
const marksSvg = byId("map-marks") as unknown as SVGSVGElement;
/** The picture, under its rings; given a source once the shell has saved it. */
const image = document.createElement("img");
image.alt = "";
image.draggable = false;
image.hidden = true;
/** The picture's fitted box, which cuts it while zoomed in. */
const crop = document.createElement("div");
crop.className = "map-crop";
crop.append(image);
picture.prepend(crop);
const loading = byId("map-loading");
const failed = byId("map-failed");
const rows = byId("portal-rows");
const card = byId("mark-card");
const panelEl = byId("panel");

/** The preview last rendered, and the shown map's name for its label. */
let shown: MapPreview | null = null;
let mapName = "";
/** Every Map Mark, as the rows were last drawn with, for the card's destination. */
let cardMarks: MapMarks = {};
/** The overlay couldn't load the picture at this URL. */
let brokenUrl: string | null = null;
/** What the rings were last drawn for: the marks and the picture's drawn size. */
let drawnFor = "";
/** The mark the pointer is on (or on whose row), if any. */
let lit: { mark: number; onRing: boolean } | null = null;
/**
 * How far the player has zoomed in, over the fit: 1 shows the whole map. Kept while the
 * same map shows, as is the spot in the picture's pixels at the middle of the box (null:
 * the picture's middle), so the section opening again or the panel's width changing
 * shows the same part of it.
 */
let zoom = 1;
let centre: { x: number; y: number } | null = null;
/** How the picture was last laid out, while it shows. */
let frame: Frame | null = null;
/** A drag moving the zoomed picture: where it started, and the picture's offset then. */
let panning: { x: number; y: number; left: number; top: number } | null = null;
/** Whether the mark card may show now: the panel expanded, on its normal view. */
let cardAllowed: () => boolean = () => true;
/** The panel's height may have changed: the overlay measures it again. */
let laidOut: () => void = () => {};

/** Called once: what the preview asks of the panel. */
export function initMapPreview(options: { cardAllowed: () => boolean; laidOut: () => void }) {
  cardAllowed = options.cardAllowed;
  laidOut = options.laidOut;
}

function svg<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number>) {
  const el = document.createElementNS(SVG, tag);
  for (const [name, value] of Object.entries(attrs)) el.setAttribute(name, String(value));
  return el;
}

/** The classes a mark's ring is drawn with: where it leads, and dashed when hidden. */
function ringClass(mark: PortalMark): string {
  const dashed = mark.kind === "hidden" || mark.kind === "quest" ? " dashed" : "";
  return `ring ${mark.leads}${dashed}`;
}

/** A ring: a white halo, then the ring in its colour over a faint fill. */
function ring(mark: PortalMark, cx: number, cy: number): SVGGElement {
  const g = svg("g", { class: ringClass(mark) });
  g.append(
    svg("circle", { class: "halo", cx, cy, r: RING }),
    svg("circle", { class: "stroke", cx, cy, r: RING }),
    svg("circle", { class: "lit-ring", cx, cy, r: LIT_RING }),
  );
  return g;
}

/** The ring a Hidden Portal's row leads with: its mark's, as on the map. */
function rowRing(mark: PortalMark | undefined): SVGSVGElement {
  const el = svg("svg", { class: "row-ring", viewBox: "0 0 12 12", "aria-hidden": "true" });
  const leads: PortalMark = mark ?? {
    x: 0,
    y: 0,
    leads: "anotherMap",
    kind: "hidden",
    toMapId: "",
    toMapName: "",
    toStreetName: "",
    toMark: null,
    landsAt: null,
    hiddenPortal: null,
    nextOnRoute: false,
  };
  el.append(ring(leads, 6, 6));
  return el;
}

/**
 * A row per Hidden Portal, each led by its mark's ring: "Northern Top of Construction Site
 * [Hidden Street]". Hovering one lights its mark.
 */
export function renderPortalRows(portals: HiddenPortal[], marks: PortalMark[], mapMarks: MapMarks) {
  cardMarks = mapMarks;
  rows.replaceChildren(
    ...portals.map((portal, i) => {
      const at = marks.findIndex((m) => m.hiddenPortal === i);
      const row = document.createElement("li");
      if (at >= 0) row.dataset.mark = String(at);
      const to = document.createElement("div");
      to.className = "dest";
      to.append(rowRing(marks[at]), mapMark(mapMarks, portal.toMark), span("to", portal.toMapName));
      if (portal.toHiddenStreet) to.append(span("tag", "Hidden Street"));
      if (portal.quest) to.append(span("tag quest", "Quest"));
      row.append(to);
      return row;
    }),
  );
  relight();
}

/**
 * The Map Preview, or nothing while there is none (no map, no picture, or not settled
 * which map it is). While the picture loads, "Loading map…" in a box of fixed height; if
 * it can't be had, "Couldn't load the map picture" in its place. Whether either shows.
 */
export function renderMapPreview(next: MapPreview | null, name: string): boolean {
  shown = next;
  mapName = name;
  const url = next?.image.status === "ready" ? next.image.url : null;
  const unavailable = next?.image.status === "unavailable" || (url !== null && url === brokenUrl);
  preview.hidden = !next || unavailable;
  failed.hidden = !next || !unavailable;
  if (!next || unavailable) {
    hoverMark(null);
    return !!next;
  }
  if (url && image.getAttribute("src") !== url) {
    // Another map: nothing on it is lit until the pointer moves onto it, and it shows whole.
    lit = null;
    drawnFor = "";
    zoom = 1;
    centre = null;
    image.hidden = true;
    image.src = url;
  }
  if (!url) image.removeAttribute("src");
  layOutMapPreview();
  return true;
}

/**
 * The picture fitted and centred, zoomed and moved inside its box as the player has, its
 * rings drawn over it; or the placeholder while it loads. Again whenever the section
 * opens, the panel changes width, or the player zooms or drags.
 */
export function layOutMapPreview() {
  const ready = !!shown && image.hasAttribute("src") && image.complete && image.naturalWidth > 0;
  preview.classList.toggle("loading", !ready);
  loading.hidden = ready;
  picture.hidden = !ready;
  image.hidden = !ready;
  frame = null;
  // Collapsed, the box has no width to fit the picture to.
  if (!ready || !shown || preview.clientWidth === 0) return;
  // Fitted as if the panel's scrollbar showed, whether it does or not. Fitted to the room
  // the scrollbar leaves, the picture would be taller without it: in a panel just short of
  // the picture's height the scrollbar would come and go every frame.
  const room = preview.clientWidth - (scrollbarShows() ? 0 : scrollbarWidth()) - 2 * PAPER;
  const scale = Math.min(room / image.naturalWidth, MAX_PICTURE_HEIGHT / image.naturalHeight);
  const width = Math.round(image.naturalWidth * scale);
  const height = Math.round(image.naturalHeight * scale);
  picture.style.width = `${width}px`;
  picture.style.height = `${height}px`;
  // Zoomed in no further than the picture's own size, and moved no further than its edges.
  const most = Math.max(1, image.naturalWidth / width);
  zoom = Math.min(Math.max(zoom, 1), most);
  const sx = (width * zoom) / image.naturalWidth;
  const sy = (height * zoom) / image.naturalHeight;
  const middle = centre ?? { x: image.naturalWidth / 2, y: image.naturalHeight / 2 };
  const left = Math.min(Math.max(middle.x * sx - width / 2, 0), width * zoom - width);
  const top = Math.min(Math.max(middle.y * sy - height / 2, 0), height * zoom - height);
  centre = zoom > 1 ? { x: (left + width / 2) / sx, y: (top + height / 2) / sy } : null;
  frame = { width, height, most, left, top, sx, sy };
  image.style.width = `${width * zoom}px`;
  image.style.height = `${height * zoom}px`;
  image.style.transform = `translate(${-left}px, ${-top}px)`;
  preview.classList.toggle("zoomed", zoom > 1);
  const hidden = shown.marks.filter((m) => m.hiddenPortal !== null).length;
  const next = shown.marks.filter((m) => m.nextOnRoute).length;
  const portals = shown.marks.length;
  const onRoute = next > 0 ? `, ${next} next on your route` : "";
  picture.setAttribute(
    "aria-label",
    `Map of ${mapName}: ${portals} portal${portals === 1 ? "" : "s"} marked, ${hidden} of them hidden${onRoute}`,
  );
  const key = JSON.stringify([shown.marks, frame]);
  if (key !== drawnFor) {
    drawnFor = key;
    drawMarks(shown.marks, frame);
    relight();
  }
}

/** Where a ring or a landing is drawn, in screen pixels over the box. */
const at = (x: number, y: number, f: Frame) => ({ x: x * f.sx - f.left, y: y * f.sy - f.top });

/** The arrows of the teleports within the map, under every ring, then the rings. */
function drawMarks(marks: PortalMark[], f: Frame) {
  marksSvg.setAttribute("width", String(f.width));
  marksSvg.setAttribute("height", String(f.height));
  const defs = svg("defs", {});
  const head = svg("marker", {
    id: "landing-head",
    viewBox: "0 0 6 6",
    refX: 5,
    refY: 3,
    markerWidth: 6,
    markerHeight: 6,
    markerUnits: "userSpaceOnUse",
    orient: "auto-start-reverse",
  });
  head.append(svg("path", { d: "M0 0.5 6 3 0 5.5z", class: "head" }));
  defs.append(head);
  const arrows = svg("g", { class: "arrows" });
  marks.forEach((mark, i) => {
    const landing = mark.landsAt;
    if (mark.leads !== "withinTheMap" || !landing) return;
    // A pair is one line with a head at each end, drawn once.
    if (landing.bothWays && landing.mark !== null && landing.mark < i) return;
    const from = at(mark.x, mark.y, f);
    const to = at(landing.x, landing.y, f);
    const length = Math.hypot(to.x - from.x, to.y - from.y);
    // From ring edge to ring edge, or to the landing point when it has no ring.
    const start = RING + 2;
    const end = landing.mark !== null ? RING + 2 : 0;
    if (length <= start + end + 4) return;
    const ux = (to.x - from.x) / length;
    const uy = (to.y - from.y) / length;
    const line = {
      x1: from.x + ux * start,
      y1: from.y + uy * start,
      x2: to.x - ux * end,
      y2: to.y - uy * end,
    };
    const g = svg("g", { class: "arrow", "data-from": i, "data-to": landing.mark ?? "" });
    const dash = svg("line", { ...line, class: "dash", "marker-end": "url(#landing-head)" });
    if (landing.bothWays) dash.setAttribute("marker-start", "url(#landing-head)");
    g.append(svg("line", { ...line, class: "halo" }), dash);
    arrows.append(g);
  });
  // Next on the Hunting Target's route: a wide ring around the portal's own, on a halo of
  // its own, always shown. Over the arrows and under every ring, so it never hides one.
  const routeRings = svg("g", { class: "route-rings" });
  const rings = svg("g", { class: "rings" });
  marks.forEach((mark, i) => {
    const { x, y } = at(mark.x, mark.y, f);
    if (mark.nextOnRoute) {
      routeRings.append(
        svg("circle", { class: "route-halo", cx: x, cy: y, r: ROUTE_RING }),
        svg("circle", { class: "route-ring", cx: x, cy: y, r: ROUTE_RING }),
      );
    }
    const g = ring(mark, x, y);
    g.dataset.mark = String(i);
    // A target larger than the ring, so a small ring is easy to rest on.
    g.append(svg("circle", { class: "hit", cx: x, cy: y, r: 8 }));
    rings.append(g);
  });
  marksSvg.replaceChildren(defs, arrows, routeRings, rings);
}

/** Light `mark` (or nothing) and its row, its arrows, and its card if the pointer is on its ring. */
function hoverMark(mark: number | null, onRing = false) {
  lit = mark === null ? null : { mark, onRing };
  relight();
}

/** Light the lit mark again after a redraw: its ring, its arrows, its row and its card. */
function relight() {
  const i = lit?.mark ?? -1;
  for (const g of marksSvg.querySelectorAll<SVGGElement>(".rings > g")) {
    const on = Number(g.dataset.mark) === i;
    g.classList.toggle("lit", on);
    // Over its neighbours while lit.
    if (on && g.nextSibling) g.parentNode?.append(g);
  }
  for (const g of marksSvg.querySelectorAll<SVGGElement>(".arrow")) {
    g.classList.toggle("lit", g.dataset.from === String(i) || g.dataset.to === String(i));
  }
  for (const row of rows.querySelectorAll<HTMLElement>("li")) {
    row.classList.toggle("lit", row.dataset.mark === String(i));
  }
  renderCard();
}

/** The tags saying what kind of portal a mark is. A visible portal to a known map has none. */
function kindTags(mark: PortalMark): HTMLElement[] {
  const tags: HTMLElement[] = [];
  if (mark.kind === "hidden") tags.push(span("tag", "Hidden"));
  if (mark.kind === "quest") tags.push(span("tag quest", "Quest"));
  if (mark.kind === "door") tags.push(span("tag", "Door"));
  if (mark.leads === "withinTheMap") tags.push(span("tag", "Within the map"));
  if (mark.leads === "mapNotInTheData") tags.push(span("tag", "Not in the map list"));
  if (mark.nextOnRoute) tags.push(span("tag", "Next on your route"));
  return tags;
}

/**
 * The card of the ring the pointer is on: where it leads, its street and its kind. Above
 * the ring, centred on it, or below it when there's no room above; never outside the
 * panel or over its window bar. The pointer passes through it.
 */
function renderCard() {
  const mark = lit?.onRing && shown ? shown.marks[lit.mark] : undefined;
  const target = mark ? marksSvg.querySelector<SVGGElement>(`.rings > g[data-mark="${lit!.mark}"] .stroke`) : null;
  if (!mark || !target || !cardAllowed() || preview.hidden) {
    card.hidden = true;
    return;
  }
  const head = document.createElement("div");
  head.className = "mark-card-to";
  head.append(mapMark(cardMarks, mark.toMark), mark.toMapName);
  const parts: Node[] = [head];
  if (mark.toStreetName) parts.push(span("mark-card-street", mark.toStreetName));
  const tags = kindTags(mark);
  if (tags.length) {
    const line = document.createElement("div");
    line.className = "mark-card-kinds";
    line.append(...tags);
    parts.push(line);
  }
  card.replaceChildren(...parts);
  card.hidden = false;
  const spot = target.getBoundingClientRect();
  const { offsetWidth: width, offsetHeight: height } = card;
  const centre = spot.left + spot.width / 2 - panelEl.getBoundingClientRect().left;
  const left = Math.max(8, Math.min(centre - width / 2, panelEl.clientWidth - 8 - width));
  card.style.left = `${left}px`;
  card.style.top = `${cardTop(panelEl, panelEl.querySelector(".window-bar"), spot, height, CARD_GAP, "above")}px`;
}

// Resting on a ring lights it and its row and shows its card; on a row, lights its ring.
// Nothing lights while a drag moves the picture.
marksSvg.addEventListener("mouseover", (e) => {
  if (panning) return;
  const g = e.target instanceof Element ? e.target.closest<SVGGElement>(".rings > g") : null;
  hoverMark(g ? Number(g.dataset.mark) : null, true);
});
marksSvg.addEventListener("mouseleave", () => hoverMark(null));
rows.addEventListener("mouseover", (e) => {
  const row = e.target instanceof Element ? e.target.closest<HTMLElement>("li[data-mark]") : null;
  hoverMark(row ? Number(row.dataset.mark) : null);
});
rows.addEventListener("mouseleave", () => hoverMark(null));

/** Light the ring at a point the pointer rests on, if any: the picture moved under it. */
function hoverAt(x: number, y: number) {
  const g = document.elementFromPoint(x, y)?.closest<SVGGElement>(".rings > g");
  hoverMark(g ? Number(g.dataset.mark) : null, true);
}

/** Show the picture at `next` times the fit, the spot under the pointer staying under it. */
function zoomAt(next: number, e: MouseEvent) {
  if (!frame) return;
  const box = picture.getBoundingClientRect();
  const x = e.clientX - box.left;
  const y = e.clientY - box.top;
  const spot = { x: (frame.left + x) / frame.sx, y: (frame.top + y) / frame.sy };
  zoom = Math.min(Math.max(next, 1), frame.most);
  // Back out to the whole map, never a hair's breadth from it.
  if (zoom < 1.001) zoom = 1;
  const sx = (frame.width * zoom) / image.naturalWidth;
  const sy = (frame.height * zoom) / image.naturalHeight;
  centre = { x: spot.x + (frame.width / 2 - x) / sx, y: spot.y + (frame.height / 2 - y) / sy };
  layOutMapPreview();
  hoverAt(e.clientX, e.clientY);
}

// Over the shown picture's box, its paper included, the wheel zooms and never scrolls the
// panel: in as far as the picture's own size, out as far as the whole map. While it loads,
// the wheel scrolls the panel as anywhere else.
preview.addEventListener("wheel", (e) => {
  if (!frame) return;
  e.preventDefault();
  zoomAt(zoom * ZOOM_PER_NOTCH ** (-e.deltaY / NOTCH), e);
});

// Zoomed in, a drag moves the picture, no further than its edges.
preview.addEventListener("pointerdown", (e) => {
  if (e.button !== 0 || !frame || zoom === 1) return;
  panning = { x: e.clientX, y: e.clientY, left: frame.left, top: frame.top };
  preview.setPointerCapture(e.pointerId);
});
preview.addEventListener("pointermove", (e) => {
  if (!panning || !frame || (e.clientX === panning.x && e.clientY === panning.y)) return;
  if (!preview.classList.contains("panning")) {
    preview.classList.add("panning");
    hoverMark(null);
  }
  const left = panning.left - (e.clientX - panning.x);
  const top = panning.top - (e.clientY - panning.y);
  centre = { x: (left + frame.width / 2) / frame.sx, y: (top + frame.height / 2) / frame.sy };
  layOutMapPreview();
});
function endPan() {
  panning = null;
  preview.classList.remove("panning");
}
// Let go, the ring it rests on lights.
preview.addEventListener("pointerup", (e) => {
  if (!panning) return;
  const moved = preview.classList.contains("panning");
  endPan();
  if (moved) hoverAt(e.clientX, e.clientY);
});
preview.addEventListener("pointercancel", endPan);

// A double-click shows the whole map again.
preview.addEventListener("dblclick", (e) => {
  if (frame && zoom > 1) zoomAt(1, e);
});

image.addEventListener("load", () => {
  layOutMapPreview();
  laidOut();
});
// The overlay couldn't load the picture the source saved: as if it couldn't be had.
image.addEventListener("error", () => {
  if (!image.hasAttribute("src")) return;
  brokenUrl = image.getAttribute("src");
  renderMapPreview(shown, mapName);
  laidOut();
});
/** Whether the panel's scrolling view shows its scrollbar now. */
function scrollbarShows(): boolean {
  const view = preview.closest<HTMLElement>(".view");
  return !!view && view.offsetWidth - view.clientWidth - view.clientLeft * 2 > 0;
}

/** How wide a scrollbar is here, in CSS px: it changes with the display's scale. */
function scrollbarWidth(): number {
  const probe = document.createElement("div");
  probe.style.cssText = "position: absolute; visibility: hidden; width: 50px; height: 50px; overflow-y: scroll";
  document.body.append(probe);
  const width = probe.offsetWidth - probe.clientWidth;
  probe.remove();
  return width;
}

// The box changed width (the player resized the panel, or a scrollbar came or went): the
// picture is fitted again, and the panel's height, which follows it, reported.
let fittedTo = 0;
new ResizeObserver(() => {
  if (preview.clientWidth === fittedTo) return;
  fittedTo = preview.clientWidth;
  if (shown && !preview.hidden) {
    layOutMapPreview();
    laidOut();
  }
}).observe(preview);
