// The Map Leaderboards page (leaderboards/index.html): find a map by its name, and the
// most-uploaded maps. What it shows is worked out in leaderboards.ts.
import { byId, mapItem, names, read, UNREACHABLE } from "./api.ts";
import { linkTo, type MapsAnswer, mostUploaded, searchMaps, searchSaid } from "./leaderboards.ts";

const query = byId<HTMLInputElement>("map-query");
const said = byId("search-said");
const found = byId<HTMLUListElement>("search-maps");

/** The maps the query finds, under the field. */
function search() {
  const result = searchMaps(names, query.value);
  said.textContent = searchSaid(query.value, result);
  found.replaceChildren(
    ...result.maps.map((map) => mapItem({ name: map.name, street: map.street, href: linkTo({ map: map.id, band: null, name: null }) })),
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
  const shown = mostUploaded(names, answer);
  mostSaid.textContent = shown.empty ?? "";
  mostSaid.hidden = shown.empty === null;
  most.replaceChildren(...shown.maps.map((map) => mapItem({ ...map, aside: map.characters })));
  most.hidden = shown.maps.length === 0;
});
