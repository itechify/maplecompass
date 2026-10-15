// The Map Leaderboards page lists the most-uploaded maps, as GET /api/maps gives them.
import { describe, expect, it } from "vitest";
import { mapNames, mostUploaded } from "../leaderboards.ts";

// Made-up maps, in the fixture's shape.
const names = mapNames([
  { id: 100000000, street_name: "Made-up Road", name: "Field of Tests", region: "Victoria Island" },
  { id: 100000001, street_name: "Made-up Road", name: "Rich Field", region: "Victoria Island" },
]);

describe("the most-uploaded maps", () => {
  it("are named and linked, with how many entries each has", () => {
    const shown = mostUploaded(names, {
      maps: [
        { map: "100000001", characters: 12, entries: 15 },
        { map: "100000000", characters: 1, entries: 1 },
        { map: "100000002", characters: 1, entries: 1 },
      ],
    });

    expect(shown).toEqual({
      empty: null,
      maps: [
        { name: "Rich Field", street: "Made-up Road", href: "/leaderboards/map/?map=100000001", entries: "15 entries" },
        { name: "Field of Tests", street: "Made-up Road", href: "/leaderboards/map/?map=100000000", entries: "1 entry" },
        { name: "Map 100000002", street: "Not in the map list", href: "/leaderboards/map/?map=100000002", entries: "1 entry" },
      ],
    });
  });

  it("are the first 50 of every map with entries", () => {
    const maps = Array.from({ length: 60 }, (_, i) => ({ map: String(100_000_100 + i), characters: 1, entries: 60 - i }));

    const shown = mostUploaded(names, { maps });

    expect(shown.maps).toHaveLength(50);
    expect(shown.maps[49].entries).toBe("11 entries");
  });

  it("say so when no one has uploaded a best yet, and how to put yours there from Minerva", () => {
    expect(mostUploaded(names, { maps: [] })).toEqual({
      empty: {
        text: "No one has put a best on the Map Leaderboards yet.",
        link: { text: "How to put yours here from Minerva", href: "/faq/#leaderboard-appear" },
      },
      maps: [],
    });
  });
});
