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

/// MeowDB's display region for event maps.
const EVENT_REGION: &str = "Event";

impl MapEntry {
    /// An Event map: the player can't normally enter it, so mob search neither lists
    /// it nor routes through it.
    pub(crate) fn is_event(&self) -> bool {
        self.region == EVENT_REGION
    }
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

/// What a source got from MeowDB: fresh data, or a copy saved earlier because MeowDB
/// couldn't be reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fetched<T> {
    /// Just fetched, or a cached copy young enough to use as it is.
    Fresh(T),
    /// MeowDB couldn't be reached: a copy saved at `saved_at_unix_ms` (ms since the
    /// Unix epoch). The panel shows it, saying so and since when.
    SavedCopy { data: T, saved_at_unix_ms: u64 },
}

/// Where the core gets MeowDB's data. The shell supplies one that fetches over HTTP
/// with a disk cache; tests supply fakes.
///
/// Both operations may take a while over the network, and the core never waits for
/// them: on the first call start the fetch and answer [`Poll::Pending`]; the core asks
/// again (each tick and frame) until the answer is ready. A source should answer at
/// once when it can (from a cache, say).
pub trait MapDataSource {
    /// Every map's ID, street name, map name and region. The core asks at startup,
    /// and again only if asking failed.
    fn map_index(&mut self) -> Poll<Result<Fetched<Vec<MapEntry>>, SourceError>>;

    /// One map's data, by its 9-digit ID, for the panel: the core asks once the panel
    /// is sure of the map (or of the title it shares with other maps). A cached copy
    /// may be used as it is for up to a day.
    fn map_data(&mut self, id: &str) -> Poll<Result<Fetched<MapData>, SourceError>>;

    /// One map's data for the world crawl (ADR 0004), which asks for every map in the
    /// index in turn, one at a time. A cached copy up to 7 days old may be used as it
    /// is; answer at once when it is, so the crawl moves on without waiting out its
    /// pace. Kept apart from [`MapDataSource::map_data`]: a crawl request must never
    /// delay or replace the shown map's.
    fn crawl_data(&mut self, id: &str) -> Poll<Result<Fetched<MapData>, SourceError>>;
}
