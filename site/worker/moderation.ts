// Reports against entries, and the maintainer's routes: reading the reports, deleting an
// entry, and reassigning a name to the key its player has now.
import { type Answer, badRequest, body } from "./http";
import { hash } from "./keys";
import { REPORTS, limit } from "./rate-limits";

/// A report's reason is cut to this many characters.
const REASON_LENGTH = 500;

/// Record a report against the entry `{ name, map, band }`, with an optional `reason`.
export async function report(env: Env, request: Request): Promise<Answer> {
  const { name, map, band, reason } = ((await body(request)) ?? {}) as Record<string, unknown>;
  if (typeof name !== "string" || typeof map !== "string" || !Number.isInteger(band)) {
    return badRequest("a report is { name, map, band, reason? }");
  }
  if (reason !== undefined && typeof reason !== "string") return badRequest("a report's reason is text");
  const reporter = await hash(request.headers.get("cf-connecting-ip") ?? "unknown");
  const limited = await limit(env, `report:${reporter}`, REPORTS);
  if (limited !== undefined) return limited;
  const reported = await env.DB.prepare(
    `INSERT INTO reports (name_key, map_id, band, exp_per_hour, reason, reported_at)
     SELECT name_key, map_id, band, exp_per_hour, ?4, ?5 FROM entries WHERE name_key = ?1 AND map_id = ?2 AND band = ?3`,
  )
    .bind(name.toLowerCase(), map, band, (reason ?? "").slice(0, REASON_LENGTH), Date.now())
    .run();
  if (reported.meta.changes === 0) return { status: 404, body: { error: "no-entry" } };
  return { status: 201, body: {} };
}

/// Every report, newest first, with the reported entry's name as it is now.
export async function readReports(env: Env): Promise<Answer> {
  const rows = await env.DB.prepare(
    `SELECT r.id, COALESCE(c.name, r.name_key) AS name, r.map_id AS map, r.band, r.exp_per_hour AS expPerHour,
            r.reason, r.reported_at AS reportedAt
     FROM reports r LEFT JOIN characters c USING (name_key) ORDER BY r.reported_at DESC, r.id DESC`,
  ).all();
  return { status: 200, body: { reports: rows.results } };
}

/// Delete `name`'s entry on `map` in the Level Band starting at `band`, and its reports.
/// The same Visit (the same EXP/h there) uploaded again is rejected as "removed".
export async function deleteEntry(env: Env, name: string, map: string, band: string): Promise<Answer> {
  const at = [name.toLowerCase(), map, Number(band)];
  const [removed] = await env.DB.batch([
    env.DB.prepare(
      `INSERT OR IGNORE INTO removed_entries (name_key, map_id, band, exp_per_hour)
       SELECT name_key, map_id, band, exp_per_hour FROM entries WHERE name_key = ?1 AND map_id = ?2 AND band = ?3`,
    ).bind(...at),
    env.DB.prepare("DELETE FROM entries WHERE name_key = ?1 AND map_id = ?2 AND band = ?3").bind(...at),
    env.DB.prepare("DELETE FROM reports WHERE name_key = ?1 AND map_id = ?2 AND band = ?3").bind(...at),
  ]);
  if (removed.meta.changes === 0) return { status: 404, body: { error: "no-entry" } };
  return { status: 204 };
}

/// Move `name`'s claim to the last key refused for it: the player's after they lost the
/// key that claimed it (a reinstall, a new PC). The maintainer never sees either key; the
/// answer says when that key was refused, to check with the player.
export async function reassign(env: Env, name: string): Promise<Answer> {
  const nameKey = name.toLowerCase();
  const character = await env.DB.prepare("SELECT name, refused_key_hash, refused_at FROM characters WHERE name_key = ?")
    .bind(nameKey)
    .first<{ name: string; refused_key_hash: string | null; refused_at: number | null }>();
  if (character === null) return { status: 404, body: { error: "no-character" } };
  if (character.refused_key_hash === null) return { status: 409, body: { error: "no-refused-key" } };
  await env.DB.prepare(
    `UPDATE characters SET key_hash = refused_key_hash, claimed_at = ?3, refused_key_hash = NULL, refused_at = NULL
     WHERE name_key = ?1 AND refused_key_hash = ?2`,
  )
    .bind(nameKey, character.refused_key_hash, Date.now())
    .run();
  return { status: 200, body: { name: character.name, refusedAt: character.refused_at } };
}
