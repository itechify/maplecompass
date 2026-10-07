//! The Session (ADR 0004): EXP bar readings, with their capture times, turned into Time
//! to Level and EXP rates. There is no clock here and no EXP table.
//!
//! - The level's total EXP comes from the bar itself: the game rounds the percentage to
//!   hundredths, so each reading allows a range of totals ("149 [0.89%]": 16,649 to
//!   16,836). The range learned is what every reading in the level allows; it narrows
//!   as the percentage grows.
//! - A reading that fits the range is the same level: less EXP than before is a death.
//!   One that doesn't fit is a level-up if the bar was near 100%, a misread otherwise,
//!   unless [`SWITCH_READINGS`] in a row fit another level: a character switch, which
//!   starts a new Session.
//! - The Session starts on the first EXP change, from the EXP the bar shows then.
//! - Active time runs while the EXP keeps changing. More than [`IDLE_MS`] without a
//!   change pauses the Session: the first [`IDLE_MS`] of the stretch count, the rest
//!   doesn't, so the rates hold while paused.

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
    pub session: Option<ExpRate>,
    /// The Rolling Rate's EXP/h as a share of the shown map's MeowDB solo EXP/h ceiling
    /// (its Map Summary), in hundredths of a percent: 7300 for "73%". `None` without a
    /// recognised map with a Map Summary, or without a Rolling Rate.
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

/// A Session pauses after this long without an EXP change.
const IDLE_MS: u64 = 2 * 60_000;
/// A Session warms up for this much active time.
const WARM_UP_MS: u64 = 60_000;
/// The Rolling Rate covers this much active time.
const ROLLING_MS: u64 = 5 * 60_000;
const HOUR_MS: i128 = 3_600_000;
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
        let exp = u128::from(bar.exp) * 20_000;
        let p = u128::from(bar.percent_hundredths);
        let lo = exp.div_ceil(2 * p + 1).max(u128::from(bar.exp) + 1);
        let hi = if p == 0 { u128::from(u64::MAX) } else { exp / (2 * p - 1) };
        (lo <= hi).then(|| Total { lo: lo as u64, hi: hi.min(u128::from(u64::MAX)) as u64 })
    }

    /// The totals both allow, if any.
    fn meet(self, other: Total) -> Option<Total> {
        let t = Total { lo: self.lo.max(other.lo), hi: self.hi.min(other.hi) };
        (t.lo <= t.hi).then_some(t)
    }

    /// Twice the best estimate of the total (the middle of the range, kept whole);
    /// `None` while unbounded.
    fn twice(self) -> Option<u64> {
        (self.hi != u64::MAX).then(|| self.lo + self.hi)
    }
}

/// The Session and what it has learned about the level from the EXP bar.
#[derive(Debug, Default)]
pub(crate) struct Session {
    /// The level's total, learned from the readings accepted in this level.
    total: Option<Total>,
    /// The last reading accepted.
    last: Option<ExpBar>,
    /// Readings in a row that don't fit `total`: the totals they all allow, and how
    /// many there are.
    misfits: Option<(Total, u32)>,
    /// The largest gain between two readings in this Session, in hundredths of a
    /// percent: how near 100% the bar must have been for a level-up.
    largest_gain_hundredths: u32,
    /// The latest time seen, from a reading or a tick.
    now: u64,
    /// The figures since the Session started; `None` while waiting.
    run: Option<Run>,
}

/// A started Session's figures.
#[derive(Debug)]
struct Run {
    last_change_at: u64,
    /// Active time up to `last_change_at`.
    active_at_change: u64,
    /// Net EXP gained since the start.
    gained: i64,
    /// (active time, `gained` then) at the start and each change since, back to the
    /// last one before the Rolling Rate's window.
    marks: Vec<(u64, i64)>,
    deaths: Deaths,
}

impl Run {
    fn new(at: u64) -> Self {
        Run { last_change_at: at, active_at_change: 0, gained: 0, marks: vec![(0, 0)], deaths: Deaths::default() }
    }

    fn idle(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_change_at)
    }

    fn paused(&self, now: u64) -> bool {
        self.idle(now) > IDLE_MS
    }

    /// Active time at `now`: the idle stretch since the last change counts up to
    /// [`IDLE_MS`].
    fn active(&self, now: u64) -> u64 {
        self.active_at_change + self.idle(now).min(IDLE_MS)
    }

    fn change(&mut self, gain: i64, at: u64) {
        self.active_at_change = self.active(at);
        self.last_change_at = at;
        self.gained += gain;
        self.marks.push((self.active_at_change, self.gained));
        let from = self.active_at_change.saturating_sub(ROLLING_MS);
        let before = self.marks.iter().rposition(|&(a, _)| a <= from).unwrap_or(0);
        self.marks.drain(..before);
    }

    /// The net EXP gained over the Rolling Rate's window at `now`, and its length.
    fn rolling(&self, now: u64) -> (i64, u64) {
        let active = self.active(now);
        let window = active.min(ROLLING_MS);
        let from = active - window;
        let then = self.marks.iter().rev().find(|&&(a, _)| a <= from).map_or(0, |&(_, g)| g);
        (self.gained - then, window)
    }
}

