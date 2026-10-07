//! The world crawl (ADR 0004): every map in the index has its MeowDB page asked for in
//! the background, one at a time and politely paced, so mob search knows the whole
//! world. It has its own requests, and never touches the shown map's.

use crate::map_data::MapData;
use crate::map_index::{Fetched, MapDataSource, SourceError};
use serde::Serialize;
use std::task::Poll;

/// The least time between two crawl requests that went to MeowDB: about one page every
/// 2 seconds. An answer the source gives at once (from its cache) doesn't wait.
const PACE_MS: u64 = 2_000;

/// How long an unreachable page waits before it is asked for again.
const RETRY_MS: u64 = 5 * 60_000;

/// How many answers given at once (from the source's cache) are taken in one tick: a
/// warm cache must not hold up recognition while hundreds of pages are read.
const AT_ONCE_PER_TICK: usize = 8;

/// How far the crawl has got.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlProgress {
    /// Maps whose page has been read, fresh or from a saved copy.
    pub known: u32,
    /// Maps in the map index; 0 until it arrives.
    pub total: u32,
    /// Every map has been asked for and answered for good: no page is still to ask
    /// for, in flight, or waiting to be retried. (Pages in a changed format are left
    /// out, so `known` may stay below `total`.)
    pub done: bool,
}

/// Where the crawl is with one map.
enum Page {
    Unasked,
    Known(Box<MapData>),
    /// Its page is in a changed format: left out, and not asked for again.
    Skipped,
    /// MeowDB couldn't be reached; asked for again once `retry_at_ms` has come.
    Unreachable { retry_at_ms: u64 },
}

/// The crawl over every map in the index.
pub(crate) struct Crawl {
    /// Each map in the index, in index order.
    maps: Vec<(String, Page)>,
    /// The map whose answer is awaited.
    in_flight: Option<usize>,
    /// When the last request that went to MeowDB (didn't answer at once) started.
    last_sent_ms: Option<u64>,
}

impl Crawl {
    /// A crawl over these maps' pages, none asked for yet.
    pub fn new(ids: impl IntoIterator<Item = String>) -> Self {
        Self {
            maps: ids.into_iter().map(|id| (id, Page::Unasked)).collect(),
            in_flight: None,
            last_sent_ms: None,
        }
    }

    /// Time passed: check on the request in flight, and ask for the next page when
    /// the pace allows.
    pub fn poll(&mut self, source: &mut dyn MapDataSource, at_ms: u64) {
        for _ in 0..AT_ONCE_PER_TICK {
            let (at, first_ask) = match self.in_flight {
                Some(at) => (at, false),
                None if self.last_sent_ms.is_some_and(|t| at_ms.saturating_sub(t) < PACE_MS) => return,
                None => match self.next(at_ms) {
                    Some(at) => (at, true),
                    None => return,
                },
            };
            let answer = match source.crawl_data(&self.maps[at].0) {
                Poll::Pending => {
                    if first_ask {
                        self.last_sent_ms = Some(at_ms);
                    }
                    self.in_flight = Some(at);
                    return;
                }
                Poll::Ready(answer) => answer,
            };
            self.in_flight = None;
            self.maps[at].1 = match answer {
                Ok(Fetched::Fresh(data) | Fetched::SavedCopy { data, .. }) => Page::Known(Box::new(data)),
                Err(SourceError::FormatChanged(_)) => Page::Skipped,
                Err(SourceError::Unreachable(_)) => Page::Unreachable { retry_at_ms: at_ms + RETRY_MS },
            };
            if !first_ask {
                // It came over the network: the next one waits out the pace.
                return;
            }
        }
    }

    /// The next map to ask for: the first never asked, else the unreachable one whose
    /// retry has waited longest.
    fn next(&self, at_ms: u64) -> Option<usize> {
        let unasked = self.maps.iter().position(|(_, page)| matches!(page, Page::Unasked));
        unasked.or_else(|| {
            self.maps
                .iter()
                .enumerate()
                .filter_map(|(at, (_, page))| match page {
                    Page::Unreachable { retry_at_ms } if *retry_at_ms <= at_ms => Some((*retry_at_ms, at)),
                    _ => None,
                })
                .min()
                .map(|(_, at)| at)
        })
    }

    /// The maps whose page has been read so far, by ID.
    pub fn known(&self) -> impl Iterator<Item = (&str, &MapData)> {
        self.maps.iter().filter_map(|(id, page)| match page {
            Page::Known(data) => Some((id.as_str(), &**data)),
            _ => None,
        })
    }

    pub fn progress(&self) -> CrawlProgress {
        let settled = |page: &Page| matches!(page, Page::Known(_) | Page::Skipped);
        CrawlProgress {
            known: self.known().count() as u32,
            total: self.maps.len() as u32,
            done: self.in_flight.is_none() && self.maps.iter().all(|(_, page)| settled(page)),
        }
    }
}
