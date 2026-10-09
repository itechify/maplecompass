//! Personal Bests (GLOSSARY): the best Rolling Rate the player has had on each map, kept
//! Visit by Visit.
//!
//! - A Visit's best on a map is the highest Last 5 min EXP/h the Grind Tracker showed at
//!   a refresh whose window was a full 5 minutes of active play, all on the map: the
//!   Session decides what counts, and records it here.
//! - A map's Personal Best is the best of its Visits'. During a Visit the rate is
//!   compared with the earlier Visits' best, which can't change until the Visit ends:
//!   this Visit's best joins them then.
//! - Bests are per map (MeowDB map ID), for every character: nothing about the Session
//!   touches them.

use serde::{Deserialize, Serialize};

/// The shape [`SavedBests`] has now. Bumped whenever it changes.
pub const BESTS_VERSION: u32 = 1;

/// Every map's Visit bests, as plain data the shell can keep between launches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedBests {
    /// The [`BESTS_VERSION`] it was saved by.
    pub version: u32,
    /// The maps with at least one Visit's best.
    pub maps: Vec<MapBests>,
}

impl Default for SavedBests {
    /// No bests anywhere.
    fn default() -> Self {
        Self { version: BESTS_VERSION, maps: Vec::new() }
    }
}

/// One map's bests: the best of each Visit to it that had one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapBests {
    /// MeowDB's 9-digit map ID.
    pub id: String,
    /// Highest first: the first is the map's Personal Best.
    pub visits: Vec<VisitBest>,
}

/// The best one Visit had on a map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisitBest {
    /// The Last 5 min EXP/h shown, over a full 5 minutes of active play on the map.
    pub exp_per_hour: u64,
}

/// The Grind Tracker's line on how Last 5 min compares with the Personal Best on the map
/// the player is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalBestLine {
    /// The best the line names, in EXP/h: the map's Personal Best from earlier Visits,
    /// fixed on arrival; on a first Visit, this Visit's best so far.
    pub exp_per_hour: u64,
    /// No earlier Visit had a best here: "Your first best EXP/h here".
    pub first: bool,
    /// Last 5 min's EXP/h as a share of the earlier Visits' best, in hundredths of a
    /// percent: 9200 for "92%".
    pub share_hundredths: Option<i64>,
    /// This Visit has beaten the earlier Visits' best.
    pub new_best: bool,
}

/// Every map's Visit bests, and the Visit under way's.
#[derive(Debug, Default)]
pub(crate) struct Bests {
    /// The best of each Visit that has ended.
    earlier: SavedBests,
    /// The Visit under way's best on each map it was certain on: one, unless the player
    /// chose among Candidates.
    visit: Vec<(String, u64)>,
    /// The map the Visit under way was last certain on: the one its best is for. The
    /// hotkey goes past the other Candidates on the way to the one the player chooses.
    on: Option<String>,
}

impl Bests {
    /// Last 5 min was shown afresh with the player certain to be on `map`. `rate` is its
    /// EXP/h if it counts towards this Visit's best there: over a full 5 minutes of
    /// active play all on the map, with the EXP bar read.
    pub fn shown(&mut self, map: &str, rate: Option<u64>) {
        if self.on.as_deref() != Some(map) {
            self.on = Some(map.to_string());
        }
        let Some(rate) = rate else { return };
        match self.visit.iter_mut().find(|(id, _)| id == map) {
            Some((_, best)) => *best = (*best).max(rate),
            None => self.visit.push((map.to_string(), rate)),
        }
    }

    /// The Visit under way ended: its best joins its map's.
    pub fn end_visit(&mut self) {
        let on = self.on.take();
        let Some((map, exp_per_hour)) = self.visit.drain(..).find(|(id, _)| Some(id) == on.as_ref()) else { return };
        let maps = &mut self.earlier.maps;
        let at = match maps.iter().position(|m| m.id == map) {
            Some(at) => at,
            None => {
                maps.push(MapBests { id: map, visits: Vec::new() });
                maps.len() - 1
            }
        };
        let visits = &mut maps[at].visits;
        let below = visits.iter().position(|v| v.exp_per_hour < exp_per_hour).unwrap_or(visits.len());
        visits.insert(below, VisitBest { exp_per_hour });
    }

    /// The line for `map`, where `share` gives Last 5 min's share of a best, if it is
    /// over a full 5 minutes there; `None` while there is nothing to compare with.
    pub fn line(&self, map: &str, share: impl FnOnce(u64) -> Option<i64>) -> Option<PersonalBestLine> {
        let this = self.visit.iter().find(|(id, _)| id == map).map(|&(_, best)| best);
        let earlier = self.earlier.maps.iter().find(|m| m.id == map).and_then(|m| m.visits.first());
        match earlier {
            Some(best) => Some(PersonalBestLine {
                exp_per_hour: best.exp_per_hour,
                first: false,
                share_hundredths: share(best.exp_per_hour),
                // A best this Visit set stays its best, whatever the rate does after.
                new_best: this.is_some_and(|this| this > best.exp_per_hour),
            }),
            None => this.map(|exp_per_hour| PersonalBestLine {
                exp_per_hour,
                first: true,
                share_hundredths: None,
                new_best: false,
            }),
        }
    }
}
