//! Personal Bests (GLOSSARY): the best Rolling Rate the player has had on each map, kept
//! Visit by Visit.
//!
//! - A Visit's best on a map is the highest Rolling Rate, in EXP/h, the Grind Tracker
//!   showed at a refresh whose window was a full 5 minutes of active play, all on the map,
//!   with the EXP bar read: the Session decides what counts. Only while the map is
//!   certain and has a Map Summary: the core says which map that is. Maps that share a
//!   title but would show the same count as certain, as the first of them, which the
//!   panel shows: their bests are kept under its ID.
//! - A map's Personal Best is the best of its Visits'. During a Visit the rate is
//!   compared with the earlier Visits' best, which can't change until the Visit ends:
//!   this Visit's best joins them then. No rate is capped or rejected.
//! - A Visit's best is for the map it was last certain on with a rate to count, and only
//!   that map's line and Forget see it: on any other they see only the earlier Visits'.
//!   The player may choose among Candidates, and the hotkey goes past the others on the
//!   way. One shown with nothing to count yet (just after a reset, say) doesn't take the
//!   Visit's best, even if it had one of its own earlier in the Visit, so closing
//!   MapleCompass then keeps it for the map it came from. A Candidate's own best from
//!   earlier in the Visit counts again once it has a rate to count again.
//! - A best is dated by the wall clock at the refresh that first showed it, for the line's
//!   hover to say when it was set. The UI words the date, in the player's local time.
//! - Bests are per map (MeowDB map ID), for every character: nothing about the Session
//!   touches them.
//! - Each map keeps its 10 best Visits; one that doesn't make them is dropped.
//! - They outlive the launch, as the saved world does: the core hands them to the shell
//!   as [`SavedBests`], with a revision so it saves them only when they change, and takes
//!   them back at startup. What it hands out includes the Visit under way's best, as if
//!   it had ended: closing MapleCompass ends it.
//! - The player can forget a best that came from a bug: the higher of the two the line's
//!   hover shows, this Visit's once it has beaten the earlier Visits', theirs otherwise.
//!   The map's next-best Visit's takes over. After forgetting this Visit's, only windows
//!   that start after the Forget count: the Session sees to that. Undo brings the best
//!   back until the Visit ends.

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

/// The Grind Tracker's line on how the Rolling Rate compares with the Personal Best on the
/// map the player is on, and what its hover holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalBestLine {
    /// The best the line names, and how the Rolling Rate compares with it.
    #[serde(flatten)]
    pub named: BestNamed,
    /// Whose best the hover's Forget takes: the higher of the two the hover shows.
    pub forgets: Whose,
}

/// The best the Personal Best line names: the earlier Visits', or on a first Visit, this
/// one's. Each best is dated for the line's hover to say when it was set ("set today").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum BestNamed {
    /// No earlier Visit had a best here: "Your first best EXP/h here: …", this Visit's
    /// best so far.
    FirstVisit { best: VisitBest },
    /// The map's Personal Best from earlier Visits, fixed on arrival: "Your best EXP/h
    /// here: …", or "N% of your best EXP/h here (…)".
    EarlierVisits {
        best: VisitBest,
        /// The Rolling Rate's EXP/h as a share of `best`, in hundredths of a percent: 9200
        /// for "92%". `None` until the Rolling Rate is over a full 5 minutes of active
        /// play here, or while its rate is zero or negative.
        share_hundredths: Option<i64>,
        /// This Visit's best, once it has beaten `best`: the "new best" tag, for the rest
        /// of the Visit, whatever the rate does after, and the line's hover shows it.
        new_best: Option<VisitBest>,
    },
}

/// Whose best on a map: this Visit's, or the earlier Visits'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Whose {
    ThisVisit,
    EarlierVisits,
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
    /// best is for, and the only one whose line, Forget and [`Bests::saved`] see a best
    /// from this Visit. The hotkey goes past the other Candidates on the way to the one
    /// the player chooses; one it shows with nothing to count doesn't take the best, even
    /// if it had one of its own earlier in the Visit.
    on: Option<String>,
    /// Changes whenever what [`Bests::saved`] hands out does.
    revision: u64,
    /// The best the player last forgot, until the Visit ends: Undo brings it back.
    forgotten: Option<Forgotten>,
}

/// A best the player forgot, to bring back if they undo it.
#[derive(Debug)]
struct Forgotten {
    /// The map it was on.
    map: String,
    best: VisitBest,
    whose: Whose,
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

