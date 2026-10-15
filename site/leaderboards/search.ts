// The Map Leaderboards page (leaderboards/index.html): find a map by its name, each with how
// many entries it has, and the most-uploaded maps. What it shows is worked out in
// leaderboards.ts.
import { byId, mapItem, names, read, said as saidWith, UNREACHABLE } from "./api.ts";
import { type EntryCounts, entryCounts, linkTo, type MapsAnswer, mostUploaded, searchMaps, searchSaid } from "./leaderboards.ts";

const query = byId<HTMLInputElement>("map-query");
const said = byId("search-said");
const found = byId<HTMLUListElement>("search-maps");
/** How many entries each map has, once GET /api/maps has answered. */
let counts: EntryCounts | null = null;

/** The maps the query finds, under the field, the one Enter opens first and marked. */
function search() {
  const result = searchMaps(names, query.value, { counts });
  said.textContent = searchSaid(query.value, result);
  found.replaceChildren(
    ...result.maps.map((map, i) =>
      mapItem({ name: map.name, street: map.street, href: linkTo({ map: map.id, band: null, name: null }), aside: map.entries }, i === 0),
    ),
  );
  found.hidden = result.maps.length === 0;
}
query.addEventListener("input", search);
// Enter opens the first map found.
query.form!.addEventListener("submit", (event) => {
  event.preventDefault();
  const first = found.querySelector<HTMLAnchorElement>("a");
  if (first) location.assign(first.href);
});
// A query the browser kept (back to the page) finds its maps again.
search();

const mostSaid = byId("most-said");
const most = byId<HTMLOListElement>("most-maps");
read<MapsAnswer>("/api/maps").then((answer) => {
  if (answer === null) {
    mostSaid.textContent = UNREACHABLE;
    return;
  }
  // The search says how many entries each map has from now on, and lists those with some first.
  counts = entryCounts(answer);
  if (query.value.trim() !== "") search();
  const shown = mostUploaded(names, answer);
  if (shown.empty !== null) saidWith(mostSaid, shown.empty);
  mostSaid.hidden = shown.empty === null;
  most.replaceChildren(...shown.maps.map((map) => mapItem({ ...map, aside: map.entries })));
  most.hidden = shown.maps.length === 0;
});
