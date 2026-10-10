//! MeowDB reader: turns MeowDB pages, fetched by the shell, into map and mob data, and its
//! search's answers into items (ADR 0007), and gives the pages' links. The map pages are
//! read only for the world crawl, until #57, and the mob pages for Mob Cards, until #58;
//! credit "NiaMeowDB (meowdb.com)".

use crate::item_lookup::ItemHit;
use crate::map_data::{HiddenPortal, MapData, MapSummary, Mob, PerParty, Portal};
use crate::map_index::SourceError;
use crate::mob_card::{DamageRange, Elements, HpPerExp, MobDetails};
use crate::portals::{self, Exit, MapFacts};
use crate::portal_overrides::{PORTAL_OVERRIDES, PortalOverride};
use serde_json::Value;

/// The MeowDB page of the map with this 9-digit ID.
pub fn map_url(id: &str) -> String {
    format!("https://meowdb.com/msclassic/maps/{id}")
}

/// Read one map's data from its MeowDB page ([`map_url`]).
///
/// The page is a Next.js app: its data travels in `self.__next_f.push([1, "…"])`
/// script chunks, which joined make the "flight" stream. In it, the map component's
/// props start `{"map":{"id":"…` and, on maps with mobs, the Map Summary's start
/// `{"rates":{"id":"…`. HP is only in the page's server-rendered Monsters table.
///
/// Anything not as expected is [`SourceError::FormatChanged`], never partial data: no
/// chunks, a missing or repeated payload, a payload for another map, missing fields,
/// or a mob that the summary or the Monsters table doesn't list. A map without mobs
/// and without a summary (a town) is valid.
///
/// The map's portals to other maps, with their kinds, and its Hidden Portals follow the
/// Hidden Portal rule and the app's hand-kept [`PORTAL_OVERRIDES`]. Its connected maps
/// come from the map's `connectedMaps`, its Mob Rate from `mobRate` and its nearest
/// town from `returnMap`.
pub fn read_map_page(html: &str, id: &str) -> Result<MapData, SourceError> {
    read_page(html, id, PORTAL_OVERRIDES).map_err(|why| SourceError::FormatChanged(format!("map page {id}: {why}")))
}

