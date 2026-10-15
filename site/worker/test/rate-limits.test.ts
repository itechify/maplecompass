// Writes are limited per Leaderboard Key (30 in 10 minutes, whichever names they are for),
// and reports per reporter (10 in 10 minutes).
import { describe, expect, it } from "vitest";
import { FIELD, entry, key, optOut, report, upload } from "./api";

describe("rate limits", () => {
  it("refuses a key's 31st write in 10 minutes, and no other key's", async () => {
    const busy = key("busy");
    for (let i = 0; i < 29; i++) {
      expect((await upload("Busy", busy, [entry(FIELD, 20, 1_000 + i)])).status).toBe(200);
    }
    expect((await optOut("Busy", busy)).status).toBe(204);

    const refused = await upload("Busy", busy, [entry(FIELD, 20, 2_000)]);
    const other = await upload("Calm", key("calm"), [entry(FIELD, 20, 2_000)]);

    expect(refused.status).toBe(429);
    expect(await refused.json()).toEqual({ error: "rate-limited" });
    const retryAfter = Number(refused.headers.get("retry-after"));
    expect(retryAfter).toBeGreaterThan(0);
    expect(retryAfter).toBeLessThanOrEqual(600);
    expect(other.status).toBe(200);
  });

  it("refuses a reporter's 11th report in 10 minutes", async () => {
    await upload("Reported", key("reported"), [entry(FIELD, 20, 3_000)]);
    const reportIt = (from: string) => report({ name: "Reported", map: FIELD, band: 20 }, from);
    for (let i = 0; i < 10; i++) expect((await reportIt("198.51.100.1")).status).toBe(201);

    expect((await reportIt("198.51.100.1")).status).toBe(429);
    expect((await reportIt("198.51.100.2")).status).toBe(201);
  });
});
