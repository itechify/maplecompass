//! The world crawl (ADR 0004, until #57): every map in OSMS's map list has its MeowDB
//! page asked for in the background, one at a time and politely paced, so mob search
//! knows the whole world. It has its own requests, and never touches the shown map's.
//!
//! What it learns is kept between launches ([`SavedWorld`]), each map with the date its
//! data is from. A map over [`STALE_AFTER_MS`] old still counts as known, so search
//! works from the start; it is asked for again in the background, oldest first and no
//! more than one every [`REFRESH_EVERY_MS`], so the maps' dates spread out rather than
//! all falling due on one day. Maps never seen come first, at MeowDB's usual pace.

use crate::map_data::MapData;
use crate::map_index::{Fetched, MapDataSource, MapEntry, SourceError};
use crate::pace::Pace;
use crate::saved_world::{SavedMap, SavedWorld};
use serde::Serialize;
use std::task::Poll;

/// How long an unreachable page waits before it is asked for again.
const RETRY_MS: u64 = 5 * 60_000;

/// How many answers given at once (from the source's cache) are taken in one tick: a
/// warm cache must not hold up recognition while hundreds of pages are read.
const AT_ONCE_PER_TICK: usize = 8;

/// A map's data older than this is asked for again (ADR 0004: a week).
pub(crate) const STALE_AFTER_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// The least time between two refreshes of maps already known: one every 30 seconds,
/// all 422 in about three and a half hours of play.
pub(crate) const REFRESH_EVERY_MS: u64 = 30_000;

/// How far the crawl has got.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlProgress {
    /// Maps whose page has been read, fresh, from a saved copy, or at an earlier launch.
    pub known: u32,
    /// Maps in the map list; 0 until it arrives.
    pub total: u32,
    /// Every map has been asked for and answered for good: no map never seen is still
    /// to ask for, in flight, or waiting to be retried. (Pages in a changed format are
    /// left out, so `known` may stay below `total`; refreshes of known maps don't count.)
    pub done: bool,
}

/// Where the crawl is with one map.
enum Page {
    Unasked,
    Known {
        data: Box<MapData>,
        /// When MeowDB served it (ms since the Unix epoch).
        as_of_unix_ms: u64,
        /// A refresh failed: not asked for again before this (capture time).
        refresh_after_ms: u64,
    },
    /// Its page is in a changed format: left out, and not asked for again.
    Skipped,
    /// MeowDB couldn't be reached; asked for again once `retry_at_ms` has come.
    Unreachable { retry_at_ms: u64 },
}

/// The crawl over every map in the index.
pub(crate) struct Crawl {
    /// Each map in the index, in index order.
    maps: Vec<(MapEntry, Page)>,
    /// The map whose answer is awaited, and whether it is known already (a refresh).
    in_flight: Option<(usize, bool)>,
    /// When the last refresh was asked for (capture time).
    refreshed_at_ms: Option<u64>,
    /// Bumped whenever what it knows changes, so the shell saves the world only then.
    revision: u64,
}

impl Crawl {
    /// A crawl over these maps' pages, knowing what `saved` knows of them.
    pub fn new(index: Vec<MapEntry>, saved: &SavedWorld) -> Self {
        let mut crawl = Self {
            maps: index.into_iter().map(|entry| (entry, Page::Unasked)).collect(),
            in_flight: None,
            refreshed_at_ms: None,
            revision: 0,
        };
        crawl.restore(saved);
        crawl
    }

    /// Take what `saved` knows of the index's maps, where it is newer than what the
    /// crawl knows.
    pub fn restore(&mut self, saved: &SavedWorld) {
        for map in &saved.maps {
            let Some((_, page)) = self.maps.iter_mut().find(|(entry, _)| entry.id == map.id) else { continue };
            let newer = match page {
                Page::Known { as_of_unix_ms, .. } => map.as_of_unix_ms > *as_of_unix_ms,
                Page::Unasked | Page::Unreachable { .. } => true,
                Page::Skipped => false,
            };
            if newer {
                *page = Page::Known {
                    data: Box::new(map.data.clone().into()),
                    as_of_unix_ms: map.as_of_unix_ms,
                    refresh_after_ms: 0,
                };
                self.revision += 1;
            }
        }
    }

