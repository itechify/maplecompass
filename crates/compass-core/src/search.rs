//! Mob search's part of the panel state. So far: how far the world crawl has got.

use crate::crawl::CrawlProgress;
use serde::Serialize;

/// Mob search, as the overlay renders it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchState {
    /// How many of the index's maps the world crawl knows so far ("312 of 422 maps
    /// known"): search can only find mobs on those.
    pub crawl: CrawlProgress,
}
