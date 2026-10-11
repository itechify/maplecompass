//! The Item Card's data (ADR 0010): OSMS's item files, held apart from the map set
//! (fetched on its schedule, but read, saved and swapped on their own, so a change in them
//! takes down only the Item Card); and BasilMarket's price list, asked for only as cards
//! open.

use crate::basilmarket::{self, Prices};
use crate::item_card::Market;
use crate::item_lookup::ItemLookup;
use crate::map_index::{MapDataSource, SourceError};
use crate::osms_items::{self, ItemSet};
use crate::{SAVED_COPY_AFTER_MS, SET_RECHECK_MS, SET_RETRY_MS};
use std::task::Poll;

/// OSMS's item files as read, and when they were saved (ms since the Unix epoch): by an
/// earlier launch, or as they were fetched in this one (the wall clock then); `None` if the
/// wall clock wasn't known.
struct Held {
    items: ItemSet,
    saved_at_unix_ms: Option<u64>,
}

/// OSMS's item files: the ones held, and when to ask for them next.
#[derive(Default)]
pub(crate) struct ItemFilesLoader {
    held: Option<Held>,
    /// When to ask for them next (capture time); 0 at startup.
    ask_at: u64,
    /// The source is fetching them.
    in_flight: bool,
    /// The files an earlier launch saved have been looked at (on the first tick).
    saved_seen: bool,
    /// The files an earlier launch saved, read, while fresh ones are fetched at startup
    /// because they are too old to use as they are: what the card falls back on if none can
    /// be had.
    fallback: Option<Held>,
    /// Why none are held, once asking for them failed.
    failed: Option<SourceError>,
}

impl ItemFilesLoader {
    /// At startup (the first tick), the files an earlier launch saved, if they were saved
    /// under a day ago; otherwise, and every day after, ones the source fetches afresh, and
    /// with none at all, a minute after OSMS couldn't be reached. Fetched files that read
    /// replace the ones held whole. Ones that can't be had or don't read leave the ones held
    /// as they are, and at startup fall back on the saved ones, however old.
    pub fn load(&mut self, source: &mut dyn MapDataSource, at_ms: u64, wall_clock: Option<u64>) {
        if !self.in_flight {
            if at_ms < self.ask_at {
                return;
            }
            if !self.saved_seen {
                self.saved_seen = true;
                if self.take_saved(source, wall_clock) {
                    self.ask_at = at_ms + SET_RECHECK_MS;
                    return;
                }
            }
        }
        let fetched = match source.fetch_item_files() {
            Poll::Pending => {
                self.in_flight = true;
                return;
            }
            Poll::Ready(fetched) => fetched,
        };
        self.in_flight = false;
        self.ask_at = at_ms + SET_RECHECK_MS;
        match (fetched.and_then(|files| osms_items::read(&files)), self.fallback.take()) {
            // The source saved them as it fetched them: now.
            (Ok(items), _) => self.held = Some(Held { items, saved_at_unix_ms: wall_clock }),
            (Err(_), _) if self.held.is_some() => {}
            (Err(_), Some(saved)) => self.held = Some(saved),
            (Err(failed), None) => {
                if matches!(failed, SourceError::Unreachable(_)) {
                    self.ask_at = at_ms + SET_RETRY_MS;
                }
                self.failed = Some(failed);
            }
        }
    }

    /// Take the files an earlier launch saved if they were saved under a day ago by the
    /// wall clock, or else keep them to fall back on; whether they were taken. Without the
    /// wall clock their age is unknown, so they aren't taken as they are, and files that
    /// don't read are as good as none.
    fn take_saved(&mut self, source: &mut dyn MapDataSource, wall_clock: Option<u64>) -> bool {
        let Some(saved) = source.saved_item_files() else { return false };
        let Ok(items) = osms_items::read(&saved.files) else { return false };
        let held = Held { items, saved_at_unix_ms: Some(saved.saved_at_unix_ms) };
        let age = wall_clock.and_then(|now| now.checked_sub(saved.saved_at_unix_ms));
        if age.is_some_and(|age| age < SET_RECHECK_MS) {
            self.held = Some(held);
            return true;
        }
        self.fallback = Some(held);
        false
    }

    /// The items held, or what the alert says for want of them.
    pub fn items(&self) -> Result<&ItemSet, ItemLookup> {
        match (&self.held, &self.failed) {
            (Some(held), _) => Ok(&held.items),
            (None, None) => Err(ItemLookup::ItemsLoading),
            (None, Some(SourceError::Unreachable(_))) => Err(ItemLookup::ItemsUnavailable),
            (None, Some(SourceError::FormatChanged(_))) => Err(ItemLookup::ItemsFormatChanged),
        }
    }

