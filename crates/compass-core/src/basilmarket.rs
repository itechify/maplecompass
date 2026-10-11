//! BasilMarket's price list (ADR 0010): what players pay each other for items, from the
//! sales players record on BasilMarket.net. The shell fetches the list its own "What's it
//! worth?" search uses, [`PRICE_INDEX_URL`], with MapleCompass's user agent; the core
//! reads it. Nothing from BasilMarket is bundled. Credit "BasilMarket.net".

use crate::map_index::SourceError;
use serde::Deserialize;
use std::collections::HashMap;

/// BasilMarket's price list: every item it knows, with the price it recommends and where
/// that comes from.
pub const PRICE_INDEX_URL: &str = "https://basilmarket.net/api/price-index";

/// BasilMarket's price list as the source saved it, at an earlier launch or earlier in
/// this one, unread, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedPrices {
    pub text: String,
    /// When it was saved (ms since the Unix epoch).
    pub saved_at_unix_ms: u64,
}

/// Where an item's price page is on BasilMarket: its price history and range.
const PRICE_PAGE_URL: &str = "https://basilmarket.net/price/";

/// BasilMarket's price list as read: each item it lists, by the game's item ID.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prices {
    items: HashMap<u32, Price>,
}

/// What BasilMarket says an item sells for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Price {
    /// The price it recommends from players' sales, if it has any; `None` when it has
    /// none, or only the NPC shop's price to show instead.
    pub sold_for: Option<u64>,
    /// How many sales that comes from.
    pub sales: u32,
    /// The item's price page, if its slug makes a safe one.
    pub page: Option<String>,
}

impl Prices {
    /// What BasilMarket says item `id` sells for, if it lists it.
    pub(crate) fn item(&self, id: u32) -> Option<&Price> {
        self.items.get(&id)
    }
}

/// Read BasilMarket's price list. Each item is listed by name, with the game's item ID in
/// its picture's name (`01302006.png`), which is how it is matched: an item sharing a name
/// never takes another's price. One without an ID there can't be matched, and is left
/// out. Only sales set a price ("sales" on BasilMarket, "fm-sales" in Free Market shops);
/// the NPC shop's price BasilMarket shows without any isn't a market price. A list not
/// shaped as expected is [`SourceError::FormatChanged`].
pub fn read(text: &str) -> Result<Prices, SourceError> {
    let list: PriceIndex =
        serde_json::from_str(text).map_err(|e| SourceError::FormatChanged(format!("BasilMarket: {e}")))?;
    let mut items = HashMap::new();
    for item in list.items {
        let Some(id) = item.i.strip_suffix(".png").filter(|id| id.len() == 8).and_then(|id| id.parse().ok()) else {
            continue;
        };
        let from_sales = matches!(item.b.as_str(), "sales" | "fm-sales");
        let sold_for = item.v.filter(|v| from_sales && v.is_finite() && *v >= 0.0).map(|v| v.round() as u64);
        let safe = !item.s.is_empty() && item.s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        let page = safe.then(|| format!("{PRICE_PAGE_URL}{}", item.s));
        items.entry(id).or_insert(Price { sold_for, sales: item.k, page });
    }
    Ok(Prices { items })
}

/// BasilMarket's `/api/price-index`.
#[derive(Deserialize)]
struct PriceIndex {
    items: Vec<IndexItem>,
}

/// An item on the price list.
#[derive(Deserialize)]
struct IndexItem {
    /// Its page's slug.
    s: String,
    /// Its picture, named by the game's item ID.
    i: String,
    /// The price BasilMarket recommends; `null` with none.
    v: Option<f64>,
    /// Where that comes from: "sales", "fm-sales", "npc" or "none".
    b: String,
    /// How many sales it comes from.
    k: u32,
}
