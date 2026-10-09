//! The Session (ADR 0005): EXP bar readings, with their capture times, turned into Time
//! to Level and EXP rates. There is no clock here and no EXP table.
//!
//! - The level's total EXP comes from the bar itself: the game rounds the percentage to
//!   hundredths, so each reading allows a range of totals ("149 [0.89%]": 16,649 to
//!   16,836). The range learned is what every reading in the level allows; it narrows
//!   as the percentage grows.
//! - A reading that fits the range is the same level: less EXP than before is a death.
//!   One that doesn't fit is a level-up if it allows only bigger totals and the bar was
//!   near 100%, a misread otherwise, unless [`SWITCH_READINGS`] in a row fit another
//!   level. If they carry on from the last reading taken, and their level is within
//!   [`RELEARN_PERCENT`] of the one learned, the level was learned wrong (a misread that
//!   fitted it narrowed the range past the true total): it is learned anew from them,
//!   and the Session goes on. Otherwise it is a character switch, which starts a new
//!   Session. A reading barely into the next level may fit the range too (2 [0.01%]
//!   allows a wide range of totals); when the bar went from near 100% to as near 0%, it
//!   is a level-up all the same.
//! - The Session starts on the first EXP change, from the EXP the bar shows then.
//! - Active time runs while the EXP keeps changing. A gap of up to [`IDLE_MS`] between
//!   changes counts; a longer one is a break, which pauses the Session and counts for
//!   nothing, from the last change to the next, so the rates hold while paused. Once
//!   the Session has warmed up it stays so, breaks or not.
//! - A bar that can't be read isn't a break: nothing is known of the EXP then. Once it
//!   has gone [`UNREADABLE_MS`] without reading whole the Session says so, its figures
//!   held. If it reads again within [`IDLE_MS`] of the last whole reading, the stretch
//!   was a gap like any other; after longer, the stretch counts for nothing, and neither
//!   does whatever the EXP did over it: the next reading is taken without a change. (No
//!   frames at all, as while the game window is minimised, is no evidence either way.)
//! - The player's reset can be undone until the EXP next changes, which starts the new
//!   Session; a character switch ends the old one for good.
//! - Each map the panel confirms starts a Visit (GLOSSARY), which Sessions don't end:
//!   the share of the map's solo ceiling counts only the play since. The title going
//!   unread for a while doesn't end it, nor does the map's page coming later, nor the
//!   player choosing among Candidates (they haven't moved).
//! - The figures shown hold still for each [`REFRESH_MS`] of capture time, rather than
//!   jumping with every kill. Events show at once: the state changing (the Session
//!   starting, warming up, pausing, resuming, or the bar going unread), the first figures
//!   there are, a death, a level-up, a reset or its undoing, a character switch, a new
//!   Visit, and a map with another ceiling or Personal Best. The bar's last reading shows
//!   as it comes.
//! - Last 5 min as shown afresh, over a full 5 minutes of the Visit's play, counts towards
//!   the Visit's Personal Best on the map (see [`crate::personal_best`]), unless the bar
//!   can't be read. Bests outlast Sessions, as Visits do.

use crate::ExpBar;
use crate::personal_best::{Bests, PersonalBestLine};
use serde::Serialize;