    /// When the files held were saved, if that was over 7 days ago by the wall clock `now`,
    /// so they are a Saved Copy: none could be had since.
    pub fn saved_copy_from(&self, now: Option<u64>) -> Option<u64> {
        let saved_at = self.held.as_ref()?.saved_at_unix_ms?;
        (now?.saturating_sub(saved_at) > SAVED_COPY_AFTER_MS).then_some(saved_at)
    }
}

/// How old a price list may be before an Item Card opening asks for a fresh one: prices
/// change as players trade, but a few hours make little difference.
const PRICES_RECHECK_MS: u64 = 3 * 60 * 60 * 1000;

/// After BasilMarket couldn't be reached, how long before a card opening asks again.
const PRICES_RETRY_MS: u64 = 60_000;

/// A price list over a day old says when it is from.
const PRICES_DATED_AFTER_MS: u64 = 24 * 60 * 60 * 1000;

/// BasilMarket's price list as read, and when it was saved (ms since the Unix epoch), or
/// fetched in this run (capture time) if the wall clock wasn't known.
struct HeldPrices {
    prices: Prices,
    saved_at_unix_ms: Option<u64>,
    fetched_at_ms: Option<u64>,
}

/// BasilMarket's price list (ADR 0010): asked for only when an Item Card opens and the list
/// held is over 3 hours old.
#[derive(Default)]
pub(crate) struct PriceList {
    held: Option<HeldPrices>,
    /// The list an earlier launch saved has been looked at (when the first card opened).
    saved_seen: bool,
    in_flight: bool,
    /// After an ask that failed, when the next may go (capture time).
    retry_at: Option<u64>,
    /// The last ask failed with no list held.
    failed: bool,
}

impl PriceList {
    /// An Item Card opened: the list an earlier launch saved, the first time, and a fresh
    /// one if the list held is over 3 hours old, but no sooner than a minute after the last
    /// ask.
    pub fn card_opened(&mut self, source: &mut dyn MapDataSource, at_ms: u64, wall_clock: Option<u64>) {
        if !std::mem::replace(&mut self.saved_seen, true) {
            if let Some(saved) = source.saved_prices() {
                if let Ok(prices) = basilmarket::read(&saved.text) {
                    self.held = Some(HeldPrices { prices, saved_at_unix_ms: Some(saved.saved_at_unix_ms), fetched_at_ms: None });
                }
            }
        }
        let stale = self.held.as_ref().is_none_or(|held| match (wall_clock, held.saved_at_unix_ms, held.fetched_at_ms) {
            (Some(now), Some(saved_at), _) => now.saturating_sub(saved_at) > PRICES_RECHECK_MS,
            (_, _, Some(fetched_at)) => at_ms.saturating_sub(fetched_at) > PRICES_RECHECK_MS,
            _ => true,
        });
        let waited = self.retry_at.is_none_or(|retry_at| at_ms >= retry_at);
        if stale && waited && !self.in_flight {
            self.in_flight = true;
            self.poll(source, at_ms, wall_clock);
        }
    }

    /// Time passed: collect the list being fetched, once it is in. One that reads replaces
    /// the list held; one that can't be had or doesn't read leaves it as it is.
    pub fn poll(&mut self, source: &mut dyn MapDataSource, at_ms: u64, wall_clock: Option<u64>) {
        if !self.in_flight {
            return;
        }
        let Poll::Ready(fetched) = source.fetch_prices() else { return };
        self.in_flight = false;
        match fetched.and_then(|text| basilmarket::read(&text)) {
            Ok(prices) => {
                self.held = Some(HeldPrices { prices, saved_at_unix_ms: wall_clock, fetched_at_ms: Some(at_ms) });
                self.failed = false;
                self.retry_at = None;
            }
            Err(_) => {
                self.failed = self.held.is_none();
                self.retry_at = Some(at_ms + PRICES_RETRY_MS);
            }
        }
    }

    /// What the market line says of the item with these IDs (sharing a name and a card):
    /// the first BasilMarket lists. Prices over a day old by the wall clock `now` say when
    /// they are from.
    pub fn market(&self, ids: &[u32], now: Option<u64>) -> Market {
        let Some(held) = &self.held else { return if self.failed { Market::Unavailable } else { Market::Loading } };
        let from_unix_ms = held.saved_at_unix_ms.filter(|&saved_at| {
            now.is_some_and(|now| now.saturating_sub(saved_at) > PRICES_DATED_AFTER_MS)
        });
        let Some(price) = ids.iter().find_map(|&id| held.prices.item(id)) else {
            return Market::NoSales { page: None, from_unix_ms };
        };
        match (price.sold_for, &price.page) {
            (Some(mesos), Some(page)) => Market::Sold { mesos, sales: price.sales, page: page.clone(), from_unix_ms },
            _ => Market::NoSales { page: price.page.clone(), from_unix_ms },
        }
    }
}