impl Session {
    /// Take a whole EXP bar reading captured at `at_ms`.
    pub fn on_bar(&mut self, bar: ExpBar, at_ms: u64) {
        self.tick(at_ms);
        let Some(own) = Total::of(bar) else { return };
        let (Some(total), Some(last)) = (self.total, self.last) else {
            self.total = Some(own);
            self.last = Some(bar);
            return;
        };
        if let Some(twice_total) = total.twice()
            && self.levelled_up(total, last, own, bar)
        {
            // What was left of the old level, and what is in the new one.
            let gain = (twice_total as i64 - 2 * last.exp as i64 + 1) / 2 + bar.exp as i64;
            self.accept(own, bar);
            self.change(gain, at_ms);
        } else if let Some(met) = total.meet(own) {
            self.accept(met, bar);
            if bar.percent_hundredths > last.percent_hundredths {
                let gain = bar.percent_hundredths - last.percent_hundredths;
                self.largest_gain_hundredths = self.largest_gain_hundredths.max(gain);
            }
            if let Some(run) = self.change(bar.exp as i64 - last.exp as i64, at_ms)
                && bar.exp < last.exp
            {
                // Less EXP in the same level: a death.
                run.deaths.count += 1;
                run.deaths.exp_lost += last.exp - bar.exp;
                run.deaths.percent_lost_hundredths +=
                    last.percent_hundredths.saturating_sub(bar.percent_hundredths);
            }
        } else {
            // A misread, or another character's level: that takes readings in a row
            // that fit it.
            let run = self.misfits.and_then(|(common, n)| Some((common.meet(own)?, n + 1)));
            let (common, n) = run.unwrap_or((own, 1));
            self.misfits = Some((common, n));
            if n >= SWITCH_READINGS {
                self.accept(common, bar);
                self.reset();
            }
        }
    }

    /// The reading `bar` (whose totals are `own`) after `last` is the next level's: the
    /// bar was near 100%, its EXP fell further than a death takes, and the level could
    /// be bigger than the one learned (`total`). "Near" is within 10 points, or twice
    /// the largest gain seen in the Session if that is more.
    fn levelled_up(&self, total: Total, last: ExpBar, own: Total, bar: ExpBar) -> bool {
        let near = (2 * self.largest_gain_hundredths).max(NEAR_LEVEL_UP_HUNDREDTHS);
        last.percent_hundredths + near >= 10_000
            && bar.percent_hundredths + near < last.percent_hundredths
            && bar.exp < last.exp
            && own.hi > total.hi
    }

    /// Take `bar` as the last reading, in a level of `total`.
    fn accept(&mut self, total: Total, bar: ExpBar) {
        self.total = Some(total);
        self.last = Some(bar);
        self.misfits = None;
    }

    /// Back to waiting for an EXP change; what was learned about the level stays.
    pub fn reset(&mut self) {
        self.run = None;
        self.largest_gain_hundredths = 0;
    }

    /// Time passed without a reading.
    pub fn tick(&mut self, at_ms: u64) {
        self.now = self.now.max(at_ms);
    }

    /// The EXP changed by `gain` at `at`: the Session starts, or counts it. The started
    /// Session's figures, if it counted it.
    fn change(&mut self, gain: i64, at: u64) -> Option<&mut Run> {
        if gain == 0 {
            return None;
        }
        if self.run.is_none() {
            self.run = Some(Run::new(at));
            return None;
        }
        let run = self.run.as_mut()?;
        run.change(gain, at);
        Some(run)
    }

    /// What the panel shows, with tracking on, on a map whose solo EXP/h ceiling is
    /// `ceiling` (if it is known).
    pub fn tracking(&self, ceiling: Option<u64>) -> ExpTracking {
        let Some(run) = &self.run else {
            return ExpTracking { state: TrackingState::Waiting, ..ExpTracking::default() };
        };
        let now = self.now;
        let active = run.active(now);
        let state = if run.paused(now) {
            TrackingState::Paused
        } else if active < WARM_UP_MS {
            TrackingState::WarmingUp
        } else {
            TrackingState::Active
        };
        let twice_total = self.total.and_then(Total::twice);
        let rate = |gained: i64, ms: u64| {
            (ms > 0).then(|| ExpRate {
                exp_per_hour: div_round(i128::from(gained) * HOUR_MS, i128::from(ms)) as i64,
                percent_per_hour_hundredths: twice_total.map(|t| {
                    div_round(i128::from(gained) * HOUR_MS * 20_000, i128::from(ms) * i128::from(t)) as i64
                }),
            })
        };
        let (rolling_gained, window) = run.rolling(now);
        let time_to_level_s = match (twice_total, self.last) {
            (Some(t), Some(last)) if rolling_gained > 0 && window > 0 => {
                let twice_to_go = i128::from(t.saturating_sub(2 * last.exp));
                Some(div_round(twice_to_go * i128::from(window), 2 * i128::from(rolling_gained) * 1000) as u64)
            }
            _ => None,
        };
        let ceiling_share_hundredths = ceiling.filter(|&c| c > 0 && window > 0).map(|c| {
            div_round(i128::from(rolling_gained) * HOUR_MS * 10_000, i128::from(window) * i128::from(c)) as i64
        });
        ExpTracking {
            state,
            time_to_level_s,
            time_to_level_badge: time_to_level_s.map(badge),
            rolling: rate(rolling_gained, window),
            session: rate(run.gained, active),
            ceiling_share_hundredths,
            deaths: run.deaths,
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