/// What EXP tracking shows: everything the panel's tracking section and the collapsed
/// icon's badge render.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpTracking {
    /// Off, waiting, warming up, active or paused.
    pub state: TrackingState,
    /// Time to Level in seconds at the net Rolling Rate, rounded more coarsely the
    /// further off it is (see [`rounded_time_to_level`]); `None` ("—") while there is
    /// no rate yet, the net rate is zero or negative, or the level's total isn't known.
    pub time_to_level_s: Option<u64>,
    /// Time to Level, short, for the headline and the collapsed icon's badge: "47m",
    /// "3h 10m", "14h".
    pub time_to_level_badge: Option<String>,
    /// The Rolling Rate: over the last 5 minutes of active play. `None` until the
    /// Session has some active time.
    pub rolling: Option<ExpRate>,
    /// The average over the whole Session's active time. `None` as for `rolling`.
    pub session_average: Option<ExpRate>,
    /// The Rolling Rate's EXP/h on the shown map, as a share of its MeowDB solo EXP/h
    /// ceiling (its Map Summary), in hundredths of a percent: 7300 for "73%". It counts
    /// only the play since arriving on the map (the Visit's), once there has been
    /// [`WARM_UP_MS`] of it, so one map's EXP is never measured against another's
    /// ceiling, the same or not. `None` without a recognised map with a Map Summary,
    /// until then, or while the rate there is zero or negative.
    pub ceiling_share_hundredths: Option<i64>,
    /// The ceiling the share is of: the shown map's MeowDB solo EXP/h, while there is a
    /// share.
    pub solo_exp_ceiling: Option<u64>,
    /// Deaths this Session and what they cost: "deaths: 2 (−1.4%)".
    pub deaths: Deaths,
    /// The Session started afresh by itself, on a switch to another character, and
    /// hasn't warmed up since: the panel says why the figures went. Never after a reset
    /// the player made.
    pub character_switched: bool,
    /// The Session's active time so far, in whole seconds; 0 while waiting.
    pub active_s: u64,
    /// The active time the Rolling Rate covers, in whole seconds: the Session's, up to 5
    /// minutes. Once `active_s` is more, the Session average is over a longer stretch and
    /// says something the Rolling Rate doesn't.
    pub rolling_s: u64,
    /// The player reset the Session and the EXP hasn't changed since: the reset can be
    /// undone, bringing the old Session back.
    pub reset_undoable: bool,
    /// The last EXP bar reading taken, as the bar shows it; it changes with every
    /// reading rather than holding with the figures, so the player sees it being read.
    /// `None` until one is.
    pub bar: Option<ExpBar>,
    /// How Last 5 min compares with the Personal Best on the shown map, while it is
    /// certain and has a Map Summary; `None` there until there is a best to name.
    pub personal_best: Option<PersonalBestLine>,
}

/// The deaths in a Session: drops in EXP within the level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deaths {
    /// How many deaths.
    pub count: u32,
    /// The EXP they cost, all told.
    pub exp_lost: u64,
    /// The EXP lost in hundredths of a percent of the level, as the bar showed it.
    pub percent_lost_hundredths: u32,
}

/// A net EXP rate: gains less what deaths cost. Negative when deaths cost more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpRate {
    /// Net EXP per hour of active time.
    pub exp_per_hour: i64,
    /// The same rate in hundredths of a percent of the current level per hour: 125 for
    /// "1.25%/h". `None` while the level's total isn't known (only 0.00% read yet).
    pub percent_per_hour_hundredths: Option<i64>,
}

/// Where the Session is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackingState {
    /// No EXP Bar Region to read: tracking is off, or there is no game window.
    #[default]
    Off,
    /// Tracking is on, but the EXP hasn't changed yet: the Session starts when it does.
    Waiting,
    /// The first [`WARM_UP_MS`] of active time: the rates are still jumpy.
    WarmingUp,
    /// Warmed up (once is enough), and the EXP changed within the last [`IDLE_MS`].
    Active,
    /// No EXP change for more than [`IDLE_MS`]: the rates hold until the next change.
    Paused,
    /// The EXP bar hasn't read whole for [`UNREADABLE_MS`]: something covers it, or the
    /// region no longer frames its figures. Whatever the Session was doing, its figures
    /// hold until the bar reads again.
    Unreadable,
}

/// A gap between EXP changes longer than this is a break: it pauses the Session.
const IDLE_MS: u64 = 2 * 60_000;
/// The EXP bar going this long without reading whole is a problem to tell the player
/// about, not a moment's cover (the pointer passing over it).
const UNREADABLE_MS: u64 = 10_000;
/// A Session warms up for this much active time.
const WARM_UP_MS: u64 = 60_000;
/// The Rolling Rate covers this much active time.
const ROLLING_MS: u64 = 5 * 60_000;
/// An hour, which the rates are per.
const HOUR_MS: u64 = 3_600_000;
/// 100% in hundredths of a percent, the unit of the bar and of every share here.
const HUNDRED_PERCENT: u32 = 10_000;
/// Readings in a row that fit a level of their own, and not the one learned, that make
/// a character switch (or show the level was learned wrong). A misread has been seen to
/// repeat on two frames.
const SWITCH_READINGS: u32 = 3;
/// Readings that fit a level of their own, carrying on from the last reading taken,
/// are the same level learned anew if their level is within this many percent of the
/// one learned. A misread narrows the range by far less, and another character's level
/// is apart by more unless it is the next one up or down; at that, its bar would have
/// to be where the last reading left off.
const RELEARN_PERCENT: u128 = 10;
/// A level-up needs the bar within at least this many hundredths of 100% before it.
const NEAR_LEVEL_UP_HUNDREDTHS: u32 = 1000;
/// The figures shown are measured afresh once in each stretch of this much capture time
/// (counted from capture time 0), unless an event shows them sooner.
const REFRESH_MS: u64 = 30_000;

