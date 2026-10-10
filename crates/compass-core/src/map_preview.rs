//! The Map Preview: OSMS's picture of the whole of the shown map, with every portal on it
//! marked where it is, as OSMS Data Explorer's own map view marks them (ADR 0008).
//!
//! The picture is asked of the source while the preview shows, ahead of the Mob Cards'
//! animations, and its answer kept for the run, so a map shown again shows it at once.

use crate::map_data::PortalKind;
use crate::pictures::Pictures;
use serde::Serialize;

/// The shown map's Map Preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapPreview {
    /// The picture's content-hash name, from `map_manifest.json`: the picture is
    /// `maps/<picture>.webp` under OSMS's data.
    pub picture: String,
    /// Where the overlay loads the picture from, or why it can't yet.
    pub image: PreviewImage,
    /// Every portal on the map but those to the dead end, in OSMS's order.
    pub marks: Vec<PortalMark>,
}

/// The Map Preview's picture: where the overlay loads it from, or why it can't.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PreviewImage {
    /// The source is getting it.
    Loading,
    /// The overlay loads it from `url`, in MapleCompass's own scheme.
    Ready { url: String },
    /// It couldn't be had: OSMS couldn't be reached (it is asked for again a while on),
    /// or has no such picture.
    Unavailable,
}

/// The path under OSMS's data of the picture named `picture`.
pub(crate) fn picture_path(picture: &str) -> String {
    format!("maps/{picture}.webp")
}

impl PreviewImage {
    /// The picture named `picture`, as `pictures` has it.
    pub(crate) fn of(picture: &str, pictures: &Pictures) -> Self {
        match pictures.answer(&picture_path(picture)) {
            None => Self::Loading,
            Some(Ok(url)) => Self::Ready { url: url.to_string() },
            Some(Err(_)) => Self::Unavailable,
        }
    }
}

/// A point in a map's picture, in its pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PicturePoint {
    pub x: i32,
    pub y: i32,
}

/// A portal marked on the Map Preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalMark {
    /// Where it is (`x` and `y` beside the rest for the overlay).
    #[serde(flatten)]
    pub at: PicturePoint,
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

/// Where a teleport within the map lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Landing {
    /// Where it is (`x` and `y` beside the rest for the overlay).
    #[serde(flatten)]
    pub at: PicturePoint,
    /// The mark it lands at (an index into [`MapPreview::marks`]), when the portal it
    /// lands at is marked; a spawn point isn't.
    pub mark: Option<usize>,
    /// That mark is a teleport landing back at this one: the two are a pair, one way
    /// there and the other back.
    pub both_ways: bool,
}
