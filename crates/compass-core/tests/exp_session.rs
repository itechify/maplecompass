//! The Session (ADR 0005): timestamped EXP bar readings become Time to Level, the
//! Rolling Rate and the Session average, with idle pauses, level-ups, deaths and
//! character switches told apart. Time comes only from the readings' and ticks'
//! timestamps.
//!
//! Single readings here are the real samples' figures (`160 [3.81%]`, `266 [6.33%]`,
//! `335 [7.98%]`: one level of ~4,200 EXP) and the 2026-10-07 recording's (a level of
//! ~16,682 EXP), with timestamps the tests choose.

mod common;

use common::{
    CONSTRUCTION_SITE_ID, FakeSource, SUNSET_SKY_ID, arrive, black_frame, saved_map_page, sharing_a_title, tracking,
    tracking_with,
};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Poll;
use compass_core::{
    Action, Compass, Deaths, ExpBar, ExpBarReading, ExpRate, ExpTracking, GameWindow, MapDataStatus, PanelControl,
    PanelState, ScreenRect, TrackingState,
};

const S: u64 = 1000;
const MIN: u64 = 60 * S;

/// The EXP bar read as `exp [percent_hundredths / 100 %]` at `at_ms`.
fn read(compass: &mut Compass, at_ms: u64, exp: u64, percent_hundredths: u32) -> PanelState {
    let bar = Some(ExpBar { exp, percent_hundredths });
    compass.on_exp_bar_reading(ExpBarReading { bar, at_ms }).clone()
}

#[test]
fn without_an_exp_bar_region_tracking_is_off() {
    let compass = Compass::new(FakeSource::meowdb());

    assert_eq!(compass.panel_state().exp_tracking.state, TrackingState::Off);
}

#[test]
fn tracking_waits_for_the_first_exp_change() {
    let mut compass = tracking();
    assert_eq!(compass.panel_state().exp_tracking.state, TrackingState::Waiting);

    let state = read(&mut compass, 0, 160, 381);
    let state2 = read(&mut compass, 30 * S, 160, 381);

    for state in [state, state2] {
        assert_eq!(state.exp_tracking.state, TrackingState::Waiting);
        assert_eq!(state.exp_tracking.rolling, None);
        assert_eq!(state.exp_tracking.session_average, None);
        assert_eq!(state.exp_tracking.time_to_level_s, None);
    }
}

#[test]
fn the_session_starts_on_the_first_exp_change_and_warms_up_for_a_minute() {
    let mut compass = tracking();
    read(&mut compass, 0, 160, 381);

    // The change starts the Session: the bar's EXP then is where it counts from.
    let started = read(&mut compass, 10 * S, 266, 633);
    assert_eq!(started.exp_tracking.state, TrackingState::WarmingUp);
    assert_eq!(started.exp_tracking.session_average, None, "no time has passed in the Session");

    let still = read(&mut compass, 40 * S, 266, 633);
    assert_eq!(still.exp_tracking.state, TrackingState::WarmingUp);
    assert_eq!(still.exp_tracking.session_average, Some(ExpRate { exp_per_hour: 0, percent_per_hour_hundredths: Some(0) }));
    assert_eq!(still.exp_tracking.time_to_level_s, None, "no Time to Level at a zero rate");

    // 69 EXP in a minute of the ~4,199.5 EXP level (160 [3.81%], 266 [6.33%] and
    // 335 [7.98%] all fit 4,199..=4,200): 4,140 EXP/h, 98.58%/h.
    let active = read(&mut compass, 70 * S, 335, 798);
    let rate = ExpRate { exp_per_hour: 4140, percent_per_hour_hundredths: Some(9858) };
    assert_eq!(active.exp_tracking.state, TrackingState::Active);
    assert_eq!(active.exp_tracking.session_average, Some(rate));
    assert_eq!(active.exp_tracking.rolling, Some(rate));
    // 4,199.5 - 335 = 3,864.5 EXP to go at 69 EXP a minute: 56 minutes.
    assert_eq!(active.exp_tracking.time_to_level_s, Some(3360));
    assert_eq!(active.exp_tracking.time_to_level_badge.as_deref(), Some("56m"));

    // A minute on with no change, the rate has halved and Time to Level doubled.
    let later = compass.tick(130 * S).clone();
    assert_eq!(later.exp_tracking.state, TrackingState::Active);
    assert_eq!(later.exp_tracking.session_average.map(|r| r.exp_per_hour), Some(2070));
    // 6,721 s, rounded to 5 minutes over an hour.
    assert_eq!(later.exp_tracking.time_to_level_s, Some(6600));
    assert_eq!(later.exp_tracking.time_to_level_badge.as_deref(), Some("1h 50m"));
}

/// A Session on the ~4,199.5 EXP level that started at 10 s on 266 [6.33%] and gained
/// 69 EXP by 70 s (335 [7.98%]): a minute of active time.
fn a_minute_in() -> Compass {
    let mut compass = tracking();
    read(&mut compass, 0, 160, 381);
    read(&mut compass, 10 * S, 266, 633);
    read(&mut compass, 70 * S, 335, 798);
    compass
}

fn exp_per_hour(state: &PanelState) -> (Option<i64>, Option<i64>) {
    let t = &state.exp_tracking;
    (t.rolling.map(|r| r.exp_per_hour), t.session_average.map(|r| r.exp_per_hour))
}

#[test]
fn more_than_two_minutes_without_an_exp_change_pauses_the_session_until_the_next() {
    let mut compass = a_minute_in();

    // Two minutes since the last change is still a gap in active play: 69 EXP in 3
    // minutes.
    let idle = compass.tick(190 * S).clone();
    assert_eq!(idle.exp_tracking.state, TrackingState::Active);
    assert_eq!(exp_per_hour(&idle), (Some(1380), Some(1380)));

    // Any longer is a break: it pauses, and the whole break counts for nothing, from
    // the last change on. 69 EXP in the 1 active minute.
    let paused = compass.tick(191 * S).clone();
    assert_eq!(paused.exp_tracking.state, TrackingState::Paused);
    assert_eq!(exp_per_hour(&paused), (Some(4140), Some(4140)));
    let still_paused = read(&mut compass, 20 * MIN, 335, 798);
    assert_eq!(still_paused.exp_tracking.state, TrackingState::Paused);
    assert_eq!(exp_per_hour(&still_paused), (Some(4140), Some(4140)));

    // 400 [9.52%] resumes it, the break still left out: 134 EXP in 1 active minute.
    let resumed = read(&mut compass, 30 * MIN, 400, 952);
    assert_eq!(resumed.exp_tracking.state, TrackingState::Active);
    assert_eq!(exp_per_hour(&resumed), (Some(8040), Some(8040)));
    let on = compass.tick(30 * MIN + 30 * S).clone();
    assert_eq!(on.exp_tracking.state, TrackingState::Active);
    assert_eq!(exp_per_hour(&on), (Some(5360), Some(5360)), "134 EXP in 1.5 minutes");
}

