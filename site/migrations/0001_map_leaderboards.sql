-- The Map Leaderboards (ADR 0015).

-- Each Character on the Map Leaderboards, its name claimed by the Leaderboard Key that
-- first uploaded it. Only the key's hash is kept.
CREATE TABLE characters (
  -- The name in lower case: names are one Character's whatever their case.
  name_key TEXT PRIMARY KEY,
  -- The name as last uploaded.
  name TEXT NOT NULL,
  -- SHA-256 of the Leaderboard Key, in hex.
  key_hash TEXT NOT NULL,
  claimed_at INTEGER NOT NULL,
  -- The last other key refused for the name, and when: the maintainer reassigns the name
  -- to it when the player has lost their key.
  refused_key_hash TEXT,
  refused_at INTEGER
);

-- One Character's best Visit on a map in a Level Band.
CREATE TABLE entries (
  name_key TEXT NOT NULL REFERENCES characters (name_key),
  -- The map's 9-digit ID.
  map_id TEXT NOT NULL,
  -- The Level Band, by its first level: 1 (1-9), 10 (10-19), 20 (20-29), ...
  band INTEGER NOT NULL,
  level INTEGER NOT NULL,
  job TEXT NOT NULL,
  exp_per_hour INTEGER NOT NULL,
  -- When the Visit set it (ms since the Unix epoch): the earlier wins a tie.
  set_at INTEGER NOT NULL,
  PRIMARY KEY (name_key, map_id, band)
);

CREATE INDEX entries_by_rank ON entries (map_id, band, exp_per_hour DESC, set_at, name_key);

-- Entries the maintainer deleted, so the same Visit uploaded again stays off.
CREATE TABLE removed_entries (
  name_key TEXT NOT NULL,
  map_id TEXT NOT NULL,
  band INTEGER NOT NULL,
  exp_per_hour INTEGER NOT NULL,
  PRIMARY KEY (name_key, map_id, band, exp_per_hour)
);

-- Players' reports against an entry, for the maintainer.
CREATE TABLE reports (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name_key TEXT NOT NULL,
  map_id TEXT NOT NULL,
  band INTEGER NOT NULL,
  -- The entry's EXP/h when it was reported.
  exp_per_hour INTEGER NOT NULL,
  reason TEXT NOT NULL,
  reported_at INTEGER NOT NULL
);

-- Each write per Leaderboard Key, and each report per reporter, over the last 10 minutes:
-- the rate limits count them.
CREATE TABLE rate_events (
  -- "key:" and the key's hash, or "report:" and the hash of the reporter's IP address.
  bucket TEXT NOT NULL,
  at INTEGER NOT NULL
);

CREATE INDEX rate_events_by_bucket ON rate_events (bucket, at);