    /// The Visit under way ended: its best joins its map's, and a Forget stands.
    pub fn end_visit(&mut self) {
        self.forgotten = None;
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
    fn changing<T>(&mut self, change: impl FnOnce(&mut Self) -> T) -> T {
        let before = self.saved();
        let changed = change(self);
        if self.saved() != before {
            self.revision += 1;
        }
        changed
    }

    /// This Visit's best on `map`, if the Visit's best is for `map` and it has one there:
    /// on any other map, none, whatever it had there earlier in the Visit.
    fn this_visit_on(&self, map: &str) -> Option<VisitBest> {
        self.this_visit().filter(|(id, _)| *id == map).map(|(_, best)| best)
    }

    /// `map`'s Personal Best from earlier Visits, if it has one.
    fn earlier_on(&self, map: &str) -> Option<VisitBest> {
        self.earlier.maps.iter().find(|m| m.id == map).and_then(|m| m.visits.first()).copied()
    }

    /// Whose is the higher of `map`'s Personal Best from earlier Visits and this Visit's
    /// best there: this Visit's once it has beaten the other, or with no other.
    fn higher(&self, map: &str) -> Option<Whose> {
        match (self.this_visit_on(map), self.earlier_on(map)) {
            (Some(this), Some(earlier)) if this.exp_per_hour > earlier.exp_per_hour => Some(Whose::ThisVisit),
            (_, Some(_)) => Some(Whose::EarlierVisits),
            (Some(_), None) => Some(Whose::ThisVisit),
            (None, None) => None,
        }
    }

    /// Forget the higher of `map`'s Personal Best and this Visit's best there, as the line
    /// says. The next-best Visit's takes over. It can be undone until the Visit ends, or
    /// the next Forget. Whose it was, if there was one.
    pub fn forget(&mut self, map: &str) -> Option<Whose> {
        self.changing(|bests| bests.forget_higher(map))
    }

    /// As [`Bests::forget`], leaving the revision to it.
    fn forget_higher(&mut self, map: &str) -> Option<Whose> {
        let whose = self.higher(map)?;
        let best = match whose {
            Whose::ThisVisit => {
                let at = self.visit.iter().position(|(id, _)| id == map)?;
                self.visit.remove(at).1
            }
            Whose::EarlierVisits => {
                let at = self.earlier.maps.iter().position(|m| m.id == map)?;
                let best = self.earlier.maps[at].visits.remove(0);
                if self.earlier.maps[at].visits.is_empty() {
                    self.earlier.maps.remove(at);
                }
                best
            }
        };
        self.forgotten = Some(Forgotten { map: map.to_string(), best, whose });
        Some(whose)
    }

    /// Bring back the best the player last forgot, unless the Visit has ended since: an
    /// earlier Visit's in front again, this Visit's unless a higher one was set since.
    /// Whose it was, if there was one.
    pub fn undo_forget(&mut self) -> Option<Whose> {
        self.changing(Self::bring_back)
    }

    /// As [`Bests::undo_forget`], leaving the revision to it.
    fn bring_back(&mut self) -> Option<Whose> {
        let Forgotten { map, best, whose } = self.forgotten.take()?;
        match whose {
            Whose::ThisVisit => match self.visit.iter_mut().find(|(id, _)| *id == map) {
                Some((_, since)) if since.exp_per_hour > best.exp_per_hour => {}
                Some((_, since)) => *since = best,
                None => self.visit.push((map, best)),
            },
            Whose::EarlierVisits => {
                let visits = visits_of(&mut self.earlier, map);
                // It was the highest: in front of any equal to it too, as before.
                let at = visits.iter().position(|v| v.exp_per_hour <= best.exp_per_hour).unwrap_or(visits.len());
                visits.insert(at, best);
            }
        }
        Some(whose)
    }

    /// Whether there is a Forget to undo.
    pub fn forget_undoable(&self) -> bool {
        self.forgotten.is_some()
    }

    /// The line for `map`, where `share` gives the Rolling Rate's share of a best, if it
    /// is over a full 5 minutes there; `None` while there is nothing to compare with.
    pub fn line(&self, map: &str, share: impl FnOnce(u64) -> Option<i64>) -> Option<PersonalBestLine> {
        let forgets = self.higher(map)?;
        let this = self.this_visit_on(map);
        let named = match self.earlier_on(map) {
            Some(best) => BestNamed::EarlierVisits {
                best,
                share_hundredths: share(best.exp_per_hour),
                new_best: this.filter(|_| forgets == Whose::ThisVisit),
            },
            None => BestNamed::FirstVisit { best: this? },
        };
        Some(PersonalBestLine { named, forgets })
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

    /// Changes whenever what [`Bests::saved`] hands out does, and only then: the Visit
    /// under way's best set, raised, or moved to a Candidate chosen that counts a rate,
    /// where it makes its map's [`VISITS_KEPT`]; a best forgotten or brought back; bests
    /// restored. A Visit ending changes nothing handed out.
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