/// The level's total EXP as far as readings tell: `lo..=hi`, `hi` unbounded
/// (`u64::MAX`) while only 0.00% has been read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Total {
    lo: u64,
    hi: u64,
}

impl Total {
    /// The totals a reading allows: `exp / total` rounds to the percentage shown, and
    /// the level holds more than `exp`. `None` if no total does (a misread).
    fn of(bar: ExpBar) -> Option<Total> {
        // `exp * HUNDRED_PERCENT / total` is within 0.5 of `percent`: doubled, to stay
        // whole.
        let doubled_exp = u128::from(bar.exp) * 2 * u128::from(HUNDRED_PERCENT);
        let percent = u128::from(bar.percent_hundredths);
        let lo = doubled_exp.div_ceil(2 * percent + 1).max(u128::from(bar.exp) + 1);
        let hi = if percent == 0 { u128::from(u64::MAX) } else { doubled_exp / (2 * percent - 1) };
        (lo <= hi).then(|| Total { lo: lo as u64, hi: hi.min(u128::from(u64::MAX)) as u64 })
    }

    /// The totals both allow, if any.
    fn meet(self, other: Total) -> Option<Total> {
        let t = Total { lo: self.lo.max(other.lo), hi: self.hi.min(other.hi) };
        (t.lo <= t.hi).then_some(t)
    }

    /// Twice the best estimate of the total, the middle of the range, so that it stays
    /// whole; `None` while unbounded.
    fn doubled_midpoint(self) -> Option<u64> {
        (self.hi != u64::MAX).then(|| self.lo + self.hi)
    }

    /// Whether `other`, which shares no total with this, is within [`RELEARN_PERCENT`]
    /// of it.
    fn close_to(self, other: Total) -> bool {
        let (lower, upper) = if other.lo > self.hi { (self, other) } else { (other, self) };
        u128::from(upper.lo) * 100 <= u128::from(lower.hi) * (100 + RELEARN_PERCENT)
    }
}

/// Readings in a row that don't fit the level learned.
#[derive(Debug, Clone, Copy)]
struct Misfits {
    /// The totals they all allow.
    total: Total,
    /// How many there are.
    count: u32,
}

/// The Session and what it has learned about the level from the EXP bar.
#[derive(Debug, Default)]
pub(crate) struct Session {
    /// The level's total, learned from the readings accepted in this level.
    total: Option<Total>,
    /// The last reading accepted.
    last: Option<ExpBar>,
    /// The readings rejected since the last one accepted, which may be another
    /// character's level.
    misfits: Option<Misfits>,
    /// The largest gain between two readings in this Session, in hundredths of a
    /// percent: how near 100% the bar must have been for a level-up.
    largest_gain_hundredths: u32,
    /// The latest time seen, from a reading or a tick.
    now: u64,
    /// The figures since the Session started; `None` while waiting.
    figures: Option<Figures>,
    /// Level-ups this Session.
    level_ups: u32,
    /// This Session started on a character switch, not the player's reset.
    character_switched: bool,
    /// What the panel shows, and what it was measured for; `None` until it is first
    /// measured, and after a reset.
    shown: Option<Shown>,
    /// When the EXP bar last read whole; `None` before it ever has.
    read_at: Option<u64>,
    /// The first frame since then that didn't read whole; `None` while the bar reads.
    unread_since: Option<u64>,
    /// The Session the player's reset ended, until the EXP next changes.
    undo: Option<Undo>,
    /// The Visit under way. It outlasts Sessions: a reset or a character switch doesn't
    /// move the player.
    visit: Visit,
    /// Every map's Personal Bests, which outlast Sessions too.
    bests: Bests,
}

