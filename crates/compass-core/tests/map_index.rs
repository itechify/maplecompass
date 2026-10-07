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
