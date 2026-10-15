//! Uploading to the Map Leaderboards (#110, ADR 0015): the player shows a Character there from
//! the Personal Bests window, and the core says what the site should hold of them. The shell
//! does the HTTP, with the Leaderboard Key it keeps (never the core: a Bug Report writes down
//! everything the core takes in), and tells the core each answer.
//!
//! - **Opting in** is per Character, never for the Unassigned bests. The player confirms with
//!   the count of maps there is something to upload for. Opting out deletes all of the
//!   Character's entries from the site. Which Characters are shown, and when each last upload
//!   went through, are kept with the settings.
//! - **What is uploaded**: each shown Character's whole set of entries, which replaces the set
//!   before it on the site, so a Visit removed, merged or moved in the window goes from the
//!   site too. An entry is the Character's best Visit on a map in a Level Band, of those
//!   that have ended (the Visit under way's best goes up when it ends), with the level and
//!   job that set it and its date. A Visit with no level, job or date can't be an entry: the
//!   next best dated one in its band stands in. Nor can one dated over a day ahead of the
//!   wall clock (the site refuses those), until it isn't.
//! - **When**: once per shown Character at each launch (what the site holds may have changed
//!   since), then whenever their set changes, one upload at a time each. Only on a tick, so
//!   the bests are back and the wall clock known first.
//! - **The answers**: taken in, with the over-cap and removed entries listed, but neither is a
//!   failure; a name claimed by another key says so (and the player asks on Discord), and is
//!   tried again now and then and at each change, since the maintainer can only hand the name
//!   to a key it refused; rate-limited waits as long as the site says; anything else (offline,
//!   say) is tried again quietly, later each time.
//! - **Removing a Character**, or merging one into another, takes the one gone off the site.

use crate::personal_best::{LevelBand, SavedBests};
use crate::SITE_URL;
use crate::status_bar::MAX_LEVEL;
use serde::{Deserialize, Serialize};

/// How long to wait after the first failed upload; each failure after doubles it, up to
/// [`MOST_RETRY_MS`].
const FIRST_RETRY_MS: u64 = 30_000;
const MOST_RETRY_MS: u64 = 30 * 60_000;
/// How long a Character whose name another key holds waits to try again, unless their set
/// changes first.
const CLAIMED_RETRY_MS: u64 = 30 * 60_000;
/// How far ahead of the wall clock an entry's date may be: the site takes a day.
const AHEAD_MS: u64 = 23 * 60 * 60_000;
/// The longest job name the site takes.
const MOST_JOB_CHARS: usize = 32;

/// One entry on a Map Leaderboard, as the site's API takes it: a Character's best Visit on a
/// map in a Level Band (the site works the band out from the level).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntry {
    /// The 9-digit map ID.
    pub map: String,
    pub level: u32,
    pub job: String,
    pub exp_per_hour: u64,
    /// When the Visit set it, in ms since the Unix epoch.
    pub set_at: u64,
}

/// An upload the shell is to make, from when the core asks for it until the shell tells it
/// the answer: a Character's whole set of entries (`PUT`), or, with `entries` `None`, taking
/// them all off the site (`DELETE`, opting out).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardUpload {
    /// Which upload it is: one more with each the core asks for. The answer names it.
    pub id: u64,
    /// The Character's name.
    pub name: String,
    pub entries: Option<Vec<LeaderboardEntry>>,
}

/// What the Map Leaderboards answered an upload, as the shell tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LeaderboardAnswer {
    /// The set was taken (200), but for the entries `rejected`.
    Accepted { rejected: Vec<RejectedEntry> },
    /// The entries were deleted (204).
    Deleted {},
    /// Another Leaderboard Key holds the name (409 `claimed`).
    Claimed {},
    /// Too many uploads lately (429): try again in `retry_after_secs`.
    RateLimited { retry_after_secs: u64 },
    /// The site couldn't be reached, or answered otherwise: why, in words.
    Failed { why: String },
}

