//! The shell's MeowDB source reads the map index out of MeowDB's "All Maps A-Z" page
//! with the core's reader. Recognition works on what it reads, and a page that no
//! longer lists maps leaves recognition unavailable rather than half-working.

mod common;

use common::{FakeSource, fixtures_dir, sample_frame};
use compass_core::{Compass, Recognition, meowdb};

/// The `<main>` of `https://meowdb.com/msclassic/maps/all` as saved on 2026-10-07.
fn saved_index_page() -> String {
    std::fs::read_to_string(fixtures_dir().join("meowdb-maps-all.main.html")).expect("fixture")
}

fn source_reading(page: &str) -> FakeSource {
    match meowdb::read_map_index(page) {
        Ok(index) => FakeSource::with_index(index),
        Err(e) => FakeSource::failing(e),
    }
}

#[test]
fn titles_snap_to_maps_read_from_meowdbs_index_page() {
    let mut compass = Compass::new(source_reading(&saved_index_page()));
    for (i, sample) in ["victoria-road__kerning-city--window", "victoria-road__rocky-road-i--title-region"]
        .iter()
        .enumerate()
    {
        let at_ms = 1000 * i as u64;
        compass.on_frame(sample_frame(sample, at_ms));
        let state = compass.on_frame(sample_frame(sample, at_ms + 333));

        let shown = state.map.as_ref().map(|m| (m.id.as_str(), m.street_name.as_str(), m.map_name.as_str()));
        let expected = match i {
            0 => ("010003000", "Victoria Road", "Kerning City"),
            _ => ("010004060", "Victoria Road", "Rocky Road I"),
        };
        assert_eq!(shown, Some(expected));
    }
}

#[test]
fn an_index_page_that_lists_no_maps_leaves_recognition_unavailable() {
    let changed = "<main><h1>All Maps A-Z</h1><p>Maps have moved to NiaDB.</p></main>";
    let mut compass = Compass::new(source_reading(changed));
    compass.on_frame(sample_frame("victoria-road__kerning-city--window", 0));

    let state = compass.on_frame(sample_frame("victoria-road__kerning-city--window", 333));

    assert_eq!(state.recognition, Recognition::Unavailable);
    assert_eq!(state.map, None);
}
