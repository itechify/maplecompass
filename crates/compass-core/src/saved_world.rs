//! What the world crawl has learned, as plain data the shell keeps between launches
//! (ADR 0004): every map known, with the date its data is from. The next launch takes
//! it back and knows the whole world at once, rather than reading every cached page
//! again.

use crate::map_data::{HiddenPortal, MapData, MapSummary, Mob, Portal};
use serde::{Deserialize, Serialize};

/// The shape [`SavedWorld`] has now. Bumped whenever it changes, or the MeowDB reader
/// starts reading a page differently: a world saved in another shape is left, and the
/// shell builds it again from its cached pages.
pub const WORLD_VERSION: u32 = 1;

/// Every map the world crawl knows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedWorld {
    /// The [`WORLD_VERSION`] it was saved by.
    pub version: u32,
    pub maps: Vec<SavedMap>,
}

impl Default for SavedWorld {
    /// No maps known.
    fn default() -> Self {
        Self { version: WORLD_VERSION, maps: Vec::new() }
    }
}

/// One map the crawl knows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedMap {
    /// MeowDB's 9-digit map ID.
    pub id: String,
    /// When MeowDB served the page the data was read from (ms since the Unix epoch).
    pub as_of_unix_ms: u64,
    /// What the page said: everything the crawl uses, the portal graph included.
    pub data: SavedMapData,
}

/// A map's data as saved: all of [`MapData`] a source reads, including what the overlay
/// is never sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedMapData {
    summary: Option<MapSummary>,
    mobs: Vec<Mob>,
    hidden_portals: Vec<HiddenPortal>,
    connected_maps: Vec<String>,
    portals: Vec<Portal>,
    mob_rate_percent: u32,
    nearest_town: Option<String>,
}

impl From<&MapData> for SavedMapData {
    fn from(data: &MapData) -> Self {
        Self {
            summary: data.summary,
            mobs: data.mobs.clone(),
            hidden_portals: data.hidden_portals.clone(),
            connected_maps: data.connected_maps.clone(),
            portals: data.portals.clone(),
            mob_rate_percent: data.mob_rate_percent,
            nearest_town: data.nearest_town.clone(),
        }
    }
}

impl From<SavedMapData> for MapData {
    fn from(saved: SavedMapData) -> Self {
        Self {
            summary: saved.summary,
            mobs: saved.mobs,
            hidden_portals: saved.hidden_portals,
            connected_maps: saved.connected_maps,
            portals: saved.portals,
            mob_rate_percent: saved.mob_rate_percent,
            nearest_town: saved.nearest_town,
            ..MapData::default()
        }
    }
}