/// The percentage the game shows for `exp` on the 4,200 EXP level of the single
/// samples, rounded to hundredths as the game does.
fn percent(exp: u64) -> u32 {
    ((exp * 20_000 + 4200) / (2 * 4200)) as u32
}

#[test]
fn the_rolling_rate_covers_the_last_five_minutes_of_active_time_and_drives_time_to_level() {
    let mut compass = tracking();
    read(&mut compass, 0, 266, percent(266));
    read(&mut compass, 30 * S, 276, percent(276));
    // 10 minutes at 10 EXP every 30 s, then 5 at 50.
    let mut exp = 276;
    for k in 1..=20 {
        exp += 10;
        read(&mut compass, 30 * S + k * 30 * S, exp, percent(exp));
    }
    for k in 1..=10 {
        exp += 50;
        read(&mut compass, 630 * S + k * 30 * S, exp, percent(exp));
    }
    let state = compass.panel_state();
    assert_eq!(exp, 976);

    // 700 EXP in 15 minutes over the Session; 500 in the last 5.
    assert_eq!(exp_per_hour(state), (Some(6000), Some(2800)));
    // ~4,200 - 976 = ~3,224 EXP to go at 6,000 EXP/h.
    assert_eq!(state.exp_tracking.time_to_level_s, Some(1920), "1,934 s, to the minute");
    assert_eq!(state.exp_tracking.time_to_level_badge.as_deref(), Some("32m"));
}

/// A single EXP bar sample, captured at `at_ms`.
fn sample(compass: &mut Compass, at_ms: u64, name: &str) -> PanelState {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/exp-bar").join(format!("{name}.png"));
    let img = image::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())).to_rgba8();
    compass.on_exp_bar_frame(common::frame(&img, at_ms)).clone()
}

const EXP_160: &str = "160__3.81--kerning-city-window";
const EXP_266: &str = "266__6.33--construction-site-window";
const EXP_335: &str = "335__7.98--hidden-street-window";

#[test]
fn a_death_counts_against_the_net_rate_and_is_counted() {
    let mut compass = tracking();
    sample(&mut compass, 0, EXP_160);
    sample(&mut compass, 10 * S, EXP_266);
    sample(&mut compass, 70 * S, EXP_335);

    // 335 [7.98%] then 266 [6.33%] on the same level: a death that cost 69 EXP, 1.65%.
    let died = sample(&mut compass, 80 * S, EXP_266);
    assert_eq!(died.exp_tracking.deaths, Deaths { count: 1, exp_lost: 69, percent_lost_hundredths: 165 });
    assert_eq!(exp_per_hour(&died), (Some(0), Some(0)), "the death took back what was gained");
    assert_eq!(died.exp_tracking.time_to_level_s, None);
    assert_eq!(died.exp_tracking.time_to_level_badge, None);

    // Down to 160 [3.81%]: 106 EXP and 2.52% more, and a net loss.
    let again = sample(&mut compass, 90 * S, EXP_160);
    assert_eq!(again.exp_tracking.deaths, Deaths { count: 2, exp_lost: 175, percent_lost_hundredths: 417 });
    assert_eq!(exp_per_hour(&again), (Some(-4770), Some(-4770)), "-106 EXP in 80 s");
    assert_eq!(again.exp_tracking.time_to_level_s, None);
}

#[test]
fn a_death_as_the_first_exp_change_starts_the_session_and_is_counted() {
    let mut compass = tracking();
    read(&mut compass, 0, 335, 798);

    let died = read(&mut compass, 10 * S, 266, 633);
    assert_eq!(died.exp_tracking.state, TrackingState::WarmingUp);
    assert_eq!(died.exp_tracking.deaths, Deaths { count: 1, exp_lost: 69, percent_lost_hundredths: 165 });
}

#[test]
fn misreads_in_the_middle_of_a_session_move_nothing() {
    let mut clean = a_minute_in();
    let mut misread = a_minute_in();

    // The bar going 335 -> 360 -> 400 -> 410, read right...
    let right = [(80, 360, 857), (90, 400, 952), (100, 410, 976)];
    // ...and with a misread before each change: the percentage's 9 as a 3 twice in a
    // row (as ocrs once read 1.74% as 1.24% on two frames), an EXP's 3 as an 8.
    let wrong = [(75, 335, 738), (76, 335, 738), (85, 860, 857), (95, 400, 352)];
    for (at, exp, percent) in right {
        read(&mut clean, at * S, exp, percent);
    }
    let mut all: Vec<_> = right.iter().chain(&wrong).copied().collect();
    all.sort();
    for (at, exp, percent) in all {
        read(&mut misread, at * S, exp, percent);
    }

    // The first moment of a new half minute, so the figures are measured afresh.
    let clean = clean.tick(120 * S).clone();
    let misread = misread.tick(120 * S).clone();
    assert_eq!(misread.exp_tracking, clean.exp_tracking);
    assert_eq!(exp_per_hour(&misread), (Some(4713), Some(4713)), "144 EXP in 110 s");
}

/// The percentage the game shows for `exp` on the recording's ~16,682 EXP level.
fn percent_lv15(exp: u64) -> u32 {
    ((exp * 20_000 + 16_682) / (2 * 16_682)) as u32
}

#[test]
fn a_level_up_counts_what_was_left_of_the_old_level_and_what_is_in_the_new() {
    let mut compass = tracking();
    // 30 EXP every 10 s on the 4,200 EXP level, up to 99.52%...
    for (k, exp) in (4000..=4180).step_by(30).enumerate() {
        read(&mut compass, k as u64 * 10 * S, exp, percent(exp));
    }
    let before = compass.panel_state().clone();
    assert_eq!(exp_per_hour(&before), (Some(10_800), Some(10_800)));

    // ...then 30 more: 20 to finish the level and 10 into the next, of ~16,682 EXP.
    let levelled = read(&mut compass, 70 * S, 10, percent_lv15(10));
    assert_eq!(exp_per_hour(&levelled), (Some(10_800), Some(10_800)), "no jump at the level-up");
    assert_eq!(levelled.exp_tracking.deaths, Deaths::default());
    assert_eq!(levelled.exp_tracking.state, TrackingState::Active);

    for (k, exp) in [40, 70, 100, 130].into_iter().enumerate() {
        read(&mut compass, (80 + k as u64 * 10) * S, exp, percent_lv15(exp));
    }
    // 130 at 110 s, measured at 120 s, the start of a half minute.
    let after = compass.tick(120 * S).clone();
    // 300 EXP in the Session's 110 s: 150 to 4,180, 30 over the level-up, 120 since.
    assert_eq!(exp_per_hour(&after), (Some(9818), Some(9818)));
    // The new level's readings so far put it at ~16,667 EXP, of which 9,818 EXP/h is
    // 58.91%/h, with ~16,537 EXP to go: 6,064 s, rounded to 5 minutes.
    assert_eq!(after.exp_tracking.rolling.and_then(|r| r.percent_per_hour_hundredths), Some(5891));
    assert_eq!(after.exp_tracking.time_to_level_badge.as_deref(), Some("1h 40m"));
}

