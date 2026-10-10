//! OSMS's file set is asked for at startup and every 24 hours after, and swapped whole
//! when a new one reads. When OSMS can't be reached the source answers with the set it
//! saved, and the panel says so, with its date, only once that is over 7 days old: game
//! data changes only on patch days. A set that doesn't read keeps the one in use, and
//! the panel says the format changed only when there is none.

mod common;

use common::osms::{self, SUNSET_SKY, map};
use common::{FakeSource, OsmsAnswer, arrive, fresh, fresh_files, sample_frame, saved_copy};
use compass_core::osms::OsmsFiles;
use compass_core::{Compass, MapDataStatus, PanelState, Recognition, SourceError};
use std::sync::atomic::Ordering;
use std::task::Poll;

const SUNSET: &str = "victoria-road__sunset-sky--window";

/// When a saved copy was made: 2026-10-01 00:00 UTC, in ms since the Unix epoch.
const SAVED_AT: u64 = 1_790_812_800_000;
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

fn unreachable() -> OsmsAnswer {
    Poll::Ready(Err(SourceError::Unreachable("offline".into())))
}

/// A set that doesn't read.
fn changed_format() -> OsmsAnswer {
    fresh_files(OsmsFiles { maps: "<html>".into(), ..osms::game().files() })
}

/// On Sunset Sky, with the wall clock at `now_unix_ms`.
fn on_sunset_sky(answers: Vec<OsmsAnswer>, now_unix_ms: u64) -> (Compass, PanelState) {
    let mut compass = Compass::new(FakeSource::answering_osms(answers));
    compass.on_wall_clock(now_unix_ms);
    let state = arrive(&mut compass, SUNSET, 0);
    (compass, state)
}

/// Sunset Sky's Blue Snails' Spawn Count, as the panel shows it.
fn blue_snails(state: &PanelState) -> Option<u32> {
    let data = state.data.data()?;
    data.mobs.iter().find(|m| m.name == "Blue Snail").map(|m| m.spawn_count)
}

/// The game world, with this many Blue Snail spawn points on Sunset Sky.
fn with_blue_snails(points: u32) -> osms::World {
    osms::game().changing(SUNSET_SKY, |mut m| {
        m.spawns.retain(|s| s.mob != 3);
        m.spawns(3, points)
    })
}

#[test]
fn a_saved_copy_under_7_days_old_is_shown_without_a_notice() {
    let (_, state) = on_sunset_sky(vec![saved_copy(osms::game(), SAVED_AT)], SAVED_AT + 6 * DAY_MS);

    assert_eq!(state.recognition, Recognition::Ready);
    assert!(matches!(&state.data, MapDataStatus::Ready(data) if data.mobs.len() == 5));
}

#[test]
fn a_saved_copy_over_7_days_old_is_shown_with_its_date() {
    let (_, state) = on_sunset_sky(vec![saved_copy(osms::game(), SAVED_AT)], SAVED_AT + 8 * DAY_MS);

    assert_eq!(state.recognition, Recognition::SavedCopy { saved_at_unix_ms: SAVED_AT });
    let MapDataStatus::SavedCopy { data, saved_at_unix_ms } = &state.data else { panic!("{:?}", state.data) };
    assert_eq!(*saved_at_unix_ms, SAVED_AT);
    assert_eq!(data.mobs.len(), 5);
}

/// A long session: the saved copy turns 7 days old while the player plays.
#[test]
fn a_saved_copy_that_turns_7_days_old_while_running_gets_its_notice_then() {
    let (mut compass, state) = on_sunset_sky(vec![saved_copy(osms::game(), SAVED_AT)], SAVED_AT + 7 * DAY_MS - 1000);
    assert_eq!(state.recognition, Recognition::Ready);

    compass.on_wall_clock(SAVED_AT + 7 * DAY_MS + 1000);
    let state = compass.tick(2000);

    assert_eq!(state.recognition, Recognition::SavedCopy { saved_at_unix_ms: SAVED_AT });
    assert!(matches!(state.data, MapDataStatus::SavedCopy { .. }));
}

/// The overlay gets the panel state as JSON: a saved copy is the map's data with its
/// status and date beside it, and the map list says so with its date.
#[test]
fn the_overlay_receives_a_saved_copy_with_its_date_beside_the_data() {
    let (_, state) = on_sunset_sky(vec![saved_copy(osms::game(), SAVED_AT)], SAVED_AT + 8 * DAY_MS);

    let json = serde_json::to_value(&state).unwrap();

    assert_eq!(json["data"]["status"], "savedCopy");
    assert_eq!(json["data"]["savedAtUnixMs"], SAVED_AT);
    assert_eq!(json["data"]["mobs"].as_array().map(Vec::len), Some(5));
    assert_eq!(json["recognition"], serde_json::json!({ "status": "savedCopy", "savedAtUnixMs": SAVED_AT }));
}

#[test]
fn the_file_set_is_asked_for_again_every_24_hours() {
    let source = FakeSource::osms();
    let requests = source.osms_requests.clone();
    let mut compass = Compass::new(source);
    compass.tick(0);

    compass.tick(DAY_MS - 1);
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    compass.tick(DAY_MS);
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    compass.tick(2 * DAY_MS - 1);
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    compass.tick(2 * DAY_MS);
    assert_eq!(requests.load(Ordering::SeqCst), 3);
}

/// Patch day: a day later the set has more Blue Snails on Sunset Sky, and a new map
/// listed ahead of it. The player is still on Sunset Sky, sure of it, and sees the new
/// figures.
#[test]
fn a_new_set_a_day_later_replaces_the_old_one_whole_and_the_player_stays_on_their_map() {
    let patched = with_blue_snails(9).with(map(100_000_005, "Victoria Road", "A New Field"));
    let mut patched_first = patched.clone();
    patched_first.maps.rotate_right(1);
    let (mut compass, state) = on_sunset_sky(vec![fresh(with_blue_snails(6)), fresh(patched_first)], SAVED_AT);
    assert_eq!(blue_snails(&state), Some(6));

    compass.tick(DAY_MS);
    let state = compass.on_frame(sample_frame(SUNSET, DAY_MS + 333));

    assert_eq!(state.map.as_ref().map(|m| m.map_name.as_str()), Some("Sunset Sky"));
    assert!(!state.not_sure);
    assert_eq!(blue_snails(state), Some(9));
}

#[test]
fn a_set_that_cannot_be_had_or_does_not_read_a_day_later_leaves_the_one_in_use() {
    for (how, later) in [("OSMS unreachable", unreachable()), ("format changed", changed_format())] {
        let (mut compass, _) = on_sunset_sky(vec![fresh(with_blue_snails(6)), later], SAVED_AT);

        compass.tick(DAY_MS);
        let state = compass.on_frame(sample_frame(SUNSET, DAY_MS + 333));

        assert_eq!(state.recognition, Recognition::Ready, "{how}");
        assert_eq!(blue_snails(state), Some(6), "{how}");
    }
}

/// With no set to fall back on, a set that doesn't read means MapleCompass needs an
/// update: asking again won't change that before OSMS changes again, so it waits a day.
#[test]
fn with_no_set_to_fall_back_on_a_changed_format_says_so_and_is_asked_again_a_day_later() {
    let source = FakeSource::answering_osms(vec![changed_format(), fresh(osms::game())]);
    let requests = source.osms_requests.clone();
    let mut compass = Compass::new(source);

    let state = compass.tick(0);
    assert_eq!(state.recognition, Recognition::FormatChanged);

    compass.tick(DAY_MS - 1);
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    assert_eq!(compass.tick(DAY_MS).recognition, Recognition::Ready);
}
