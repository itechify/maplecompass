// Each map's Level Bands are ranked by EXP/h, ties going to whoever set it first. A band's
// read lists its top 50, and a given Character's entry with its rank, even outside them.
import { describe, expect, it } from "vitest";
import { band, call, entry, key, names, upload } from "./api";

// Maps of their own, so no other test's entries are in their bands.
const RANKED = "100000010";
const TIED = "100000011";
const CROWDED = "100000012";

describe("ranking", () => {
  it("ranks a band by EXP/h, best first", async () => {
    await upload("Anton", key("anton"), [entry(RANKED, 25, 40_000)]);
    await upload("Berta", key("berta"), [entry(RANKED, 22, 60_000)]);
    await upload("Carla", key("carla"), [entry(RANKED, 29, 50_000)]);

    const read = await band(RANKED, 20);

    expect(read.entries.map((e) => [e.rank, e.name])).toEqual([
      [1, "Berta"],
      [2, "Carla"],
      [3, "Anton"],
    ]);
    expect(read.total).toBe(3);
    expect(read.levels).toEqual([20, 29]);
  });

  it("ranks each Level Band on its own", async () => {
    await upload("Dora", key("dora"), [entry(RANKED, 9, 90_000)]);
    await upload("Emil", key("emil"), [entry(RANKED, 10, 80_000)]);

    expect(await names(RANKED, 1)).toEqual(["Dora"]);
    expect((await band(RANKED, 1)).levels).toEqual([1, 9]);
    expect(await names(RANKED, 10)).toEqual(["Emil"]);
  });

  it("gives a tie to whoever set it first", async () => {
    await upload("Late", key("late"), [entry(TIED, 33, 70_000, 1_760_000_500_000)]);
    await upload("Early", key("early"), [entry(TIED, 35, 70_000, 1_760_000_100_000)]);

    expect(await names(TIED, 30)).toEqual(["Early", "Late"]);
  });

  it("lists the top 50, and a Character's entry and rank even outside them", async () => {
    for (let i = 1; i <= 52; i++) {
      await upload(`Crowd${i}`, key(`crowd${i}`), [entry(CROWDED, 44, 100_000 - i)]);
    }

    const read = await band(CROWDED, 40, "crowd52");

    expect(read.total).toBe(52);
    expect(read.entries).toHaveLength(50);
    expect(read.entries.at(-1)).toMatchObject({ rank: 50, name: "Crowd50" });
    expect(read.character).toMatchObject({ rank: 52, name: "Crowd52", level: 44, expPerHour: 99_948 });
  });

  it("gives no Character's entry for a name with none in the band", async () => {
    await upload("Fritz", key("fritz"), [entry(RANKED, 55, 10_000)]);

    expect((await band(RANKED, 20, "Fritz")).character).toBeNull();
    expect((await band(RANKED, 20, "Nobody")).character).toBeNull();
  });

  it("reads an empty band as no entries", async () => {
    expect(await band("100000013", 100)).toMatchObject({ total: 0, entries: [], character: null });
  });

  it("refuses a band that isn't a Level Band's first level", async () => {
    expect((await call(`/api/maps/${RANKED}/bands/25`)).status).toBe(400);
    expect((await call(`/api/maps/not-a-map/bands/20`)).status).toBe(400);
  });
});
