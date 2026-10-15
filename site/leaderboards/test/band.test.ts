// A Level Band on a map's page: its top 50, as GET /api/maps/<map>/bands/<band> ranks them,
// and the linked Character's entry with its rank, even outside them.
import { describe, expect, it } from "vitest";
import { bandTable, type BandAnswer, type Ranked } from "../leaderboards.ts";

const FIELD = "100000000";
/// Noon UTC, so the day is the same in any time zone the tests run in.
const OCT_3 = Date.UTC(2026, 9, 3, 12);
const NOW = new Date(Date.UTC(2026, 9, 14, 12));

function ranked(rank: number, name: string, expPerHour: number, setAt = OCT_3): Ranked {
  return { rank, name, job: "Magician", level: 23, expPerHour, setAt };
}

function answer(entries: Ranked[], character: Ranked | null = null, total = entries.length): BandAnswer {
  return { map: FIELD, band: 20, levels: [20, 29], total, entries, character };
}

describe("a Level Band", () => {
  it("lists its entries by rank, each EXP/h written as the Grind Tracker writes it", () => {
    const table = bandTable(
      answer([
        { rank: 1, name: "Egbert", job: "Magician", level: 23, expPerHour: 153_216, setAt: OCT_3 },
        { rank: 2, name: "Ada", job: "Bowman", level: 29, expPerHour: 66_550, setAt: Date.UTC(2025, 11, 30, 12) },
      ]),
      null,
      NOW,
    );

    expect(table.rows).toEqual([
      { rank: "1", name: "Egbert", job: "Magician", level: "23", expPerHour: "153,000", date: "3 Oct", you: false },
      { rank: "2", name: "Ada", job: "Bowman", level: "29", expPerHour: "66,600", date: "30 Dec 2025", you: false },
    ]);
    expect(table.yours).toBe(null);
    expect(table.note).toBe(null);
    expect(table.empty).toBe(null);
  });

  it("marks the linked Character's entry and says its rank", () => {
    const egbert = ranked(2, "Egbert", 50_000);
    const table = bandTable(answer([ranked(1, "Ada", 60_000), egbert, ranked(3, "Bea", 40_000)], egbert), "egbert", NOW);

    expect(table.rows.map((r) => [r.name, r.you])).toEqual([
      ["Ada", false],
      ["Egbert", true],
      ["Bea", false],
    ]);
    expect(table.yours).toBe(null);
    expect(table.note).toBe("Egbert is #2 of 3 at Lv 20–29.");
  });

  it("shows the linked Character's entry under the top 50 when it's outside them", () => {
    const top = Array.from({ length: 50 }, (_, i) => ranked(i + 1, `Player${i + 1}`, 100_000 - i * 100));
    const egbert = ranked(57, "Egbert", 1_234);
    const table = bandTable(answer(top, egbert, 63), "Egbert", NOW);

    expect(table.rows).toHaveLength(50);
    expect(table.rows.some((r) => r.you)).toBe(false);
    expect(table.yours).toEqual({
      rank: "57",
      name: "Egbert",
      job: "Magician",
      level: "23",
      expPerHour: "1,230",
      date: "3 Oct",
      you: true,
    });
    expect(table.note).toBe("Egbert is #57 of 63 at Lv 20–29.");
  });

  it("says when the linked Character's entry has gone", () => {
    const table = bandTable(answer([ranked(1, "Ada", 60_000)]), "Egbert", NOW);

    expect(table.rows.map((r) => r.you)).toEqual([false]);
    expect(table.yours).toBe(null);
    expect(table.note).toBe("Egbert has no entry here at Lv 20–29 any more.");
  });

  it("says when no one has a best in the band", () => {
    expect(bandTable(answer([]), null, NOW)).toEqual({
      rows: [],
      yours: null,
      note: null,
      empty: "No one at Lv 20–29 has put a best on this map yet.",
    });
    expect(bandTable(answer([]), "Egbert", NOW).note).toBe("Egbert has no entry here at Lv 20–29 any more.");
  });
});