/// An entry an upload held that the Map Leaderboards didn't take, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedEntry {
    /// The 9-digit map ID.
    pub map: String,
    /// The Level Band, by its first level (1, 10, 20…), as the site names it.
    pub band: u32,
    pub reason: RejectedReason,
}

/// Why the Map Leaderboards didn't take an entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RejectedReason {
    /// Its EXP/h is above 5× the map's solo ceiling: `cap` is the most an entry there may have.
    OverCap { cap: u64 },
    /// The maintainer removed it: the same Visit stays off.
    Removed {},
}

/// A Character the player has shown on the Map Leaderboards, or taken off while the site
/// hasn't yet heard: kept with the settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardCharacter {
    pub name: String,
    /// Shown; `false`: taken off, and their entries not yet deleted from the site.
    pub shown: bool,
    /// When an upload of theirs last went through, by the wall clock (ms since the Unix epoch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uploaded_at_unix_ms: Option<u64>,
}

/// The Personal Bests window's "Show on the Map Leaderboards" for a Character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardToggle {
    /// The player shows them there.
    pub on: bool,
    /// How many maps they have an entry for: what turning it on asks about ("Upload Egbert's
    /// bests on 14 maps?").
    pub maps: usize,
    pub status: LeaderboardStatus,
    /// When an upload of theirs last went through (ms since the Unix epoch); `None` if none
    /// has yet.
    pub uploaded_at_unix_ms: Option<u64>,
    /// The entries the site didn't take at the last upload.
    pub rejected: Vec<RejectedEntry>,
}

/// Where a Character stands with the Map Leaderboards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LeaderboardStatus {
    /// Not shown there, and nothing of theirs left to delete.
    Off,
    /// An upload (or, turned off, the deletion) is under way or about to be.
    Sending,
    /// The site has what it should: their entries, as last uploaded.
    UpToDate,
    /// The last upload didn't go through: it is tried again quietly.
    Retrying,
    /// Another Leaderboard Key holds the name: the player asks on Discord.
    Claimed,
}

/// A Character's standing with the site, this launch.
#[derive(Debug, Clone)]
struct Site {
    saved: LeaderboardCharacter,
    /// What the site was last told to hold of theirs and took; `None` until it has this launch.
    confirmed: Option<Vec<LeaderboardEntry>>,
    /// The upload under way.
    in_flight: Option<LeaderboardUpload>,
    /// The upload last tried didn't go through: not again until then, unless what is to be
    /// sent changes.
    wait: Option<Wait>,
    /// How many uploads in a row haven't gone through.
    failures: u32,
    /// Another key holds the name, as the last answer said.
    claimed: bool,
    rejected: Vec<RejectedEntry>,
}

#[derive(Debug, Clone)]
struct Wait {
    until_ms: u64,
    /// What was to be sent then.
    wanted: Option<Vec<LeaderboardEntry>>,
    /// The site said how long: a change doesn't cut it short.
    rate_limited: bool,
}

/// Every Character shown on the Map Leaderboards, or being taken off.
#[derive(Debug, Default)]
pub(crate) struct Leaderboards {
    sites: Vec<Site>,
    /// The id of the last upload asked for.
    last_id: u64,
    /// The latest capture time seen in a tick.
    now_ms: u64,
}

impl Leaderboards {
    /// Take back the Characters an earlier launch kept, in place of any: nothing is under way.
    pub fn restore(&mut self, saved: Vec<LeaderboardCharacter>) {
        self.sites = saved.into_iter().map(Site::new).collect();
    }

    /// The Characters to keep with the settings.
    pub fn saved(&self) -> Vec<LeaderboardCharacter> {
        self.sites.iter().map(|site| site.saved.clone()).collect()
    }