/// The percentage the game shows for `exp` on a level of 20,000 EXP.
fn percent_20k(exp: u64) -> u32 {
    ((exp * 20_000 + 20_000) / (2 * 20_000)) as u32
}

#[test]
fn a_level_up_barely_into_the_new_level_is_not_a_death() {
    let mut compass = tracking();
    // 100 EXP every 10 s on the recording's ~16,682 EXP level, up to 16,600 [99.51%]...
    for (k, exp) in (16_200..=16_600).step_by(100).enumerate() {
        read(&mut compass, k as u64 * 10 * S, exp, percent_lv15(exp));
    }

    // ...then 84 more: 82 to finish the level and 2 into the next, of 20,000 EXP. The
    // bar reads 2 [0.01%], which a level of 13,334 to 40,000 EXP allows, the old one
    // among them; but EXP dropping from 99.51% to next to nothing is the bar wrapping.
    let levelled = read(&mut compass, 50 * S, 2, percent_20k(2));
    assert_eq!(levelled.exp_tracking.deaths, Deaths::default());
    // The Session started at 10 s: 300 EXP to 16,600, then 84 over the level-up, in 40 s.
    assert_eq!(exp_per_hour(&levelled), (Some(34_560), Some(34_560)), "no jump at the level-up");

    for (k, exp) in [102, 202, 302].into_iter().enumerate() {
        read(&mut compass, (60 + k as u64 * 10) * S, exp, percent_20k(exp));
    }
    // 120 s, the start of a half minute: 300 more, 684 EXP in the Session's 110 s.
    let after = compass.tick(120 * S).clone();
    assert_eq!(after.exp_tracking.deaths, Deaths::default());
    assert_eq!(after.exp_tracking.state, TrackingState::Active);
    assert_eq!(exp_per_hour(&after), (Some(22_385), Some(22_385)));
}

#[test]
fn a_death_near_the_end_of_a_level_is_a_death_not_a_level_up() {
    let mut compass = tracking();
    // 100 EXP every 10 s on the recording's ~16,682 EXP level, up to 15,900 [95.31%]...
    for (k, exp) in (15_500..=15_900).step_by(100).enumerate() {
        read(&mut compass, k as u64 * 10 * S, exp, percent_lv15(exp));
    }

    // ...then a death that costs 2,597 EXP: 13,303 [79.74%], ~15.6 points down. The bar
    // was near 100% and EXP fell, as at a level-up, and the reading allows a bigger
    // total than the level's readings so far, but it fits the level.
    let died = read(&mut compass, 50 * S, 13_303, percent_lv15(13_303));
    assert_eq!(died.exp_tracking.deaths, Deaths { count: 1, exp_lost: 2597, percent_lost_hundredths: 1557 });
    assert_eq!(exp_per_hour(&died), (Some(-206_730), Some(-206_730)), "300 EXP gained, 2,597 lost, in 40 s");
    assert_eq!(died.exp_tracking.time_to_level_s, None);
}

#[test]
fn a_switch_to_another_character_starts_a_new_session() {
    let mut compass = a_minute_in();
    sample(&mut compass, 75 * S, EXP_266);
    let old = compass.panel_state().exp_tracking.clone();
    assert_eq!(old.deaths.count, 1);

    // The recording's character: a higher level (~16,682 EXP) with less EXP, as after a
    // level-up, but the bar was nowhere near 100%. Not one reading fits the old level;
    // two in a row could still be misreads...
    let first = read(&mut compass, 80 * S, 149, 89);
    let second = read(&mut compass, 81 * S, 157, 94);
    assert_eq!(first.exp_tracking, old);
    assert_eq!(second.exp_tracking.deaths, old.deaths);
    assert_eq!(second.exp_tracking.state, TrackingState::Active);

    // ...but three that fit a level of their own are another character. The panel says
    // why its figures went.
    let switched = read(&mut compass, 82 * S, 157, 94);
    assert_eq!(
        switched.exp_tracking,
        ExpTracking {
            state: TrackingState::Waiting,
            character_switched: true,
            bar: Some(ExpBar { exp: 157, percent_hundredths: 94 }),
            ..ExpTracking::default()
        }
    );

    let started = read(&mut compass, 90 * S, 169, 101);
    assert_eq!(started.exp_tracking.state, TrackingState::WarmingUp);
    assert!(started.exp_tracking.character_switched, "said until the new Session warms up");
    let on = read(&mut compass, 100 * S, 175, 105);
    assert_eq!(exp_per_hour(&on), (Some(2160), Some(2160)), "6 EXP in 10 s");
    assert_eq!(on.exp_tracking.deaths, Deaths::default());

    let warmed_up = read(&mut compass, 150 * S, 200, percent_lv15(200));
    assert_eq!(warmed_up.exp_tracking.state, TrackingState::Active);
    assert!(!warmed_up.exp_tracking.character_switched);
}

#[test]
fn a_switch_to_a_character_of_a_similar_level_is_a_switch_when_the_bar_jumps() {
    let mut compass = tracking();
    for (k, exp) in [149, 157, 165].into_iter().enumerate() {
        read(&mut compass, k as u64 * 10 * S, exp, percent_lv15(exp));
    }

    // A character on a level of 17,500 EXP, not 5% more than the ~16,682 learned, but
    // at 40%: the bar can't have jumped 39 points from 0.99%.
    for at in [30, 31, 32] {
        read(&mut compass, at * S, 7000, 4000);
    }

    let state = compass.panel_state();
    assert_eq!(state.exp_tracking.state, TrackingState::Waiting);
    assert!(state.exp_tracking.character_switched);
}

