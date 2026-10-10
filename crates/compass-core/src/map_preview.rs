//! The Map Preview: OSMS's picture of the whole of the shown map, with every portal on it
//! marked where it is, as OSMS Data Explorer's own map view marks them (ADR 0008).

use crate::map_data::PortalKind;
use serde::Serialize;

/// The shown map's Map Preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapPreview {
    /// The picture's content-hash name, from `map_manifest.json`: the picture is
    /// `maps/<picture>.webp` under OSMS's data.
    pub picture: String,
    /// Every portal on the map but those to the dead end, in OSMS's order.
    pub marks: Vec<PortalMark>,
}

/// A portal marked on the Map Preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalMark {
    /// Where it is, in the picture's pixels.
    pub x: i32,
    pub y: i32,
    pub leads: Leads,
    /// What kind of portal it is, by the Hidden Portal rule.
    pub kind: PortalKind,
    /// The destination's name and street; within the map, the map's own. A map not in
    /// the data is named from OSMS's lookups, with no street, or else by its ID.
    pub to_map_name: String,
    pub to_street_name: String,
    /// For a teleport within the map, where it lands.
    pub lands_at: Option<Landing>,
    /// Its row among the shown map's Hidden Portals (an index into
    /// [`crate::MapData::hidden_portals`]); `None` when it has none.
    pub hidden_portal: Option<usize>,
}

/// Where a portal leads, as OSMS colours its mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Leads {
    /// To another map in the data (blue).
    AnotherMap,
    /// To another spot on the same map: a teleport (green, with an arrow to where it lands).
    WithinTheMap,
    /// To a map that isn't in the data (grey).
    MapNotInTheData,
}

/// Where a teleport within the map lands, in the picture's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Landing {
    pub x: i32,
    pub y: i32,
    /// The mark it lands at (an index into [`MapPreview::marks`]), when the portal it
    /// lands at is marked; a spawn point isn't.
    pub mark: Option<usize>,
}