    /// The player showed the Character named `name` on the Map Leaderboards (`on`), or took them
    /// off.
    pub fn show(&mut self, name: &str, on: bool) {
        match self.sites.iter_mut().find(|site| site.saved.name == name) {
            Some(site) if site.saved.shown != on => {
                site.saved.shown = on;
                // Shown again, the site may have deleted them meanwhile; taken off, nothing
                // held stands.
                site.confirmed = None;
                site.wait = None;
                site.failures = 0;
            }
            Some(_) => {}
            None if on => self.sites.push(Site::new(LeaderboardCharacter {
                name: name.to_string(),
                shown: true,
                uploaded_at_unix_ms: None,
            })),
            None => {}
        }
    }

    /// Time passed, to `at_ms`: ask for each upload due, with `history` the bests of ended
    /// Visits and `wall_clock` the time now. None before the wall clock is known.
    pub fn tick(&mut self, at_ms: u64, history: &SavedBests, wall_clock: Option<u64>) {
        self.now_ms = self.now_ms.max(at_ms);
        let Some(now_unix_ms) = wall_clock else { return };
        for site in &mut self.sites {
            if site.in_flight.is_some() {
                continue;
            }
            let wanted = site.saved.shown.then(|| entries(history, &site.saved.name, now_unix_ms));
            let due = match &wanted {
                Some(entries) => site.confirmed.as_ref() != Some(entries),
                None => true,
            };
            let waiting = site.wait.as_ref().is_some_and(|wait| {
                self.now_ms < wait.until_ms && (wait.rate_limited || wait.wanted == wanted)
            });
            if due && !waiting {
                self.last_id += 1;
                site.in_flight = Some(LeaderboardUpload { id: self.last_id, name: site.saved.name.clone(), entries: wanted });
            }
        }
    }

    /// The site answered upload `id` with `answer`, the wall clock at `wall_clock`.
    pub fn answer(&mut self, id: u64, answer: LeaderboardAnswer, wall_clock: Option<u64>) {
        let Some(at) = self.sites.iter().position(|site| site.in_flight.as_ref().is_some_and(|u| u.id == id)) else {
            return;
        };
        let site = &mut self.sites[at];
        let Some(upload) = site.in_flight.take() else { return };
        match (answer, upload.entries) {
            (LeaderboardAnswer::Accepted { rejected }, Some(entries)) => {
                site.confirmed = Some(entries);
                site.saved.uploaded_at_unix_ms = wall_clock.or(site.saved.uploaded_at_unix_ms);
                site.rejected = rejected;
                site.wait = None;
                site.failures = 0;
                site.claimed = false;
            }
            // Deleted: nothing of theirs is left there. Shown again meanwhile, their set goes up
            // at the next tick.
            (LeaderboardAnswer::Deleted {}, None) => {
                if site.saved.shown {
                    site.confirmed = None;
                } else {
                    self.sites.remove(at);
                }
            }
            // Another key holds the name, so nothing of theirs is there to delete.
            (LeaderboardAnswer::Claimed {}, None) if !site.saved.shown => {
                self.sites.remove(at);
            }
            (LeaderboardAnswer::Claimed {}, wanted) => {
                site.confirmed = None;
                site.claimed = true;
                site.failures = 0;
                site.wait = Some(Wait { until_ms: self.now_ms + CLAIMED_RETRY_MS, wanted, rate_limited: false });
            }
            (LeaderboardAnswer::RateLimited { retry_after_secs }, wanted) => {
                site.failures += 1;
                let until_ms = self.now_ms + retry_after_secs.max(1).saturating_mul(1000);
                site.wait = Some(Wait { until_ms, wanted, rate_limited: true });
            }
            // Failed, or an answer that doesn't fit the upload: tried again, later each time.
            (_, wanted) => {
                site.failures += 1;
                let backoff = FIRST_RETRY_MS.saturating_mul(1 << (site.failures - 1).min(16)).min(MOST_RETRY_MS);
                site.wait = Some(Wait { until_ms: self.now_ms + backoff, wanted, rate_limited: false });
            }
        }
    }

