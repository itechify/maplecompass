//! The map list's entries, and the injected data source the core's data comes from: OSMS
//! Data Explorer's file set and pictures (ADR 0008), and MeowDB's search (ADR 0007).

use crate::item_lookup::ItemHit;
use crate::osms::{OsmsFiles, SavedFiles};
use std::task::Poll;

/// One map in the map list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEntry {
    /// The 9-digit map ID, zero padding kept (e.g. `"010001010"`): the game's, as MeowDB's
    /// pages are numbered.
    pub id: String,
    pub street_name: String,
    pub map_name: String,
    /// The region as OSMS gives it: "Maple Island", "Victoria Island", "Event", "Other" or
    /// "Dev".
    pub region: String,
}

/// The regions of maps the player can't normally enter: event maps, and the developers'
/// map (The Manager's Room).
const LEFT_OUT_REGIONS: [&str; 2] = ["Event", "Dev"];

impl MapEntry {
    /// An Event or Dev map: the player can't normally enter it, so mob search neither
    /// lists it nor routes through it.
    pub(crate) fn is_left_out(&self) -> bool {
        LEFT_OUT_REGIONS.contains(&self.region.as_str())
    }
}

/// Why data could not be had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// The site could not be reached and nothing usable was saved.
    Unreachable(String),
    /// The site answered, but not in the shape the reader expects.
    FormatChanged(String),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(why) => write!(f, "unreachable: {why}"),
            Self::FormatChanged(why) => write!(f, "format changed: {why}"),
        }
    }
}

impl std::error::Error for SourceError {}

/// What a source got: fresh data, or a copy saved earlier because the site couldn't be
/// reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fetched<T> {
    /// Just fetched, or a cached copy young enough to use as it is.
    Fresh(T),
    /// The site couldn't be reached: a copy saved at `saved_at_unix_ms` (ms since the
    /// Unix epoch).
    SavedCopy { data: T, saved_at_unix_ms: u64 },
}

/// Where the core gets OSMS's file set and pictures, and MeowDB's search. The shell
/// supplies one that fetches over HTTP with a disk cache; tests supply fakes.
///
/// Every ask may take a while over the network, and the core never waits for
/// them: on the first call start the fetch and answer [`Poll::Pending`]; the core asks
/// again (each tick and frame) until the answer is ready. A source should answer at
/// once when it can (from a cache, say).
pub trait MapDataSource {
    /// OSMS Data Explorer's file set as it was last saved (ADR 0008), unread, and when;
    /// `None` when none is, or it can't be read back. The core asks once, at startup, and
    /// decides whether to fetch it afresh: it uses a set saved under a day ago as it is, and
    /// falls back on an older one when a fresh one can't be had. It is on disk, so answer
    /// at once.
    fn saved_osms_files(&mut self) -> Option<SavedFiles>;

    /// OSMS Data Explorer's file set fetched afresh (ADR 0008), unread: the core reads it.
    /// The core asks at startup unless the saved set is under a day old, every 24 hours
    /// after, and a minute after asking found nothing at all. Save the set fetched whole,
    /// in place of the one saved, once it reads with [`crate::osms::read`]:
    /// [`SourceError::FormatChanged`] when it doesn't, and [`SourceError::Unreachable`]
    /// when a file can't be fetched. Either way the core keeps the set it has.
    fn fetch_osms_files(&mut self) -> Poll<Result<OsmsFiles, SourceError>>;

    /// Where the overlay can load one of OSMS's pictures from (ADR 0008): a mob's
    /// animation or a map's picture, by its `path` below
    /// [`crate::osms::PICTURES_URL`], such as `images/monsters/<hash>.webp` or
    /// `maps/<hash>.webp`. The core asks for the pictures the panel wants one at a time,
    /// with no other pace. Fetch it to keep on disk, and serve it to the overlay from
    /// there through MapleCompass's own URL scheme. A path names a picture by its
    /// content, so a saved one never goes stale: answer it at once.
    /// [`SourceError::FormatChanged`] when OSMS has no such picture.
    fn picture(&mut self, path: &str) -> Poll<Result<String, SourceError>>;

    /// The items MeowDB's search ([`crate::meowdb::ITEM_SEARCH_URL`]) gives for `name`, an
    /// item's name as read from an Item Tooltip, closest first (ADR 0007): the core asks
    /// once per Item Lookup, and keeps to MeowDB's pace after it. Read with
    /// [`crate::meowdb::read_item_search`]. A cached answer may be used as it is; answer at
    /// once when it is.
    fn item_search(&mut self, name: &str) -> Poll<Result<Fetched<Vec<ItemHit>>, SourceError>>;
}