#[test]
fn a_misread_that_fits_the_level_and_misleads_it_keeps_the_session() {
    // On the recording's ~16,682 EXP level, 8 EXP every 10 s up to 165 [0.99%] at 20 s,
    // then 100 every 10 s, read right...
    let exp = |k: u64| if k <= 2 { 149 + 8 * k } else { 165 + 100 * (k - 2) };
    let right: Vec<(u64, u64, u32)> = (0..=11).map(|k| (k * 10, exp(k), percent_lv15(exp(k)))).collect();
    let mut clean = tracking();
    for &(at, exp, percent) in &right {
        read(&mut clean, at * S, exp, percent);
    }

    // ...and with 165 [0.99%] misread as 165 [0.98%] at 20 s. That fits the level read
    // so far (16,649 to 16,791 EXP), so it is taken, and it puts the level at 16,752 or
    // more. Low in a level each reading allows a wide range of totals, so later misreads
    // could hide it, but the true readings narrow as the bar fills: from 265 [1.59%] at
    // 30 s none fits. Three in a row that carry on from the last one taken are the same
    // level after all, learned anew, and the Session goes on.
    let mut misread = tracking();
    for &(at, exp, percent) in &right {
        let percent = if at == 20 { 98 } else { percent };
        let state = read(&mut misread, at * S, exp, percent);
        // The Session starts at 10 s, on the first change.
        assert_eq!(state.exp_tracking.state == TrackingState::Waiting, at == 0, "at {at} s");
        assert!(!state.exp_tracking.character_switched, "at {at} s");
    }

    // The first moment of a new half minute, so the figures are measured afresh.
    let clean = clean.tick(120 * S).clone();
    let misread = misread.tick(120 * S).clone();
    assert_eq!(misread.exp_tracking, clean.exp_tracking);
    assert_eq!(misread.exp_tracking.state, TrackingState::Active);
    assert_eq!(exp_per_hour(&misread), (Some(29_716), Some(29_716)), "908 EXP in 110 s");
}

#[test]
fn the_same_misread_on_a_still_bar_keeps_the_session() {
    // 100 EXP every 10 s on the recording's ~16,682 EXP level, up to 5,000 [29.97%] at
    // 100 s; then the bar stands still while the player loots, and 5,100 at 120 s.
    let mut readings: Vec<(u64, u64, u32)> =
        (0..=10).map(|k| (k * 10, 4000 + 100 * k, percent_lv15(4000 + 100 * k))).collect();
    readings.extend((101..=110).map(|at| (at, 5000, percent_lv15(5000))));
    readings.push((120, 5100, percent_lv15(5100)));
    let mut clean = tracking();
    for &(at, exp, percent) in &readings {
        read(&mut clean, at * S, exp, percent);
    }

    // The still frames read the same, and the first three read 29.97% as 29.37%: a level
    // of ~17,024 EXP, three times in a row. The Session goes on, the level learned anew,
    // and learned back from the true readings after them.
    let mut misread = tracking();
    for &(at, exp, percent) in &readings {
        let percent = if (101..=103).contains(&at) { 2937 } else { percent };
        let state = read(&mut misread, at * S, exp, percent);
        assert_eq!(state.exp_tracking.state == TrackingState::Waiting, at == 0, "at {at} s");
        assert!(!state.exp_tracking.character_switched, "at {at} s");
    }

    let clean = clean.tick(150 * S).clone();
    let misread = misread.tick(150 * S).clone();
    assert_eq!(misread.exp_tracking.state, TrackingState::Active);
    assert_eq!(exp_per_hour(&misread), exp_per_hour(&clean));
    // From 4,100 at 10 s, the Session's start; the 30 s since 5,100 count too.
    assert_eq!(exp_per_hour(&misread), (Some(25_714), Some(25_714)), "1,000 EXP in 140 s");
}

#[test]
fn once_warmed_up_a_session_never_warms_up_again() {
    let mut compass = tracking();
    read(&mut compass, 0, 160, 381);
    read(&mut compass, 10 * S, 266, 633);
    read(&mut compass, 20 * S, 335, 798);

    // 10 s of gains, then a minute without a change: still a gap in play, so 70 s of
    // active time, warmed up.
    assert_eq!(compass.tick(80 * S).exp_tracking.state, TrackingState::Active);
    // Two minutes without a change is a break, which counts for nothing: back to 10 s.
    assert_eq!(compass.tick(10 * MIN).exp_tracking.state, TrackingState::Paused);

    // The figures were warmed up once, and stay so.
    let resumed = read(&mut compass, 10 * MIN + 10 * S, 360, 857);
    assert_eq!(resumed.exp_tracking.state, TrackingState::Active);
}

#[test]
fn a_reset_by_the_player_is_not_a_character_switch() {
    let mut compass = a_minute_in();
    read(&mut compass, 80 * S, 149, 89);
    read(&mut compass, 81 * S, 157, 94);
    read(&mut compass, 82 * S, 157, 94);
    assert!(compass.panel_state().exp_tracking.character_switched);

    // Nothing to undo either: the switch ended the old Session for good.
    let reset = compass.act(Action::ResetSession);
    let bar = Some(ExpBar { exp: 157, percent_hundredths: 94 });
    assert_eq!(reset.exp_tracking, ExpTracking { state: TrackingState::Waiting, bar, ..ExpTracking::default() });
}

#[test]
fn resetting_the_session_clears_it_back_to_waiting() {
    let mut compass = a_minute_in();
    sample(&mut compass, 75 * S, EXP_266);

    let reset = compass.act(Action::ResetSession).clone();
    assert_eq!(
        reset.exp_tracking,
        ExpTracking {
            state: TrackingState::Waiting,
            reset_undoable: true,
            bar: Some(ExpBar { exp: 266, percent_hundredths: 633 }),
            ..ExpTracking::default()
        }
    );
    assert_eq!(read(&mut compass, 80 * S, 266, 633).exp_tracking.state, TrackingState::Waiting);

    // The next change starts a new Session from there.
    read(&mut compass, 90 * S, 335, 798);
    let state = read(&mut compass, 100 * S, 360, 857);
    assert_eq!(state.exp_tracking.state, TrackingState::WarmingUp);
    assert_eq!(exp_per_hour(&state), (Some(9000), Some(9000)), "25 EXP in 10 s");
    assert_eq!(state.exp_tracking.deaths, Deaths::default());
}

#[test]
fn readings_rejected_before_a_reset_never_count_towards_a_character_switch() {
    let mut compass = a_minute_in();
    // Two readings of another character's level (the recording's), rejected...
    read(&mut compass, 80 * S, 149, 89);
    read(&mut compass, 81 * S, 157, 94);
    compass.act(Action::ResetSession);

    // ...then a reset and one more: one in a row, still a misread.
    read(&mut compass, 82 * S, 157, 94);

    // So the level is still the ~4,200 EXP one, and its next change starts the Session.
    read(&mut compass, 90 * S, 360, 857);
    let state = read(&mut compass, 100 * S, 400, 952);
    assert_eq!(state.exp_tracking.state, TrackingState::WarmingUp);
    assert_eq!(exp_per_hour(&state), (Some(14_400), Some(14_400)), "40 EXP in 10 s");
}

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";

