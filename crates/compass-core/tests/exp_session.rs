//! The Session (ADR 0004): timestamped EXP bar readings become Time to Level, the
//! Rolling Rate and the Session average, with idle pauses, level-ups, deaths and
//! character switches told apart. Time comes only from the readings' and ticks'
//! timestamps.
//!
//! Single readings here are the real samples' figures (`160 [3.81%]`, `266 [6.33%]`,
//! `335 [7.98%]`: one level of ~4,200 EXP) and the 2026-10-07 recording's (a level of
//! ~16,682 EXP), with timestamps the tests choose.

mod common;

use common::{FakeSource, arrive};
use std::path::Path;
use compass_core::{
    Action, Compass, Deaths, ExpBar, ExpBarReading, ExpBarRegion, ExpRate, ExpTracking, GameWindow, PanelState, Rect, ScreenRect, Settings,
    Size, TitleRegion, TrackingState,
};

const S: u64 = 1000;
const MIN: u64 = 60 * S;

/// A core with EXP tracking on: both regions kept, on a 1920x1200 game window.
fn tracking() -> Compass {
    let client = Size { width: 1920, height: 1200 };
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.restore(Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client }),
        exp_bar_region: Some(ExpBarRegion { marked: Rect { x: 1005, y: 1144, width: 240, height: 32 }, client }),
        ..Settings::default()
    });
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    compass.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    compass
}

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
    assert_eq!(later.exp_tracking.time_to_level_s, Some(6721));
    assert_eq!(later.exp_tracking.time_to_level_badge.as_deref(), Some("1h 52m"));
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
    assert_eq!(state.exp_tracking.time_to_level_s, Some(1934));
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

    let clean = clean.tick(110 * S).clone();
    let misread = misread.tick(110 * S).clone();
    assert_eq!(misread.exp_tracking, clean.exp_tracking);
    assert_eq!(exp_per_hour(&misread), (Some(5184), Some(5184)), "144 EXP in 100 s");
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

    for (k, exp) in [40, 70, 100].into_iter().enumerate() {
        read(&mut compass, (80 + k as u64 * 10) * S, exp, percent_lv15(exp));
    }
    let after = compass.panel_state();
    assert_eq!(exp_per_hour(after), (Some(10_800), Some(10_800)));
    // The new level's readings so far allow 16,529..=16,806 EXP: ~16,667.5, of which
    // 10,800 EXP/h is 64.80%/h, with ~16,567.5 EXP to go.
    assert_eq!(after.exp_tracking.rolling.and_then(|r| r.percent_per_hour_hundredths), Some(6480));
    assert_eq!(after.exp_tracking.time_to_level_badge.as_deref(), Some("1h 32m"));
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

    // ...but three that fit a level of their own are another character.
    let switched = read(&mut compass, 82 * S, 157, 94);
    assert_eq!(switched.exp_tracking, ExpTracking { state: TrackingState::Waiting, ..ExpTracking::default() });

    let started = read(&mut compass, 90 * S, 169, 101);
    assert_eq!(started.exp_tracking.state, TrackingState::WarmingUp);
    let on = read(&mut compass, 100 * S, 175, 105);
    assert_eq!(exp_per_hour(&on), (Some(2160), Some(2160)), "6 EXP in 10 s");
    assert_eq!(on.exp_tracking.deaths, Deaths::default());
}

#[test]
fn resetting_the_session_clears_it_back_to_waiting() {
    let mut compass = a_minute_in();
    sample(&mut compass, 75 * S, EXP_266);

    let reset = compass.act(Action::ResetSession).clone();
    assert_eq!(reset.exp_tracking, ExpTracking { state: TrackingState::Waiting, ..ExpTracking::default() });
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
            "deaths": { "count": 0, "expLost": 0, "percentLostHundredths": 0 },
        })
    );

    // A minute in on Sunset Sky (as above), then a death: 335 [7.98%] to 266 [6.33%].
    read(&mut compass, 70 * S, 335, 798);
    let sunset = serde_json::to_value(arrive(&mut compass, SUNSET, 70 * S)).unwrap();
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
            "deaths": { "count": 0, "expLost": 0, "percentLostHundredths": 0 },
        })
    );
    let died = serde_json::to_value(read(&mut compass, 75 * S, 266, 633)).unwrap();
    assert_eq!(died["expTracking"]["deaths"], serde_json::json!({ "count": 1, "expLost": 69, "percentLostHundredths": 165 }));

    let paused = serde_json::to_value(compass.tick(10 * MIN)).unwrap();
    assert_eq!(paused["expTracking"]["state"], "paused");
    let off = serde_json::to_value(Compass::new(FakeSource::meowdb()).panel_state()).unwrap();
    assert_eq!(off["expTracking"]["state"], "off");
    compass.act(Action::ResetSession);
    let waiting = serde_json::to_value(compass.panel_state()).unwrap();
    assert_eq!(waiting["expTracking"]["state"], "waiting");
}