/// A Visit (GLOSSARY): from the panel confirming a map until it confirms another.
#[derive(Debug, Default, Clone, Copy)]
struct Visit {
    /// Which it is: one more with each map the panel confirms, none before the first.
    count: u64,
    /// When it started: the latest capture time seen then.
    at: u64,
}

/// What the player's reset set aside, to bring back if they undo it.
#[derive(Debug)]
struct Undo {
    figures: Figures,
    largest_gain_hundredths: u32,
    level_ups: u32,
    character_switched: bool,
    /// The Visit then: on another map since, the share counts from arriving there.
    visit: u64,
}

/// The figures the panel shows, held for the rest of the [`REFRESH_MS`] stretch they
/// were measured in.
#[derive(Debug)]
struct Shown {
    /// Which stretch of capture time they were measured in.
    stretch: u64,
    /// The Visit they were measured on.
    visit: u64,
    /// The solo ceiling they were measured against.
    ceiling: Option<u64>,
    /// The map whose Personal Best they were measured against.
    map: Option<String>,
    /// The Session's level-ups by then.
    level_ups: u32,
    tracking: ExpTracking,
}

/// Where a started Session stood at an EXP change.
#[derive(Debug, Clone, Copy)]
struct Mark {
    /// Active time then.
    active: u64,
    /// Net EXP gained by then.
    gained: i64,
}

/// Net EXP gained over a stretch of active time.
#[derive(Debug, Clone, Copy)]
struct Gained {
    /// The net EXP: negative if deaths cost more than was gained.
    exp: i64,
    /// The active time it took.
    over_ms: u64,
}

impl Gained {
    /// The EXP per hour as `parts` for every `whole` EXP, rounded: EXP/h with `(1, 1)`,
    /// hundredths of a percent of a ceiling `c` with `(HUNDRED_PERCENT, c)`. `over_ms`
    /// and `whole` are more than 0.
    fn per_hour(self, parts: u64, whole: u64) -> i64 {
        let scaled = i128::from(self.exp) * i128::from(HOUR_MS) * i128::from(parts);
        div_round(scaled, i128::from(self.over_ms) * i128::from(whole)) as i64
    }

    /// As an [`ExpRate`] on a level whose doubled midpoint is `doubled_total` (if
    /// known); `None` without any active time.
    fn rate(self, doubled_total: Option<u64>) -> Option<ExpRate> {
        (self.over_ms > 0).then(|| ExpRate {
            exp_per_hour: self.per_hour(1, 1),
            // The total is doubled, so the parts are too.
            percent_per_hour_hundredths: doubled_total
                .map(|t| self.per_hour(2 * u64::from(HUNDRED_PERCENT), t)),
        })
    }
}

/// A started Session's figures.
#[derive(Debug)]
struct Figures {
    /// When the EXP last changed.
    last_change_at: u64,
    /// Active time up to `last_change_at`.
    active_at_change: u64,
    /// Net EXP gained since the start.
    gained: i64,
    /// The start and each change since, back to the last one before the Rolling
    /// Rate's window.
    marks: Vec<Mark>,
    /// The deaths so far.
    deaths: Deaths,
    /// The Session has had [`WARM_UP_MS`] of active time at some point. A break later
    /// takes active time back, but not this.
    warmed_up: bool,
    /// Active time when the Visit under way started: the share of the map's ceiling
    /// counts only the play since.
    visit_from: u64,
}

impl Figures {
    /// A Session started at `at`.
    fn new(at: u64) -> Self {
        Figures {
            last_change_at: at,
            active_at_change: 0,
            gained: 0,
            marks: vec![Mark { active: 0, gained: 0 }],
            deaths: Deaths::default(),
            warmed_up: false,
            visit_from: 0,
        }
    }

    /// Whether the Session has warmed up by `now`.
    fn warm(&self, now: u64) -> bool {
        self.warmed_up || self.active(now) >= WARM_UP_MS
    }

