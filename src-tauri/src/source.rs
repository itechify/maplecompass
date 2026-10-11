//! The shell's data source: what the core asks for, fetched over HTTPS with an honest user
//! agent ([`crate::http`]) and kept in a disk cache. That is OSMS Data Explorer's file set
//! and pictures (ADR 0008, in [`crate::osms`] and [`crate::pictures`]), MeowDB's search
//! (ADR 0007, in [`crate::meowdb`]), and the Item Card's OSMS item files and BasilMarket
//! price list (ADR 0010, in [`crate::osms_items`] and [`crate::basilmarket`]). Reading them
//! is left to the core.
//!
//! Fetches run on the async runtime and never block the caller: the first call starts
//! one and answers `Pending`, later calls collect it.

use crate::basilmarket::PriceList;
use crate::meowdb::ItemSearch;
use crate::osms::OsmsSet;
use crate::osms_items::OsmsItems;
use crate::pictures::Pictures;
use compass_core::basilmarket::SavedPrices;
use compass_core::osms::{OsmsFiles, SavedFiles};
use compass_core::osms_items::{ItemFiles, SavedItemFiles};
use compass_core::{Fetched, ItemHit, MapDataSource, SourceError};
use std::path::PathBuf;
use std::task::Poll;

/// The data source the app's core is given, fetching from OSMS Data Explorer, MeowDB and
/// BasilMarket.
pub struct WebSource {
    /// OSMS's file set.
    osms: OsmsSet,
    /// OSMS's item files.
    items: OsmsItems,
    /// OSMS's pictures.
    pictures: Pictures,
    /// MeowDB's search.
    item_search: ItemSearch,
    /// BasilMarket's price list.
    prices: PriceList,
}

impl WebSource {
    /// A source caching under `cache_dir` (created when first written).
    pub fn new(cache_dir: PathBuf) -> reqwest::Result<Self> {
        let client = crate::http::client()?;
        Ok(Self {
            osms: OsmsSet::new(client.clone(), &cache_dir),
            items: OsmsItems::new(client.clone(), &cache_dir),
            pictures: Pictures::new(client.clone(), &cache_dir),
            item_search: ItemSearch::new(client.clone(), &cache_dir),
            prices: PriceList::new(client, &cache_dir),
        })
    }
}

impl MapDataSource for WebSource {
    fn saved_osms_files(&mut self) -> Option<SavedFiles> {
        self.osms.saved()
    }

    fn fetch_osms_files(&mut self) -> Poll<Result<OsmsFiles, SourceError>> {
        self.osms.fetch()
    }

    fn picture(&mut self, path: &str) -> Poll<Result<String, SourceError>> {
        self.pictures.poll(path)
    }

    fn item_search(&mut self, name: &str) -> Poll<Result<Fetched<Vec<ItemHit>>, SourceError>> {
        self.item_search.poll(name)
    }

    fn saved_item_files(&mut self) -> Option<SavedItemFiles> {
        self.items.saved()
    }

    fn fetch_item_files(&mut self) -> Poll<Result<ItemFiles, SourceError>> {
        self.items.fetch()
    }

    fn saved_prices(&mut self) -> Option<SavedPrices> {
        self.prices.saved()
    }

    fn fetch_prices(&mut self) -> Poll<Result<String, SourceError>> {
        self.prices.fetch()
    }
}
