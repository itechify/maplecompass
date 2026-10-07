//! The Session (ADR 0004): EXP bar readings, with their capture times, turned into Time
//! to Level and EXP rates. There is no clock here and no EXP table.
//!
//! - The level's total EXP comes from the bar itself: the game rounds the percentage to
//!   hundredths, so each reading allows a range of totals ("149 [0.89%]": 16,649 to
//!   16,836). The range learned is what every reading in the level allows; it narrows
//!   as the percentage grows.
//! - A reading that fits the range is the same level: less EXP than before is a death.
//!   One that doesn't fit is a level-up if it allows only bigger totals and the bar was
//!   near 100%, a misread otherwise, unless [`SWITCH_READINGS`] in a row fit another
//!   level: a character switch, which starts a new Session.
//! - The Session starts on the first EXP change, from the EXP the bar shows then.
//! - Active time runs while the EXP keeps changing. A gap of up to [`IDLE_MS`] between
//!   changes counts; a longer one is a break, which pauses the Session and counts for
//!   nothing, from the last change to the next, so the rates hold while paused.

use crate::ExpBar;
use serde::Serialize;

/// What EXP tracking shows: everything the panel's tracking section and the collapsed
/// icon's badge render.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpTracking {
    /// Off, waiting, warming up, active or paused.
    pub state: TrackingState,
    /// Time to Level in seconds at the net Rolling Rate; `None` ("—") while there is no
    /// rate yet, the net rate is zero or negative, or the level's total isn't known.
    pub time_to_level_s: Option<u64>,
    /// Time to Level, short, for the collapsed icon's badge: "47m", "3h 10m".
    pub time_to_level_badge: Option<String>,
    /// The Rolling Rate: over the last 5 minutes of active play. `None` until the
    /// Session has some active time.
    pub rolling: Option<ExpRate>,
    /// The average over the whole Session's active time. `None` as for `rolling`.
    pub session_average: Option<ExpRate>,
    /// The Rolling Rate's EXP/h as a share of the shown map's MeowDB solo EXP/h ceiling
    /// (its Map Summary), in hundredths of a percent: 7300 for "73%". `None` without a
    /// recognised map with a Map Summary, or while the Rolling Rate is zero, negative or
    /// not there yet.
    pub ceiling_share_hundredths: Option<i64>,
    /// Deaths this Session and what they cost: "deaths: 2 (−1.4%)".
    pub deaths: Deaths,
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
    /// Warmed up, and the EXP changed within the last [`IDLE_MS`].
    Active,
    /// No EXP change for more than [`IDLE_MS`]: the rates hold until the next change.
    Paused,
}

/// A gap between EXP changes longer than this is a break: it pauses the Session.
const IDLE_MS: u64 = 2 * 60_000;
/// A Session warms up for this much active time.
const WARM_UP_MS: u64 = 60_000;
/// The Rolling Rate covers this much active time.
const ROLLING_MS: u64 = 5 * 60_000;
/// An hour, which the rates are per.
const HOUR_MS: u64 = 3_600_000;
/// 100% in hundredths of a percent, the unit of the bar and of every share here.
const HUNDRED_PERCENT: u32 = 10_000;
/// Readings in a row that fit a level of their own, and not the one learned, that make
/// a character switch. A misread has been seen to repeat on two frames.
const SWITCH_READINGS: u32 = 3;
/// A level-up needs the bar within at least this many hundredths of 100% before it.
const NEAR_LEVEL_UP_HUNDREDTHS: u32 = 1000;

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
        }
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
        let active = self.active(now);
        let window = active.min(ROLLING_MS);
        let from = active - window;
        let then = self.marks.iter().rev().find(|m| m.active <= from).map_or(0, |m| m.gained);
        Gained { exp: self.gained - then, over_ms: window }
    }

    /// The net EXP gained over the whole Session at `now`.
    fn average(&self, now: u64) -> Gained {
        Gained { exp: self.gained, over_ms: self.active(now) }
    }
}

