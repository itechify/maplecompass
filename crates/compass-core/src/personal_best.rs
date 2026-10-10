//! Personal Bests (GLOSSARY): the best Rolling Rate the player has had on each map, kept
//! Visit by Visit.
//!
//! - A Visit's best on a map is the highest Rolling Rate, in EXP/h, the Grind Tracker
//!   showed at a refresh whose window was a full 5 minutes of active play, all on the map,
//!   with the EXP bar read: the Session decides what counts. Only while the map is
//!   certain and has a Map Summary: the core says which map that is. Maps that share a
//!   title but would show the same count as certain, as the first of them, which the
//!   panel shows: their bests are kept under its ID.
//! - A map's Personal Best is the best of its Visits'. The Grind Tracker's Best row shows
//!   the best of the earlier Visits, the map's history, from arriving: it can't change
//!   until the Visit ends, when this Visit's best joins them, unless the player removes
//!   it. No rate is capped or rejected.
//! - A Visit's best is for the map it was last certain on with a rate to count. The
//!   player may choose among Candidates, and the hotkey goes past the others on the way.
//!   One shown with nothing to count yet (just after a reset, say) doesn't take the
//!   Visit's best, even if it had one of its own earlier in the Visit, so closing
//!   MapleCompass then keeps it for the map it came from. A Candidate's own best from
//!   earlier in the Visit counts again once it has a rate to count again.
//! - A best is dated by the wall clock at the refresh that first showed it, and keeps
//!   the date in the history.
//! - Bests are per map (MeowDB map ID), for every character: nothing about the Session
//!   touches them.
//! - Each map keeps its 10 best Visits; one that doesn't make them is dropped.
//! - They outlive the launch, as the saved world does: the core hands them to the shell
//!   as [`SavedBests`], with a revision so it saves them only when they change, and takes
//!   them back at startup. What it hands out includes the Visit under way's best, as if
//!   it had ended: closing MapleCompass ends it.
//! - The player can remove the best the Best row shows from the history, one that came
//!   from a bug. The map's next-best Visit's takes over the row. There is no undo. This
//!   Visit's best can't be removed: it joins the history when the Visit ends, and can be
//!   removed then.

use serde::{Deserialize, Serialize};

/// The shape [`SavedBests`] has now. Bumped whenever it changes.
pub const BESTS_VERSION: u32 = 1;

/// How many Visits' bests each map keeps: the best of them.
const VISITS_KEPT: usize = 10;

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
    /// The Rolling Rate's EXP/h shown, over a full 5 minutes of active play on the map.
    pub exp_per_hour: u64,
    /// When it was first shown, by the wall clock (ms since the Unix epoch); `None` if
    /// the shell hadn't told the wall clock by then.
    pub set_at_unix_ms: Option<u64>,
}

/// Every map's Visit bests, and the Visit under way's.
#[derive(Debug, Default)]
pub(crate) struct Bests {
    /// The best of each Visit that has ended.
    earlier: SavedBests,
    /// The Visit under way's best on each map it was certain on with a rate to count: one,
    /// unless the player chose among Candidates. Only `on`'s is the Visit's best.
    visit: Vec<(String, VisitBest)>,
    /// The map the Visit under way was last certain on with a rate to count: the one its
    /// best is for, and the only one [`Bests::saved`] hands out a best from this Visit
    /// for. The hotkey goes past the other Candidates on the way to the one the player
    /// chooses; one it shows with nothing to count doesn't take the best, even if it had
    /// one of its own earlier in the Visit.
    on: Option<String>,
    /// Changes whenever what [`Bests::saved`] hands out does.
    revision: u64,
}

impl Bests {
    /// The Rolling Rate was shown afresh with the player certain to be on `map`, at
    /// `now_unix_ms` by the wall clock. `rate` is its EXP/h if it counts towards this
    /// Visit's best there: over a full 5 minutes of active play all on the map, with the
    /// EXP bar read. A best is set when it is first shown: the same rate again keeps it.
    pub fn on_shown(&mut self, map: &str, rate: Option<u64>, now_unix_ms: Option<u64>) {
        self.changing(|bests| bests.record(map, rate, now_unix_ms));
    }

