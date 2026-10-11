//! BasilMarket's price list (ADR 0010), for the Item Card's market line: the list its own
//! "What's it worth?" search downloads, [`compass_core::basilmarket::PRICE_INDEX_URL`],
//! fetched with MapleCompass's own user agent. Nothing from BasilMarket is bundled.
//!
//! The core asks for it only when an Item Card opens and the list it holds is over 3 hours
//! old. The list is saved on disk as BasilMarket served it, once the core's reader reads
//! it, and the file's modification time says when it was saved. If it can't be fetched,
//! or doesn't read, nothing is saved, and the core keeps the list it has.

use crate::http;
use compass_core::SourceError;
use compass_core::basilmarket::{self, SavedPrices};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::task::Poll;
use std::time::UNIX_EPOCH;

/// The saved list's file in the cache.
const PRICES_FILE: &str = "basilmarket-prices.json";

type Answer = Result<String, SourceError>;

/// BasilMarket's price list: saved, and fetched when the core asks.
pub struct PriceList {
    client: reqwest::Client,
    file: PathBuf,
    /// The list being fetched in the background.
    in_flight: Option<Receiver<Answer>>,
}

impl PriceList {
    /// A list saved under `cache_dir`, fetched with `client`.
    pub fn new(client: reqwest::Client, cache_dir: &Path) -> Self {
        Self { client, file: cache_dir.join(PRICES_FILE), in_flight: None }
    }

    /// The list saved, and when; `None` if none is.
    pub fn saved(&self) -> Option<SavedPrices> {
        let text = std::fs::read_to_string(&self.file).ok()?;
        let saved_at = std::fs::metadata(&self.file).and_then(|m| m.modified()).ok()?;
        let since_epoch = saved_at.duration_since(UNIX_EPOCH).unwrap_or_default();
        Some(SavedPrices { text, saved_at_unix_ms: since_epoch.as_millis() as u64 })
    }

    /// A list fetched afresh, in the background: the first call starts fetching it, later
    /// ones collect it.
    pub fn fetch(&mut self) -> Poll<Answer> {
        if let Some(arriving) = &self.in_flight {
            let Poll::Ready(answer) = http::collect(arriving) else { return Poll::Pending };
            self.in_flight = None;
            if let Err(e) = &answer {
                eprintln!("BasilMarket's price list: {e}");
            }
            return Poll::Ready(answer);
        }
        let (client, file) = (self.client.clone(), self.file.clone());
        self.in_flight = Some(http::in_background(async move { fetch_and_save(&client, &file).await }));
        Poll::Pending
    }
}

/// Fetch BasilMarket's price list, and save it at `file` once it reads.
async fn fetch_and_save(client: &reqwest::Client, file: &Path) -> Answer {
    let text = http::text(client, basilmarket::PRICE_INDEX_URL).await.map_err(SourceError::Unreachable)?;
    basilmarket::read(&text)?;
    if let Err(e) = crate::files::write_atomically(file, text.as_bytes()) {
        eprintln!("BasilMarket: couldn't save {}: {e}", file.display());
    }
    Ok(text)
}

/// Fetch BasilMarket's live price list now, whatever is saved, for the probe.
pub fn fetch_live() -> Result<String, String> {
    let client = http::client().map_err(|e| e.to_string())?;
    tauri::async_runtime::block_on(http::text(&client, basilmarket::PRICE_INDEX_URL))
}
