//! OSMS Data Explorer's file set (ADR 0008): the raw texts the shell fetches from
//! [`DATA_URL`], and the reader that turns them into every map's data. Credit "OSMS Data
//! Explorer (osmsdataexplorer.com)".

use crate::map_data::{MapData, MapSummary, Mob, PerParty};
use crate::map_index::{MapEntry, SourceError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Where OSMS Data Explorer publishes its current data set: each of [`OsmsFiles::NAMES`]
/// is a file under it.
pub const DATA_URL: &str = "https://osmsdataexplorer.com/data/current/";

/// OSMS's file set as fetched: each file's text, unread. Saved and swapped whole.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OsmsFiles {
    /// `maps.json`: every map by region, with its names, Town flag, nearest town, Mob
    /// Rate, exits and spawn points.
    pub maps: String,
    /// `portals.json`: every map's portals.
    pub portals: String,
    /// `monsters.json`: every mob's figures and the maps it spawns on.
    pub monsters: String,
    /// `map_manifest.json`: each map's picture, by content hash.
    pub map_manifest: String,
    /// `lookups.json`: names, among them those of maps outside `maps.json`.
    pub lookups: String,
}

impl OsmsFiles {
    /// The files' names under [`DATA_URL`], in the order of the fields.
    pub const NAMES: [&str; 5] = ["maps.json", "portals.json", "monsters.json", "map_manifest.json", "lookups.json"];
}

/// Every map's data from OSMS's file set: the map list, and each listed map's figures.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapSet {
    /// In OSMS's order: by region, then as `maps.json` lists them.
    maps: Vec<(MapEntry, MapData)>,
}

impl MapSet {
    /// Every map in the map list, with its data as read (mobs in any order).
    pub fn maps(&self) -> impl Iterator<Item = (&MapEntry, &MapData)> {
        self.maps.iter().map(|(entry, data)| (entry, data))
    }

    /// The map list.
    pub(crate) fn entries(&self) -> Vec<MapEntry> {
        self.maps.iter().map(|(entry, _)| entry.clone()).collect()
    }

    /// Map `id`'s data, if it is in the map list.
    pub(crate) fn data(&self, id: &str) -> Option<&MapData> {
        self.maps.iter().find(|(entry, _)| entry.id == id).map(|(_, data)| data)
    }
}

/// Read OSMS's file set. All or nothing: a file that isn't JSON, or isn't shaped as the
/// reader expects, is [`SourceError::FormatChanged`], never partial figures.
///
/// Map IDs are zero-padded to 9 digits, and names trimmed. A map with no name or no
/// street can't be told by its title, so it is left out.
pub fn read(files: &OsmsFiles) -> Result<MapSet, SourceError> {
    read_set(files).map_err(|why| SourceError::FormatChanged(format!("OSMS: {why}")))
}

fn read_set(files: &OsmsFiles) -> Result<MapSet, String> {
    let maps: MapsFile = parse(&files.maps, "maps.json")?;
    let monsters: MonstersFile = parse(&files.monsters, "monsters.json")?;
    let mobs: HashMap<u64, &OsmsMob> = monsters.monsters.iter().map(|mob| (mob.id, mob)).collect();
    let mut set = Vec::new();
    for region in maps.regions {
        for map in region.maps {
            let entry = MapEntry {
                id: map_id(map.id)?,
                street_name: map.street_name.trim().to_string(),
                map_name: map.name.trim().to_string(),
                region: map.region.clone(),
            };
            let (summary, mobs) = spawns(&map, &mobs).map_err(|why| format!("map {}: {why}", entry.id))?;
            if entry.street_name.is_empty() || entry.map_name.is_empty() {
                continue;
            }
            set.push((entry, MapData { summary, mobs, ..MapData::default() }));
        }
    }
    Ok(MapSet { maps: set })
}

/// How long the game takes to respawn a mob on an untimed spawn point, in seconds: a
/// full party kills each as soon as it spawns.
const RESPAWN_SECONDS: f64 = 7.56;

/// The share of a full party's kills a player alone gets on untimed spawn points.
const SOLO_SHARE: f64 = 0.75;