    /// How long the EXP has gone unchanged at `now`.
    fn idle(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_change_at)
    }

    /// Whether the player is on a break at `now`.
    fn paused(&self, now: u64) -> bool {
        self.idle(now) > IDLE_MS
    }

    /// Active time at `now`: the idle stretch since the last change counts while it is
    /// a gap in play, and not at all once it is a break.
    fn active(&self, now: u64) -> u64 {
        self.active_at_change + if self.paused(now) { 0 } else { self.idle(now) }
    }

    /// Count a change of `gain` EXP at `at`.
    fn change(&mut self, gain: i64, at: u64) {
        self.active_at_change = self.active(at);
        self.last_change_at = at;
        self.gained += gain;
        self.marks.push(Mark { active: self.active_at_change, gained: self.gained });
        let from = self.active_at_change.saturating_sub(ROLLING_MS);
        let before = self.marks.iter().rposition(|m| m.active <= from).unwrap_or(0);
        self.marks.drain(..before);
    }

    /// The net EXP gained over the Rolling Rate's window at `now`.
    fn rolling(&self, now: u64) -> Gained {
        self.rolling_since(now, 0)
    }

    /// The net EXP gained over the Rolling Rate's window at `now`, or over the part of
    /// it since active time `since`, if that is later.
    fn rolling_since(&self, now: u64, since: u64) -> Gained {
        let active = self.active(now);
        let from = active.saturating_sub(ROLLING_MS).max(since).min(active);
        let then = self.marks.iter().rev().find(|m| m.active <= from).map_or(0, |m| m.gained);
        Gained { exp: self.gained - then, over_ms: active - from }
    }

    /// The net EXP gained over the whole Session at `now`.
    fn average(&self, now: u64) -> Gained {
        Gained { exp: self.gained, over_ms: self.active(now) }
    }
}

impl Session {
    /// Take a whole EXP bar reading captured at `at_ms`. After frames that didn't read
    /// whole, for longer than [`IDLE_MS`] since the last that did, a reading of the same
    /// level, or of the next, is taken as where the Session stands now, without counting
    /// the change since the last one: no one saw how it came about. One that fits neither
    /// is weighed as ever, towards a misread or a character switch.
    pub fn on_bar(&mut self, bar: ExpBar, at_ms: u64) {
        self.tick(at_ms);
        let unseen = self.unread_since.is_some()
            && self.read_at.is_some_and(|read_at| at_ms.saturating_sub(read_at) > IDLE_MS);
        // Even a reading no level allows (a misread) shows the bar is there to read.
        self.read_at = Some(at_ms);
        self.unread_since = None;
        let Some(allowed) = Total::of(bar) else { return };
        let (Some(total), Some(last)) = (self.total, self.last) else {
            self.total = Some(allowed);
            self.last = Some(bar);
            return;
        };
        let wrapped = self.wrapped(total, last, bar, allowed);
        if !wrapped && let Some(narrowed) = total.meet(allowed) {
            if unseen {
                self.accept(narrowed, bar);
            } else {
                self.same_level(narrowed, last, bar, at_ms);
            }
        } else if let Some(doubled_total) = total.doubled_midpoint()
            && (wrapped || self.levelled_up(total, last, allowed))
        {
            // What was left of the old level, and what is in the new one.
            let gain = (doubled_total as i64 - 2 * last.exp as i64 + 1) / 2 + bar.exp as i64;
            // The new level is bigger than the old: of the totals the reading allows,
            // only those count.
            self.accept(Total { lo: allowed.lo.max(total.hi + 1), ..allowed }, bar);
            if !unseen {
                self.change(gain, at_ms);
            }
            self.level_ups += 1;
        } else {
            // A misread, or another character's level: that takes readings in a row
            // that fit it.
            let more = self.misfits.and_then(|m| Some(Misfits { total: m.total.meet(allowed)?, count: m.count + 1 }));
            let misfits = more.unwrap_or(Misfits { total: allowed, count: 1 });
            self.misfits = Some(misfits);
            if misfits.count < SWITCH_READINGS {
                // Not yet.
            } else if total.close_to(misfits.total) && carries_on(last, bar, self.near()) {
                // The level was learned wrong: the Session goes on.
                self.same_level(misfits.total, last, bar, at_ms);
            } else {
                self.accept(misfits.total, bar);
                self.reset();
                self.character_switched = true;
            }
        }
    }

