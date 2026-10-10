//! MeowDB reader: turns MeowDB's mob pages, fetched by the shell, into mob data, and its
//! search's answers into items (ADR 0007), and gives the pages' links. No map page is
//! read; the mob pages are read for Mob Cards, until #58. Credit "NiaMeowDB (meowdb.com)".

use crate::item_lookup::ItemHit;
use crate::map_index::SourceError;
use crate::mob_card::{DamageRange, Elements, HpPerExp, MobDetails};
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

/// Read a mob's Mob Card figures from its MeowDB page ([`mob_url`]).
///
/// Unlike a map page's, a mob page's figures aren't in its flight data as such, only
/// as rendered text, so they're read from the page's text: from the `Level` heading up
/// to "Check your build against …", each figure the text after its label (`ACC`, `39`).
/// The labels come in this order:
///
/// - `Level`, its value, then the element tags ("Fire weak", "Ice strong", "Poison
///   immune", among others such as "Boss") up to `Mesos per kill`;
/// - `HP per EXP`, its value and its rank ("#34 of 182 · Lower is better");
/// - `Touch DMG` ("5-7"), `P.DEF`, `M.DEF`, `ACC`, `AVOID`, "—" when there is none;
/// - `Speed` ("Slow") and `Movement` ("Stays on its floor");
/// - `Other Attributes`, whose list may hold "MP 30". A boss's may not, and a mob with
///   nothing to list (Pig) has no such section at all.
///
/// A page missing any of these labels is [`SourceError::FormatChanged`], never a card
/// with figures missing; only MP (and the section that holds it) may be absent.
pub fn read_mob_page(html: &str) -> Result<MobDetails, SourceError> {
    mob_details(&text_runs(html)).map_err(|why| SourceError::FormatChanged(format!("mob page: {why}")))
}

fn mob_details(text: &[String]) -> Result<MobDetails, String> {
    let level = text.windows(2).position(|w| w[0] == "Level" && w[1].parse::<u32>().is_ok());
    let start = level.ok_or("no Level")?;
    let end = text[start..].iter().position(|t| t.starts_with("Check your build")).ok_or("no build check")?;
    let stats = &text[start..start + end];
    let at = |label: &str| stats.iter().position(|t| t == label).ok_or_else(|| format!("no {label}"));
    let after = |label: &str, n: usize| -> Result<&str, String> {
        let i = at(label)?;
        stats.get(i + n).map(String::as_str).ok_or_else(|| format!("nothing after {label}"))
    };
    let figure = |label: &str| -> Result<Option<u64>, String> {
        let value = after(label, 1)?;
        if value == NONE {
            return Ok(None);
        }
        whole(value).map(Some).ok_or_else(|| format!("{label} is {value:?}"))
    };
    let small = |label: &str| -> Result<Option<u32>, String> {
        figure(label)?.map(|n| u32::try_from(n).map_err(|_| format!("{label} is {n}"))).transpose()
    };

    let mut elements = Elements::default();
    for tag in &stats[2..at("Mesos per kill")?] {
        let Some((element, affinity)) = tag.split_once(' ') else { continue };
        match affinity {
            "weak" => elements.weak.push(element.to_string()),
            "strong" => elements.strong.push(element.to_string()),
            "immune" => elements.immune.push(element.to_string()),
            _ => {}
        }
    }

    let touch = after("Touch DMG", 1)?;
    let touch_damage = match touch {
        NONE => None,
        _ => {
            let range = touch.split_once('-');
            let range = range.and_then(|(min, max)| Some(DamageRange { min: whole(min)?, max: whole(max)? }));
            Some(range.ok_or_else(|| format!("Touch DMG is {touch:?}"))?)
        }
    };

    let hp_per_exp = match after("HP per EXP", 1)? {
        NONE => None,
        value => {
            let hundredths = hundredths(value).ok_or_else(|| format!("HP per EXP is {value:?}"))?;
            let rank = after("HP per EXP", 2)?;
            let (place, of) = rank.strip_prefix('#').and_then(|r| r.split_once(" of ")).ok_or("no HP per EXP rank")?;
            let of: String = of.chars().take_while(char::is_ascii_digit).collect();
            match (place.parse(), of.parse()) {
                (Ok(rank), Ok(ranked)) => Some(HpPerExp { hundredths, rank, ranked }),
                _ => return Err(format!("HP per EXP rank is {rank:?}")),
            }
        }
    };

    // MP is listed under Other Attributes, which a mob with nothing to list doesn't have.
    let attributes = at("Other Attributes").map_or(&[][..], |i| &stats[i + 1..]);
    let mp = attributes.iter().find_map(|t| whole(t.strip_prefix("MP ")?));

    Ok(MobDetails {
        elements,
        touch_damage,
        mp,
        accuracy: small("ACC")?,
        avoid: small("AVOID")?,
        physical_defense: small("P.DEF")?,
        magic_defense: small("M.DEF")?,
        speed: after("Speed", 1)?.to_string(),
        movement: after("Movement", 1)?.to_string(),
        hp_per_exp,
    })
}

/// What a mob page shows for a figure the mob doesn't have.
const NONE: &str = "—";

/// A whole number, maybe with thousands separators ("34,440").
fn whole(s: &str) -> Option<u64> {
    let digits = s.replace(',', "");
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then(|| digits.parse().ok())?
}

/// A number with two decimals ("12.75", "1,012.50") in hundredths.
fn hundredths(s: &str) -> Option<u64> {
    let (units, cents) = s.split_once('.')?;
    (cents.len() == 2).then_some(())?;
    Some(whole(units)? * 100 + whole(cents)?)
}

/// The page's text outside scripts and styles: each run between two tags, decoded and
/// trimmed, empty ones left out. React's empty comments between adjacent text nodes are
/// dropped first, so "No. <!-- -->003" is one run.
fn text_runs(html: &str) -> Vec<String> {
    let html = html.replace("<!-- -->", "");
    let mut runs = Vec::new();
    let mut rest = html.as_str();
    while let Some(lt) = rest.find('<') {
        let run = decode_entities(&rest[..lt]);
        if !run.trim().is_empty() {
            runs.push(run.trim().to_string());
        }
        rest = &rest[lt..];
        let skip_to = ["<script", "<style"]
            .into_iter()
            .find(|open| rest.starts_with(open))
            .map(|open| format!("</{}>", &open[1..]));
        let end = match skip_to {
            Some(close) => rest.find(&close).map(|e| e + close.len()),
            None => rest.find('>').map(|e| e + 1),
        };
        rest = &rest[end.unwrap_or(rest.len())..];
    }
    runs
}

fn decode_entities(s: &str) -> String {
    s.replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}
