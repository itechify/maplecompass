/** Mirror of `compass_core::CrawlProgress`. */
export type CrawlProgress = { known: number; total: number; done: boolean };

/** Mirror of `compass_core::MobChoice`. */
export type MobChoice = { id: string; name: string; level: number };

/** Mirror of `compass_core::DensityTier`. */
export type DensityTier = "high" | "medium" | "low";

/** Mirror of `compass_core::Distance`: map changes from the map shown, or why there's no number. */
export type Distance = { mapChanges: number } | "noRoute" | "questOnly" | "unknown";

/** Mirror of `compass_core::Step`: how a route goes from one map to the next. */
export type Step = "portal" | "ship" | "taxi" | "teleportScroll";

/** Mirror of `compass_core::RouteStop`: `by` is null for the first stop, the map you're on. */
export type RouteStop = { mapId: string; streetName: string; mapName: string; by: Step | null };

/** Mirror of `compass_core::SearchResult`. */
export type SearchResult = {
  mapId: string;
  streetName: string;
  mapName: string;
  densityHundredths: number;
  spawnSharePercent: number;
  tier: DensityTier;
  distance: Distance;
  route: RouteStop[];
  /** The kinds of travel besides portals the route takes. */
  travel: Step[];
};

/** Mirror of `compass_core::SearchState`. */
export type SearchState = {
  windowOpen: boolean;
  query: string;
  suggestions: MobChoice[];
  highlighted: number | null;
  picked: MobChoice | null;
  results: SearchResult[];
  highlightedResult: number | null;
  nearest: SearchResult | null;
  credit: string | null;
  crawl: CrawlProgress;
};

/** Mirror of `compass_core::PANEL_RESULTS`: the panel shows this many results. */
export const PANEL_RESULTS = 5;

export const TIER_NAMES: Record<DensityTier, string> = { high: "High", medium: "Medium", low: "Low" };

/** A Density in hundredths as the player reads it: 1500 is "15", 1250 is "12.5". */
export const density = (hundredths: number) =>
  (hundredths / 100).toLocaleString("en-US", { maximumFractionDigits: 2 });

/** "Orange Mushroom · Lv 8" */
export const mobTitle = (mob: MobChoice) => `${mob.name} · Lv ${mob.level}`;

/** Why a picked mob has no results, or that the list may be incomplete mid-crawl. */
export function resultsNote(search: SearchState): string {
  if (!search.picked) return "";
  const { crawl } = search;
  if (search.results.length === 0) {
    return crawl.done ? "It only spawns on Event maps." : "None of the maps known so far.";
  }
  return crawl.done ? "" : `From the ${crawl.known} of ${crawl.total} maps known so far.`;
}

const DISTANCE_TEXT = {
  noRoute: { text: "no route", title: "No known route leads there" },
  questOnly: { text: "quest only", title: "Only a Quest Portal leads there" },
  unknown: { text: "?", title: "Not known until your map is recognised" },
} as const;

/** A Distance as its table cell reads ("3", "no route", "quest only", "?"), with why. */
export function distanceText(distance: Distance): { text: string; title: string } {
  if (typeof distance === "string") return DISTANCE_TEXT[distance];
  const n = distance.mapChanges;
  return { text: String(n), title: n === 0 ? "You're on this map" : `${n} map change${n === 1 ? "" : "s"} away` };
}

/** Each Step as a route and a travel marker name it. */
export const STEP_NAMES: Record<Step, string> = {
  portal: "portal",
  ship: "ship",
  taxi: "taxi",
  teleportScroll: "Teleport Scroll",
};

/** A result's travel marker: "ship", "taxi, ship", or "" for portals alone. */
export const travelMarker = (result: SearchResult) => result.travel.map((step) => STEP_NAMES[step]).join(", ");

/** The headline: "Nearest: Mushroom Field, 2 map changes away", or "" with none. */
export function nearestLine(search: SearchState): string {
  const { nearest } = search;
  if (!nearest || typeof nearest.distance === "string") return "";
  const n = nearest.distance.mapChanges;
  const away = n === 0 ? "the map you're on" : `${n} map change${n === 1 ? "" : "s"} away`;
  const travel = travelMarker(nearest);
  return `Nearest: ${nearest.mapName}, ${away}${travel ? ` · ${travel}` : ""}`;
}

/** One result as table cells: map (name over street), Density, Spawn Share, tier, Distance. */
export function resultRow(result: SearchResult): HTMLTableRowElement {
  const row = document.createElement("tr");
  const where = document.createElement("td");
  const name = document.createElement("span");
  name.className = "result-map";
  name.textContent = result.mapName;
  const street = document.createElement("span");
  street.className = "result-street";
  street.textContent = result.streetName;
  where.append(name, street);
  const cell = (text: string, className = "") => {
    const td = document.createElement("td");
    td.textContent = text;
    td.className = className;
    return td;
  };
  // "5 · ship": a route that isn't just portals says what else it takes.
  const distance = distanceText(result.distance);
  const travel = travelMarker(result);
  const distanceCell = cell(
    travel ? `${distance.text} · ${travel}` : distance.text,
    typeof result.distance === "string" ? "distance none" : "distance",
  );
  distanceCell.title = travel ? `${distance.title}, by ${travel}` : distance.title;
  row.append(
    where,
    cell(density(result.densityHundredths)),
    cell(`${result.spawnSharePercent}%`),
    cell(TIER_NAMES[result.tier], `tier ${result.tier}`),
    distanceCell,
  );
  return row;
}

/** A route's maps in order, each with how it's reached: "You're here", "Portal", "Ship"… */
export function routeStops(result: SearchResult): HTMLLIElement[] {
  return result.route.map((stop) => {
    const item = document.createElement("li");
    const step = document.createElement("span");
    step.className = `step ${stop.by ?? "start"}`;
    step.textContent = stop.by ? STEP_NAMES[stop.by] : "you're here";
    const map = document.createElement("span");
    map.className = "route-map";
    map.textContent = stop.mapName;
    item.append(step, map);
    if (stop.streetName) {
      const street = document.createElement("span");
      street.className = "route-street";
      street.textContent = stop.streetName;
      item.append(street);
    }
    return item;
  });
}
