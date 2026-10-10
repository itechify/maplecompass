//! OSMS Data Explorer's file set (ADR 0008): the raw texts the shell fetches from
//! [`DATA_URL`], and the reader that turns them into every map's data. Credit "OSMS Data
//! Explorer (osmsdataexplorer.com)".

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
