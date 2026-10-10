/**
 * The Portals section: the Map Preview, OSMS's picture of the whole map with every portal
 * ringed where it is, then a row per Hidden Portal. Rings are drawn in screen pixels over
 * the fitted picture, whatever its scale: blue to another map (Doors included), green
 * within the map with a dashed arrow to where it lands, grey to a map not in the data;
 * dashed when hidden or a Quest Portal. Hovering a ring says where it leads and lights its
 * row; hovering a row lights its ring. A click does nothing.
 */

/** Mirror of `compass_core::HiddenPortal`. */
export type HiddenPortal = {
  name: string;
  toMapId: string;
  toMapName: string;
  toStreetName: string;
  toHiddenStreet: boolean;
  quest: boolean;
};

/** Mirror of `compass_core::Leads`. */
type Leads = "anotherMap" | "withinTheMap" | "mapNotInTheData";

/** Mirror of `compass_core::PortalKind`. */
type PortalKind = "visible" | "hidden" | "door" | "quest";

/** Mirror of `compass_core::PortalMark`: where it is in the picture's pixels. */
type PortalMark = {
  x: number;
  y: number;
  leads: Leads;
  kind: PortalKind;
  toMapName: string;
  toStreetName: string;
  /** A teleport within the map: where it lands, and the mark there if it has one. */
  landsAt: { x: number; y: number; mark: number | null } | null;
  /** Its row among the Hidden Portals. */
  hiddenPortal: number | null;
};

/** Mirror of `compass_core::MapPreview`. */
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
/** The space kept between the card and the ring, and the panel's edges. */
const CARD_GAP = 6;

const SVG = "http://www.w3.org/2000/svg";
const byId = (id: string) => document.getElementById(id)!;

const preview = byId("map-preview");
const picture = byId("map-picture");
const marksSvg = byId("map-marks") as unknown as SVGSVGElement;
/** The picture, under its rings; given a source once the shell has saved it. */
const image = document.createElement("img");
image.alt = "";
image.draggable = false;
image.hidden = true;
picture.prepend(image);
const loading = byId("map-loading");
const failed = byId("map-failed");
const rows = byId("portal-rows");
const card = byId("mark-card");
const panelEl = byId("panel");

/** The preview last rendered, and the shown map's name for its label. */
let shown: MapPreview | null = null;
let mapName = "";
/** The overlay couldn't load the picture at this URL. */
let brokenUrl: string | null = null;
/** What the rings were last drawn for: the marks and the picture's drawn size. */
let drawnFor = "";
/** The mark the pointer is on (or on whose row), if any. */
let lit: { mark: number; onRing: boolean } | null = null;
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
    toMapName: "",
    toStreetName: "",
    landsAt: null,
    hiddenPortal: null,
  };
  el.append(ring(leads, 6, 6));
  return el;
}

function span(className: string, text: string): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = className;
  el.textContent = text;
  return el;
}

/**
 * A row per Hidden Portal, each led by its mark's ring: "Northern Top of Construction Site
 * [Hidden Street]". Hovering one lights its mark.
 */