#[test]
fn the_overlay_gets_tracking_as_the_panel_renders_it() {
    let mut compass = tracking();
    arrive(&mut compass, SUNSET, 0);
    read(&mut compass, 0, 160, 381);
    let warming_up = serde_json::to_value(read(&mut compass, 10 * S, 266, 633)).unwrap();
    assert_eq!(
        warming_up["expTracking"],
        serde_json::json!({
            "state": "warmingUp",
            "timeToLevelS": null,
            "timeToLevelBadge": null,
            "rolling": null,
            "sessionAverage": null,
            "ceilingShareHundredths": null,
            "soloExpCeiling": null,
            "deaths": { "count": 0, "expLost": 0, "percentLostHundredths": 0 },
            "characterSwitched": false,
            "activeS": 0,
            "rollingS": 0,
            "resetUndoable": false,
            "bar": { "exp": 266, "percentHundredths": 633 },
            "personalBest": null,
            "forgetUndoable": false,
        })
    );

    // A minute in on Sunset Sky (as above), then a death: 335 [7.98%] to 266 [6.33%].
    let sunset = serde_json::to_value(read(&mut compass, 70 * S, 335, 798)).unwrap();
    let rate = serde_json::json!({ "expPerHour": 4140, "percentPerHourHundredths": 9858 });
    assert_eq!(
        sunset["expTracking"],
        serde_json::json!({
            "state": "active",
            "timeToLevelS": 3360,
            "timeToLevelBadge": "56m",
            "rolling": rate,
            "sessionAverage": rate,
            "ceilingShareHundredths": 185,
            "soloExpCeiling": 223_928,
            "deaths": { "count": 0, "expLost": 0, "percentLostHundredths": 0 },
            "characterSwitched": false,
            "activeS": 60,
            "rollingS": 60,
            "resetUndoable": false,
            "bar": { "exp": 335, "percentHundredths": 798 },
            "personalBest": null,
            "forgetUndoable": false,
        })
    );
    let died = serde_json::to_value(read(&mut compass, 75 * S, 266, 633)).unwrap();
    assert_eq!(died["expTracking"]["deaths"], serde_json::json!({ "count": 1, "expLost": 69, "percentLostHundredths": 165 }));

    let paused = serde_json::to_value(compass.tick(10 * MIN)).unwrap();
    assert_eq!(paused["expTracking"]["state"], "paused");
    unread(&mut compass, 10 * MIN);
    let unreadable = serde_json::to_value(unread(&mut compass, 10 * MIN + 10 * S)).unwrap();
    assert_eq!(unreadable["expTracking"]["state"], "unreadable");
    let off = serde_json::to_value(Compass::new(FakeSource::meowdb()).panel_state()).unwrap();
    assert_eq!(off["expTracking"]["state"], "off");
    read(&mut compass, 10 * MIN + 20 * S, 266, 633);
    compass.act(Action::ResetSession);
    let waiting = serde_json::to_value(compass.panel_state()).unwrap();
    assert_eq!(waiting["expTracking"]["state"], "waiting");
    assert_eq!(waiting["expTracking"]["resetUndoable"], true);
}

#[test]
fn the_rate_on_a_map_is_compared_with_its_solo_ceiling_after_a_minute_of_play_there() {
    let mut compass = tracking();
    read(&mut compass, 0, 160, 381);
    read(&mut compass, 10 * S, 266, 633);
    let unrecognised = read(&mut compass, 70 * S, 335, 798);
    assert_eq!(unrecognised.exp_tracking.ceiling_share_hundredths, None, "no map recognised");

    // Arriving on Sunset Sky, nothing has been earned there yet: the minute's 69 EXP came
    // before it showed. The Session carries on all the same.
    let sunset = arrive(&mut compass, SUNSET, 70 * S);
    assert_eq!(sunset.exp_tracking, unrecognised.exp_tracking);

    // A minute there, 65 EXP: 3,900 EXP/h of its 223,928 solo, 1.74%.
    assert_eq!(read(&mut compass, 100 * S, 360, 857).exp_tracking.ceiling_share_hundredths, None);
    let there = read(&mut compass, 130 * S, 400, 952);
    assert_eq!(there.exp_tracking.ceiling_share_hundredths, Some(174));
    assert_eq!(there.exp_tracking.solo_exp_ceiling, Some(223_928));
    assert_eq!(exp_per_hour(&there), (Some(4020), Some(4020)), "134 EXP in the Session's 120 s");

    // On the Construction Site its 297,499 is measured against what is earned there, from
    // nothing again: never Sunset Sky's EXP against its ceiling.
    let site = arrive(&mut compass, CONSTRUCTION_SITE, 131 * S);
    assert_eq!(site.exp_tracking.ceiling_share_hundredths, None);
    assert_eq!(site.exp_tracking.solo_exp_ceiling, None);
    assert_eq!(site.exp_tracking.state, TrackingState::Active);

    // A town has no Map Summary, so no ceiling. The Session carries on.
    let town = arrive(&mut compass, KERNING, 140 * S);
    assert_eq!(town.exp_tracking.ceiling_share_hundredths, None);
    let on = read(&mut compass, 150 * S, 410, 976);
    assert_eq!(on.exp_tracking.state, TrackingState::Active);
    assert_eq!(exp_per_hour(&on), (Some(3703), Some(3703)), "144 EXP in 140 s");
}

/// MeowDB's saved index and pages, but for each `(map, stand_in)` the map's page is the
/// stand-in's: to give two maps the same solo ceiling, or a ceiling of choice.
fn standing_in(source: FakeSource, stand_ins: &[(&'static str, &'static str)]) -> FakeSource {
    let stand_ins = stand_ins.to_vec();
    source.with_pages(move |id| {
        let page = stand_ins.iter().find(|(map, _)| *map == id).map_or(id, |&(_, stand_in)| stand_in);
        Poll::Ready(saved_map_page(page))
    })
}

