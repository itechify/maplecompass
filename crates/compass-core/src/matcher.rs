//! Name matcher: snaps a raw title reading to the closest street name and map name
//! pair in the map index, scoring both lines together.

use crate::map_index::MapEntry;

/// Lowest score (0..=1) that counts as a confident match. A whole-letter slip or two
/// (e.g. "Keming City" for "Kerning City") stays well above it; a half-covered or
/// unrelated title falls below.
const CONFIDENT: f32 = 0.8;

/// A map from the index, with its names trimmed and normalised for matching.
pub struct KnownMap {
    pub entry: MapEntry,
    street_key: Vec<char>,
    map_key: Vec<char>,
}

/// A confident match: which map, and how well it matched (0..=1).
#[derive(Debug, Clone, Copy)]
pub struct Snap {
    pub index: usize,
    pub score: f32,
}

pub struct Matcher {
    maps: Vec<KnownMap>,
}

impl Matcher {
    pub fn new(index: Vec<MapEntry>) -> Self {
        let maps = index
            .into_iter()
            .map(|e| {
                // MeowDB names can carry trailing spaces ("A Hill West of Henesys ").
                let entry = MapEntry {
                    id: e.id.trim().to_string(),
                    street_name: e.street_name.trim().to_string(),
                    map_name: e.map_name.trim().to_string(),
                    region: e.region.trim().to_string(),
                };
                KnownMap { street_key: key(&entry.street_name), map_key: key(&entry.map_name), entry }
            })
            .collect();
        Self { maps }
    }

    pub fn map(&self, index: usize) -> &MapEntry {
        &self.maps[index].entry
    }

    /// Every map with the same title as map `index` (itself included), in index order.
    pub fn same_title_as(&self, index: usize) -> Vec<usize> {
        (0..self.maps.len()).filter(|&i| self.same_title(i, index)).collect()
    }

    /// The length, in characters, of the longest street name or map name.
    pub fn longest_name(&self) -> usize {
        let len = |s: &str| s.chars().count();
        self.maps
            .iter()
            .map(|m| len(&m.entry.street_name).max(len(&m.entry.map_name)))
            .max()
            .unwrap_or(0)
    }

    /// The map whose street name and map name best match the reading, if the match
    /// is confident and no other title matches as well.
    ///
    /// Maps sharing one title score the same, and the first in the index stands for
    /// them all: [`Matcher::same_title_as`] gives the rest.
    pub fn snap(&self, street: &str, map: &str) -> Option<Snap> {
        let (street, map) = (key(street), key(map));
        let mut best: Option<(usize, f32)> = None;
        let mut runner_up = 0.0f32;
        for (i, known) in self.maps.iter().enumerate() {
            let s = score(&street, &map, known);
            match best {
                Some((b, bs)) if s > bs => {
                    if !self.same_title(i, b) {
                        runner_up = bs;
                    }
                    best = Some((i, s));
                }
                Some((b, _)) if !self.same_title(i, b) => runner_up = runner_up.max(s),
                Some(_) => {}
                None => best = Some((i, s)),
            }
        }
        let (index, score) = best?;
        (score >= CONFIDENT && score > runner_up).then_some(Snap { index, score })
    }

    fn same_title(&self, a: usize, b: usize) -> bool {
        let (a, b) = (&self.maps[a], &self.maps[b]);
        a.street_key == b.street_key && a.map_key == b.map_key
    }
}

/// Letters and digits only, lower-cased: OCR often drops or adds spaces ("Rocky
/// RoadI") and punctuation carries little signal.
fn key(s: &str) -> Vec<char> {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// 1 minus the edits needed on both lines, over their combined length.
fn score(street: &[char], map: &[char], known: &KnownMap) -> f32 {
    let edits = levenshtein(street, &known.street_key) + levenshtein(map, &known.map_key);
    let len = street.len().max(known.street_key.len()) + map.len().max(known.map_key.len());
    if len == 0 { 0.0 } else { 1.0 - edits as f32 / len as f32 }
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitute = prev[j] + usize::from(ca != cb);
            cur[j + 1] = substitute.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}