    /// Take `bar`, a reading after `last` in the same level, whose total it narrows to
    /// `total`: a gain, or with less EXP, a death.
    fn same_level(&mut self, total: Total, last: ExpBar, bar: ExpBar, at_ms: u64) {
        self.accept(total, bar);
        if bar.percent_hundredths > last.percent_hundredths {
            let gain = bar.percent_hundredths - last.percent_hundredths;
            self.largest_gain_hundredths = self.largest_gain_hundredths.max(gain);
        }
        if let Some(figures) = self.change(bar.exp as i64 - last.exp as i64, at_ms)
            && bar.exp < last.exp
        {
            // Less EXP in the same level: a death.
            figures.deaths.count += 1;
            figures.deaths.exp_lost += last.exp - bar.exp;
            figures.deaths.percent_lost_hundredths += last.percent_hundredths.saturating_sub(bar.percent_hundredths);
        }
    }

    /// A reading that doesn't fit the level learned (`total`), and allows the totals
    /// `allowed`, is the next level's after `last`: its total is higher, and the bar was
    /// near 100%. "Near" is within 10 points, or twice the largest gain seen in the
    /// Session if that is more.
    fn levelled_up(&self, total: Total, last: ExpBar, allowed: Total) -> bool {
        last.percent_hundredths + self.near() >= HUNDRED_PERCENT && allowed.lo > total.hi
    }

    /// A reading `bar` barely into the next level after `last`: the bar was near 100%
    /// and now is as near 0%, with less EXP, and it allows a bigger total than the level
    /// learned (`total`). It may allow the old level's total too (2 [0.01%] allows
    /// 13,334 to 40,000 EXP), but a death costs far less than a whole level.
    fn wrapped(&self, total: Total, last: ExpBar, bar: ExpBar, allowed: Total) -> bool {
        let near = self.near();
        last.percent_hundredths + near >= HUNDRED_PERCENT
            && bar.percent_hundredths <= near
            && bar.exp < last.exp
            && allowed.hi > total.hi
    }

    /// How near 100% (or 0%) the bar must be for a level-up: within 10 points, or twice
    /// the largest gain seen in the Session if that is more.
    fn near(&self) -> u32 {
        (2 * self.largest_gain_hundredths).max(NEAR_LEVEL_UP_HUNDREDTHS)
    }

    /// Take `bar` as the last reading, in a level of `total`.
    fn accept(&mut self, total: Total, bar: ExpBar) {
        self.total = Some(total);
        self.last = Some(bar);
        self.misfits = None;
    }

    /// Back to waiting for an EXP change; what was learned about the level stays, but
    /// readings rejected so far no longer count towards a character switch. Nothing is
    /// left to undo.
    pub fn reset(&mut self) {
        self.figures = None;
        self.misfits = None;
        self.largest_gain_hundredths = 0;
        self.level_ups = 0;
        self.character_switched = false;
        self.shown = None;
        self.undo = None;
    }

    /// The player's reset: as [`Session::reset`], but the Session it ends is set aside
    /// until the EXP next changes, for [`Session::undo_reset`]. Pressed again while
    /// waiting, the one set aside stays.
    pub fn reset_by_player(&mut self) {
        let undo = match self.figures.take() {
            Some(figures) => Some(Undo {
                figures,
                largest_gain_hundredths: self.largest_gain_hundredths,
                level_ups: self.level_ups,
                character_switched: self.character_switched,
                visit: self.visit.count,
            }),
            None => self.undo.take(),
        };
        self.reset();
        self.undo = undo;
    }

    /// Bring back the Session the player's reset ended, if the EXP hasn't changed since.
    /// The time since counts as it would have: a gap in play, or a break.
    pub fn undo_reset(&mut self) {
        let Some(undo) = self.undo.take() else { return };
        let mut figures = undo.figures;
        if undo.visit != self.visit.count {
            // Reset on another map: the EXP hasn't changed since, so the time from then
            // to arriving here counts as it would have.
            figures.visit_from = figures.active(self.visit.at);
        }
        self.figures = Some(figures);
        self.largest_gain_hundredths = undo.largest_gain_hundredths;
        self.level_ups = undo.level_ups;
        self.character_switched = undo.character_switched;
        self.misfits = None;
        self.shown = None;
    }