#[test]
fn on_the_second_of_two_maps_with_the_same_ceiling_the_share_counts_only_play_since_arriving() {
    // The Construction Site with Sunset Sky's page: the same 223,928 solo ceiling.
    let mut compass = tracking_with(standing_in(FakeSource::meowdb(), &[(CONSTRUCTION_SITE_ID, SUNSET_SKY_ID)]));
    arrive(&mut compass, SUNSET, 0);
    read(&mut compass, 0, 160, 381);
    read(&mut compass, 10 * S, 266, 633);
    let sunset = read(&mut compass, 70 * S, 335, 798);
    assert_eq!(sunset.exp_tracking.ceiling_share_hundredths, Some(185), "4,140 EXP/h of 223,928");

    // Straight on to the Construction Site: nothing has been earned there yet, though its
    // ceiling is the same. Sunset Sky's minute is never measured against it.
    let site = arrive(&mut compass, CONSTRUCTION_SITE, 70 * S);
    assert_eq!(site.exp_tracking.ceiling_share_hundredths, None);
    assert_eq!(site.exp_tracking.solo_exp_ceiling, None);
    assert_eq!(read(&mut compass, 100 * S, 360, 857).exp_tracking.ceiling_share_hundredths, None);

    // A minute there, 65 EXP: 3,900 EXP/h of 223,928, 1.74%.
    let there = read(&mut compass, 130 * S, 400, 952);
    assert_eq!(there.exp_tracking.ceiling_share_hundredths, Some(174));
    assert_eq!(there.exp_tracking.solo_exp_ceiling, Some(223_928));
}

#[test]
fn a_map_whose_page_arrives_after_the_player_counts_play_from_arriving() {
    // MeowDB's pages are held back until `page_in` lets them through.
    let page_in = Arc::new(AtomicBool::new(false));
    let source = FakeSource::meowdb().with_pages({
        let page_in = page_in.clone();
        move |id| if page_in.load(Ordering::SeqCst) { Poll::Ready(saved_map_page(id)) } else { Poll::Pending }
    });
    let mut compass = tracking_with(source);
    read(&mut compass, 0, 160, 381);
    read(&mut compass, 10 * S, 266, 633);
    read(&mut compass, 70 * S, 335, 798);
    let arrived = arrive(&mut compass, SUNSET, 70 * S);
    assert_eq!(arrived.data, MapDataStatus::Loading);

    // The minimap covered a moment (a loading screen) doesn't end the Visit.
    compass.on_frame(black_frame(90 * S));
    arrive(&mut compass, SUNSET, 91 * S);
    read(&mut compass, 100 * S, 360, 857);

    // Sunset Sky's page comes 40 s after the player did.
    page_in.store(true, Ordering::SeqCst);
    let paged = compass.tick(110 * S).clone();
    assert_eq!(paged.data.solo_exp_ceiling(), Some(223_928));
    assert_eq!(paged.exp_tracking.ceiling_share_hundredths, None, "40 s there");

    // A minute after arriving, not after the page: 65 EXP, 3,900 EXP/h of 223,928.
    let there = read(&mut compass, 130 * S, 400, 952);
    assert_eq!(there.exp_tracking.ceiling_share_hundredths, Some(174));
}

#[test]
fn choosing_among_candidates_with_the_hotkey_counts_play_since_arriving_for_the_map_chosen() {
    // The Construction Site and Sunset Sky share the Kerning City sample's title here.
    // The first in the map index, the Construction Site, shows first.
    let mut compass = tracking_with(sharing_a_title());
    arrive(&mut compass, KERNING, 0);
    read(&mut compass, 0, 160, 381);
    read(&mut compass, 10 * S, 266, 633);
    let first = read(&mut compass, 70 * S, 335, 798);
    assert_eq!(first.map.map(|m| m.id).as_deref(), Some(CONSTRUCTION_SITE_ID));
    assert_eq!(first.exp_tracking.ceiling_share_hundredths, Some(139), "4,140 EXP/h of 297,499");

    // The player hasn't moved: the play since arriving counts for the map they choose.
    // 69 EXP in 70 s is 3,549 EXP/h, of Sunset Sky's 223,928.
    compass.tick(80 * S);
    compass.on_hotkey("ctrl+shift+n");
    let chosen = compass.panel_state();
    assert_eq!(chosen.map.as_ref().map(|m| m.id.as_str()), Some(SUNSET_SKY_ID));
    assert_eq!(chosen.exp_tracking.ceiling_share_hundredths, Some(158));
    assert_eq!(chosen.exp_tracking.solo_exp_ceiling, Some(223_928));
}

#[test]
fn undoing_a_reset_made_on_another_map_counts_the_share_from_arriving_on_this_one() {
    let mut compass = tracking();
    arrive(&mut compass, SUNSET, 0);
    read(&mut compass, 0, 160, 381);
    read(&mut compass, 10 * S, 266, 633);
    read(&mut compass, 70 * S, 335, 798);
    compass.tick(75 * S);
    compass.press(PanelControl::ResetSession);

    // On to the Construction Site at 80 s, then the reset undone at 90 s: Sunset Sky's
    // minute comes back, but not into the share here.
    compass.tick(80 * S);
    arrive(&mut compass, CONSTRUCTION_SITE, 80 * S);
    compass.tick(90 * S);
    let undone = press(&mut compass, PanelControl::UndoReset);
    assert_eq!(exp_per_hour(&undone).1, Some(3105), "69 EXP in the Session's 80 s");
    assert_eq!(undone.exp_tracking.ceiling_share_hundredths, None);

    // A minute from arriving, 25 EXP: 1,500 EXP/h of the Construction Site's 297,499.
    let there = read(&mut compass, 140 * S, 360, 857);
    assert_eq!(there.exp_tracking.ceiling_share_hundredths, Some(50));
}

#[test]
fn there_is_no_share_of_the_ceiling_at_a_zero_or_negative_rate_on_the_map() {
    let mut compass = tracking();
    arrive(&mut compass, SUNSET, 0);
    read(&mut compass, 0, 160, 381);
    read(&mut compass, 10 * S, 266, 633);
    let earning = read(&mut compass, 70 * S, 335, 798);
    assert_eq!(earning.exp_tracking.ceiling_share_hundredths, Some(185), "4,140 EXP/h of 223,928");

    // A death takes back the minute's 69 EXP, then another loses 106 more.
    let zero = read(&mut compass, 75 * S, 266, 633);
    assert_eq!(exp_per_hour(&zero), (Some(0), Some(0)));
    assert_eq!(zero.exp_tracking.ceiling_share_hundredths, None);
    let negative = read(&mut compass, 80 * S, 160, 381);
    assert_eq!(negative.exp_tracking.rolling.map(|r| r.exp_per_hour < 0), Some(true));
    assert_eq!(negative.exp_tracking.ceiling_share_hundredths, None);
    assert_eq!(negative.exp_tracking.solo_exp_ceiling, None);
}