#[test]
fn the_rolling_rate_is_compared_with_the_map_s_solo_ceiling_and_a_map_change_keeps_the_session() {
    let mut compass = tracking();
    read(&mut compass, 0, 160, 381);
    read(&mut compass, 10 * S, 266, 633);
    let unrecognised = read(&mut compass, 70 * S, 335, 798);
    assert_eq!(unrecognised.exp_tracking.ceiling_share_hundredths, None, "no map recognised");

    // 4,140 EXP/h of Sunset Sky's 223,928 solo: 1.85%.
    let sunset = arrive(&mut compass, SUNSET, 70 * S);
    assert_eq!(sunset.exp_tracking.ceiling_share_hundredths, Some(185));
    assert_eq!(
        sunset.exp_tracking,
        ExpTracking { ceiling_share_hundredths: Some(185), ..unrecognised.exp_tracking.clone() }
    );

    // Of the Construction Site's 297,499: 1.39%. The Session carries on.
    let site = arrive(&mut compass, CONSTRUCTION_SITE, 71 * S);
    assert_eq!(
        site.exp_tracking,
        ExpTracking { ceiling_share_hundredths: Some(139), ..unrecognised.exp_tracking.clone() }
    );

    // A town has no Map Summary, so no ceiling.
    let town = arrive(&mut compass, KERNING, 72 * S);
    assert_eq!(town.exp_tracking, unrecognised.exp_tracking);
    let on = read(&mut compass, 80 * S, 360, 857);
    assert_eq!(on.exp_tracking.state, TrackingState::Active);
    assert_eq!(exp_per_hour(&on), (Some(4834), Some(4834)), "94 EXP in 70 s");
}

#[test]
fn there_is_no_share_of_the_ceiling_at_a_zero_or_negative_rolling_rate() {
    let mut compass = a_minute_in();
    arrive(&mut compass, SUNSET, 70 * S);

    // A death takes back the minute's 69 EXP, then another loses 106 more.
    let zero = read(&mut compass, 75 * S, 266, 633);
    assert_eq!(exp_per_hour(&zero), (Some(0), Some(0)));
    assert_eq!(zero.exp_tracking.ceiling_share_hundredths, None);
    let negative = read(&mut compass, 80 * S, 160, 381);
    assert_eq!(negative.exp_tracking.rolling.map(|r| r.exp_per_hour < 0), Some(true));
    assert_eq!(negative.exp_tracking.ceiling_share_hundredths, None);
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

    // 492 [2.95%] at 149 s: 335 EXP in 148 s. Every reading fits a level of
    // 16,677..=16,682 EXP (~16,679.5), so that is 48.85%/h, with ~16,187.5 EXP to go.
    let end = &at[&149_000];
    let rate = ExpRate { exp_per_hour: 8149, percent_per_hour_hundredths: Some(4885) };
    assert_eq!(end.state, TrackingState::Active);
    assert_eq!(end.session_average, Some(rate));
    assert_eq!(end.rolling, Some(rate), "under 5 minutes, the Rolling Rate is the whole Session");
    assert_eq!(end.time_to_level_s, Some(7151));
    assert_eq!(end.time_to_level_badge.as_deref(), Some("1h 59m"));
    assert_eq!(end.deaths, Deaths::default());
}

#[test]
fn time_passes_while_the_bar_cannot_be_read() {
    let mut compass = a_minute_in();
    for at in (80..=200).step_by(10) {
        compass.on_exp_bar_reading(ExpBarReading { bar: None, at_ms: at * S });
    }

    let state = compass.panel_state();
    assert_eq!(state.exp_tracking.state, TrackingState::Paused);
    assert_eq!(exp_per_hour(state), (Some(4140), Some(4140)), "69 EXP in the 1 active minute");
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
