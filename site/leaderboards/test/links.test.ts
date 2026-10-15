// A map's page opens where its link says: the map, the Level Band and the Character Minerva
// sends (ADR 0015, "Deep links").
import { describe, expect, it } from "vitest";
import { linkTo, readLink } from "../leaderboards.ts";

describe("deep links", () => {
  it("read the map, the Level Band and the Character's name", () => {
    expect(readLink("?map=010001010&band=20&name=Egbert")).toEqual({ map: "010001010", band: 20, name: "Egbert" });
  });

  it("pad a map's ID to its 9 digits, as the map names fixture writes it unpadded", () => {
    expect(readLink("?map=10001010").map).toBe("010001010");
  });

  it("drop what they can't hold, keeping the rest", () => {
    expect(readLink("?map=Henesys&band=20&name=Egbert")).toEqual({ map: null, band: 20, name: "Egbert" });
    expect(readLink("?map=010001010&band=twenty&name=Egbert")).toEqual({ map: "010001010", band: null, name: "Egbert" });
    expect(readLink("?map=010001010&band=20&name=Eg bert!")).toEqual({ map: "010001010", band: 20, name: null });
    expect(readLink("?map=0100010100&band=0&name=ThisNameIsTooLong")).toEqual({ map: null, band: null, name: null });
    expect(readLink("")).toEqual({ map: null, band: null, name: null });
  });

  it("take a level for the Level Band that holds it", () => {
    expect(readLink("?map=010001010&band=23").band).toBe(20);
    expect(readLink("?map=010001010&band=1").band).toBe(1);
    expect(readLink("?map=010001010&band=7").band).toBe(1);
    expect(readLink("?map=010001010&band=201").band).toBe(null);
  });

  it("are written as the page reads them, with the name encoded", () => {
    expect(linkTo({ map: "010001010", band: 20, name: "Egbert" })).toBe("/leaderboards/map/?map=010001010&band=20&name=Egbert");
    expect(linkTo({ map: "010001010", band: null, name: null })).toBe("/leaderboards/map/?map=010001010");
    expect(readLink(new URL(linkTo({ map: "010001010", band: 30, name: "Ada" }), "https://x").search)).toEqual({
      map: "010001010",
      band: 30,
      name: "Ada",
    });
  });
});
