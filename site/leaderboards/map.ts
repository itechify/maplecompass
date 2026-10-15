// A map's page (leaderboards/map/index.html): its Level Bands, one band's top 50, the
// linked Character's entry marked with its rank, and a report link on each entry. Where it
// opens and what it shows are worked out in leaderboards.ts.
import { byId, make, names, read, UNREACHABLE } from "./api.ts";
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
} from "./leaderboards.ts";

const link = readLink(location.search);
const said = byId("said");

/// What the page says in place of the leaderboard: why there isn't one, or what's loading.
function say(text: string, more?: HTMLElement) {
  said.replaceChildren(text, ...(more ? [" ", more] : []));
  said.hidden = false;
}

async function open() {
  if (link.map === null) {
    const all = make("a", "", "Find a map on the Map Leaderboards.");
    all.href = "/leaderboards/";
    say("This link doesn't name a map.", all);
    return;
  }
  const map = link.map;
  const place = placeOf(names, map);
  byId("map-name").textContent = place.name;
  byId("map-street").textContent = place.region ? `${place.street} · ${place.region}` : place.street;
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
  const rows = byId<HTMLTableSectionElement>("rows");
  rows.replaceChildren(...table.rows.map((row) => drawRow(row, place, page.open!)));
  if (table.yours !== null) {
    const gap = make("tr", "lb-gap");
    const cell = make("td", "", "⋯");
    cell.colSpan = 7;
    gap.append(cell);
    rows.append(gap, drawRow(table.yours, place, page.open));
  }
  byId("table").hidden = false;
  // The linked Character's entry is what the player came to see.
  rows.querySelector<HTMLElement>(".lb-you")?.scrollIntoView({ block: "center" });
}

/// The band choices, each a link to its band on the page, the open one marked.
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

/// An entry's row: its rank, name, job, level, EXP/h, date, and its report link.
function drawRow(row: Row, place: MapPlace, band: number): HTMLTableRowElement {
  const tr = make("tr", row.you ? "lb-you" : "");
  if (row.you) tr.setAttribute("aria-current", "true");
  const name = make("th", "lb-name", row.name);
  name.scope = "row";
  const report = make("button", "lb-report-button", "Report");
  report.type = "button";
  report.setAttribute("aria-label", `Report ${row.name}'s entry`);
  report.addEventListener("click", () => openReport(row, place, band));
  const reportCell = make("td", "lb-report");
  reportCell.append(report);
  tr.append(
    make("td", "lb-rank", row.rank),
    name,
    make("td", "lb-job", row.job),
    make("td", "lb-level", row.level),
    make("td", "lb-exp", row.expPerHour),
    make("td", "lb-date", row.date),
    reportCell,
  );
  return tr;
}

const dialog = byId<HTMLDialogElement>("report");
const reason = byId<HTMLTextAreaElement>("report-reason");
const reportSaid = byId("report-said");
const send = byId<HTMLButtonElement>("report-send");
const cancel = byId<HTMLButtonElement>("report-cancel");
/// The entry the report window is open for.
let reporting: { name: string; map: string; band: number } | null = null;

function openReport(row: Row, place: MapPlace, band: number) {
  reporting = { name: row.name, map: place.id, band };
  byId("report-what").textContent = `${row.name}'s ${row.expPerHour} EXP/h at level ${row.level}, #${row.rank} on ${place.name}.`;
  reason.value = "";
  reason.disabled = false;
  reportSaid.hidden = true;
  send.hidden = false;
  send.disabled = false;
  cancel.textContent = "Cancel";
  dialog.showModal();
}

dialog.addEventListener("submit", async (event) => {
  if (event.submitter !== send || reporting === null) return;
  event.preventDefault();
  send.disabled = true;
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
  const outcome = reportOutcome(status);
  reportSaid.textContent = outcome.said;
  reportSaid.hidden = false;
  // Taken, or gone already: nothing more to send. Otherwise the player may send it again.
  const done = outcome.taken || status === 404;
  send.hidden = done;
  send.disabled = false;
  reason.disabled = done;
  cancel.textContent = done ? "Close" : "Cancel";
});
// A click on the veil around the window closes it, as Esc does.
dialog.addEventListener("click", (event) => {
  if (event.target === dialog) dialog.close();
});

open();
