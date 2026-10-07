//! Every minimap sample, fed through the core's public inputs, shows the map its
//! filename names: `street-name__map-name[--variant].png`.

mod common;

use common::{FakeSource, sample_frame, samples_dir};
use compass_core::{Compass, CurrentMap};
use std::path::PathBuf;

/// ocrs reads this title as "Rocky RoadI"; the panel still shows the real map.
#[test]
fn a_slightly_misread_title_snaps_to_the_right_map() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_frame(sample_frame("victoria-road__rocky-road-i--title-region", 0));
    let state = compass.on_frame(sample_frame("victoria-road__rocky-road-i--title-region", 333));

    assert_eq!(
        state.map,
        Some(CurrentMap {
            id: "010004060".into(),
            street_name: "Victoria Road".into(),
            map_name: "Rocky Road I".into(),
            url: "https://meowdb.com/msclassic/maps/010004060".into(),
        })
    );
    assert!(!state.not_sure);
}

/// A name as the sample filenames spell it: lower-case words joined by `-`.
/// `"Rocky Road I"` gives `"rocky-road-i"`; a reading such as `"Rocky RoadI"` would not.
fn slug(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Every sample, in one session, each read on two consecutive frames (the panel only
/// switches maps after repeated confident reads).
#[test]
fn every_sample_shows_the_street_name_and_map_name_its_filename_names() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(samples_dir())
        .expect("samples dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 6, "expected the minimap samples, found {}", paths.len());

    let mut compass = Compass::new(FakeSource::meowdb());
    let mut wrong = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let answer = name.split_once("--").map_or(name.as_str(), |(a, _)| a);
        let (street, map) = answer.split_once("__").expect("street__map filename");
        let at_ms = 1000 * i as u64;
        compass.on_frame(sample_frame(&name, at_ms));
        let state = compass.on_frame(sample_frame(&name, at_ms + 333));

        let shown = state.map.as_ref().map(|m| (slug(&m.street_name), slug(&m.map_name)));
        if shown != Some((street.to_string(), map.to_string())) || state.not_sure {
            wrong.push(format!("{name}: panel shows {:?} (not sure: {})", state.map, state.not_sure));
        }
    }
    assert!(wrong.is_empty(), "wrong maps:\n{}", wrong.join("\n"));
}
