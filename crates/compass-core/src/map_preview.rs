//! The Map Preview: OSMS's picture of the whole of the shown map, with every portal on it
//! marked where it is, as OSMS Data Explorer's own map view marks them (ADR 0008).
//!
//! The picture is asked of the source while the preview shows, one at a time, and its
//! answer kept for the run, so a map shown again shows it at once.

use crate::map_data::PortalKind;
use crate::map_index::{MapDataSource, SourceError};
use serde::Serialize;
use std::task::Poll;

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

/// How long a picture OSMS couldn't serve waits before it is asked for again, while its
/// map is still shown.
const RETRY_MS: u64 = 5 * 60_000;

/// The pictures asked of the source: each answer, kept for the run, and the one awaited.
#[derive(Default)]
pub(crate) struct Pictures {
    /// Each picture asked for, by path, its latest answer, and when it came.
    answers: Vec<(String, Result<String, SourceError>, u64)>,
    in_flight: Option<String>,
}

impl Pictures {
    /// Time passed: check on the picture in flight (even one no longer wanted), then ask
    /// for `wanted` (a path under OSMS's data) if it has no answer yet, or OSMS couldn't
    /// be reached for it [`RETRY_MS`] ago.
    pub fn poll(&mut self, source: &mut dyn MapDataSource, wanted: Option<&str>, at_ms: u64) {
        if let Some(path) = self.in_flight.take() {
            match source.picture(&path) {
                Poll::Pending => {
                    self.in_flight = Some(path);
                    return;
                }
                Poll::Ready(answer) => self.keep(path, answer, at_ms),
            }
        }
        let Some(wanted) = wanted else { return };
        let due = match self.answers.iter().find(|(path, ..)| path == wanted) {
            None => true,
            Some((_, Err(SourceError::Unreachable(_)), came_ms)) => at_ms.saturating_sub(*came_ms) >= RETRY_MS,
            Some(_) => false,
        };
        if due {
            match source.picture(wanted) {
                Poll::Pending => self.in_flight = Some(wanted.to_string()),
                Poll::Ready(answer) => self.keep(wanted.to_string(), answer, at_ms),
            }
        }
    }

    fn keep(&mut self, path: String, answer: Result<String, SourceError>, at_ms: u64) {
        self.answers.retain(|(asked, ..)| *asked != path);
        self.answers.push((path, answer, at_ms));
    }

    /// The picture at `path`, as the preview shows it.
    pub fn image(&self, path: &str) -> PreviewImage {
        match self.answers.iter().find(|(asked, ..)| asked == path) {
            None => PreviewImage::Loading,
            Some((_, Ok(url), _)) => PreviewImage::Ready { url: url.clone() },
            Some((_, Err(_), _)) => PreviewImage::Unavailable,
        }
    }
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
