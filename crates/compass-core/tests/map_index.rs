//! Recognition rests on MeowDB's map index: asked for once, and without it the panel
//! never passes raw readings off as real maps.

mod common;

use common::{FakeSource, black_frame, sample_frame, saved_index};
use compass_core::{Compass, Recognition};
use std::sync::atomic::Ordering;

const KERNING: &str = "victoria-road__kerning-city--window";

#[test]
fn when_the_map_index_cannot_be_fetched_recognition_is_unavailable_and_no_map_is_shown() {
    let mut compass = Compass::new(FakeSource::offline());
    compass.on_frame(sample_frame(KERNING, 0));

    let state = compass.on_frame(sample_frame(KERNING, 333));

    assert_eq!(state.recognition, Recognition::Unavailable);
    assert_eq!(state.map, None);
}

#[test]
fn a_title_that_matches_no_known_map_is_not_shown_as_a_map() {
    let without_kerning =
        saved_index().into_iter().filter(|m| m.map_name != "Kerning City").collect();
    let mut compass = Compass::new(FakeSource::with_index(without_kerning));
    compass.on_frame(sample_frame(KERNING, 0));

    let state = compass.on_frame(sample_frame(KERNING, 333));

    assert_eq!(state.recognition, Recognition::Ready);
    assert_eq!(state.map, None);
    assert!(state.not_sure);
}

#[test]
fn the_map_index_is_fetched_once_not_on_every_frame() {
    let source = FakeSource::meowdb();
    let requests = source.index_requests.clone();
    let mut compass = Compass::new(source);

    for i in 0..5 {
        compass.on_frame(sample_frame(KERNING, 333 * i));
    }
    compass.on_frame(black_frame(2000));

    assert_eq!(requests.load(Ordering::SeqCst), 1);
}

#[test]
fn a_failed_map_index_fetch_is_retried_after_a_minute_not_on_every_frame() {
    let source = FakeSource::offline();
    let requests = source.index_requests.clone();
    let mut compass = Compass::new(source);

    for i in 0..10 {
        compass.on_frame(black_frame(333 * i));
    }
    assert_eq!(requests.load(Ordering::SeqCst), 1);

    compass.on_frame(black_frame(60_000));
    assert_eq!(requests.load(Ordering::SeqCst), 2);
}

#[test]
fn recognition_starts_working_once_a_retried_map_index_fetch_succeeds() {
    let mut compass = Compass::new(FakeSource::offline_then(saved_index()));
    compass.on_frame(sample_frame(KERNING, 0));

    compass.on_frame(sample_frame(KERNING, 60_000));
    let state = compass.on_frame(sample_frame(KERNING, 60_333));

    assert_eq!(state.recognition, Recognition::Ready);
    assert_eq!(state.map.as_ref().map(|m| m.id.as_str()), Some("010003000"));
}

/// When a saved copy was made: 2026-10-01 00:00 UTC, in ms since the Unix epoch.
const SAVED_AT: u64 = 1_790_812_800_000;

#[test]
fn when_meowdb_is_unreachable_a_saved_copy_of_the_map_index_still_recognises_maps_and_says_so() {
    let mut compass = Compass::new(FakeSource::meowdb().serving_saved_copies(SAVED_AT));
    compass.on_frame(sample_frame(KERNING, 0));

    let state = compass.on_frame(sample_frame(KERNING, 333));

    assert_eq!(state.recognition, Recognition::SavedCopy { saved_at_unix_ms: SAVED_AT });
    assert_eq!(state.map.as_ref().map(|m| m.id.as_str()), Some("010003000"));
}

/// Setup's test read needs the map index before the first frame: there are no frames
/// until a Title Region is marked.
#[test]
fn the_map_index_is_asked_for_at_startup_before_any_frame() {
    let source = FakeSource::meowdb();
    let requests = source.index_requests.clone();
    let mut compass = Compass::new(source);

    let state = compass.tick(0);

    assert_eq!(requests.load(Ordering::SeqCst), 1);
    assert_eq!(state.recognition, Recognition::Ready);
}

/// The index arrives on the third time the core checks on it. Nothing waits for it
/// meanwhile, and frames read before it arrives aren't recognised.
#[test]
fn a_slow_map_index_never_holds_up_the_panel_and_recognition_starts_once_it_arrives() {
    let mut compass = Compass::new(FakeSource::with_slow_index(saved_index(), 3));

    assert_eq!(compass.tick(0).recognition, Recognition::Starting);
    let state = compass.on_frame(sample_frame(KERNING, 333));
    assert_eq!(state.recognition, Recognition::Starting);
    assert_eq!(state.map, None);
    assert_eq!(compass.tick(400).recognition, Recognition::Ready);

    compass.on_frame(sample_frame(KERNING, 666));
    let state = compass.on_frame(sample_frame(KERNING, 1000));
    assert_eq!(state.map.as_ref().map(|m| m.id.as_str()), Some("010003000"));
}
