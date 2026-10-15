// The Map Leaderboards pages (site/leaderboards), worked out as plain data: where a link
// opens, the maps a search finds, and what a page shows from the API's answers (ADR 0015).
// The pages' scripts (search.ts, map.ts) only fetch and draw what this gives them.

import { dayText } from "../../src/dates.ts";
import { levels, rate } from "../../src/figures.ts";

/// A map's page as its address names it: `/leaderboards/map/?map=<ID>&band=<first level>&name=<name>`.
/// Any part may be missing, or dropped for not being one.
export interface Link {
  /// The map's 9-digit ID ("010001010").
  map: string | null;
  /// The Level Band, by its first level: 1 (1-9), 10 (10-19), 20 (20-29)...
  band: number | null;
  /// The Character whose entry the page shows: 1 to 16 letters and digits.
  name: string | null;
}

/// The highest level a Character can have.
const TOP_LEVEL = 200;

/// The Level Band holding `level`, by its first level.
export function bandOf(level: number): number {
  return level < 10 ? 1 : Math.floor(level / 10) * 10;
}

/// The map a link names, as its 9-digit ID: the map names fixture writes IDs as numbers,
/// without their leading zeros, so a shorter one is padded.
function mapIn(value: string | null): string | null {
  return value !== null && /^\d{1,9}$/.test(value) ? value.padStart(9, "0") : null;
}

/// The Level Band a link names, or the one holding the level it names.
function bandIn(value: string | null): number | null {
  if (value === null || !/^\d{1,3}$/.test(value)) return null;
  const level = Number(value);
  return level >= 1 && level <= TOP_LEVEL ? bandOf(level) : null;
}

function nameIn(value: string | null): string | null {
  return value !== null && /^[A-Za-z0-9]{1,16}$/.test(value) ? value : null;
}

/// Where a map page's address (its `location.search`) opens.
export function readLink(search: string): Link {
  const query = new URLSearchParams(search);
  return { map: mapIn(query.get("map")), band: bandIn(query.get("band")), name: nameIn(query.get("name")) };
}

/// A row of the map names fixture (crates/minerva-core/tests/fixtures/osms-map-names.json),
/// as OSMS writes it: its ID a number without its leading zeros, its text untrimmed.
export interface MapNameRow {
  id: number;
  street_name: string;
  name: string;
  region: string;
}

/// A map as a page names it: its Map Name under its Street Name, as the panel does.
export interface MapPlace {
  /// The 9-digit ID.
  id: string;
  name: string;
  street: string;
  region: string;
  /// Whether the map names fixture names it.
  known: boolean;
}

/// The maps the fixture names, by 9-digit ID.
export type MapNames = Map<string, MapPlace>;

/// The regions of maps the player can't normally enter, which search leaves out, as the
/// panel's does (`MapEntry::is_left_out`).
const LEFT_OUT_REGIONS = ["Event", "Dev"];

/// The maps the fixture names, trimmed as the core reads them, leaving out any with no name
/// or street.
export function mapNames(rows: MapNameRow[]): MapNames {
  const names: MapNames = new Map();
  for (const row of rows) {
    const [name, street] = [row.name.trim(), row.street_name.trim()];
    if (name === "" || street === "") continue;
    const id = String(row.id).padStart(9, "0");
    names.set(id, { id, name, street, region: row.region, known: true });
  }
  return names;
}

/// Map `id` as a page names it: a map the fixture doesn't name (one newer than it) by its
/// ID, "Not in the map list", as the Personal Bests window names it.
export function placeOf(names: MapNames, id: string): MapPlace {
  return names.get(id) ?? { id, name: `Map ${id}`, street: "Not in the map list", region: "", known: false };
}

/// How well a name holds a query: it starts with it, a word in it does, or it holds it
/// elsewhere. Lower is better.
function fit(name: string, query: string): number | null {
  const lower = name.toLowerCase();
  let best: number | null = null;
  for (let at = lower.indexOf(query); at !== -1; at = lower.indexOf(query, at + 1)) {
    const fit = at === 0 ? 0 : /[\p{L}\p{N}]/u.test(lower[at - 1]) ? 2 : 1;
    best = best === null ? fit : Math.min(best, fit);
  }
  return best;
}

