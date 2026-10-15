// A map's page: its Level Bands, as GET /api/maps/<map> gives them, and which one opens.
import { describe, expect, it } from "vitest";
import { mapNames, mapPage, readLink } from "../leaderboards.ts";

const FIELD = "100000000";
const names = mapNames([{ id: 100000000, street_name: "Made-up Road", name: "Field of Tests", region: "Victoria Island" }]);

const bands = [
  { band: 10, levels: [10, 19] as [number, number], entries: 1 },
  { band: 20, levels: [20, 29] as [number, number], entries: 7 },
];

describe("a map's page", () => {
  it("names the map and lists its Level Bands, opening the lowest when the link names none", () => {
    const page = mapPage(readLink(`?map=${FIELD}`), names, { map: FIELD, bands });

    expect(page.place.name).toBe("Field of Tests");
    expect(page.open).toBe(10);
    expect(page.empty).toBe(null);
    expect(page.bands).toEqual([
      { band: 10, levels: "Lv 10–19", entries: "1 entry", href: `/leaderboards/map/?map=${FIELD}&band=10`, open: true },
      { band: 20, levels: "Lv 20–29", entries: "7 entries", href: `/leaderboards/map/?map=${FIELD}&band=20`, open: false },
    ]);
  });

  it("opens at the link's band, and keeps the Character's name in every band's link", () => {
    const page = mapPage(readLink(`?map=${FIELD}&band=20&name=Egbert`), names, { map: FIELD, bands });

    expect(page.open).toBe(20);
    expect(page.bands.map((b) => [b.href, b.open])).toEqual([
      [`/leaderboards/map/?map=${FIELD}&band=10&name=Egbert`, false],
      [`/leaderboards/map/?map=${FIELD}&band=20&name=Egbert`, true],
    ]);
  });

  it("still opens at the link's band when it has no entries, in its place among the others", () => {
    const page = mapPage(readLink(`?map=${FIELD}&band=1&name=Egbert`), names, { map: FIELD, bands });

    expect(page.open).toBe(1);
    expect(page.bands.map((b) => [b.levels, b.entries, b.open])).toEqual([
      ["Lv 1–9", "No entries", true],
      ["Lv 10–19", "1 entry", false],
      ["Lv 20–29", "7 entries", false],
    ]);
  });

  it("says so when no one has a best on the map", () => {
    const page = mapPage(readLink(`?map=${FIELD}`), names, { map: FIELD, bands: [] });

    expect(page.open).toBe(null);
    expect(page.bands).toEqual([]);
    expect(page.empty).toBe("No one has a best on this Map Leaderboard yet.");
  });
});
