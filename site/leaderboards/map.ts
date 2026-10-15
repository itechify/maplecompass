// A map's page (leaderboards/map/index.html): its Level Bands, one band's top 50, the
// linked Character's entry marked with the You Yellow star and its rank, and a report link on
// each other entry. Where it opens and what it shows are worked out in leaderboards.ts.
import { byId, make, names, read, said as saidWith, UNREACHABLE } from "./api.ts";
import {
  type BandAnswer,
  bandTable,
  type MapAnswer,
  type MapPlace,
  mapPage,
  placeOf,
  readLink,
  reportOutcome,
  type Row,
  type Said,
} from "./leaderboards.ts";

const link = readLink(location.search);
const said = byId("said");

/** What the page says in place of the leaderboard: why there isn't one, or what's loading. */
function say(text: Said | string) {
  saidWith(said, typeof text === "string" ? { text } : text);
  said.hidden = false;
}

async function open() {
  if (link.map === null) {
    say({ text: "This link doesn't name a map.", link: { text: "Find a map on the Map Leaderboards.", href: "/leaderboards/" } });
    return;
  }
  const map = link.map;
  const place = placeOf(names, map);
  byId("map-name").textContent = place.name;
  const street = byId("map-street");
  street.textContent = place.region ? `${place.street} · ${place.region}` : place.street;
  street.hidden = false;
  document.title = `${place.name} · Map Leaderboard · Minerva`;

  const answer = await read<MapAnswer>(`/api/maps/${map}`);
  if (answer === null) return say(UNREACHABLE);
  const page = mapPage(link, names, answer);
  drawBands(page.bands);
  if (page.open === null) return say(page.empty!);

  const query = link.name === null ? "" : `?name=${encodeURIComponent(link.name)}`;
  const band = await read<BandAnswer>(`/api/maps/${map}/bands/${page.open}${query}`);
  if (band === null) return say(UNREACHABLE);
  const table = bandTable(band, link.name, new Date());
  const choice = page.bands.find((b) => b.open)!;

  const title = byId("band-title");
  title.textContent = choice.levels;
  title.hidden = false;
  const note = byId("note");
  note.textContent = table.note ?? "";
  note.hidden = table.note === null;
  note.classList.toggle("gone", band.character === null);

  if (table.empty !== null) return say(table.empty);
  said.hidden = true;
  byId("caption").textContent = table.caption ?? "";
  const rows = byId<HTMLTableSectionElement>("rows");
  rows.replaceChildren();
  for (const row of table.rows) rows.append(...drawRow(row, place, page.open, table.atYours));
  if (table.yours !== null) {
    if (table.gap !== null) rows.append(lineRow("lb-gap", table.gap));
    rows.append(...drawRow(table.yours, place, page.open, table.atYours));
  }
  rovingReports(rows);
  byId("table").hidden = false;
  // The linked Character's entry is what the player came to see.
  rows.querySelector<HTMLElement>(".lb-you")?.scrollIntoView({ block: "center" });
}

/** The band choices, each a link to its band on the page, the open one marked. */
function drawBands(bands: ReturnType<typeof mapPage>["bands"]) {
  const nav = byId("bands");
  nav.replaceChildren(
    ...bands.map((band) => {
      const choice = make("a", "lb-band");
      choice.href = band.href;
      if (band.open) choice.setAttribute("aria-current", "page");
      choice.append(make("span", "lb-band-levels", band.levels), make("span", "lb-band-entries", band.entries));
      return choice;
    }),
  );
  nav.hidden = bands.length === 0;
}

/** A row of one cell across the table, holding a line: how many entries are between the top
 *  50 and the linked Character's, or where the linked Character stands. */
function lineRow(className: string, text: string): HTMLTableRowElement {
  const tr = make("tr", className);
  tr.setAttribute("role", "row");
  const cell = make("td", "", text);
  cell.setAttribute("role", "cell");
  cell.colSpan = 7;
  tr.append(cell);
  return tr;
}

/** The You Yellow star, the Personal Bests window's mark for a best of the player's. */
function star(): SVGSVGElement {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 16 16");
  svg.setAttribute("class", "lb-star");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
  path.setAttribute("d", "M8 1.6l1.9 4.1 4.5.5-3.3 3.1.9 4.4L8 11.5l-3.9 2.2.9-4.4-3.3-3.1 4.5-.5z");
  svg.append(path);
  return svg;
}

/** An entry's rows: its rank, name, job, level, EXP/h, date and report link; or, for the
 *  linked Character's, the You Yellow star before the name, no report link (the player
 *  takes their own entries off from Minerva), and under it where they stand. */
