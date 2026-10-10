//! OSMS Data Explorer's file set (ADR 0008): the raw texts the shell fetches from
//! [`DATA_URL`], and the reader that turns them into every map's data. Credit "OSMS Data
//! Explorer (osmsdataexplorer.com)".

use crate::map_data::MapData;
use crate::map_index::{MapEntry, SourceError};
use serde::{Deserialize, Serialize};

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
    let mut set = Vec::new();
    for region in maps.regions {
        for map in region.maps {
            let entry = MapEntry {
                id: map_id(map.id)?,
                street_name: map.street_name.trim().to_string(),
                map_name: map.name.trim().to_string(),
                region: map.region,
            };
            if entry.street_name.is_empty() || entry.map_name.is_empty() {
                continue;
            }
            set.push((entry, MapData::default()));
        }
    }
    Ok(MapSet { maps: set })
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
}
