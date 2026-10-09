//! Personal Bests (GLOSSARY): the best Rolling Rate the player has had on each map, kept
//! Visit by Visit.
//!
//! - A Visit's best on a map is the highest Last 5 min EXP/h the Grind Tracker showed at
//!   a refresh whose window was a full 5 minutes of active play, all on the map, with the
//!   EXP bar read: the Session decides what counts. Only while the map is certain and has
//!   a Map Summary: the core says which map that is.
//! - A map's Personal Best is the best of its Visits'. During a Visit the rate is
//!   compared with the earlier Visits' best, which can't change until the Visit ends:
//!   this Visit's best joins them then. Nothing is capped or rejected.
//! - A Visit's best is for the map it was last certain on: the player may choose among
//!   Candidates, and the hotkey goes past the others on the way.
//! - A best is dated by the wall clock at the refresh that first showed it, for the line's
//!   hover to say when it was set. The UI words the date, in the player's local time.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisitBest {
    /// The Last 5 min EXP/h shown, over a full 5 minutes of active play on the map.
    pub exp_per_hour: u64,
    /// When it was first shown, by the wall clock (ms since the Unix epoch); `None` if
    /// the shell hadn't told the wall clock by then.
    pub set_at_unix_ms: Option<u64>,
}

/// The Grind Tracker's line on how Last 5 min compares with the Personal Best on the map
/// the player is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalBestLine {
    /// The best the line names, in EXP/h: the map's Personal Best from earlier Visits,
    /// fixed on arrival; on a first Visit, this Visit's best so far.
    pub exp_per_hour: u64,
    /// When that best was set, by the wall clock (ms since the Unix epoch), for the
    /// line's hover to say ("set today"); `None` if the shell hadn't told the wall clock.
    pub set_at_unix_ms: Option<u64>,
    /// No earlier Visit had a best here: "Your first best EXP/h here".
    pub first: bool,
    /// Last 5 min's EXP/h as a share of the earlier Visits' best, in hundredths of a
    /// percent: 9200 for "92%". `None` on a first Visit, until Last 5 min is a full 5
    /// minutes of active play here, or while its rate is zero or negative.
    pub share_hundredths: Option<i64>,
    /// This Visit's best, once it has beaten the earlier Visits': the "new best" tag, for
    /// the rest of the Visit, whatever the rate does after, and the line's hover shows it.
    /// `None` on a first Visit, whose best the line names.
    pub new_best: Option<VisitBest>,
}

/// Every map's Visit bests, and the Visit under way's.
#[derive(Debug, Default)]
pub(crate) struct Bests {
    /// The best of each Visit that has ended.
    earlier: SavedBests,
    /// The Visit under way's best on each map it was certain on: one, unless the player
    /// chose among Candidates.
    visit: Vec<(String, VisitBest)>,
    /// The map the Visit under way was last certain on: the one its best is for. The
    /// hotkey goes past the other Candidates on the way to the one the player chooses.
    on: Option<String>,
}

impl Bests {
    /// Last 5 min was shown afresh with the player certain to be on `map`, at
    /// `now_unix_ms` by the wall clock. `rate` is its EXP/h if it counts towards this
    /// Visit's best there: over a full 5 minutes of active play all on the map, with the
    /// EXP bar read. A best is set when it is first shown: the same rate again keeps it.
    pub fn shown(&mut self, map: &str, rate: Option<u64>, now_unix_ms: Option<u64>) {
        if self.on.as_deref() != Some(map) {
            self.on = Some(map.to_string());
        }
        let Some(rate) = rate else { return };
        let shown = VisitBest { exp_per_hour: rate, set_at_unix_ms: now_unix_ms };
        match self.visit.iter_mut().find(|(id, _)| id == map) {
            Some((_, best)) if best.exp_per_hour >= rate => {}
            Some((_, best)) => *best = shown,
            None => self.visit.push((map.to_string(), shown)),
        }
    }

    /// The Visit under way ended: its best joins its map's.
    pub fn end_visit(&mut self) {
        let on = self.on.take();
        let Some((map, best)) = self.visit.drain(..).find(|(id, _)| Some(id) == on.as_ref()) else { return };
        let maps = &mut self.earlier.maps;
        let at = match maps.iter().position(|m| m.id == map) {
            Some(at) => at,
            None => {
                maps.push(MapBests { id: map, visits: Vec::new() });
                maps.len() - 1
            }
        };
        let visits = &mut maps[at].visits;
        let below = visits.iter().position(|v| v.exp_per_hour < best.exp_per_hour).unwrap_or(visits.len());
        visits.insert(below, best);
    }

    /// The line for `map`, where `share` gives Last 5 min's share of a best, if it is
    /// over a full 5 minutes there; `None` while there is nothing to compare with.
    pub fn line(&self, map: &str, share: impl FnOnce(u64) -> Option<i64>) -> Option<PersonalBestLine> {
        let this = self.visit.iter().find(|(id, _)| id == map).map(|&(_, best)| best);
        let earlier = self.earlier.maps.iter().find(|m| m.id == map).and_then(|m| m.visits.first());
        match earlier {
            Some(best) => Some(PersonalBestLine {
                exp_per_hour: best.exp_per_hour,
                set_at_unix_ms: best.set_at_unix_ms,
                first: false,
                share_hundredths: share(best.exp_per_hour),
                new_best: this.filter(|this| this.exp_per_hour > best.exp_per_hour),
            }),
            None => this.map(|this| PersonalBestLine {
                exp_per_hour: this.exp_per_hour,
                set_at_unix_ms: this.set_at_unix_ms,
                first: true,
                share_hundredths: None,
                new_best: None,
            }),
        }
    }

    /// Every map's Visit bests, for the shell to save.
    pub fn saved(&self) -> SavedBests {
        self.earlier.clone()
    }

    /// Take back the bests an earlier launch saved, in place of any there are.
    pub fn restore(&mut self, saved: SavedBests) {
        self.earlier = saved;
    }
}
