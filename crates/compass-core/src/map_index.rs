//! The map list's entries, and the injected data source the core's data comes from: OSMS
//! Data Explorer's file set (ADR 0008), and MeowDB's pages and search (ADR 0003).

use crate::item_lookup::ItemHit;
use crate::mob_card::MobDetails;
use crate::osms::OsmsFiles;
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

/// Where the core gets OSMS's file set, MeowDB's pages and search, and maplestory.io's
/// mob images. The shell supplies one that fetches over HTTP with a disk cache; tests
/// supply fakes.
///
/// Every ask may take a while over the network, and the core never waits for
/// them: on the first call start the fetch and answer [`Poll::Pending`]; the core asks
/// again (each tick and frame) until the answer is ready. A source should answer at
/// once when it can (from a cache, say).
pub trait MapDataSource {
    /// OSMS Data Explorer's file set (ADR 0008), unread: the core reads it. The core asks
    /// at startup and every 24 hours after, and a minute after asking found nothing at
    /// all. Fetch the set when the one saved is over a day old (or there is none), and
    /// answer it fresh once it reads with [`crate::osms::read`], saving it whole. Answer the
    /// saved set fresh, at once, while it is younger; and as a saved copy, with when it
    /// was saved, when a file can't be fetched or the set fetched doesn't read.
    fn osms_files(&mut self) -> Poll<Result<Fetched<OsmsFiles>, SourceError>>;

    /// One mob's figures from its MeowDB page, by MeowDB's mob ID (ADR 0006): the core
    /// asks for each mob the shown map lists, one at a time. A cached copy up to 7 days
    /// old may be used as it is; answer at once when it is.
    fn mob_page(&mut self, id: &str) -> Poll<Result<Fetched<MobDetails>, SourceError>>;

    /// Where the overlay can load one mob's image from, by mob ID: the shell's copy of
    /// maplestory.io's (ADR 0006), asked for like [`MapDataSource::mob_page`] but kept
    /// apart from it, since it comes from another site. A cached image up to 30 days old
    /// may be used as it is. [`SourceError::FormatChanged`] when maplestory.io has none.
    fn mob_image(&mut self, id: &str) -> Poll<Result<String, SourceError>>;

    /// The items MeowDB's search ([`crate::meowdb::ITEM_SEARCH_URL`]) gives for `name`, an
    /// item's name as read from an Item Tooltip, closest first (ADR 0007): the core asks
    /// once per Item Lookup, and keeps to MeowDB's pace after it. Read with [`crate::meowdb::read_item_search`]. A cached answer may be used as
    /// it is; answer at once when it is.
    fn item_search(&mut self, name: &str) -> Poll<Result<Fetched<Vec<ItemHit>>, SourceError>>;
}
