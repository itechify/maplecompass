//! The market prices the player keeps (ADR 0010): what they found an item sells for between
//! players, typed into its Item Card, for reference. MapleCompass reads no market prices of
//! its own (no source allows it), so the player keeps them, and they are saved between
//! launches as the Personal Bests are.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The shape of [`SavedMarketPrices`] this core writes. Bumped whenever it changes.
pub const MARKET_PRICES_VERSION: u32 = 1;

/// A market price the player set for an item, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPrice {
    pub mesos: u64,
    /// When they set it (ms since the Unix epoch); `None` if the wall clock wasn't known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_at_unix_ms: Option<u64>,
}

/// The market prices the player keeps, by the game's item ID (8 digits): saved between
/// launches. Missing fields (an empty file) take their defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SavedMarketPrices {
    pub version: u32,
    pub items: BTreeMap<String, MarketPrice>,
}

impl Default for SavedMarketPrices {
    fn default() -> Self {
        Self { version: MARKET_PRICES_VERSION, items: BTreeMap::new() }
    }
}

/// The market prices held, and a count of their changes, so the shell saves them only then.
#[derive(Default)]
pub(crate) struct MarketPrices {
    saved: SavedMarketPrices,
    revision: u64,
}

impl MarketPrices {
    pub fn restore(&mut self, saved: SavedMarketPrices) {
        self.saved = saved;
    }

    pub fn saved(&self) -> &SavedMarketPrices {
        &self.saved
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The price the player keeps for item `id` (8 digits), if any.
    pub fn get(&self, id: &str) -> Option<MarketPrice> {
        self.saved.items.get(id).copied()
    }

    /// Keep `price` for item `id`, or forget its price (`None`).
    pub fn set(&mut self, id: &str, price: Option<MarketPrice>) {
        let changed = match price {
            Some(price) => self.saved.items.insert(id.to_string(), price) != Some(price),
            None => self.saved.items.remove(id).is_some(),
        };
        if changed {
            self.revision += 1;
        }
    }
}

/// The most a price may be: more mesos than the game lets a player hold, many times over.
const MOST_MESOS: f64 = 1e13;

/// A price as the player typed it: digits, with commas, spaces or "mesos" as they like, and
/// "k", "m" or "b" for thousands, millions and billions ("450k", "1.2m"). `Ok(None)` for
/// nothing typed: the price is cleared. `Err` for anything else.
pub(crate) fn parse(typed: &str) -> Result<Option<u64>, ()> {
    let mut text: String = typed.to_ascii_lowercase().chars().filter(|c| !c.is_whitespace() && *c != ',' && *c != '_').collect();
    for unit in ["mesos", "meso"] {
        if let Some(number) = text.strip_suffix(unit) {
            text = number.to_string();
            break;
        }
    }
    if text.is_empty() {
        return Ok(None);
    }
    let (number, times) = match text.chars().last() {
        Some('k') => (&text[..text.len() - 1], 1e3),
        Some('m') => (&text[..text.len() - 1], 1e6),
        Some('b') => (&text[..text.len() - 1], 1e9),
        _ => (&text[..], 1.0),
    };
    let well_formed = !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit() || c == '.')
        && number.chars().filter(|&c| c == '.').count() <= 1
        && number.chars().any(|c| c.is_ascii_digit());
    if !well_formed {
        return Err(());
    }
    let mesos = number.parse::<f64>().map_err(|_| ())? * times;
    if !mesos.is_finite() || mesos > MOST_MESOS {
        return Err(());
    }
    Ok(Some(mesos.round() as u64))
}