    /// The panel confirmed another map: a new Visit starts, as of the latest time seen.
    /// The share of the map's ceiling counts only the play since.
    pub fn arrive(&mut self) {
        let now = self.now;
        self.visit = Visit { count: self.visit.count + 1, at: now };
        self.bests.end_visit();
        if let Some(figures) = &mut self.figures {
            figures.visit_from = figures.active(now);
        }
    }

    /// Time passed without a reading.
    pub fn tick(&mut self, at_ms: u64) {
        self.now = self.now.max(at_ms);
        if let Some(figures) = &mut self.figures {
            figures.warmed_up = figures.warm(self.now);
        }
    }

    /// A frame of the EXP bar, captured at `at_ms`, whose figures didn't read whole.
    pub fn on_unread(&mut self, at_ms: u64) {
        self.tick(at_ms);
        self.unread_since.get_or_insert(at_ms);
    }

    /// Whether the bar has gone [`UNREADABLE_MS`] without reading whole.
    fn unreadable(&self) -> bool {
        self.unread_since.is_some_and(|since| self.now.saturating_sub(since) >= UNREADABLE_MS)
    }

    /// The EXP changed by `gain` at `at`: the Session starts (from the EXP the bar shows
    /// now), or counts it. The Session's figures, if the EXP changed.
    fn change(&mut self, gain: i64, at: u64) -> Option<&mut Figures> {
        if gain == 0 {
            return None;
        }
        match &mut self.figures {
            Some(figures) => figures.change(gain, at),
            None => {
                // A new Session: the one a reset ended is gone for good.
                self.undo = None;
                self.figures = Some(Figures::new(at));
            }
        }
        self.figures.as_mut()
    }

    /// What the panel shows, with tracking on, on a map whose solo EXP/h ceiling is
    /// `ceiling` (if it is known), and which is `map` (its MeowDB ID) if Personal Bests
    /// are kept there: the figures measured earlier in this [`REFRESH_MS`] stretch, unless
    /// an event since asks for them afresh; and the bar's last reading. A best is dated
    /// by `now_unix_ms`, the wall clock, if the shell has told it.
    pub fn tracking(&mut self, ceiling: Option<u64>, map: Option<&str>, now_unix_ms: Option<u64>) -> ExpTracking {
        let mut fresh = self.measure(ceiling);
        let stretch = self.now / REFRESH_MS;
        // While the bar can't be read nothing new is known: the figures shown stay as
        // they were, unless the player arrives on another map or the share would be of
        // another ceiling or best (the map's page come, or another Candidate chosen).
        let unreadable = fresh.state == TrackingState::Unreadable;
        let visit = self.visit.count;
        let holds = self.shown.as_ref().is_some_and(|shown| {
            shown.visit == visit
                && shown.ceiling == ceiling
                && shown.map.as_deref() == map
                && (unreadable
                    || shown.stretch == stretch
                        && shown.level_ups == self.level_ups
                        && shown.tracking.state == fresh.state
                        && shown.tracking.deaths == fresh.deaths
                        && shown.tracking.character_switched == fresh.character_switched
                        && shown.tracking.reset_undoable == fresh.reset_undoable
                        // The first figures show as soon as there are any, and the first
                        // share.
                        && shown.tracking.rolling.is_some() == fresh.rolling.is_some()
                        && shown.tracking.ceiling_share_hundredths.is_some()
                            == fresh.ceiling_share_hundredths.is_some())
        });
        match &mut self.shown {
            Some(shown) if holds => shown.tracking.state = fresh.state,
            _ => {
                if let Some(map) = map {
                    // Last 5 min shown over a full 5 minutes of active play here counts
                    // towards this Visit's best, unless nothing is known of the EXP.
                    let here = self.figures.as_ref().map(|f| f.rolling_since(self.now, f.visit_from));
                    let full = here.filter(|h| h.over_ms >= ROLLING_MS && h.exp > 0);
                    let counted = full.filter(|_| !unreadable).map(|here| here.per_hour(1, 1) as u64);
                    self.bests.shown(map, counted, now_unix_ms);
                    let share = |best| full.map(|here| here.per_hour(u64::from(HUNDRED_PERCENT), best));
                    fresh.personal_best = self.bests.line(map, share);
                }
                let map = map.map(String::from);
                self.shown = Some(Shown { stretch, visit, ceiling, map, level_ups: self.level_ups, tracking: fresh });
            }
        }
        let held = self.shown.as_ref().map(|shown| shown.tracking.clone()).unwrap_or_default();
        ExpTracking { bar: self.last, ..held }
    }

