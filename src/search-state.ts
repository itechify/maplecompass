import { mapMark, type MapMarks } from "./map-marks";

/** Mirror of `compass_core::MobChoice`. */
export type MobChoice = { id: string; name: string; level: number };

/** Mirror of `compass_core::DensityTier`. */
export type DensityTier = "high" | "medium" | "low";

/** Mirror of `compass_core::Distance`: map changes from the map shown, or why there's no number. */
export type Distance = { mapChanges: number } | "noRoute" | "questOnly" | "unknown";

/** Mirror of `compass_core::Step`: how a route goes from one map to the next. */
export type Step = "portal" | "ship" | "taxi" | "teleportScroll";

/** Mirror of `compass_core::RouteStop`: `by` is null for the first stop, the map you're on. */
export type RouteStop = { mapId: string; streetName: string; mapName: string; mark: string | null; by: Step | null };

/** Mirror of `compass_core::SearchResult`. */
export type SearchResult = {
  mapId: string;
  streetName: string;
  mapName: string;
  /** Its Map Mark, by OSMS's name for it. */
  mark: string | null;
  densityHundredths: number;
  spawnSharePercent: number;
  tier: DensityTier;
  distance: Distance;
  route: RouteStop[];
  /** The kinds of travel besides portals the route takes. */
  travel: Step[];
};

/** Mirror of `compass_core::Travel`: taxi rides between towns, and Teleport Scrolls to towns. */
export type Travel = { taxis: boolean; teleportScrolls: boolean };

/** Mirror of `compass_core::SearchView`: the search view's open step. */
export type SearchView = "mobs" | "maps";

/** Mirror of `compass_core::Trip`: the way to a Hunting Target as it stood when picked. */
export type Trip = { from: string; mapChanges: number; travel: Step[] };

/**
 * Mirror of `compass_core::HuntingTarget`: the mob and map picked, with the map's figures
 * and route, and once the player is there, that they've arrived. `image` is where the
 * mob's OSMS animation loads from, once it's in.
 */
export type HuntingTarget = {
  mob: MobChoice;
  map: SearchResult;
  arrived: boolean;
  trip: Trip | null;
  image: string | null;
};

/** An arrival's trip: "2 map changes from Sunset Sky, by teleport scroll". */
export function tripLine({ from, mapChanges, travel }: Trip): string {
  const by = travel.map((step) => STEP_TITLES[step].toLowerCase());
  const ways = by.length > 1 ? `${by.slice(0, -1).join(", ")} and ${by[by.length - 1]}` : by[0];
  return `${mapChanges} map change${mapChanges === 1 ? "" : "s"} from ${from}${ways ? `, by ${ways}` : ""}`;
}

/** Mirror of `compass_core::SearchState`. */
export type SearchState = {
  /** The search view's open step, which takes over the panel; null for the normal view. */
  view: SearchView | null;
  query: string;
  suggestions: MobChoice[];
  highlighted: number | null;
  /** The mob whose map list the search view shows. */
  picked: MobChoice | null;
  /** The picked mob's ranked map list. */
  results: SearchResult[];
  /** The picked mob spawns only on Event maps, which search leaves out. */
  onlyOnEventMaps: boolean;
  /** The map Enter would make the Hunting Target. */
  highlightedResult: number | null;
  /** Which result is the nearest with a route. */
  nearest: number | null;
  credit: string | null;
  /** What Distances count besides portals and ships (the checkboxes). */
  travel: Travel;
  /** The panel's Navigation section, until the player leaves its map having arrived, or clears it. */
  hunting: HuntingTarget | null;
};

export const TIER_NAMES: Record<DensityTier, string> = { high: "High", medium: "Medium", low: "Low" };

/** A Density in hundredths as the player reads it: 1500 is "15", 1250 is "12.5". */
export const density = (hundredths: number) =>
  (hundredths / 100).toLocaleString("en-US", { maximumFractionDigits: 2 });

/** Why a picked mob has no results, as the core says. */
export const resultsNote = (search: SearchState) =>
  search.onlyOnEventMaps ? "Found only on Event maps, which search leaves out." : "";

const DISTANCE_TEXT = {
  noRoute: { text: "no route", title: "No known route leads there" },
  questOnly: { text: "quest only", title: "Only a Quest Portal leads there" },
  unknown: { text: "?", title: "Not known until your map is recognised" },
} as const;

/** A Distance as its table cell reads ("3", "here", "no route", "quest only", "?"), with why. */
export function distanceText(distance: Distance): { text: string; title: string } {
  if (typeof distance === "string") return DISTANCE_TEXT[distance];
  const n = distance.mapChanges;
  if (n === 0) return { text: "here", title: "You're on this map" };
  return { text: String(n), title: `${n} map change${n === 1 ? "" : "s"} away` };
}

/** Each Step as a route and a travel marker name it ("5 · taxi, scroll"). */
export const STEP_NAMES: Record<Step, string> = {
  portal: "portal",
  ship: "ship",
  taxi: "taxi",
  teleportScroll: "scroll",
};

