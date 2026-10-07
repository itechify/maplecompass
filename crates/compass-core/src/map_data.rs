//! One map's hunting information from MeowDB, and the panel's progress getting it.

use crate::map_index::{MapDataSource, SourceError};
use serde::Serialize;
use std::task::Poll;

/// One map's data, as the map data source reads it from the map's MeowDB page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapData {
    /// `None` on a map without mobs, such as a town: MeowDB has no summary for it.
    pub summary: Option<MapSummary>,
    /// In the panel, sorted by level (then name); from a source, in any order.
    pub mobs: Vec<Mob>,
    /// The exits the player can't see, in MeowDB's order. Empty when there are none.
    pub hidden_portals: Vec<HiddenPortal>,
}

/// A portal to another map that is drawn neither on the minimap nor in the world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenPortal {
    /// MeowDB's name for the portal on its map (e.g. `"in00"`); with the map ID, what
    /// the portal override list is keyed by.
    pub name: String,
    pub to_map_id: String,
    pub to_map_name: String,
    pub to_street_name: String,
    /// The destination is a Hidden Street.
    pub to_hidden_street: bool,
    /// A Quest Portal: it only works while the player meets a quest condition.
    pub quest: bool,
    /// Roughly where the portal is on the map, in words: "far right, lower", or
    /// "position unknown" when MeowDB has no minimap bounds for the map.
    pub position: String,
}

/// The Map Summary: MeowDB's ceiling figures (every mob killed as soon as it spawns),
/// with the map's mob rate not applied, as MeowDB gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapSummary {
    pub mobs_per_hour: PerParty,
    /// The EXP-per-hour ceiling.
    pub exp_per_hour: PerParty,
}

/// A figure for a player alone on the map, and for a party of 6 or more. MeowDB gives
/// only these two; sizes in between are not derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerParty {
    pub solo: u64,
    pub full_party: u64,
}

/// A mob that spawns on the map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mob {
    pub name: String,
    pub level: u32,
    pub hp: u64,
    pub exp: u64,
    /// How many of this mob the map holds at once.
    pub spawn_count: u32,
    /// Its spawn count as a share of all the map's spawns, in whole percent.
    pub spawn_share_percent: u32,
}

/// The shown map's MeowDB data, or why there is none (yet).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum MapDataStatus {
    /// No map is shown.
    #[default]
    NoMap,
    /// The map's page is being fetched.
    Loading,
    Ready(MapData),
    /// MeowDB couldn't be reached.
    Unavailable,
    /// MeowDB's page no longer looks the way the reader expects. Nothing from it is
    /// shown, rather than partial or wrong numbers.
    FormatChanged,
}

/// Asks the source for a map's data once the panel switches to that map, and only
/// then (ADR 0003: never ahead of time).
#[derive(Default)]
pub(crate) struct MapDataLoader {
    /// The map the status is for.
    id: Option<String>,
    status: MapDataStatus,
}

impl MapDataLoader {
    /// The panel shows map `id`: ask for its data if it's new, or check on a request
    /// still in flight.
    pub fn show(&mut self, id: Option<&str>, source: &mut dyn MapDataSource) -> &MapDataStatus {
        if id != self.id.as_deref() {
            self.id = id.map(str::to_string);
            self.status = if id.is_some() { MapDataStatus::Loading } else { MapDataStatus::NoMap };
        }
        if let (MapDataStatus::Loading, Some(id)) = (&self.status, &self.id) {
            self.status = match source.map_data(id) {
                Poll::Pending => MapDataStatus::Loading,
                Poll::Ready(Ok(mut data)) => {
                    data.mobs.sort_by(|a, b| a.level.cmp(&b.level).then_with(|| a.name.cmp(&b.name)));
                    MapDataStatus::Ready(data)
                }
                Poll::Ready(Err(SourceError::Unreachable(_))) => MapDataStatus::Unavailable,
                Poll::Ready(Err(SourceError::FormatChanged(_))) => MapDataStatus::FormatChanged,
            };
        }
        &self.status
    }
}
