//! Item Lookup (ADR 0007): the player hovers an item in the game and presses the hotkey.
//! The shell captures the game's client area and the pointer, the Item Tooltip reader
//! reads the item's name, and MeowDB's search turns the name into the item's page.
//!
//! A search result counts only if its name is the name read, but for the letters the
//! recogniser confuses (I, l and 1; O and 0), case, spacing and punctuation, or if it is
//! clearly closer to the name read than any other result. Otherwise no page opens: the
//! alert says no MeowDB item has that name, rather than open a guess. Several results
//! with the name are listed for the player to pick from.
//!
//! What a lookup comes to shows in an alert of its own at the top centre of the game
//! window, where the player is looking, for a few seconds; the panel is left as it is.

use crate::item_tooltip::{ItemTooltip, ItemTooltipReading};
use crate::map_index::{Fetched, MapDataSource, SourceError};
use crate::matcher::{key, levenshtein};
use crate::meowdb;
use crate::pace::Pace;
use crate::panel::{GameWindow, ScreenRect, Size};
use serde::Serialize;
use std::task::Poll;

/// One item in MeowDB's answer to a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemHit {
    /// MeowDB's own ID for the item, not the game's item ID.
    pub id: String,
    pub name: String,
    /// MeowDB's category for it ("Ore & Mineral", "One-Handed Sword"); `None` when it
    /// gives none (chairs, for one).
    pub category: Option<String>,
}

/// What the alert says about the latest Item Lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ItemLookup {
    /// The Item Tooltip's name read as `read`, and MeowDB is being asked for it.
    LookingUp { read: String },
    /// The item's MeowDB page was opened.
    Opened { name: String, url: String },
    /// Several MeowDB items have the name read as `read`: the player picks one.
    Choose { read: String, items: Vec<ItemChoice> },
    /// No Item Tooltip near the pointer.
    NoTooltip,
    /// An Item Tooltip near the pointer, but its name couldn't be read.
    Unreadable,
    /// One of our windows covers the Item Tooltip's name, so it wasn't read: our windows
    /// show in the capture.
    Covered { by: OurWindow },
    /// No MeowDB item has the name read as `read`.
    NotFound { read: String },
    /// MeowDB couldn't be reached.
    Unreachable,
    /// MeowDB's search answered in a shape the reader doesn't know.
    FormatChanged,
}

/// One of several MeowDB items that share the name read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemChoice {
    /// MeowDB's own ID for the item.
    pub id: String,
    pub name: String,
    /// MeowDB's category for it; `None` when it gives none.
    pub category: Option<String>,
    pub url: String,
}

/// One of MapleCompass's own windows, which show in our captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OurWindow {
    Panel,
    GrindTracker,
    /// The last Item Lookup's alert, still showing.
    Alert,
    PersonalBests,
}

/// The latest Item Lookup's alert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemLookupLine {
    /// Counts the player's lookups, so the overlay can tell a new line from the last one
    /// when both say the same.
    pub lookup: u32,
    #[serde(flatten)]
    pub outcome: ItemLookup,
}

/// How long an alert stays, from when it was set (capture time). Its window fades it out
/// over its last second (`LINE_MS` and `CHOICES_MS` in `src/item-lookup.ts`).
const LINE_MS: u64 = 6_000;
/// How long the list of items sharing a name stays: long enough to read and pick.
const CHOICES_MS: u64 = 30_000;
/// A result whose name isn't the one read counts if it is at least this similar to it
/// (1 minus edits over length)...
const CLOSE_ENOUGH: f32 = 0.85;
/// ...and every other name in the answer is at least this many more edits away.
const CLEAR_LEAD: usize = 2;

/// The latest Item Lookup: the capture asked for, the search in flight, and the line.
#[derive(Default)]
pub(crate) struct ItemLookups {
    /// A capture was asked for and its reading hasn't come back: another press waits.
    capturing: bool,
    /// The name being looked up, until MeowDB answers, and whether it was asked for yet.
    searching: Option<(String, bool)>,
    line: Option<ItemLookupLine>,
    /// When the line goes (capture time); `None` while it stays (looking up).
    line_until: Option<u64>,
    lookups: u32,
    /// The latest time heard (capture time).
    now_ms: u64,
}