/// Order as the core's strings order: by code unit, not the locale's.
function byText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/// The maps whose Map Name holds `query`, whatever the case, as the panel's search finds
/// them (`search.rs`): those that start with it, then those with a word that starts with it,
/// then the rest, each by name; Event and Dev maps aside. The first `shown`, and how many
/// more.
export function searchMaps(names: MapNames, query: string, shown = 10): { maps: MapPlace[]; more: number } {
  const lower = query.trim().toLowerCase();
  if (lower === "") return { maps: [], more: 0 };
  const found: { fit: number; map: MapPlace }[] = [];
  for (const map of names.values()) {
    if (LEFT_OUT_REGIONS.includes(map.region)) continue;
    const f = fit(map.name, lower);
    if (f !== null) found.push({ fit: f, map });
  }
  found.sort(
    (a, b) =>
      a.fit - b.fit || byText(a.map.name, b.map.name) || byText(a.map.street, b.map.street) || byText(a.map.id, b.map.id),
  );
  return { maps: found.slice(0, shown).map((f) => f.map), more: Math.max(0, found.length - shown) };
}

/// What the search says under its field, for `query` and what it found.
export function searchSaid(query: string, found: { maps: MapPlace[]; more: number }): string {
  if (query.trim() === "") return "Type part of a map's name, as the game writes it.";
  if (found.maps.length === 0) return `No map's name holds “${query.trim()}”.`;
  const shown = counted(found.maps.length, "map");
  return found.more === 0 ? shown : `${shown}, and ${found.more.toLocaleString("en-US")} more: type more of the name.`;
}

/// `count` of `thing`, plural when it isn't 1: "1 character", "12 characters".
function counted(count: number, thing: string): string {
  return `${count.toLocaleString("en-US")} ${thing}${count === 1 ? "" : "s"}`;
}

/// GET /api/maps: the most-uploaded maps, by Characters with entries there.
export interface MapsAnswer {
  maps: { map: string; characters: number; entries: number }[];
}

/// A most-uploaded map as the Map Leaderboards page lists it.
export interface UploadedMap {
  name: string;
  street: string;
  href: string;
  characters: string;
}

/// The most-uploaded maps as the Map Leaderboards page lists them, or why it lists none.
export function mostUploaded(names: MapNames, answer: MapsAnswer): { maps: UploadedMap[]; empty: string | null } {
  const maps = answer.maps.map(({ map, characters }) => {
    const place = placeOf(names, map);
    return {
      name: place.name,
      street: place.street,
      href: linkTo({ map, band: null, name: null }),
      characters: counted(characters, "character"),
    };
  });
  return { maps, empty: maps.length === 0 ? "No one has put a best on the Map Leaderboards yet." : null };
}

/// A Level Band's levels as the panel writes mob levels: "Lv 20–29".
function bandLevels(band: number): string {
  return levels({ lowest: band, highest: band === 1 ? 9 : band + 9 });
}

/// GET /api/maps/<map>: the map's Level Bands with entries, lowest first.
export interface MapAnswer {
  map: string;
  bands: { band: number; levels: [number, number]; entries: number }[];
}

/// A Level Band as a map's page offers it.
export interface BandChoice {
  band: number;
  levels: string;
  entries: string;
  href: string;
  /// Whether it's the band the page shows.
  open: boolean;
}

/// A map's page: the map, its Level Bands, and the one it opens at (`null` with none to
/// show), or why it has none.
export interface MapPage {
  place: MapPlace;
  bands: BandChoice[];
  open: number | null;
  empty: string | null;
}