    /// Every map known, as the shell saves it.
    pub fn saved(&self) -> SavedWorld {
        let maps = self
            .maps
            .iter()
            .filter_map(|(entry, page)| match page {
                Page::Known { data, as_of_unix_ms, .. } => Some(SavedMap {
                    id: entry.id.clone(),
                    as_of_unix_ms: *as_of_unix_ms,
                    data: (&**data).into(),
                }),
                _ => None,
            })
            .collect();
        SavedWorld { maps, ..SavedWorld::default() }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Time passed: check on the request in flight, and ask for the next page when
    /// MeowDB's pace allows (about one page every 2 seconds; an answer the source gives
    /// at once doesn't wait). `now_unix_ms` is the wall clock, if the shell has told it.
    pub fn poll(&mut self, source: &mut dyn MapDataSource, meowdb: &mut Pace, at_ms: u64, now_unix_ms: Option<u64>) {
        for _ in 0..AT_ONCE_PER_TICK {
            let ((at, refresh), first_ask) = match self.in_flight {
                Some(asked) => (asked, false),
                None if !meowdb.ready(at_ms) => return,
                None => match self.next(at_ms, now_unix_ms) {
                    Some(next) => (next, true),
                    None => return,
                },
            };
            if first_ask && refresh {
                self.refreshed_at_ms = Some(at_ms);
            }
            let answer = match source.crawl_data(&self.maps[at].0.id) {
                Poll::Pending => {
                    if first_ask {
                        meowdb.sent(at_ms);
                    }
                    self.in_flight = Some((at, refresh));
                    return;
                }
                Poll::Ready(answer) => answer,
            };
            self.in_flight = None;
            self.answered(at, answer, at_ms, now_unix_ms);
            if !first_ask {
                // It came over the network: the next one waits out the pace.
                return;
            }
        }
    }

    /// Map `at`'s page came back as `answer`.
    fn answered(&mut self, at: usize, answer: Result<Fetched<MapData>, SourceError>, at_ms: u64, now: Option<u64>) {
        let page = &mut self.maps[at].1;
        let (data, as_of_unix_ms) = match answer {
            Ok(Fetched::Fresh(data)) => (data, now.unwrap_or_default()),
            Ok(Fetched::SavedCopy { data, saved_at_unix_ms }) => (data, saved_at_unix_ms),
            Err(error) => {
                match page {
                    // A refresh that failed: what is known stays, and waits to be asked
                    // for again (never, if its page changed format).
                    Page::Known { refresh_after_ms, .. } => {
                        *refresh_after_ms = match error {
                            SourceError::FormatChanged(_) => u64::MAX,
                            SourceError::Unreachable(_) => at_ms + RETRY_MS,
                        };
                    }
                    _ => {
                        *page = match error {
                            SourceError::FormatChanged(_) => Page::Skipped,
                            SourceError::Unreachable(_) => Page::Unreachable { retry_at_ms: at_ms + RETRY_MS },
                        };
                    }
                }
                return;
            }
        };
        if let Page::Known { as_of_unix_ms: known_as_of, refresh_after_ms, .. } = page
            && *known_as_of >= as_of_unix_ms
        {
            // MeowDB couldn't be reached, and the saved copy is no newer.
            *refresh_after_ms = at_ms + RETRY_MS;
            return;
        }
        *page = Page::Known { data: Box::new(data), as_of_unix_ms, refresh_after_ms: 0 };
        self.revision += 1;
    }

    /// The next map to ask for, and whether it is a refresh: the first never asked,
    /// else the unreachable one whose retry has waited longest, else (once
    /// [`REFRESH_EVERY_MS`] has passed since the last) the known map whose data is
    /// oldest, if it is stale.
    fn next(&self, at_ms: u64, now_unix_ms: Option<u64>) -> Option<(usize, bool)> {
        let unasked = self.maps.iter().position(|(_, page)| matches!(page, Page::Unasked));
        let retry = || {
            self.maps
                .iter()
                .enumerate()
                .filter_map(|(at, (_, page))| match page {
                    Page::Unreachable { retry_at_ms } if *retry_at_ms <= at_ms => Some((*retry_at_ms, at)),
                    _ => None,
                })
                .min()
                .map(|(_, at)| at)
        };
        let refresh = || {
            let now = now_unix_ms?;
            if self.refreshed_at_ms.is_some_and(|t| at_ms.saturating_sub(t) < REFRESH_EVERY_MS) {
                return None;
            }
            self.maps
                .iter()
                .enumerate()
                .filter_map(|(at, (_, page))| match page {
                    Page::Known { as_of_unix_ms, refresh_after_ms, .. }
                        if now.saturating_sub(*as_of_unix_ms) >= STALE_AFTER_MS && *refresh_after_ms <= at_ms =>
                    {
                        Some((*as_of_unix_ms, at))
                    }
                    _ => None,
                })
                .min()
                .map(|(_, at)| at)
        };
        unasked.or_else(retry).map(|at| (at, false)).or_else(|| refresh().map(|at| (at, true)))
    }

    /// The maps whose page has been read so far, with their index entry.
    pub fn known(&self) -> impl Iterator<Item = (&MapEntry, &MapData)> {
        self.maps.iter().filter_map(|(entry, page)| match page {
            Page::Known { data, .. } => Some((entry, &**data)),
            _ => None,
        })
    }

    /// Every map in the index, whether or not its page has been read.
    pub fn entries(&self) -> impl Iterator<Item = &MapEntry> {
        self.maps.iter().map(|(entry, _)| entry)
    }

    /// Map `id`'s index entry, whether or not its page has been read.
    pub fn entry(&self, id: &str) -> Option<&MapEntry> {
        self.entries().find(|entry| entry.id == id)
    }

    pub fn progress(&self) -> CrawlProgress {
        let settled = |page: &Page| matches!(page, Page::Known { .. } | Page::Skipped);
        let mapping = self.in_flight.is_some_and(|(_, refresh)| !refresh);
        CrawlProgress {
            known: self.known().count() as u32,
            total: self.maps.len() as u32,
            done: !mapping && self.maps.iter().all(|(_, page)| settled(page)),
        }
    }
}