impl Session {
    /// Take a whole EXP bar reading captured at `at_ms`.
    pub fn on_bar(&mut self, bar: ExpBar, at_ms: u64) {
        self.tick(at_ms);
        let Some(allowed) = Total::of(bar) else { return };
        let (Some(total), Some(last)) = (self.total, self.last) else {
            self.total = Some(allowed);
            self.last = Some(bar);
            return;
        };
        if let Some(narrowed) = total.meet(allowed) {
            self.accept(narrowed, bar);
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
                figures.deaths.percent_lost_hundredths +=
                    last.percent_hundredths.saturating_sub(bar.percent_hundredths);
            }
        } else if let Some(doubled_total) = total.doubled_midpoint()
            && self.levelled_up(total, last, allowed)
        {
            // What was left of the old level, and what is in the new one.
            let gain = (doubled_total as i64 - 2 * last.exp as i64 + 1) / 2 + bar.exp as i64;
            self.accept(allowed, bar);
            self.change(gain, at_ms);
        } else {
            // A misread, or another character's level: that takes readings in a row
            // that fit it.
            let more = self.misfits.and_then(|m| Some(Misfits { total: m.total.meet(allowed)?, count: m.count + 1 }));
            let misfits = more.unwrap_or(Misfits { total: allowed, count: 1 });
            self.misfits = Some(misfits);
            if misfits.count >= SWITCH_READINGS {
                self.accept(misfits.total, bar);
                self.reset();
            }
        }
    }

    /// A reading that doesn't fit the level learned (`total`), and allows the totals
    /// `allowed`, is the next level's after `last`: its total is higher, and the bar was
    /// near 100%. "Near" is within 10 points, or twice the largest gain seen in the
    /// Session if that is more.
    fn levelled_up(&self, total: Total, last: ExpBar, allowed: Total) -> bool {
        let near = (2 * self.largest_gain_hundredths).max(NEAR_LEVEL_UP_HUNDREDTHS);
        last.percent_hundredths + near >= HUNDRED_PERCENT && allowed.lo > total.hi
    }

    /// Take `bar` as the last reading, in a level of `total`.
    fn accept(&mut self, total: Total, bar: ExpBar) {
        self.total = Some(total);
        self.last = Some(bar);
        self.misfits = None;
    }

    /// Back to waiting for an EXP change; what was learned about the level stays, but
    /// readings rejected so far no longer count towards a character switch.
    pub fn reset(&mut self) {
        self.figures = None;
        self.misfits = None;
        self.largest_gain_hundredths = 0;
    }

    /// Time passed without a reading.
    pub fn tick(&mut self, at_ms: u64) {
        self.now = self.now.max(at_ms);
    }

    /// The EXP changed by `gain` at `at`: the Session starts (from the EXP the bar shows
    /// now), or counts it. The Session's figures, if the EXP changed.
    fn change(&mut self, gain: i64, at: u64) -> Option<&mut Figures> {
        if gain == 0 {
            return None;
        }
        match &mut self.figures {
            Some(figures) => figures.change(gain, at),
            None => self.figures = Some(Figures::new(at)),
        }
        self.figures.as_mut()
    }

    /// What the panel shows, with tracking on, on a map whose solo EXP/h ceiling is
    /// `ceiling` (if it is known).
    pub fn tracking(&self, ceiling: Option<u64>) -> ExpTracking {
        let Some(figures) = &self.figures else {
            return ExpTracking { state: TrackingState::Waiting, ..ExpTracking::default() };
        };
        let now = self.now;
        let state = if figures.paused(now) {
            TrackingState::Paused
        } else if figures.active(now) < WARM_UP_MS {
            TrackingState::WarmingUp
        } else {
            TrackingState::Active
        };
        let doubled_total = self.total.and_then(Total::doubled_midpoint);
        let rolling = figures.rolling(now);
        let gaining = rolling.exp > 0 && rolling.over_ms > 0;
        let time_to_level_s = match (doubled_total, self.last) {
            (Some(t), Some(last)) if gaining => {
                let doubled_to_go = i128::from(t.saturating_sub(2 * last.exp));
                let doubled_ms = doubled_to_go * i128::from(rolling.over_ms);
                Some(div_round(doubled_ms, 2 * i128::from(rolling.exp) * 1000) as u64)
            }
            _ => None,
        };
        ExpTracking {
            state,
            time_to_level_s,
            time_to_level_badge: time_to_level_s.map(badge),
            rolling: rolling.rate(doubled_total),
            session_average: figures.average(now).rate(doubled_total),
            ceiling_share_hundredths: ceiling
                .filter(|&c| c > 0 && gaining)
                .map(|c| rolling.per_hour(u64::from(HUNDRED_PERCENT), c)),
            deaths: figures.deaths,
        }
    }
}

/// Time to Level for the badge: "<1m", "47m", "3h 10m".
fn badge(seconds: u64) -> String {
    let minutes = seconds / 60;
    match (minutes / 60, minutes % 60) {
        (0, 0) => "<1m".into(),
        (0, m) => format!("{m}m"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// `n / d` rounded to the nearest whole number, halves away from zero (`d` > 0).
fn div_round(n: i128, d: i128) -> i128 {
    if n >= 0 { (n + d / 2) / d } else { -((-n + d / 2) / d) }
}
