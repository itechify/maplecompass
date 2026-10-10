//! The Personal Bests page (GLOSSARY): every map's Personal Best side by side, for the
//! player to compare maps in their browser rather than over the game. The Grind Tracker's
//! window bar opens it, while some map has a best.
//!
//! - A snapshot: the bests as saved when the player asks for it, the Visit under way's
//!   included, as closing MapleCompass would keep them. So a map's row may be higher than
//!   the Best row there, which shows only the earlier Visits'.
//! - One row a map, its Personal Best, highest EXP/h first. The page re-sorts as the
//!   player likes; the core gives the figures, never text.
//! - Each map's names, region, mob levels and solo EXP/h ceiling (its Map Summary, worked
//!   out as NiaMeowDB does) from OSMS's file set (ADR 0008), the same for every map. Bests
//!   are shared by all the player's characters and record no level, so the levels and the
//!   share of the ceiling say what a best was worth. A best on a map not in the map list
//!   is kept, with none of these, and a map without mobs has no levels or ceiling.
//! - The map the player is on is marked: only while the panel is certain of it.

use crate::map_data::MapData;
use crate::map_index::MapEntry;
use crate::personal_best::SavedBests;
use crate::shown_map::LevelRange;
use crate::{OSMS_CREDIT, meowdb};
use serde::Serialize;

/// The Personal Bests page, as the shell renders it into a page for the browser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BestsPage {
    /// When the player asked for it, by the wall clock (ms since the Unix epoch); `None` if
    /// the shell hadn't told the wall clock by then.
    pub as_of_unix_ms: Option<u64>,
    /// Every map with a Personal Best, highest EXP/h first.
    pub maps: Vec<BestsPageMap>,
    /// [`OSMS_CREDIT`]: the names, levels and ceilings are from OSMS's data, and the rows
    /// open MeowDB's pages.
    pub credit: &'static str,
}

/// One map's row on the Personal Bests page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BestsPageMap {
    /// The 9-digit map ID.
    pub id: String,
    /// The map's names and region, as OSMS's map list gives them; empty while no file set
    /// has arrived, or if the map isn't in the list: the page goes by the ID.
    pub street_name: String,
    pub map_name: String,
    pub region: String,
    /// The map's MeowDB page.
    pub url: String,
    /// The lowest and highest of its mobs' levels; `None` if it isn't in the map list, or
    /// has no mobs.
    pub mob_levels: Option<LevelRange>,
    /// Its Personal Best: the Rolling Rate's EXP/h, as the Visit's best was set.
    pub exp_per_hour: u64,
    /// The map's solo EXP/h ceiling (its Map Summary), what the share is of; `None` if it
    /// isn't in the map list, or has no Map Summary.
    pub solo_exp_ceiling: Option<u64>,
    /// The best as a share of the map's solo EXP/h ceiling (its Map Summary), in
    /// hundredths of a percent, as the Grind Tracker writes its share: 322 for "3.2%".
    /// `None` with no ceiling.
    pub ceiling_share_hundredths: Option<u64>,
    /// When it was set, by the wall clock (ms since the Unix epoch); `None` if the shell
    /// hadn't told the wall clock by then.
    pub set_at_unix_ms: Option<u64>,
    /// The panel shows this map and is certain the player is on it.
    pub here: bool,
}

/// The Personal Bests page for the bests `saved`, at `now_unix_ms` by the wall clock.
/// `map` gives a map's entry in the map list and its data, if it is in the list; `here`
/// is the map the player is certainly on, if any.
pub(crate) fn page<'a>(
    saved: &SavedBests,
    map: impl Fn(&str) -> Option<(&'a MapEntry, &'a MapData)>,
    here: Option<&str>,
    now_unix_ms: Option<u64>,
) -> BestsPage {
    let mut maps: Vec<BestsPageMap> = saved
        .maps
        .iter()
        .filter_map(|bests| {
            let best = bests.visits.first()?;
            let (entry, data) = map(&bests.id).unzip();
            let ceiling = data.and_then(|d| d.summary).map(|s| s.exp_per_hour.solo).filter(|&c| c > 0);
            Some(BestsPageMap {
                id: bests.id.clone(),
                street_name: entry.map_or_else(String::new, |e| e.street_name.clone()),
                map_name: entry.map_or_else(String::new, |e| e.map_name.clone()),
                region: entry.map_or_else(String::new, |e| e.region.clone()),
                url: meowdb::map_url(&bests.id),
                mob_levels: data.and_then(mob_levels),
                exp_per_hour: best.exp_per_hour,
                solo_exp_ceiling: ceiling,
                ceiling_share_hundredths: ceiling.map(|c| share_hundredths(best.exp_per_hour, c)),
                set_at_unix_ms: best.set_at_unix_ms,
                here: here == Some(bests.id.as_str()),
            })
        })
        .collect();
    maps.sort_by(|a, b| b.exp_per_hour.cmp(&a.exp_per_hour).then_with(|| a.id.cmp(&b.id)));
    BestsPage { as_of_unix_ms: now_unix_ms, maps, credit: OSMS_CREDIT }
}

/// The lowest and highest of the map's mobs' levels. A map's mobs are read in any order
/// (only the panel's are sorted).
fn mob_levels(data: &MapData) -> Option<LevelRange> {
    let lowest = data.mobs.iter().map(|m| m.level).min()?;
    let highest = data.mobs.iter().map(|m| m.level).max()?;
    Some(LevelRange { lowest, highest })
}

/// `exp_per_hour` as a share of `ceiling` (> 0), in hundredths of a percent, rounded, halves
/// up.
fn share_hundredths(exp_per_hour: u64, ceiling: u64) -> u64 {
    let scaled = u128::from(exp_per_hour) * 10_000;
    ((scaled + u128::from(ceiling) / 2) / u128::from(ceiling)) as u64
}
