//! MeowDB: its search's answers, fetched by the shell, read into items (ADR 0007), and the
//! links to its map, mob and item pages. No MeowDB page is read. Credit "NiaMeowDB
//! (meowdb.com)".

use crate::item_lookup::ItemHit;
use crate::map_index::SourceError;
use serde_json::Value;

/// The MeowDB page of the map with this 9-digit ID.
pub fn map_url(id: &str) -> String {
    format!("https://meowdb.com/msclassic/maps/{id}")
}

/// The endpoint MeowDB's own search box asks (ADR 0007): `?q=<text>&locale=en` answers
/// with JSON holding up to six close items, among monsters, maps and more.
pub const ITEM_SEARCH_URL: &str = "https://meowdb.com/msclassic/api/search";

/// The MeowDB page of the item with MeowDB's own ID `id` (e.g. `"418"`, Orihalcon Ore),
/// which isn't the game's item ID.
pub fn item_url(id: &str) -> String {
    format!("https://meowdb.com/msclassic/item-db/{id}")
}

/// Read the items from an answer of MeowDB's search ([`ITEM_SEARCH_URL`]): its `items`,
/// each `{"id": 418, "name": "Orihalcon Ore", "sub": "Ore & Mineral", …}`, closest
/// first. `sub` (the category) may be `null`. An answer without `items`, or with an
/// item lacking its ID or name, is [`SourceError::FormatChanged`].
pub fn read_item_search(json: &str) -> Result<Vec<ItemHit>, SourceError> {
    let changed = |why: &str| SourceError::FormatChanged(format!("item search: {why}"));
    let answer: Value = serde_json::from_str(json).map_err(|_| changed("not JSON"))?;
    let items = answer.get("items").and_then(Value::as_array).ok_or_else(|| changed("no items"))?;
    items
        .iter()
        .map(|item| {
            let id = match item.get("id") {
                Some(Value::Number(n)) => n.as_u64().map(|n| n.to_string()),
                Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
                _ => None,
            };
            let name = item.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty());
            let category = item.get("sub").and_then(Value::as_str).map(str::trim).filter(|c| !c.is_empty());
            match (id, name) {
                (Some(id), Some(name)) => Ok(ItemHit { id, name: name.to_string(), category: category.map(String::from) }),
                _ => Err(changed("an item without its ID or name")),
            }
        })
        .collect()
}

/// The MeowDB page of the mob with this ID (e.g. `"3"`, Blue Snail).
pub fn mob_url(id: &str) -> String {
    format!("https://meowdb.com/msclassic/monsters/{id}")
}

