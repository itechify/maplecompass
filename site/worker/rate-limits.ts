// Rate limits, counted in D1 over a sliding window: per Leaderboard Key for writes, per
// reporter for reports.
import type { Answer } from "./http";

/// The window the limits count over.
const WINDOW_MS = 10 * 60_000;

/// Writes one Leaderboard Key may make in the window, whatever names they are for: an
/// upload or an opt-out each. The app writes a Character's set once at opting in and then
/// as each Visit ends, so a player never comes near it.
export const WRITES = 30;

/// Reports one reporter (by IP address) may make in the window.
export const REPORTS = 10;

/// Count a request in `bucket`, and refuse it if `limit` were already made in the window.
/// A refused request isn't counted.
export async function limit(env: Env, bucket: string, most: number): Promise<Answer | undefined> {
  const now = Date.now();
  const since = now - WINDOW_MS;
  const [, made] = await env.DB.batch<{ n: number; oldest: number | null }>([
    env.DB.prepare("DELETE FROM rate_events WHERE at <= ?").bind(since),
    env.DB.prepare("SELECT COUNT(*) AS n, MIN(at) AS oldest FROM rate_events WHERE bucket = ? AND at > ?").bind(bucket, since),
  ]);
  const { n, oldest } = made.results[0];
  if (n >= most) {
    const retryAfter = Math.max(1, Math.ceil(((oldest ?? now) + WINDOW_MS - now) / 1000));
    return { status: 429, body: { error: "rate-limited" }, headers: { "retry-after": String(retryAfter) } };
  }
  await env.DB.prepare("INSERT INTO rate_events (bucket, at) VALUES (?, ?)").bind(bucket, now).run();
  return undefined;
}