    /// The figures as they stand now.
    fn measure(&self, ceiling: Option<u64>) -> ExpTracking {
        let unreadable = self.unreadable();
        let Some(figures) = &self.figures else {
            return ExpTracking {
                state: if unreadable { TrackingState::Unreadable } else { TrackingState::Waiting },
                character_switched: self.character_switched,
                reset_undoable: self.undo.is_some(),
                ..ExpTracking::default()
            };
        };
        let now = self.now;
        let warm = figures.warm(now);
        let state = if unreadable {
            TrackingState::Unreadable
        } else if figures.paused(now) {
            TrackingState::Paused
        } else if warm {
            TrackingState::Active
        } else {
            TrackingState::WarmingUp
        };
        let doubled_total = self.total.and_then(Total::doubled_midpoint);
        let rolling = figures.rolling(now);
        let gaining = rolling.exp > 0 && rolling.over_ms > 0;
        let time_to_level_s = match (doubled_total, self.last) {
            (Some(t), Some(last)) if gaining => {
                let doubled_to_go = i128::from(t.saturating_sub(2 * last.exp));
                let doubled_ms = doubled_to_go * i128::from(rolling.over_ms);
                Some(rounded_time_to_level(div_round(doubled_ms, 2 * i128::from(rolling.exp) * 1000) as u64))
            }
            _ => None,
        };
        // The share counts only the play on the shown map, once there is enough of it.
        let here = figures.rolling_since(now, figures.visit_from);
        let ceiling = ceiling.filter(|&c| c > 0 && here.exp > 0 && here.over_ms >= WARM_UP_MS);
        let average = figures.average(now);
        ExpTracking {
            state,
            time_to_level_s,
            time_to_level_badge: time_to_level_s.map(badge),
            rolling: rolling.rate(doubled_total),
            session_average: average.rate(doubled_total),
            ceiling_share_hundredths: ceiling.map(|c| here.per_hour(u64::from(HUNDRED_PERCENT), c)),
            solo_exp_ceiling: ceiling,
            deaths: figures.deaths,
            character_switched: self.character_switched && !warm,
            active_s: average.over_ms / 1000,
            rolling_s: rolling.over_ms / 1000,
            reset_undoable: false,
            bar: None,
            personal_best: None,
        }
    }
}

/// Whether `bar` carries on from `last` as the same level could: the percentage moved
/// no more than `near` points, and the same way as the EXP (or one of them stayed).
fn carries_on(last: ExpBar, bar: ExpBar, near: u32) -> bool {
    use std::cmp::Ordering::{Greater, Less};
    let ways = (bar.exp.cmp(&last.exp), bar.percent_hundredths.cmp(&last.percent_hundredths));
    bar.percent_hundredths.abs_diff(last.percent_hundredths) <= near && !matches!(ways, (Greater, Less) | (Less, Greater))
}

/// Time to Level rounded no finer than it can be trusted: to the minute under an hour,
/// to 5 minutes under 10 hours, and to the hour beyond.
fn rounded_time_to_level(seconds: u64) -> u64 {
    let step = if seconds < 3_600 {
        60
    } else if seconds < 36_000 {
        300
    } else {
        3_600
    };
    div_round(i128::from(seconds), step) as u64 * step as u64
}

/// Time to Level (already rounded) for the badge: "<1m", "47m", "3h 10m", "14h".
fn badge(seconds: u64) -> String {
    let minutes = seconds / 60;
    match (minutes / 60, minutes % 60) {
        (0, 0) => "<1m".into(),
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// `n / d` rounded to the nearest whole number, halves away from zero (`d` > 0).
fn div_round(n: i128, d: i128) -> i128 {
    if n >= 0 { (n + d / 2) / d } else { -((-n + d / 2) / d) }
}