#[test]
fn replaying_the_2026_10_07_recording_waits_warms_up_and_then_shows_its_rates() {
    // 150 s of hunting at Lv 15, one EXP Bar Region crop a second, named by capture ms.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/exp-bar/recording-2026-10-07");
    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .expect("recording")
        .map(|e| e.expect("dir entry").file_name().to_string_lossy().into_owned())
        .filter(|f| f.ends_with(".png"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 150);

    let mut compass = tracking();
    let mut at = std::collections::BTreeMap::new();
    for file in &files {
        let at_ms: u64 = file.trim_end_matches(".png").parse().expect("capture ms");
        let img = image::open(dir.join(file)).expect("crop").to_rgba8();
        at.insert(at_ms, compass.on_exp_bar_frame(common::frame(&img, at_ms)).exp_tracking.clone());
    }

    // 149 [0.89%] at the start; 157 [0.94%] a second later starts the Session.
    assert_eq!(at[&2].state, TrackingState::Waiting);
    assert_eq!(at[&1000].state, TrackingState::WarmingUp);
    // 207 [1.24%] by 23 s: 50 EXP in 29 s of the Session.
    assert_eq!(at[&30_000].state, TrackingState::WarmingUp);
    assert_eq!(at[&30_000].session_average.map(|r| r.exp_per_hour), Some(6207));
    // 278 [1.67%] by 58 s: 121 EXP in its first minute.
    assert_eq!(at[&61_000].state, TrackingState::Active);
    assert_eq!(at[&61_000].session_average.map(|r| r.exp_per_hour), Some(7260));

    // 456 [2.73%] since 118 s, measured at 120 s: 299 EXP in 119 s. The level's total is
    // ~16,679.5 EXP, so that is 54.23%/h, with ~16,223.5 EXP to go.
    let measured = &at[&120_000];
    let rate = ExpRate { exp_per_hour: 9045, percent_per_hour_hundredths: Some(5423) };
    assert_eq!(measured.state, TrackingState::Active);
    assert_eq!(measured.session_average, Some(rate));
    assert_eq!(measured.rolling, Some(rate), "under 5 minutes, the Rolling Rate is the whole Session");
    // 6,457 s, rounded to 5 minutes.
    assert_eq!(measured.time_to_level_s, Some(6600));
    assert_eq!(measured.time_to_level_badge.as_deref(), Some("1h 50m"));
    assert_eq!(measured.deaths, Deaths::default());
    // The EXP rose to 492 [2.95%] by 149 s, but the panel holds what it measured at 120 s
    // for the rest of that half minute. Only the bar's reading moves on.
    let later = &at[&149_000];
    assert_eq!(later.bar, Some(ExpBar { exp: 492, percent_hundredths: 295 }));
    assert_eq!(&ExpTracking { bar: measured.bar, ..later.clone() }, measured);
}

/// A frame of the EXP bar at `at_ms` whose figures don't read whole.
fn unread(compass: &mut Compass, at_ms: u64) -> PanelState {
    compass.on_exp_bar_reading(ExpBarReading { bar: None, at_ms }).clone()
}

#[test]
fn a_bar_that_cannot_be_read_says_so_rather_than_pausing() {
    let mut compass = a_minute_in();

    // The pointer passing over it is nothing to tell.
    unread(&mut compass, 80 * S);
    assert_eq!(unread(&mut compass, 89 * S).exp_tracking.state, TrackingState::Active);

    // Ten seconds of it is.
    let covered = unread(&mut compass, 90 * S);
    assert_eq!(covered.exp_tracking.state, TrackingState::Unreadable);

    // However long it lasts, it isn't a break: nothing is known of the EXP. The figures
    // hold, and so does the last reading.
    for at in (100..=600).step_by(10) {
        unread(&mut compass, at * S);
    }
    let still = compass.tick(10 * MIN).clone();
    assert_eq!(still.exp_tracking.state, TrackingState::Unreadable);
    assert_eq!(exp_per_hour(&still), (Some(4140), Some(4140)), "69 EXP in the 1 active minute");
    assert_eq!(still.exp_tracking.bar, Some(ExpBar { exp: 335, percent_hundredths: 798 }));
}

#[test]
fn a_bar_that_cannot_be_read_from_the_start_says_so_while_waiting() {
    let mut compass = tracking();
    unread(&mut compass, 0);

    let state = unread(&mut compass, 10 * S);

    assert_eq!(state.exp_tracking.state, TrackingState::Unreadable);
    assert_eq!(state.exp_tracking.rolling, None);
    assert_eq!(state.exp_tracking.bar, None);
}

#[test]
fn a_bar_read_again_soon_after_counts_the_gap_as_play() {
    let mut compass = a_minute_in();
    for at in (75..=110).step_by(5) {
        unread(&mut compass, at * S);
    }

    // 400 [9.52%] at 120 s, 50 s after the last whole reading: the gain counts, and the
    // time it took, as between any two kills. 134 EXP in the Session's 110 s.
    let state = read(&mut compass, 120 * S, 400, 952);
    assert_eq!(state.exp_tracking.state, TrackingState::Active);
    assert_eq!(exp_per_hour(&state), (Some(4385), Some(4385)));
    assert_eq!(state.exp_tracking.bar, Some(ExpBar { exp: 400, percent_hundredths: 952 }));
}

#[test]
fn a_bar_unread_for_longer_than_a_break_counts_neither_the_time_nor_the_exp() {
    let mut compass = a_minute_in();
    for at in (80..=600).step_by(10) {
        unread(&mut compass, at * S);
    }

    // 1,000 [23.81%] at 610 s: 665 EXP that no one saw come. Counted at once, as a break
    // would count it, it would show 44,040 EXP/h (734 EXP in the 1 active minute). It is
    // where the Session stands now, nothing more; the figures hold until the next change.
    let back = read(&mut compass, 610 * S, 1000, percent(1000));
    assert_eq!(back.exp_tracking.state, TrackingState::Paused);
    assert_eq!(exp_per_hour(&back), (Some(4140), Some(4140)));
    assert_eq!(back.exp_tracking.bar, Some(ExpBar { exp: 1000, percent_hundredths: 2381 }));

    // The next change counts from there: 10 EXP more, 79 in the 1 active minute.
    let on = read(&mut compass, 620 * S, 1010, percent(1010));
    assert_eq!(on.exp_tracking.state, TrackingState::Active);
    assert_eq!(exp_per_hour(&on), (Some(4740), Some(4740)));
}

#[test]
fn a_reset_can_be_undone_until_the_exp_next_changes() {
    let mut compass = a_minute_in();
    let before = compass.panel_state().exp_tracking.clone();
    assert!(!before.reset_undoable);

    // Pressed by mistake, twice: still the one Session to bring back.
    compass.press(PanelControl::ResetSession);
    let reset = press(&mut compass, PanelControl::ResetSession);
    assert_eq!(reset.exp_tracking.state, TrackingState::Waiting);
    assert!(reset.exp_tracking.reset_undoable);

    let undone = press(&mut compass, PanelControl::UndoReset);
    assert_eq!(undone.exp_tracking, before);
}

#[test]
fn once_the_exp_changes_after_a_reset_it_can_t_be_undone() {
    let mut compass = a_minute_in();
    compass.press(PanelControl::ResetSession);

    // The change starts the new Session...
    let started = read(&mut compass, 80 * S, 360, 857);
    assert_eq!(started.exp_tracking.state, TrackingState::WarmingUp);
    assert!(!started.exp_tracking.reset_undoable);

    // ...which Undo leaves as it is.
    let new = press(&mut compass, PanelControl::UndoReset);
    assert_eq!(new.exp_tracking, started.exp_tracking);
}

#[test]
fn a_character_switch_after_a_reset_leaves_nothing_to_undo() {
    let mut compass = a_minute_in();
    compass.press(PanelControl::ResetSession);

    for at in [80, 81, 82] {
        read(&mut compass, at * S, 157, 94);
    }

    let switched = press(&mut compass, PanelControl::UndoReset);
    assert_eq!(switched.exp_tracking.state, TrackingState::Waiting);
    assert!(switched.exp_tracking.character_switched);
    assert!(!switched.exp_tracking.reset_undoable);
}

#[test]
fn the_session_says_how_long_it_has_run_and_how_much_the_rolling_rate_covers() {
    let mut compass = a_minute_in();
    let minute = &compass.panel_state().exp_tracking;
    assert_eq!((minute.active_s, minute.rolling_s), (60, 60), "one rate: the whole Session");

    // 25 EXP a minute for 10 minutes more.
    let mut exp = 335;
    for k in 1..=10 {
        exp += 25;
        read(&mut compass, (70 + k * 60) * S, exp, percent(exp));
    }
    let later = &compass.panel_state().exp_tracking;
    assert_eq!((later.active_s, later.rolling_s), (660, 300), "11 minutes, the last 5 rolling");
    // 125 EXP in the last 5 minutes; 319 in the Session's 11.
    assert_eq!(exp_per_hour(compass.panel_state()), (Some(1500), Some(1740)));
}

/// Press `control`, then the panel state after it.
fn press(compass: &mut Compass, control: PanelControl) -> PanelState {
    compass.press(control);
    compass.panel_state().clone()
}

#[test]
fn without_the_game_window_tracking_shows_off_and_the_session_carries_on() {
    let mut compass = a_minute_in();
    let before = compass.panel_state().exp_tracking.clone();

    let gone = compass.on_game_window(None).clone();
    assert_eq!(gone.exp_tracking, ExpTracking::default());
    assert_eq!(gone.exp_tracking.state, TrackingState::Off);

    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    let back = compass.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    assert_eq!(back.exp_tracking, before);
}

#[test]
fn a_session_paused_while_warming_up_shows_paused_and_warms_up_only_with_active_time() {
    let mut compass = tracking();
    read(&mut compass, 0, 160, 381);
    read(&mut compass, 10 * S, 266, 633);
    read(&mut compass, 20 * S, 335, 798);

    // 10 s of active time, then a break.
    let state = compass.tick(10 * MIN).clone();
    assert_eq!(state.exp_tracking.state, TrackingState::Paused);
    assert_eq!(exp_per_hour(&state), (Some(24_840), Some(24_840)), "69 EXP in 10 s");

    // The break didn't warm it up: still 10 s of active time when the EXP changes again.
    let resumed = read(&mut compass, 10 * MIN + 10 * S, 360, 857);
    assert_eq!(resumed.exp_tracking.state, TrackingState::WarmingUp);
    assert_eq!(exp_per_hour(&resumed), (Some(33_840), Some(33_840)), "94 EXP in 10 s");
}

// The figures hold still for each half minute of capture time, so they don't jump with
// every kill; events show at once.

/// A Session on the 4,200 EXP level that started at 5 s and gains faster and faster: 1
/// EXP in the first 5 s, 2 in the next, and so on, read every 5 s up to `until_s`.
fn speeding_up(until_s: u64) -> Compass {
    let mut compass = tracking();
    let mut exp = 160;
    read(&mut compass, 0, exp, percent(exp));
    for k in 1..=until_s / 5 {
        exp += k;
        read(&mut compass, k * 5 * S, exp, percent(exp));
    }
    compass
}

#[test]
fn the_figures_hold_for_the_rest_of_each_half_minute_then_refresh() {
    let compass = speeding_up(150);
    let measured = speeding_up(120).panel_state().exp_tracking.clone();
    assert_eq!(measured.state, TrackingState::Active);

    // Read on to 145 s, the EXP climbing faster: the panel holds the figures of 120 s,
    // though the bar's reading moves on.
    let mut held = speeding_up(145);
    let held = held.tick(149 * S).exp_tracking.clone();
    assert_ne!(held.bar, measured.bar);
    assert_eq!(ExpTracking { bar: measured.bar, ..held }, measured);

    // The next half minute measures them afresh.
    let refreshed = &compass.panel_state().exp_tracking;
    assert!(
        refreshed.rolling.unwrap().exp_per_hour > measured.rolling.unwrap().exp_per_hour,
        "{refreshed:?} after {measured:?}"
    );
}

#[test]
fn a_death_shows_at_once_with_the_figures_measured_then() {
    let mut compass = a_minute_in();
    // 360 [8.57%] at 80 s, in the half minute whose figures were measured at 70 s...
    read(&mut compass, 80 * S, 360, 857);

    // ...then a death to 266 [6.33%] at 85 s takes it all back, and shows straight away.
    let died = read(&mut compass, 85 * S, 266, 633);
    assert_eq!(died.exp_tracking.deaths.count, 1);
    assert_eq!(exp_per_hour(&died), (Some(0), Some(0)));
}

#[test]
fn time_to_level_is_rounded_to_the_hour_beyond_ten_hours() {
    let mut compass = tracking();
    read(&mut compass, 0, 149, percent_lv15(149));
    read(&mut compass, 10 * S, 150, percent_lv15(150));

    // 20 EXP a minute, with ~16,500 EXP of the ~16,682 EXP level to go: ~13.8 hours.
    let state = read(&mut compass, 70 * S, 170, percent_lv15(170));
    assert_eq!(state.exp_tracking.time_to_level_s, Some(14 * 3600));
    assert_eq!(state.exp_tracking.time_to_level_badge.as_deref(), Some("14h"));
}