    /// As [`Bests::on_shown`], leaving the revision to it.
    fn record(&mut self, map: &str, rate: Option<u64>, now_unix_ms: Option<u64>) {
        // A Candidate shown with nothing to count doesn't take the Visit's best, even one
        // with a best of its own from earlier in the Visit.
        let Some(rate) = rate else { return };
        if self.on.as_deref() != Some(map) {
            self.on = Some(map.to_string());
        }
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
        join(&mut self.earlier, map, best);
    }

    /// The Visit under way's best on the map it is for, if it has one.
    fn this_visit(&self) -> Option<(&String, VisitBest)> {
        let on = self.on.as_ref()?;
        self.visit.iter().find(|(id, _)| id == on).map(|(id, best)| (id, *best))
    }

    /// Do `change`, moving the revision on if it changed what [`Bests::saved`] hands out.
    fn changing(&mut self, change: impl FnOnce(&mut Self)) {
        let before = self.saved();
        change(self);
        if self.saved() != before {
            self.revision += 1;
        }
    }

    /// `map`'s Personal Best from its history, the best of the earlier Visits, if it has
    /// one: what the Best row shows.
    pub fn history_best(&self, map: &str) -> Option<VisitBest> {
        self.earlier.maps.iter().find(|m| m.id == map).and_then(|m| m.visits.first()).copied()
    }

    /// Remove `map`'s Personal Best from its history, the best of the earlier Visits. The
    /// map's next-best Visit's takes over. This Visit's best isn't in the history until
    /// the Visit ends.
    pub fn remove(&mut self, map: &str) {
        self.changing(|bests| {
            let maps = &mut bests.earlier.maps;
            let Some(at) = maps.iter().position(|m| m.id == map) else { return };
            if !maps[at].visits.is_empty() {
                maps[at].visits.remove(0);
            }
            if maps[at].visits.is_empty() {
                maps.remove(at);
            }
        });
    }

    /// Every map's Visit bests, for the shell to save: the Visit under way's as if it
    /// ended now, since closing MapleCompass ends it.
    pub fn saved(&self) -> SavedBests {
        let mut saved = self.earlier.clone();
        if let Some((map, best)) = self.this_visit() {
            join(&mut saved, map.clone(), best);
        }
        saved
    }

    /// Take back the bests an earlier launch saved, in place of any there are. Bests
    /// saved in another shape are left, and so is a best of no EXP, which no Visit sets
    /// (the file was edited): nothing is measured against it.
    pub fn restore(&mut self, mut saved: SavedBests) {
        if saved.version != BESTS_VERSION {
            return;
        }
        for map in &mut saved.maps {
            map.visits.retain(|best| best.exp_per_hour > 0);
        }
        saved.maps.retain(|map| !map.visits.is_empty());
        self.changing(|bests| bests.earlier = saved);
    }

    /// Changes whenever what [`Bests::saved`] hands out does: a best removed from the
    /// history; the Visit under way's best set, raised, or moved to a Candidate chosen that
    /// counts a rate, where it makes its map's [`VISITS_KEPT`]; bests restored. A Visit
    /// ending changes nothing handed out.
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

/// A Visit's `best` on `map` joins the map's, highest first; one that doesn't make the
/// map's [`VISITS_KEPT`] is dropped.
fn join(bests: &mut SavedBests, map: String, best: VisitBest) {
    let visits = visits_of(bests, map);
    let below = visits.iter().position(|v| v.exp_per_hour < best.exp_per_hour).unwrap_or(visits.len());
    visits.insert(below, best);
    visits.truncate(VISITS_KEPT);
}

/// `map`'s Visit bests, highest first: none yet if it had none.
fn visits_of(bests: &mut SavedBests, map: String) -> &mut Vec<VisitBest> {
    let maps = &mut bests.maps;
    let at = match maps.iter().position(|m| m.id == map) {
        Some(at) => at,
        None => {
            maps.push(MapBests { id: map, visits: Vec::new() });
            maps.len() - 1
        }
    };
    &mut maps[at].visits
}