impl ItemLookups {
    /// The player pressed the hotkey: whether to capture the game for a new lookup. Not
    /// while the last capture is still being read.
    pub fn start(&mut self) -> bool {
        !std::mem::replace(&mut self.capturing, true)
    }

    /// What the capture read as (`None`: it failed), with where our windows are on
    /// screen. Returns the page to open, if MeowDB answered at once.
    pub fn read(
        &mut self,
        reading: Option<ItemTooltipReading>,
        ours: &[(OurWindow, ScreenRect)],
        source: &mut dyn MapDataSource,
        pace: &mut Pace,
    ) -> Option<String> {
        self.capturing = false;
        self.searching = None;
        self.lookups += 1;
        let at_ms = reading.as_ref().map_or(self.now_ms, |r| r.at_ms);
        self.now_ms = self.now_ms.max(at_ms);
        let outcome = match reading.map(|r| r.tooltip) {
            Some(ItemTooltip::None) => ItemLookup::NoTooltip,
            Some(ItemTooltip::Found { area, name }) => {
                match ours.iter().find(|(_, window)| overlaps(*window, area)) {
                    Some((by, _)) => ItemLookup::Covered { by: *by },
                    None => match name.as_deref().and_then(item_name) {
                        Some(read) => {
                            self.searching = Some((read.clone(), false));
                            ItemLookup::LookingUp { read }
                        }
                        None => ItemLookup::Unreadable,
                    },
                }
            }
            // The capture failed: nothing was read.
            None => ItemLookup::Unreadable,
        };
        self.show(outcome);
        self.poll(source, pace, at_ms)
    }

    /// Time passed: check on the search in flight, and let the line go when its time is
    /// up. Returns the page to open, once MeowDB has answered.
    pub fn poll(&mut self, source: &mut dyn MapDataSource, pace: &mut Pace, at_ms: u64) -> Option<String> {
        self.now_ms = self.now_ms.max(at_ms);
        if self.line_until.is_some_and(|until| self.now_ms >= until) {
            self.line = None;
            self.line_until = None;
        }
        let (read, asked) = self.searching.take()?;
        // A search not yet asked waits out MeowDB's pace from the last one that went over
        // the network.
        if !asked && !pace.ready(at_ms) {
            self.searching = Some((read, false));
            return None;
        }
        let answer = match source.item_search(&read) {
            Poll::Pending => {
                if !asked {
                    pace.sent(at_ms);
                }
                self.searching = Some((read, true));
                return None;
            }
            Poll::Ready(answer) => answer,
        };
        let (outcome, open) = match answer {
            Ok(Fetched::Fresh(hits) | Fetched::SavedCopy { data: hits, .. }) => match resolve(&read, &hits) {
                Resolved::One(hit) => {
                    let url = meowdb::item_url(&hit.id);
                    (ItemLookup::Opened { name: hit.name, url: url.clone() }, Some(url))
                }
                Resolved::Several(hits) => {
                    let items = hits
                        .into_iter()
                        .map(|hit| ItemChoice { url: meowdb::item_url(&hit.id), id: hit.id, name: hit.name, category: hit.category })
                        .collect();
                    (ItemLookup::Choose { read, items }, None)
                }
                Resolved::Neither => (ItemLookup::NotFound { read }, None),
            },
            Err(SourceError::Unreachable(_)) => (ItemLookup::Unreachable, None),
            Err(SourceError::FormatChanged(_)) => (ItemLookup::FormatChanged, None),
        };
        self.show(outcome);
        open
    }

    /// The player picked one of the items sharing the name read: its page, if it is one
    /// of them.
    pub fn pick(&mut self, id: &str) -> Option<String> {
        let Some(ItemLookupLine { outcome: ItemLookup::Choose { items, .. }, .. }) = &self.line else {
            return None;
        };
        let choice = items.iter().find(|choice| choice.id == id)?.clone();
        self.show(ItemLookup::Opened { name: choice.name, url: choice.url.clone() });
        Some(choice.url)
    }

    pub fn line(&self) -> Option<ItemLookupLine> {
        self.line.clone()
    }

