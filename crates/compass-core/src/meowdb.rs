//! MeowDB reader: turns MeowDB pages, fetched by the shell, into map data. Personal use
//! only until MeowDB grants permission (ADR 0003); credit "NiaMeowDB (meowdb.com)".

use crate::map_index::{MapEntry, SourceError};

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
