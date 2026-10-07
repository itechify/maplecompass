//! The panel switches maps only after repeated confident reads, and keeps the last
//! confident map, marked not sure, while the title can't be trusted.

mod common;

use common::{FakeSource, black_frame, sample_frame};
use compass_core::{Compass, PanelState};

const KERNING: &str = "victoria-road__kerning-city--window";
const SUNSET: &str = "victoria-road__sunset-sky--window";

fn shown_map_name(state: &PanelState) -> Option<&str> {
    state.map.as_ref().map(|m| m.map_name.as_str())
}

#[test]
fn after_two_frames_of_kerning_city_then_a_blank_frame_the_panel_still_shows_kerning_city_marked_not_sure()
 {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_frame(sample_frame(KERNING, 0));
    compass.on_frame(sample_frame(KERNING, 333));

    let state = compass.on_frame(black_frame(666));

    assert_eq!(shown_map_name(state), Some("Kerning City"));
    assert!(state.not_sure);
}

#[test]
fn a_single_stray_read_does_not_switch_maps() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_frame(sample_frame(KERNING, 0));
    compass.on_frame(sample_frame(KERNING, 333));

    let state = compass.on_frame(sample_frame(SUNSET, 666));

    assert_eq!(shown_map_name(state), Some("Kerning City"));
    assert!(state.not_sure);
}

#[test]
fn a_second_confident_read_of_a_new_map_switches_to_it() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_frame(sample_frame(KERNING, 0));
    compass.on_frame(sample_frame(KERNING, 333));
    compass.on_frame(sample_frame(SUNSET, 666));

    let state = compass.on_frame(sample_frame(SUNSET, 1000));

    assert_eq!(shown_map_name(state), Some("Sunset Sky"));
    assert!(!state.not_sure);
}

#[test]
fn reads_of_a_new_map_must_be_consecutive_to_switch() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_frame(sample_frame(KERNING, 0));
    compass.on_frame(sample_frame(KERNING, 333));
    compass.on_frame(sample_frame(SUNSET, 666));
    compass.on_frame(black_frame(1000));

    let state = compass.on_frame(sample_frame(SUNSET, 1333));

    assert_eq!(shown_map_name(state), Some("Kerning City"));
    assert!(state.not_sure);
}

#[test]
fn a_single_read_does_not_show_a_first_map() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.on_frame(sample_frame(KERNING, 0));

    assert_eq!(state.map, None);
}