/// A map's mobs, from its spawn points grouped by mob, with their figures from the mobs'
/// records; and its Map Summary, worked out as MeowDB works it out (`None` without mobs).
///
/// An untimed spawn point gives a full party `3600 / 7.56` mobs an hour, and a player
/// alone 0.75 of that. A timed one gives `3600 / timer`, alone or in a party; a ranged
/// timer, the average OSMS gives. Each mob's EXP an hour is its EXP times its mobs an
/// hour. Each mob's figures are rounded, the map's are their sums, and the map's Mob
/// Rate isn't applied.
fn spawns(map: &OsmsMap, mobs: &HashMap<u64, &OsmsMob>) -> Result<(Option<MapSummary>, Vec<Mob>), String> {
    let mut points: Vec<(u64, u64)> = Vec::new();
    for point in &map.mob_positions {
        match points.iter_mut().find(|(mob, _)| *mob == point.id) {
            Some((_, count)) => *count += 1,
            None => points.push((point.id, 1)),
        }
    }
    let total: u64 = points.iter().map(|(_, count)| count).sum();
    let mut sums = [0u64; 4];
    let mut listed = Vec::new();
    for (id, count) in points {
        let mob = mobs.get(&id).ok_or_else(|| format!("mob {id} isn't in monsters.json"))?;
        let timer = mob.maps.iter().find(|on| on.id == map.id).and_then(|on| on.mob_time.as_deref());
        let (solo, full_party) = match timer.map(seconds).transpose()? {
            Some(timer) => (3600.0 / timer, 3600.0 / timer),
            None => (SOLO_SHARE * 3600.0 / RESPAWN_SECONDS, 3600.0 / RESPAWN_SECONDS),
        };
        let (solo, full_party) = (count as f64 * solo, count as f64 * full_party);
        let exp = mob.exp as f64;
        for (sum, figure) in sums.iter_mut().zip([solo, full_party, solo * exp, full_party * exp]) {
            *sum += figure.round() as u64;
        }
        listed.push(Mob {
            id: id.to_string(),
            name: mob.name.trim().to_string(),
            level: mob.level,
            hp: mob.hp,
            exp: mob.exp,
            spawn_count: u32::try_from(count).map_err(|e| e.to_string())?,
            // Rounded half up, as MeowDB shows it.
            spawn_share_percent: ((count * 200 + total) / (2 * total)) as u32,
        });
    }
    let [mobs_solo, mobs_party, exp_solo, exp_party] = sums;
    let summary = (!listed.is_empty()).then_some(MapSummary {
        mobs_per_hour: PerParty { solo: mobs_solo, full_party: mobs_party },
        exp_per_hour: PerParty { solo: exp_solo, full_party: exp_party },
    });
    Ok((summary, listed))
}

/// A spawn timer in seconds: `"300.00"`, or for a ranged timer the average in brackets,
/// `"180.00-1800.00 [720.00]"`.
fn seconds(timer: &str) -> Result<f64, String> {
    let number = |s: &str| s.trim().parse::<f64>().ok().filter(|n| n.is_finite() && *n > 0.0);
    let average = match timer.split_once('[') {
        Some((range, average)) => {
            let bounds = range.split_once('-').and_then(|(low, high)| Some((number(low)?, number(high)?)));
            bounds.and(average.strip_suffix(']')).and_then(number)
        }
        None => number(timer),
    };
    average.ok_or_else(|| format!("mob_time {timer:?} isn't a timer"))
}

/// One of the set's files, read into `T`.
fn parse<T: serde::de::DeserializeOwned>(text: &str, file: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|e| format!("{file}: {e}"))
}

/// A map ID as MapleCompass writes it: 9 digits, zero padding kept.
fn map_id(id: u64) -> Result<String, String> {
    if id > 999_999_999 {
        return Err(format!("map ID {id} has more than 9 digits"));
    }
    Ok(format!("{id:09}"))
}

/// `maps.json`.
#[derive(Deserialize)]
struct MapsFile {
    regions: Vec<Region>,
}

#[derive(Deserialize)]
struct Region {
    maps: Vec<OsmsMap>,
}

/// A map in `maps.json`. Optional fields are absent, never null.
#[derive(Deserialize)]
struct OsmsMap {
    id: u64,
    name: String,
    street_name: String,
    region: String,
    /// One per spawn point.
    #[serde(default)]
    mob_positions: Vec<SpawnPoint>,
}

#[derive(Deserialize)]
struct SpawnPoint {
    /// The mob's ID.
    id: u64,
}

/// `monsters.json`.
#[derive(Deserialize)]
struct MonstersFile {
    monsters: Vec<OsmsMob>,
}

/// A mob in `monsters.json`.
#[derive(Deserialize)]
struct OsmsMob {
    id: u64,
    name: String,
    level: u32,
    hp: u64,
    exp: u64,
    /// The maps it spawns on; absent for a mob on none.
    #[serde(default)]
    maps: Vec<MobOnMap>,
}

#[derive(Deserialize)]
struct MobOnMap {
    id: u64,
    /// Its spawn points' timer on the map, if they're timed.
    mob_time: Option<String>,
}
