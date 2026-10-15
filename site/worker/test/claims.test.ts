// A Character's first upload ties its name to the Leaderboard Key it carries; later writes
// for the name need the same key, and one with another key is refused as "claimed".
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { FIELD, band, call, entry, key, names, optOut, upload } from "./api";

describe("name claims", () => {
  it("refuses a write with no Leaderboard Key", async () => {
    const response = await call("/api/characters/Ida/entries", {
      method: "PUT",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ entries: [entry(FIELD, 12, 10_000)] }),
    });

    expect(response.status).toBe(401);
    expect(await response.json()).toEqual({ error: "no-key" });
    expect(await names(FIELD, 10)).not.toContain("Ida");
  });

  it("refuses another key's upload for a claimed name, and keeps the claimant's entries", async () => {
    await upload("Jasper", key("jasper"), [entry(FIELD, 14, 20_000)]);

    const response = await upload("Jasper", key("impostor"), [entry(FIELD, 14, 99_000)]);

    expect(response.status).toBe(409);
    expect(await response.json()).toEqual({ error: "claimed" });
    const jasper = (await band(FIELD, 10)).entries.filter((e) => e.name === "Jasper");
    expect(jasper.map((e) => e.expPerHour)).toEqual([20_000]);
  });

  it("refuses another key's opt-out for a claimed name", async () => {
    await upload("Konrad", key("konrad"), [entry(FIELD, 15, 21_000)]);

    const response = await optOut("Konrad", key("impostor"));

    expect(response.status).toBe(409);
    expect(await names(FIELD, 10)).toContain("Konrad");
  });

  it("claims a name whatever its case", async () => {
    await upload("Lotte", key("lotte"), [entry(FIELD, 16, 22_000)]);

    const response = await upload("LOTTE", key("impostor"), [entry(FIELD, 16, 23_000)]);

    expect(response.status).toBe(409);
  });

  it("lets the claimant write again", async () => {
    const mine = key("magnus");
    await upload("Magnus", mine, [entry(FIELD, 17, 24_000)]);

    const response = await upload("Magnus", mine, [entry(FIELD, 18, 25_000)]);

    expect(response.status).toBe(200);
  });

  it("keeps a hash of the key, never the key itself", async () => {
    const mine = key("nora");
    await upload("Nora", mine, [entry(FIELD, 19, 26_000)]);

    const tables = ["characters", "entries", "rate_events"];
    for (const table of tables) {
      const rows = await env.DB.prepare(`SELECT * FROM ${table}`).all();
      expect(JSON.stringify(rows.results)).not.toContain(mine);
    }
  });

  it("refuses a name that can't be a Character's", async () => {
    const response = await upload("not a name!", key("odo"), [entry(FIELD, 19, 26_000)]);

    expect(response.status).toBe(400);
  });
});