export function renderPortalRows(portals: HiddenPortal[], marks: PortalMark[]) {
  rows.replaceChildren(
    ...portals.map((portal, i) => {
      const at = marks.findIndex((m) => m.hiddenPortal === i);
      const row = document.createElement("li");
      if (at >= 0) row.dataset.mark = String(at);
      const to = document.createElement("div");
      to.className = "dest";
      to.append(rowRing(marks[at]), span("to", portal.toMapName));
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
    // Another map: nothing on it is lit until the pointer moves onto it.
    lit = null;
    drawnFor = "";
    image.hidden = true;
    image.src = url;
  }
  if (!url) image.removeAttribute("src");
  layOutMapPreview();
  return true;
}

/**
 * The picture fitted and centred, its rings drawn over it; or the placeholder while it
 * loads. Again whenever the section opens or the panel changes width.
 */
export function layOutMapPreview() {
  const ready = !!shown && image.hasAttribute("src") && image.complete && image.naturalWidth > 0;
  preview.classList.toggle("loading", !ready);
  loading.hidden = ready;
  picture.hidden = !ready;
  image.hidden = !ready;
  // Collapsed, the box has no width to fit the picture to.
  if (!ready || !shown || preview.clientWidth === 0) return;
  const room = preview.clientWidth - 2 * PAPER;
  const scale = Math.min(room / image.naturalWidth, MAX_PICTURE_HEIGHT / image.naturalHeight);
  const width = Math.round(image.naturalWidth * scale);
  const height = Math.round(image.naturalHeight * scale);
  picture.style.width = `${width}px`;
  picture.style.height = `${height}px`;
  const hidden = shown.marks.filter((m) => m.hiddenPortal !== null).length;
  const portals = shown.marks.length;
  picture.setAttribute(
    "aria-label",
    `Map of ${mapName}: ${portals} portal${portals === 1 ? "" : "s"} marked, ${hidden} of them hidden`,
  );
  const key = JSON.stringify([shown.marks, width, height]);
  if (key !== drawnFor) {
    drawnFor = key;
    drawMarks(shown.marks, scale, width, height);
    relight();
  }
}

/** Where a ring or a landing is drawn, in screen pixels over the picture. */
const at = (x: number, y: number, scale: number) => ({ x: x * scale, y: y * scale });

/** The arrows of the teleports within the map, under every ring, then the rings. */
function drawMarks(marks: PortalMark[], scale: number, width: number, height: number) {
  marksSvg.setAttribute("width", String(width));
  marksSvg.setAttribute("height", String(height));
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
    const back = landing.mark !== null ? marks[landing.mark]?.landsAt?.mark === i : false;
    // A pair is one line with a head at each end, drawn once.
    if (back && landing.mark !== null && landing.mark < i) return;
    const from = at(mark.x, mark.y, scale);
    const to = at(landing.x, landing.y, scale);
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
    if (back) dash.setAttribute("marker-start", "url(#landing-head)");
    g.append(svg("line", { ...line, class: "halo" }), dash);
    arrows.append(g);
  });
  const rings = svg("g", { class: "rings" });
  marks.forEach((mark, i) => {
    const { x, y } = at(mark.x, mark.y, scale);
    const g = ring(mark, x, y);
    g.dataset.mark = String(i);
    // A target larger than the ring, so a small ring is easy to rest on.
    g.append(svg("circle", { class: "hit", cx: x, cy: y, r: 8 }));
    rings.append(g);
  });
  marksSvg.replaceChildren(defs, arrows, rings);
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
  head.textContent = mark.toMapName;
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
  const frame = panelEl.getBoundingClientRect();
  const spot = target.getBoundingClientRect();
  const bar = panelEl.querySelector(".window-bar")?.getBoundingClientRect();
  const { offsetWidth: width, offsetHeight: height } = card;
  const centre = spot.left + spot.width / 2 - frame.left;
  const left = Math.max(8, Math.min(centre - width / 2, panelEl.clientWidth - 8 - width));
  const highest = (bar ? bar.bottom - frame.top : 0) + CARD_GAP;
  const above = spot.top - frame.top - CARD_GAP - height;
  const below = spot.bottom - frame.top + CARD_GAP;
  card.style.left = `${left}px`;
  card.style.top = `${above >= highest ? above : below}px`;
}

// Resting on a ring lights it and its row and shows its card; on a row, lights its ring.
marksSvg.addEventListener("mouseover", (e) => {
  const g = e.target instanceof Element ? e.target.closest<SVGGElement>(".rings > g") : null;
  hoverMark(g ? Number(g.dataset.mark) : null, true);
});
marksSvg.addEventListener("mouseleave", () => hoverMark(null));
rows.addEventListener("mouseover", (e) => {
  const row = e.target instanceof Element ? e.target.closest<HTMLElement>("li[data-mark]") : null;
  hoverMark(row ? Number(row.dataset.mark) : null);
});
rows.addEventListener("mouseleave", () => hoverMark(null));

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
// The box changed width (the player resized the panel, or a scrollbar came or went): the
// picture is fitted again.
let fittedTo = 0;
new ResizeObserver(() => {
  if (preview.clientWidth === fittedTo) return;
  fittedTo = preview.clientWidth;
  if (shown && !preview.hidden) layOutMapPreview();
}).observe(preview);
