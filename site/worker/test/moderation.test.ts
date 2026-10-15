// Anyone can report an entry; the maintainer (with the ADMIN_TOKEN secret) reads the
// reports, deletes entries, and reassigns a name to the key its player has now, after they
// lost the one that claimed it.
import { describe, expect, it } from "vitest";
import { FIELD, admin, band, entry, key, names, optOut, report, upload } from "./api";

describe("reports", () => {
  it("records a report against an entry, for the maintainer to read", async () => {
    await upload("Ulla", key("ulla"), [entry(FIELD, 91, 400_000)]);

    const response = await report({ name: "ulla", map: FIELD, band: 90, reason: "Too fast for that level" });

    expect(response.status).toBe(201);
    const reports = await (await admin("/reports")).json();
    expect(reports).toMatchObject({
      reports: [{ name: "Ulla", map: FIELD, band: 90, expPerHour: 400_000, reason: "Too fast for that level" }],
    });
  });

  it("refuses a report against an entry there isn't", async () => {
    const response = await report({ name: "Nobody", map: FIELD, band: 90 });

    expect(response.status).toBe(404);
  });
});

describe("the maintainer", () => {
  it("is the only one who reads reports, deletes entries or reassigns names", async () => {
    await upload("Viktor", key("viktor"), [entry(FIELD, 93, 300_000)]);

    const tries = [
      await admin("/reports", {}, "not-the-token"),
      await admin(`/entries/Viktor/${FIELD}/90`, { method: "DELETE" }, "not-the-token"),
      await admin("/characters/Viktor/reassign", { method: "POST" }, key("viktor")),
    ];

    expect(tries.map((r) => r.status)).toEqual([401, 401, 401]);
    expect(await names(FIELD, 90)).toContain("Viktor");
  });

  it("deletes an entry, and the same Visit uploaded again stays off", async () => {
    const mine = key("wanda");
    await upload("Wanda", mine, [entry(FIELD, 95, 450_000)]);
    await report({ name: "Wanda", map: FIELD, band: 90 });

    const response = await admin(`/entries/Wanda/${FIELD}/90`, { method: "DELETE" });
    const again = await upload("Wanda", mine, [entry(FIELD, 96, 450_000)]);

    expect(response.status).toBe(204);
    expect(await again.json()).toEqual({ accepted: 0, rejected: [{ map: FIELD, band: 90, reason: "removed" }] });
    expect(await names(FIELD, 90)).not.toContain("Wanda");
    const reports = (await (await admin("/reports")).json()) as { reports: { name: string }[] };
    expect(reports.reports.map((r) => r.name)).not.toContain("Wanda");
  });

  it("lets a Visit with another EXP/h back on after a deletion", async () => {
    const mine = key("xaver");
    await upload("Xaver", mine, [entry(FIELD, 97, 450_000)]);
    await admin(`/entries/Xaver/${FIELD}/90`, { method: "DELETE" });

    await upload("Xaver", mine, [entry(FIELD, 98, 460_000)]);

    expect(await names(FIELD, 90)).toContain("Xaver");
  });

  it("reassigns a name to the last key refused for it, which may then write", async () => {
    await upload("Yvonne", key("yvonne-old-pc"), [entry(FIELD, 99, 410_000)]);
    const refused = await upload("Yvonne", key("yvonne-new-pc"), [entry(FIELD, 99, 420_000)]);

    const response = await admin("/characters/Yvonne/reassign", { method: "POST" });
    const retried = await upload("Yvonne", key("yvonne-new-pc"), [entry(FIELD, 99, 420_000)]);

    expect(refused.status).toBe(409);
    expect(response.status).toBe(200);
    expect(await response.json()).toMatchObject({ name: "Yvonne" });
    expect(retried.status).toBe(200);
    expect((await band(FIELD, 90, "Yvonne")).character).toMatchObject({ expPerHour: 420_000 });
    expect((await optOut("Yvonne", key("yvonne-old-pc"))).status).toBe(409);
  });

  it("can't reassign a name no other key has asked for", async () => {
    await upload("Zora", key("zora"), [entry(FIELD, 99, 400_000)]);

    const response = await admin("/characters/Zora/reassign", { method: "POST" });

    expect(response.status).toBe(409);
    expect(await response.json()).toEqual({ error: "no-refused-key" });
  });
});