    /// The uploads under way, for the shell to make.
    pub fn uploads(&self) -> Vec<LeaderboardUpload> {
        self.sites.iter().filter_map(|site| site.in_flight.clone()).collect()
    }

    /// The toggle for the Character named `name`, with `history` the bests of ended Visits.
    pub fn toggle(&self, name: &str, history: &SavedBests, wall_clock: Option<u64>) -> LeaderboardToggle {
        let mut maps: Vec<String> = Vec::new();
        for entry in entries(history, name, wall_clock.unwrap_or(u64::MAX - AHEAD_MS)) {
            if !maps.contains(&entry.map) {
                maps.push(entry.map);
            }
        }
        let site = self.sites.iter().find(|site| site.saved.name == name);
        let status = match site {
            None => LeaderboardStatus::Off,
            Some(site) if site.claimed => LeaderboardStatus::Claimed,
            Some(site) if site.failures > 0 => LeaderboardStatus::Retrying,
            Some(site) if site.in_flight.is_some() || !site.saved.shown || site.confirmed.is_none() => {
                LeaderboardStatus::Sending
            }
            Some(_) => LeaderboardStatus::UpToDate,
        };
        LeaderboardToggle {
            on: site.is_some_and(|site| site.saved.shown),
            maps: maps.len(),
            status,
            uploaded_at_unix_ms: site.and_then(|site| site.saved.uploaded_at_unix_ms),
            rejected: site.map_or_else(Vec::new, |site| site.rejected.clone()),
        }
    }
}

impl Site {
    fn new(saved: LeaderboardCharacter) -> Self {
        Self { saved, confirmed: None, in_flight: None, wait: None, failures: 0, claimed: false, rejected: Vec::new() }
    }
}

/// Map `map`'s Map Leaderboard on the site (ADR 0015's deep link), opened at Level Band `band`
/// with the Character named `name`'s entry marked, as much as is known of them.
pub(crate) fn map_url(map: &str, band: Option<LevelBand>, name: Option<&str>) -> String {
    let mut url = format!("{SITE_URL}leaderboards/map/?map={map}");
    if let Some(band) = band {
        url += &format!("&band={}", band.lowest());
    }
    if let Some(name) = name {
        url += "&name=";
        for byte in name.bytes() {
            if byte.is_ascii_alphanumeric() {
                url.push(char::from(byte));
            } else {
                url += &format!("%{byte:02X}");
            }
        }
    }
    url
}

/// The entries of the Character named `name` in `history`, the bests of ended Visits: their best
/// Visit on each map in each Level Band that has a level, a job and a date, and isn't dated over
/// a day after `now_unix_ms`. By map in the order they are saved, then by band, lowest first.
fn entries(history: &SavedBests, name: &str, now_unix_ms: u64) -> Vec<LeaderboardEntry> {
    let mut entries = Vec::new();
    for map in history.maps_of(Some(name)) {
        let mut bands: Vec<(LevelBand, LeaderboardEntry)> = Vec::new();
        // Highest first: the first in each band is its best.
        for visit in &map.visits {
            let (Some(level), Some(job), Some(set_at)) = (visit.level, &visit.job, visit.set_at_unix_ms) else { continue };
            let job_fits = (1..=MOST_JOB_CHARS).contains(&job.chars().count());
            if !(1..=MAX_LEVEL).contains(&level) || !job_fits || set_at > now_unix_ms.saturating_add(AHEAD_MS) {
                continue;
            }
            let band = LevelBand::of(level);
            if bands.iter().all(|(b, _)| *b != band) {
                let entry = LeaderboardEntry {
                    map: map.id.clone(),
                    level,
                    job: job.clone(),
                    exp_per_hour: visit.exp_per_hour,
                    set_at,
                };
                bands.push((band, entry));
            }
        }
        bands.sort_by_key(|(band, _)| *band);
        entries.extend(bands.into_iter().map(|(_, entry)| entry));
    }
    entries
}
