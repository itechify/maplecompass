//! The map index and the injected map data source it comes from (MeowDB, ADR 0003).

use crate::map_data::MapData;
use std::task::Poll;

/// One map in MeowDB's map index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEntry {
    /// MeowDB's 9-digit map ID, zero padding kept (e.g. `"010001010"`).
    pub id: String,
    pub street_name: String,
    pub map_name: String,
    /// MeowDB's display region, e.g. "Victoria Island" (includes pseudo-regions such as
    /// "Hidden Street" and "Event").
    pub region: String,
}

/// Why map data could not be had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// MeowDB could not be reached and nothing usable was cached.
    Unreachable(String),
    /// MeowDB answered, but its page no longer looks the way the reader expects.
    FormatChanged(String),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(why) => write!(f, "MeowDB unreachable: {why}"),
            Self::FormatChanged(why) => write!(f, "MeowDB format changed: {why}"),
        }
    }
}

impl std::error::Error for SourceError {}

/// Where the core gets MeowDB's data. The shell supplies one that fetches over HTTP
/// with a disk cache; tests supply fakes.
pub trait MapDataSource {
    /// Every map's ID, street name, map name and region. The core asks once at
    /// startup, and again only if asking failed.
    fn map_index(&mut self) -> Result<Vec<MapEntry>, SourceError>;

    /// One map's data, by its 9-digit ID. The core asks when the panel switches to the
    /// map, then again on each frame while the answer is [`Poll::Pending`], so a slow
    /// fetch never holds up the panel: start it on the first call and report it on a
    /// later one. A source should answer at once when it can (from a cache, say).
    fn map_data(&mut self, id: &str) -> Poll<Result<MapData, SourceError>>;
}