    fn show(&mut self, outcome: ItemLookup) {
        self.line_until = match outcome {
            ItemLookup::LookingUp { .. } => None,
            ItemLookup::Choose { .. } => Some(self.now_ms + CHOICES_MS),
            _ => Some(self.now_ms + LINE_MS),
        };
        self.line = Some(ItemLookupLine { lookup: self.lookups, outcome });
    }
}

/// The alert's top edge sits this share of the game's client area down from its top.
const ALERT_TOP_SHARE: f32 = 0.05;
/// Gap kept between the alert and a captured region it moves clear of (physical px).
const ALERT_GAP: i32 = 8;

/// Where the alert of `size` goes: centred at the top of the game's client area, and
/// below any of `avoid` (the captured regions) it would cover, since our windows show in
/// our captures. Never wider than the client area.
pub(crate) fn alert_placement(game: &GameWindow, size: Size, avoid: &[ScreenRect]) -> ScreenRect {
    let client = game.client;
    let width = size.width.min(client.width);
    let x = client.x + (client.width - width) as i32 / 2;
    let mut alert = ScreenRect { x, y: client.y + (client.height as f32 * ALERT_TOP_SHARE) as i32, width, height: size.height };
    // Each move only goes down, so going over the regions twice settles it.
    for _ in 0..2 {
        for region in avoid {
            if overlaps(alert, *region) {
                alert.y = region.y + region.height as i32 + ALERT_GAP;
            }
        }
    }
    alert
}

fn overlaps(a: ScreenRect, b: ScreenRect) -> bool {
    let (a_right, b_right) = (a.x + a.width as i32, b.x + b.width as i32);
    let (a_bottom, b_bottom) = (a.y + a.height as i32, b.y + b.height as i32);
    a.x < b_right && b.x < a_right && a.y < b_bottom && b.y < a_bottom
}

/// The item's name in what the Item Tooltip's name line read as: trimmed, spaces
/// collapsed, without a scrolled item's "(+3)". `None` without two letters or digits.
fn item_name(read: &str) -> Option<String> {
    let mut name = read.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some(open) = name.rfind('(') {
        let upgrades = name[open + 1..].trim_end_matches(')');
        if upgrades.trim_start_matches('+').chars().all(|c| c.is_ascii_digit()) {
            name.truncate(open);
            name = name.trim_end().to_string();
        }
    }
    (name.chars().filter(|c| c.is_alphanumeric()).count() >= 2).then_some(name)
}

/// What a name read resolves to among MeowDB's answer.
enum Resolved {
    One(ItemHit),
    Several(Vec<ItemHit>),
    Neither,
}

/// A name for matching: the title matcher's (letters and digits only, lower case, the
/// upright strokes I, l, 1, |, ! as one), with O and 0 as one too.
fn item_key(name: &str) -> Vec<char> {
    key(name).into_iter().map(|c| if c == '0' { 'o' } else { c }).collect()
}

fn resolve(read: &str, hits: &[ItemHit]) -> Resolved {
    let read = item_key(read);
    // The answer's names, each with its items and its edits from the name read, closest
    // first.
    let mut names: Vec<(Vec<char>, Vec<ItemHit>, usize)> = Vec::new();
    for hit in hits {
        let name = item_key(&hit.name);
        match names.iter_mut().find(|(known, ..)| *known == name) {
            Some((_, items, _)) => items.push(hit.clone()),
            None => {
                let edits = levenshtein(&read, &name);
                names.push((name, vec![hit.clone()], edits));
            }
        }
    }
    names.sort_by_key(|(_, _, edits)| *edits);
    let Some((name, items, edits)) = names.first() else { return Resolved::Neither };
    let len = read.len().max(name.len());
    let similar = len > 0 && 1.0 - *edits as f32 / len as f32 >= CLOSE_ENOUGH;
    let clear = names.get(1).is_none_or(|(_, _, next)| *next >= edits + CLEAR_LEAD);
    if *edits > 0 && !(similar && clear) {
        return Resolved::Neither;
    }
    match &items[..] {
        [one] => Resolved::One(one.clone()),
        _ => Resolved::Several(items.clone()),
    }
}