function drawRow(row: Row, place: MapPlace, band: number, atYours: string | null): HTMLTableRowElement[] {
  const tr = make("tr", row.you ? "lb-you" : "");
  tr.setAttribute("role", "row");
  if (row.you) tr.setAttribute("aria-current", "true");
  const name = make("th", "lb-name");
  name.scope = "row";
  name.setAttribute("role", "rowheader");
  if (row.you) name.append(star());
  name.append(row.name);
  const reportCell = make("td", "lb-report");
  if (!row.you) {
    const report = make("button", "lb-report-button", "Report");
    report.type = "button";
    report.setAttribute("aria-label", `Report ${row.name}'s entry`);
    report.setAttribute("aria-describedby", "report-keys");
    report.addEventListener("click", () => openReport(row, place, band));
    reportCell.append(report);
  }
  const cells = [make("td", "lb-rank", row.rank), make("td", "lb-job", row.job), make("td", "lb-level", row.level)];
  const figures = [make("td", "lb-exp", row.expPerHour), make("td", "lb-date", row.date), reportCell];
  for (const cell of [...cells, ...figures]) cell.setAttribute("role", "cell");
  tr.append(cells[0], name, cells[1], cells[2], ...figures);
  return row.you && atYours !== null ? [tr, lineRow("lb-you lb-you-says", atYours)] : [tr];
}

/** The table's Report buttons as one Tab stop: the up and down arrows (Home and End too)
 *  move between them, and Tab goes on past the table. */
function rovingReports(rows: HTMLElement) {
  const reports = [...rows.querySelectorAll<HTMLButtonElement>(".lb-report-button")];
  reports.forEach((report, i) => (report.tabIndex = i === 0 ? 0 : -1));
  rows.onkeydown = (event) => {
    const at = reports.indexOf(event.target as HTMLButtonElement);
    if (at === -1) return;
    const to = { ArrowDown: at + 1, ArrowUp: at - 1, Home: 0, End: reports.length - 1 }[event.key];
    if (to === undefined || to < 0 || to >= reports.length) return;
    event.preventDefault();
    reports[at].tabIndex = -1;
    reports[to].tabIndex = 0;
    reports[to].focus();
  };
}

const dialog = byId<HTMLDialogElement>("report");
const reason = byId<HTMLTextAreaElement>("report-reason");
const reportSaid = byId("report-said");
const send = byId<HTMLButtonElement>("report-send");
const cancel = byId<HTMLButtonElement>("report-cancel");
/** The entry the report window is open for. */
let reporting: { name: string; map: string; band: number } | null = null;
/** Whether a report is on its way, so a second press of Send doesn't send it twice. */
let sending = false;
/** A touch screen, where focusing the box for why would raise the keyboard over the window. */
const touch = matchMedia("(hover: none) and (pointer: coarse)");

function openReport(row: Row, place: MapPlace, band: number) {
  reporting = { name: row.name, map: place.id, band };
  byId("report-what").textContent = `${row.name}'s ${row.expPerHour} EXP/h at level ${row.level}, #${row.rank} on ${place.name}.`;
  reason.value = "";
  reason.disabled = false;
  reportSaid.hidden = true;
  send.hidden = false;
  send.removeAttribute("aria-disabled");
  cancel.textContent = "Cancel";
  // The window opens on the box for why, but on a touch screen on Cancel: the box is
  // optional, and its keyboard would cover the window.
  cancel.autofocus = touch.matches;
  dialog.showModal();
}

dialog.addEventListener("submit", async (event) => {
  if (event.submitter !== send || reporting === null) return;
  event.preventDefault();
  // Send keeps the focus while the report goes (disabling it would drop the focus out of
  // the window), and says it's busy.
  if (sending) return;
  sending = true;
  send.setAttribute("aria-disabled", "true");
  let status: number | null = null;
  try {
    const response = await fetch("/api/reports", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ ...reporting, reason: reason.value.trim() || undefined }),
    });
    status = response.status;
  } catch {
    status = null;
  }
  sending = false;
  send.removeAttribute("aria-disabled");
  const outcome = reportOutcome(status);
  reportSaid.textContent = outcome.said;
  reportSaid.hidden = false;
  // Taken, or gone already: nothing more to send, and the focus goes to Close. Otherwise
  // the player may send it again, from where they are.
  const done = outcome.taken || status === 404;
  reason.disabled = done;
  cancel.textContent = done ? "Close" : "Cancel";
  if (done) {
    send.hidden = true;
    cancel.focus();
  }
});
// A click on the veil around the window closes it, as Esc does.
dialog.addEventListener("click", (event) => {
  if (event.target === dialog) dialog.close();
});

open();
