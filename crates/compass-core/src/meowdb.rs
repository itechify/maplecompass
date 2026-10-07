//! MeowDB reader: turns MeowDB pages, fetched by the shell, into map data. Personal use
//! only until MeowDB grants permission (ADR 0003); credit "NiaMeowDB (meowdb.com)".

use crate::map_data::{HiddenPortal, MapData, MapSummary, Mob, PerParty};
use crate::map_index::{MapEntry, SourceError};
use crate::portals::{self, Exit, MapFacts, Minimap};
use crate::portal_overrides::{PORTAL_OVERRIDES, PortalOverride};
use serde_json::Value;

/// MeowDB's "All Maps A-Z" page: every map, grouped by region and street.
pub const MAP_INDEX_URL: &str = "https://meowdb.com/msclassic/maps/all";

const MAP_LINK: &str = "href=\"/msclassic/maps/";

/// Read the map index from the "All Maps A-Z" page ([`MAP_INDEX_URL`]).
///
/// The page's `<main>` lists, in document order, `<h2>Region</h2>`, then per street
/// `<h3>Street Name <span>(N)</span></h3>` and a list of
/// `<a href="/msclassic/maps/<9-digit id>">Map Name[<span> (town)</span>]</a>`.
/// Names are HTML-decoded and trimmed. A page listing no maps is
/// [`SourceError::FormatChanged`].
pub fn read_map_index(html: &str) -> Result<Vec<MapEntry>, SourceError> {
    let format_changed = |why: &str| SourceError::FormatChanged(format!("map index page: {why}"));
    let start = html.find("<main").ok_or_else(|| format_changed("no <main>"))?;
    let end = html[start..].find("</main>").map_or(html.len(), |e| start + e);
    // React separates adjacent text nodes with empty comments.
    let main = html[start..end].replace("<!-- -->", "");

    let (mut region, mut street) = (None, None);
    let mut maps = Vec::new();
    let mut rest = main.as_str();
    while let Some(lt) = rest.find('<') {
        rest = &rest[lt..];
        let tag_end = rest.find('>').map_or(rest.len(), |e| e + 1);
        let (tag, after) = rest.split_at(tag_end);
        if tag.starts_with("<h2") {
            region = Some(text_before_tag(after));
        } else if tag.starts_with("<h3") {
            street = Some(text_before_tag(after));
        } else if tag.starts_with("<a ") {
            if let (Some(id), Some(region), Some(street)) = (map_id(tag), &region, &street) {
                maps.push(MapEntry {
                    id: id.to_string(),
                    street_name: street.clone(),
                    map_name: text_before_tag(after),
                    region: region.clone(),
                });
            }
        }
        rest = after;
    }
    if maps.is_empty() {
        return Err(format_changed("no maps listed"));
    }
    Ok(maps)
}

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
/// The map's Hidden Portals follow the Hidden Portal rule and the app's hand-kept
/// [`PORTAL_OVERRIDES`]. Its connected maps come from the map's `connectedMaps`.
pub fn read_map_page(html: &str, id: &str) -> Result<MapData, SourceError> {
    read_map_page_with(html, id, PORTAL_OVERRIDES)
}

/// [`read_map_page`], with these portal overrides in place of [`PORTAL_OVERRIDES`].
/// Only for tests of the override list (integration tests can't see test-only items);
/// the app reads with [`read_map_page`].
#[doc(hidden)]
pub fn read_map_page_with(
    html: &str,
    id: &str,
    overrides: &[PortalOverride],
) -> Result<MapData, SourceError> {
    read_page(html, id, overrides)
        .map_err(|why| SourceError::FormatChanged(format!("map page {id}: {why}")))
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
    Ok(MapData { summary, mobs, hidden_portals: hidden_portals(map, id, overrides)?, connected_maps })
}

/// The map's Hidden Portals from its `portals` (exits to other maps). `intraPortals`,
/// teleports within the map, are never Hidden Portals, but must be there all the same.
fn hidden_portals(
    map: &Value,
    id: &str,
    overrides: &[PortalOverride],
) -> Result<Vec<HiddenPortal>, String> {
    let text = |v: &Value, what: &str| -> Result<String, String> {
        v.as_str().map(|s| s.trim().to_string()).ok_or_else(|| format!("{what} is {v}, not text"))
    };
    let number = |v: &Value, what: &str| v.as_f64().ok_or_else(|| format!("{what} is {v}, not a number"));
    let facts = MapFacts {
        id,
        is_town: map["isTown"].as_bool().ok_or("isTown is not true or false")?,
        minimap: Minimap {
            width: number(&map["minimapWidth"], "minimapWidth")?,
            height: number(&map["minimapHeight"], "minimapHeight")?,
            center_x: number(&map["miniMapCenterX"], "miniMapCenterX")?,
            center_y: number(&map["miniMapCenterY"], "miniMapCenterY")?,
        },
    };
    map["intraPortals"].as_array().ok_or("no intraPortals list")?;
    let exits = map["portals"]
        .as_array()
        .ok_or("no portals list")?
        .iter()
        .map(|p| {
            Ok(Exit {
                name: text(&p["name"], "portal name")?,
                kind: uint(&p["type"], "portal type")?,
                x: number(&p["x"], "portal x")?,
                y: number(&p["y"], "portal y")?,
                to_map_id: text(&p["toMapId"], "toMapId")?,
                to_map_name: text(&p["toMapName"], "toMapName")?,
                to_street_name: text(&p["toStreetName"], "toStreetName")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(portals::hidden_portals(&facts, exits, overrides))
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

/// The 9-digit map ID in an `<a … href="/msclassic/maps/<id>">` tag.
fn map_id(tag: &str) -> Option<&str> {
    let at = tag.find(MAP_LINK)? + MAP_LINK.len();
    let rest = &tag[at..];
    let id = rest.get(..9)?;
    (id.bytes().all(|b| b.is_ascii_digit()) && rest[9..].starts_with('"')).then_some(id)
}

/// The text up to the next tag, decoded and trimmed: `"Mushroom Town<span> (town)…"`
/// gives `"Mushroom Town"`.
fn text_before_tag(s: &str) -> String {
    let text = &s[..s.find('<').unwrap_or(s.len())];
    decode_entities(text).trim().to_string()
}

fn decode_entities(s: &str) -> String {
    s.replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}
