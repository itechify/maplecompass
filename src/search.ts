import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  crawlLine,
  distanceText,
  mobTitle,
  nearestLine,
  resultRow,
  resultsNote,
  routeStops,
  type SearchResult,
  type SearchState,
} from "./search-state";

/** What the shell sends the search window. */
type SearchView = { search: SearchState };

const byId = (id: string) => document.getElementById(id)!;
const query = byId("query") as HTMLInputElement;
const suggestions = byId("suggestions");
const noMatch = byId("no-match");
const crawl = byId("crawl");
const picked = byId("picked");
const pickedTitle = byId("picked-title");
const pickedNearest = byId("picked-nearest");
const pickedNote = byId("picked-note");
const results = byId("results");
const resultRows = byId("result-rows");
const route = byId("route");
const routeTitle = byId("route-title");
const routeList = byId("route-stops");
const routeNone = byId("route-none");
const credit = byId("credit");
const taxis = byId("taxis") as HTMLInputElement;
const teleportScrolls = byId("teleport-scrolls") as HTMLInputElement;

let open = false;

function update({ search }: SearchView) {
  // Opened afresh: nothing typed yet, and the typing goes to the search box.
  if (search.windowOpen && !open) {
    query.value = search.query;
    query.focus();
  }
  open = search.windowOpen;

  crawl.textContent = search.crawl.total === 0 ? "Loading MeowDB's Map list…" : crawlLine(search);
  // The core keeps the toggles (and saves them); the checkboxes follow it.
  taxis.checked = search.taxis;
  teleportScrolls.checked = search.teleportScrolls;

  suggestions.replaceChildren(
    ...search.suggestions.map((mob, i) => {
      const row = document.createElement("li");
      row.id = `mob-${mob.id}`;
      row.setAttribute("role", "option");
      const name = document.createElement("span");
      name.className = "name";
      name.textContent = mob.name;
      const level = document.createElement("span");
      level.className = "level";
      level.textContent = `Lv ${mob.level}`;
      row.append(name, level);
      const current = i === search.highlighted;
      row.classList.toggle("current", current);
      row.setAttribute("aria-selected", String(current));
      return row;
    }),
  );
  const listed = search.suggestions.length > 0;
  suggestions.hidden = !listed;
  query.setAttribute("aria-expanded", String(listed));
  const highlighted = search.highlighted === null ? null : search.suggestions[search.highlighted];
  if (highlighted) {
    query.setAttribute("aria-activedescendant", `mob-${highlighted.id}`);
    byId(`mob-${highlighted.id}`).scrollIntoView({ block: "nearest" });
  } else {
    query.removeAttribute("aria-activedescendant");
  }
  noMatch.hidden = listed || search.query.trim() === "";

  picked.hidden = !search.picked;
  if (search.picked) {
    pickedTitle.textContent = mobTitle(search.picked);
    pickedNearest.textContent = nearestLine(search);
    pickedNearest.hidden = pickedNearest.textContent === "";
    pickedNote.textContent = resultsNote(search);
    pickedNote.hidden = pickedNote.textContent === "";
    results.hidden = search.results.length === 0;
    resultRows.replaceChildren(
      ...search.results.map((result, i) => {
        const row = resultRow(result);
        const current = i === search.highlightedResult;
        row.classList.toggle("current", current);
        row.setAttribute("aria-selected", String(current));
        return row;
      }),
    );
    showRoute(search.highlightedResult === null ? null : search.results[search.highlightedResult]);
    credit.textContent = search.credit ? `Data: ${search.credit}` : "";
  }
}

/** The highlighted result's full route, under the table. */
function showRoute(result: SearchResult | null) {
  route.hidden = !result;
  if (!result) return;
  routeTitle.textContent = `Route to ${result.mapName}`;
  routeList.replaceChildren(...routeStops(result));
  routeList.hidden = result.route.length === 0;
  // No route to show: say why ("No known route leads there"…).
  routeNone.textContent = result.route.length === 0 ? distanceText(result.distance).title : "";
  routeNone.hidden = result.route.length > 0;
  resultRows.querySelector("tr.current")?.scrollIntoView({ block: "nearest" });
}

query.addEventListener("input", () => invoke("search_query", { query: query.value }));
query.addEventListener("keydown", (e) => {
  if (e.isComposing) return;
  switch (e.key) {
    case "ArrowDown":
    case "ArrowUp":
      e.preventDefault();
      invoke("search_move", { down: e.key === "ArrowDown" });
      break;
    case "Enter":
      e.preventDefault();
      invoke("search_enter");
      break;
    case "Escape":
      e.preventDefault();
      invoke("search_escape");
      break;
  }
});
// After a click on a checkbox, the typing goes back to the search box.
taxis.addEventListener("change", () => {
  invoke("search_toggle_taxis");
  query.focus();
});
teleportScrolls.addEventListener("change", () => {
  invoke("search_toggle_teleport_scrolls");
  query.focus();
});
// Esc closes the window from the checkboxes too.
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && e.target !== query) {
    e.preventDefault();
    invoke("search_escape");
  }
});
// Whenever the window is in front, the typing goes to the search box.
window.addEventListener("focus", () => query.focus());

listen<SearchView>("search-state", (e) => update(e.payload));
// The shell may have sent the state before this page was listening.
invoke<SearchView | null>("search_view").then((view) => {
  if (view) update(view);
});
