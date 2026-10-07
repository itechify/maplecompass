/** Mirror of `compass_core::CrawlProgress`. */
export type CrawlProgress = { known: number; total: number; done: boolean };

/** Mirror of `compass_core::MobChoice`. */
export type MobChoice = { id: string; name: string; level: number };

/** Mirror of `compass_core::DensityTier`. */
export type DensityTier = "high" | "medium" | "low";

/** Mirror of `compass_core::SearchResult`. */
export type SearchResult = {
  mapId: string;
  streetName: string;
  mapName: string;
  densityHundredths: number;
  spawnSharePercent: number;
  tier: DensityTier;
};

/** Mirror of `compass_core::SearchState`. */
export type SearchState = {
  windowOpen: boolean;
  query: string;
  suggestions: MobChoice[];
  highlighted: number | null;
  picked: MobChoice | null;
  results: SearchResult[];
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

/** One result as table cells: map (name over street), Density, Spawn Share, tier. */
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
  row.append(
    where,
    cell(density(result.densityHundredths)),
    cell(`${result.spawnSharePercent}%`),
    cell(TIER_NAMES[result.tier], `tier ${result.tier}`),
  );
  return row;
}