/// A map's page as `link` opens it, from its bands as the API gives them. It opens at the
/// link's band, even with no entries there (Minerva links a Character's own band, and the
/// page says their entry has gone), or else at the lowest band with entries.
export function mapPage(link: Link, names: MapNames, answer: MapAnswer): MapPage {
  const open = link.band ?? answer.bands[0]?.band ?? null;
  const counts = new Map(answer.bands.map((b) => [b.band, b.entries]));
  if (open !== null && !counts.has(open)) counts.set(open, 0);
  const bands = [...counts]
    .sort(([a], [b]) => a - b)
    .map(([band, entries]) => ({
      band,
      levels: bandLevels(band),
      entries: entries === 0 ? "No entries" : `${entries.toLocaleString("en-US")} ${entries === 1 ? "entry" : "entries"}`,
      href: linkTo({ map: answer.map, band, name: link.name }),
      open: band === open,
    }));
  return {
    place: placeOf(names, answer.map),
    bands,
    open,
    empty: open === null ? "No one has a best on this Map Leaderboard yet." : null,
  };
}

/// An entry as the API ranks it: one Character's best Visit on the map in the band.
export interface Ranked {
  rank: number;
  name: string;
  job: string;
  level: number;
  expPerHour: number;
  /// When the Visit set it, in ms since the Unix epoch.
  setAt: number;
}

/// GET /api/maps/<map>/bands/<band>?name=<name>: the band's top 50, and the named
/// Character's entry with its rank, wherever it is (`null` with none there).
export interface BandAnswer {
  map: string;
  band: number;
  levels: [number, number];
  total: number;
  entries: Ranked[];
  character: Ranked | null;
}

/// An entry as a band's table writes it.
export interface Row {
  rank: string;
  name: string;
  job: string;
  level: string;
  expPerHour: string;
  date: string;
  /// Whether it's the linked Character's.
  you: boolean;
}

/// A band's table: its rows, the linked Character's under them when it's outside them, a
/// line about the linked Character, and why there are no rows when there are none.
export interface BandTable {
  rows: Row[];
  yours: Row | null;
  note: string | null;
  empty: string | null;
}

/// A band's table from the API's answer, for the Character `name` the link names (if
/// any), with dates written as of `now`.
export function bandTable(answer: BandAnswer, name: string | null, now: Date): BandTable {
  const yours = answer.character;
  const row = (entry: Ranked): Row => {
    const set = new Date(entry.setAt);
    return {
      rank: String(entry.rank),
      name: entry.name,
      job: entry.job,
      level: String(entry.level),
      expPerHour: rate(entry.expPerHour),
      date: dayText(set, set.getFullYear() !== now.getFullYear()),
      you: entry.rank === yours?.rank,
    };
  };
  const rows = answer.entries.map(row);
  const at = bandLevels(answer.band);
  let note: string | null = null;
  if (yours !== null) note = `${yours.name} is #${yours.rank} of ${answer.total.toLocaleString("en-US")} at ${at}.`;
  else if (name !== null) note = `${name} has no entry here at ${at} any more.`;
  return {
    rows,
    yours: yours !== null && !rows.some((r) => r.you) ? row(yours) : null,
    note,
    empty: rows.length === 0 ? `No one at ${at} has put a best on this map yet.` : null,
  };
}

/// What the page says once POST /api/reports answers with `status` (`null` when it
/// couldn't be reached), and whether the report was taken.
export function reportOutcome(status: number | null): { taken: boolean; said: string } {
  switch (status) {
    case 201:
      return { taken: true, said: "Reported. Thank you: Minerva's maker will look at it." };
    case 404:
      return { taken: false, said: "This entry isn't on the Map Leaderboard any more." };
    case 429:
      return { taken: false, said: "That's a lot of reports from you. Try again in a few minutes." };
    default:
      return { taken: false, said: "The report didn't go through. Try again later." };
  }
}

/// The address of a map's page, from the site's root.
export function linkTo(link: Link): string {
  const query = new URLSearchParams();
  if (link.map !== null) query.set("map", link.map);
  if (link.band !== null) query.set("band", String(link.band));
  if (link.name !== null) query.set("name", link.name);
  return `/leaderboards/map/?${query}`;
}
