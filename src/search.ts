import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { mobTitle, resultRow, resultsNote, type SearchState } from "./search-state";

/** What the shell sends the search window. */
type SearchView = { search: SearchState };

const byId = (id: string) => document.getElementById(id)!;
const query = byId("query") as HTMLInputElement;
const suggestions = byId("suggestions");
const noMatch = byId("no-match");
const crawl = byId("crawl");
const picked = byId("picked");
const pickedTitle = byId("picked-title");
const pickedNote = byId("picked-note");
const results = byId("results");
const resultRows = byId("result-rows");
const credit = byId("credit");

let open = false;

function update({ search }: SearchView) {
  // Opened afresh: nothing typed yet, and the typing goes to the search box.
  if (search.windowOpen && !open) {
    query.value = search.query;
    query.focus();
  }
  open = search.windowOpen;

  const { known, total, done } = search.crawl;
  crawl.textContent = total === 0 ? "Loading MeowDB's Map list…" : done ? "" : `${known} of ${total} maps known`;

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
    pickedNote.textContent = resultsNote(search);
    pickedNote.hidden = pickedNote.textContent === "";
    results.hidden = search.results.length === 0;
    resultRows.replaceChildren(...search.results.map(resultRow));
    credit.textContent = search.credit ? `Data: ${search.credit}` : "";
  }
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
// Whenever the window is in front, the typing goes to the search box.
window.addEventListener("focus", () => query.focus());

listen<SearchView>("search-state", (e) => update(e.payload));
// The shell may have sent the state before this page was listening.
invoke<SearchView | null>("search_view").then((view) => {
  if (view) update(view);
});
