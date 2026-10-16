import { describe, expect, it } from "vitest";
import { mapPicture } from "../leaderboards.ts";

describe("a map's live picture", () => {
  const map = "100000000";
  const hash = "0123456789abcdef0123456789abcdef";

  it("uses the selected map's content hash from OSMS's manifest", () => {
    expect(mapPicture({ [map]: hash, "100000001": "abcdef0123456789abcdef0123456789" }, map)).toBe(
      `https://osmsdataexplorer.com/data/maps/${hash}.webp`,
    );
  });

  it("has no picture when the map is absent or the manifest didn't read", () => {
    for (const manifest of [null, [], "unreadable", {}, { "100000001": hash }]) {
      expect(mapPicture(manifest, map)).toBeNull();
    }
  });

  it("rejects paths, URLs and malformed hashes", () => {
    for (const value of [null, 7, {}, "", "../another-map", "https://example.com/map.webp", "a".repeat(31), "z".repeat(32)]) {
      expect(mapPicture({ [map]: value }, map)).toBeNull();
    }
  });
});
