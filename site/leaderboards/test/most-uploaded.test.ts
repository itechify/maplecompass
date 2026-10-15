// The Map Leaderboards page lists the most-uploaded maps, as GET /api/maps gives them.
import { describe, expect, it } from "vitest";
import { mapNames, mostUploaded } from "../leaderboards.ts";

// Made-up maps, in the fixture's shape.
const names = mapNames([
  { id: 100000000, street_name: "Made-up Road", name: "Field of Tests", region: "Victoria Island" },
  { id: 100000001, street_name: "Made-up Road", name: "Rich Field", region: "Victoria Island" },
]);

describe("the most-uploaded maps", () => {
  it("are named and linked, with how many Characters have entries there", () => {
    const shown = mostUploaded(names, {
      maps: [
        { map: "100000001", characters: 12, entries: 15 },
        { map: "100000000", characters: 1, entries: 1 },
        { map: "100000002", characters: 1, entries: 2 },
      ],
    });

    expect(shown).toEqual({
      empty: null,
      maps: [
        { name: "Rich Field", street: "Made-up Road", href: "/leaderboards/map/?map=100000001", characters: "12 characters" },
        { name: "Field of Tests", street: "Made-up Road", href: "/leaderboards/map/?map=100000000", characters: "1 character" },
        { name: "Map 100000002", street: "Not in the map list", href: "/leaderboards/map/?map=100000002", characters: "1 character" },
      ],
    });
  });

  it("say so when no one has uploaded a best yet", () => {
    expect(mostUploaded(names, { maps: [] })).toEqual({
      empty: "No one has put a best on the Map Leaderboards yet.",
      maps: [],
    });
  });
});