fn read_page(html: &str, id: &str, overrides: &[PortalOverride]) -> Result<MapData, String> {
    let flight = flight(html)?;
    let map = props_at(&flight, r#"{"map":{"id":""#)?.ok_or("no map payload")?;
    let map = &map["map"];
    if map["id"].as_str() != Some(id) {
        return Err(format!("payload is for map {}", map["id"]));
    }
    let monsters = map["monsters"].as_array().ok_or("no monsters list")?;
    let connected_maps = map["connectedMaps"]
        .as_array()
        .ok_or("no connectedMaps list")?
        .iter()
        .map(|c| c["mapId"].as_str().map(|id| id.trim().to_string()).ok_or("connected map without a mapId"))
        .collect::<Result<Vec<_>, _>>()?;
    let total_spawns = uint(&map["totalMobCount"], "totalMobCount")?;

    let rates = props_at(&flight, r#"{"rates":{"id":""#)?;
    let rates = rates.as_ref().map(|r| &r["rates"]);
    let rates = match rates {
        Some(rates) if rates["id"].as_str() != Some(id) => {
            return Err(format!("summary is for map {}", rates["id"]));
        }
        None if !monsters.is_empty() => return Err("mobs but no summary".into()),
        rates => rates,
    };
    let summary = rates
        .map(|r| -> Result<MapSummary, String> {
            Ok(MapSummary {
                mobs_per_hour: PerParty {
                    solo: figure(&r["mobsPerHour"], "mobsPerHour")?,
                    full_party: figure(&r["mobsPerHourFullParty"], "mobsPerHourFullParty")?,
                },
                exp_per_hour: PerParty {
                    solo: figure(&r["expPerHour"], "expPerHour")?,
                    full_party: figure(&r["expPerHourFullParty"], "expPerHourFullParty")?,
                },
            })
        })
        .transpose()?;

    let hp = if monsters.is_empty() { Vec::new() } else { monster_table_hp(html)? };
    let per_mob = rates.map_or(&[][..], |r| r["perMob"].as_array().map_or(&[][..], Vec::as_slice));
    let mobs = monsters
        .iter()
        .map(|m| {
            let mob_id = m["id"].as_str().ok_or("mob without an id")?;
            let number: u64 = mob_id.parse().map_err(|_| format!("mob id {mob_id:?}"))?;
            let name = m["name"].as_str().ok_or_else(|| format!("mob {mob_id} has no name"))?;
            let spawn_count = uint(&m["count"], "count")?;
            let rate = per_mob
                .iter()
                .find(|r| r["id"].as_u64() == Some(number))
                .ok_or_else(|| format!("mob {mob_id} not in the summary"))?;
            let &(_, hp) = hp
                .iter()
                .find(|(n, _)| *n == number)
                .ok_or_else(|| format!("mob {mob_id} not in the Monsters table"))?;
            if total_spawns == 0 {
                return Err("mobs but no spawns".to_string());
            }
            Ok(Mob {
                id: mob_id.to_string(),
                name: name.trim().to_string(),
                level: u32::try_from(uint(&rate["level"], "level")?).map_err(|e| e.to_string())?,
                hp,
                exp: uint(&rate["exp"], "exp")?,
                spawn_count: u32::try_from(spawn_count).map_err(|e| e.to_string())?,
                // Rounded half up, as MeowDB shows it.
                spawn_share_percent: ((spawn_count * 200 + total_spawns) / (2 * total_spawns)) as u32,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mob_rate_percent = match map["mobRate"].as_f64() {
        Some(rate) if rate >= 0.0 && rate.is_finite() => (rate * 100.0).round() as u32,
        _ => return Err(format!("mobRate is {}, not a rate", map["mobRate"])),
    };
    let nearest_town = match map["returnMap"].as_str().map(str::trim) {
        Some(portals::DEAD_END) => None,
        Some(town) => Some(town.to_string()),
        None => return Err(format!("returnMap is {}, not a map ID", map["returnMap"])),
    };
    let (portals, hidden_portals) = portals(map, id, overrides)?;
    Ok(MapData {
        summary,
        mobs,
        hidden_portals,
        connected_maps,
        portals,
        mob_rate_percent,
        nearest_town,
        // The panel picks these out when it loads the data.
        ..MapData::default()
    })
}

/// The map's portals to other maps and its Hidden Portals, from its `portals`.
/// `intraPortals`, teleports within the map, are neither, but must be there all the
/// same.
fn portals(
    map: &Value,
    id: &str,
    overrides: &[PortalOverride],
) -> Result<(Vec<Portal>, Vec<HiddenPortal>), String> {
    let text = |v: &Value, what: &str| -> Result<String, String> {
        v.as_str().map(|s| s.trim().to_string()).ok_or_else(|| format!("{what} is {v}, not text"))
    };
    let facts = MapFacts { id, is_town: map["isTown"].as_bool().ok_or("isTown is not true or false")? };
    map["intraPortals"].as_array().ok_or("no intraPortals list")?;
    let exits = map["portals"]
        .as_array()
        .ok_or("no portals list")?
        .iter()
        .map(|p| {
            Ok(Exit {
                name: text(&p["name"], "portal name")?,
                kind: uint(&p["type"], "portal type")?,
                to_map_id: text(&p["toMapId"], "toMapId")?,
                to_map_name: text(&p["toMapName"], "toMapName")?,
                to_street_name: text(&p["toStreetName"], "toStreetName")?,
                // Only OSMS marks portals on a picture.
                to_known: true,
                within_map: false,
                landing: String::new(),
                x: 0,
                y: 0,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let sorted = portals::portals(&facts, exits, overrides);
    Ok((sorted.portals, sorted.hidden_portals))
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

/// The page's flight stream: the strings of every `self.__next_f.push([1, "…"])`, joined
/// (a payload may straddle chunks).
fn flight(html: &str) -> Result<String, String> {
    const PUSH: &str = "self.__next_f.push(";
    let mut flight = String::new();
    let mut chunks = 0;
    for (at, _) in html.match_indices(PUSH) {
        let chunk: Value = first_json(&html[at + PUSH.len()..]).ok_or("unreadable flight chunk")?;
        chunks += 1;
        if let (Some(1), Some(text)) = (chunk[0].as_u64(), chunk[1].as_str()) {
            flight.push_str(text);
        }
    }
    if chunks == 0 {
        return Err("no flight data".into());
    }
    Ok(flight)
}

/// The JSON object starting at the only occurrence of `anchor`, `None` if there is none.
fn props_at(flight: &str, anchor: &str) -> Result<Option<Value>, String> {
    let mut hits = flight.match_indices(anchor).map(|(at, _)| at);
    let Some(at) = hits.next() else { return Ok(None) };
    if hits.next().is_some() {
        return Err(format!("{anchor} more than once"));
    }
    first_json(&flight[at..]).map(Some).ok_or_else(|| format!("unreadable {anchor}…"))
}

/// The one JSON value at the start of `s`, ignoring whatever follows it.
fn first_json(s: &str) -> Option<Value> {
    serde_json::Deserializer::from_str(s).into_iter::<Value>().next()?.ok()
}

fn uint(value: &Value, what: &str) -> Result<u64, String> {
    value.as_u64().ok_or_else(|| format!("{what} is {value}, not a whole number"))
}

/// A Map Summary figure: a non-negative number, rounded.
fn figure(value: &Value, what: &str) -> Result<u64, String> {
    match value.as_f64() {
        Some(n) if n >= 0.0 && n.is_finite() => Ok(n.round() as u64),
        _ => Err(format!("{what} is {value}, not a figure")),
    }
}

/// Each mob's HP from the page's Monsters table, by mob ID. Rows are
/// `Monster | Lv | HP | EXP | Count | Share`; the mob's ID is in its
/// `/msclassic/monsters/<id>` link, and HP may have thousands separators ("1,419").
fn monster_table_hp(html: &str) -> Result<Vec<(u64, u64)>, String> {
    const MOB_LINK: &str = "href=\"/msclassic/monsters/";
    let start = html.find(">Monsters (").ok_or("no Monsters table")?;
    let end = html[start..].find("</table>").ok_or("Monsters table never ends")? + start;
    let mut hp = Vec::new();
    for row in html[start..end].split("<tr").skip(1) {
        let Some(link) = row.find(MOB_LINK) else { continue }; // the header row
        let digits = &row[link + MOB_LINK.len()..];
        let id = digits[..digits.find('"').unwrap_or(0)].parse().map_err(|_| "bad mob link")?;
        let cells = cells(row);
        if cells.len() != 6 {
            return Err(format!("Monsters row with {} cells", cells.len()));
        }
        let mob_hp = cells[2].replace(',', "").parse().map_err(|_| format!("HP {:?}", cells[2]))?;
        hp.push((id, mob_hp));
    }
    Ok(hp)
}

/// The text of each `<td>` in a table row, tags removed, decoded and trimmed.
fn cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut rest = row;
    while let Some(open) = rest.find("<td") {
        let Some(body) = rest[open..].find('>').map(|e| open + e + 1) else { break };
        let close = rest[body..].find("</td>").map_or(rest.len(), |e| body + e);
        cells.push(decode_entities(&strip_tags(&rest[body..close])).trim().to_string());
        rest = &rest[close..];
    }
    cells
}

fn strip_tags(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    text
}

fn decode_entities(s: &str) -> String {
    s.replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}
