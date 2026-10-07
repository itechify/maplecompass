//! Reading a title takes a while, so the shell reads Title Region frames with a
//! `TitleReader` on its own thread and hands the core each `TitleReading`: the window
//! tracking and the panel never wait for recognition. A reading counts exactly as the
//! frame it came from would.

mod common;

use common::{FakeSource, sample_frame};
use compass_core::{Compass, PanelState, TitleLines, TitleReader, TitleReading};

const KERNING: &str = "victoria-road__kerning-city--window";

fn shown(state: &PanelState) -> Option<(&str, &str)> {
    state.map.as_ref().map(|m| (m.id.as_str(), m.map_name.as_str()))
}

fn read_as(street: &str, map: &str, at_ms: u64) -> TitleReading {
    TitleReading { lines: Some(TitleLines { street: street.into(), map: map.into() }), at_ms }
}

#[test]
fn frames_read_on_another_thread_switch_the_map_like_frames_fed_to_the_core() {
    let readings = std::thread::spawn(|| {
        let reader = TitleReader::bundled();
        [0, 333].map(|at_ms| reader.read(&sample_frame(KERNING, at_ms)))
    })
    .join()
    .unwrap();
    let mut compass = Compass::new(FakeSource::meowdb());

    let [first, second] = readings;
    compass.on_reading(first);
    let state = compass.on_reading(second);

    assert_eq!(shown(state), Some(("010003000", "Kerning City")));
    assert!(!state.not_sure);
}

/// Reported in play: the recogniser ran "Mountain" and "I" together.
#[test]
fn a_title_read_as_west_rocky_mountaini_snaps_to_west_rocky_mountain_i() {
    let mut compass = Compass::new(FakeSource::meowdb());

    compass.on_reading(read_as("Victoria Road", "West Rocky MountainI", 0));
    let state = compass.on_reading(read_as("Victoria Road", "West Rocky MountainI", 333));

    assert_eq!(shown(state), Some(("010004020", "West Rocky Mountain I")));
    assert!(!state.not_sure);
}

#[test]
fn an_unreadable_reading_keeps_the_last_map_marked_not_sure() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_reading(read_as("Victoria Road", "Kerning City", 0));
    compass.on_reading(read_as("Victoria Road", "Kerning City", 333));

    let state = compass.on_reading(TitleReading { lines: None, at_ms: 666 });

    assert_eq!(shown(state), Some(("010003000", "Kerning City")));
    assert!(state.not_sure);
}