/** Each Step in full, for tooltips. */
export const STEP_TITLES: Record<Step, string> = {
  portal: "Portal",
  ship: "Ship",
  taxi: "Taxi",
  teleportScroll: "Teleport Scroll",
};

/** A result's travel marker: "ship", "taxi, ship", or "" for portals alone. */
export const travelMarker = (result: SearchResult) => result.travel.map((step) => STEP_NAMES[step]).join(", ");

/** The headline: "Nearest: Mushroom Field, 2 map changes away", or "" with none. */
export function nearestLine(search: SearchState): string {
  const nearest = search.nearest === null ? null : search.results[search.nearest];
  if (!nearest || typeof nearest.distance === "string") return "";
  const n = nearest.distance.mapChanges;
  const away = n === 0 ? "the map you're on" : `${n} map change${n === 1 ? "" : "s"} away`;
  const travel = travelMarker(nearest);
  return `Nearest: ${nearest.mapName}, ${away}${travel ? ` · ${travel}` : ""}`;
}

/** A results table's header row: Map, Density, Share, Tier, Distance. */
export function resultsHead(): HTMLTableSectionElement {
  const head = document.createElement("thead");
  const row = head.insertRow();
  const columns: [string, string?, string?][] = [
    ["Map"],
    ["Density", "Spawn Count × Mob Rate"],
    ["Share", "Share of all mob spawns on the Map", "Spawn Share"],
    ["Tier"],
    ["Distance", "Map changes from the map you're on"],
  ];
  for (const [text, title, label] of columns) {
    const th = document.createElement("th");
    th.textContent = text;
    if (title) th.title = title;
    if (label) th.setAttribute("aria-label", label);
    row.append(th);
  }
  return head;
}

/** A result's Distance as the map list and the Navigation section read it ("5 · ship"), with why. */
export function distanceWithTravel(result: SearchResult): { text: string; title: string } {
  const distance = distanceText(result.distance);
  const travel = travelMarker(result);
  const travelTitle = result.travel.map((step) => STEP_TITLES[step]).join(", ");
  return travel
    ? { text: `${distance.text} · ${travel}`, title: `${distance.title}, by ${travelTitle}` }
    : distance;
}

/**
 * One result as its own row group, so the map reads as one unit: the map's name across
 * the row with its Distance at the right, then its street under Map, with Density, Spawn
 * Share and tier under theirs.
 */
export function resultGroup(result: SearchResult, marks: MapMarks): HTMLTableSectionElement {
  const group = document.createElement("tbody");
  group.id = `map-${result.mapId}`;
  group.dataset.id = result.mapId;
  const cell = (text: string, className = "") => {
    const td = document.createElement("td");
    td.textContent = text;
    td.className = className;
    return td;
  };
  const name = cell(result.mapName, "result-map");
  name.prepend(mapMark(marks, result.mark));
  name.colSpan = 4;
  name.title = result.mapName;
  // "5 · ship": a route that isn't just portals says what else it takes.
  const distance = distanceWithTravel(result);
  const distanceCell = cell(distance.text, typeof result.distance === "string" ? "distance none" : "distance");
  distanceCell.title = distance.title;
  distanceCell.rowSpan = 2;
  group.insertRow().append(name, distanceCell);
  group.insertRow().append(
    cell(result.streetName, "result-street"),
    cell(density(result.densityHundredths)),
    cell(`${result.spawnSharePercent}%`),
    cell(TIER_NAMES[result.tier], `tier ${result.tier}`),
  );
  return group;
}

/** A route's maps in order, each with how it's reached: "you're here", "portal", "taxi", "scroll"… */
export function routeStops(result: SearchResult, marks: MapMarks): HTMLLIElement[] {
  const span = (className: string, text: string) => {
    const el = document.createElement("span");
    el.className = className;
    el.textContent = text;
    return el;
  };
  const last = result.route.length - 1;
  return result.route.map((stop, i) => {
    // Drawn as the minimap draws it: your yellow dot where you start, a portal ring for
    // each portal, a plain ring for a ride, the red mob dot where the mob is.
    const kind = i === 0 ? "start" : i === last ? "end" : (stop.by ?? "portal");
    const item = document.createElement("li");
    item.className = kind;
    const glyph = span("glyph", "");
    glyph.setAttribute("aria-hidden", "true");
    // Screen readers hear what the glyph shows.
    const how = span("sr-only", stop.by ? `then by ${STEP_TITLES[stop.by].toLowerCase()}: ` : "you're here: ");
    const map = span("route-map", stop.mapName);
    // Every stop's mark, so the names line up and the route shows where it crosses areas.
    map.prepend(mapMark(marks, stop.mark));
    item.append(glyph, how, map);
    if (stop.by) item.title = `By ${STEP_TITLES[stop.by].toLowerCase()}`;
    // Anything but a portal is worth noticing on the way.
    if (stop.by && stop.by !== "portal") map.append(span("tag", STEP_TITLES[stop.by]));
    // The street only where it changes: most routes stay on one.
    if (stop.streetName && stop.streetName !== result.route[i - 1]?.streetName) {
      item.append(span("route-street", stop.streetName));
    }
    return item;
  });
}
