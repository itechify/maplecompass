//! The recognised map shows straight away; its MeowDB data follows. A map's page is
//! asked for only once the panel is sure of the map (ADR 0003: never ahead of time),
//! and a slow or unreachable MeowDB never hides which map the player is on.

mod common;

use common::{FakeSource, black_frame, requests_for, sample_frame};
use compass_core::{Compass, MEOWDB_CREDIT, MapData, MapDataStatus, Mob, PanelState, Settings};
use std::task::Poll;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const SUNSET_SKY: &str = "010003070";
const KERNING_CITY: &str = "010003000";

fn shown_map_name(state: &PanelState) -> Option<&str> {
    state.map.as_ref().map(|m| m.map_name.as_str())
}

#[test]
fn while_a_slow_page_loads_the_map_is_already_shown_and_its_data_follows() {
    // The page arrives on the third time the core checks on it.
    let mut compass = Compass::new(FakeSource::meowdb().with_slow_pages(3));
    compass.on_frame(sample_frame(SUNSET, 0));

    let state = compass.on_frame(sample_frame(SUNSET, 333));
    assert_eq!(shown_map_name(state), Some("Sunset Sky"));
    assert_eq!(state.data, MapDataStatus::Loading);

    let state = compass.on_frame(black_frame(666));
    assert_eq!(state.data, MapDataStatus::Loading);

    let state = compass.on_frame(black_frame(1000));
    assert_eq!(shown_map_name(state), Some("Sunset Sky"));
    assert!(matches!(&state.data, MapDataStatus::Ready(data) if data.mobs.len() == 5));
}

#[test]
fn when_meowdb_is_unreachable_the_map_is_still_shown_with_its_data_unavailable() {
    let source = FakeSource::meowdb().with_pages_offline();
    let requests = source.map_requests.clone();
    let mut compass = Compass::new(source);
    compass.on_frame(sample_frame(SUNSET, 0));

    let state = compass.on_frame(sample_frame(SUNSET, 333));
    assert_eq!(shown_map_name(state), Some("Sunset Sky"));
    assert_eq!(state.data, MapDataStatus::Unavailable);

    // Not asked again while the player stays: only a map change fetches.
    compass.on_frame(sample_frame(SUNSET, 666));
    let state = compass.on_frame(black_frame(1000));
    assert_eq!(state.data, MapDataStatus::Unavailable);
    assert_eq!(requests_for(&requests, SUNSET_SKY), 1);
}

#[test]
fn a_maps_page_is_asked_for_only_once_the_panel_is_sure_of_the_map_and_then_only_once() {
    let source = FakeSource::meowdb();
    let requests = source.map_requests.clone();
    let mut compass = Compass::new(source);
    compass.on_frame(sample_frame(KERNING, 0));
    compass.on_frame(sample_frame(KERNING, 333));

    // One stray read of Sunset Sky: not sure yet, so nothing is fetched for it.
    compass.on_frame(sample_frame(SUNSET, 666));
    assert_eq!(requests_for(&requests, SUNSET_SKY), 0);

    compass.on_frame(sample_frame(SUNSET, 1000));
    for i in 0..3 {
        compass.on_frame(sample_frame(SUNSET, 1333 + 333 * i));
    }
    compass.on_frame(black_frame(2333));

    assert_eq!(requests_for(&requests, KERNING_CITY), 1);
    assert_eq!(requests_for(&requests, SUNSET_SKY), 1);
}

#[test]
fn before_any_map_is_shown_no_page_is_asked_for_and_there_is_no_data() {
    let source = FakeSource::meowdb();
    let requests = source.map_requests.clone();
    let mut compass = Compass::new(source);

    let state = compass.on_frame(sample_frame(SUNSET, 0));

    assert_eq!(state.data, MapDataStatus::NoMap);
    assert_eq!(state.credit, None);
    assert!(requests.lock().unwrap().is_empty());
}

#[test]
fn mobs_are_sorted_by_level_then_name_whatever_order_the_source_gives() {
    let mob = |name: &str, level| Mob {
        name: name.into(),
        level,
        hp: 100,
        exp: 10,
        spawn_count: 1,
        spawn_share_percent: 25,
    };
    let unsorted = MapData {
        summary: None,
        mobs: vec![mob("Stump", 5), mob("Ice Sentinel", 30), mob("Fire Sentinel", 30), mob("Snail", 1)],
        ..MapData::default()
    };
    let source = FakeSource::meowdb().with_pages(move |_| Poll::Ready(Ok(unsorted.clone())));
    let mut compass = Compass::new(source);
    compass.on_frame(sample_frame(SUNSET, 0));

    let state = compass.on_frame(sample_frame(SUNSET, 333));

    let MapDataStatus::Ready(data) = &state.data else { panic!("{:?}", state.data) };
    let order: Vec<_> = data.mobs.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(order, ["Snail", "Stump", "Fire Sentinel", "Ice Sentinel"]);
}

#[test]
fn the_shown_map_carries_its_meowdb_page_and_the_meowdb_credit() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_frame(sample_frame(SUNSET, 0));

    let state = compass.on_frame(sample_frame(SUNSET, 333));

    assert_eq!(
        state.map.as_ref().map(|m| m.url.as_str()),
        Some("https://meowdb.com/msclassic/maps/010003070")
    );
    assert_eq!(state.credit, Some("NiaMeowDB (meowdb.com)"));
    assert_eq!(MEOWDB_CREDIT, "NiaMeowDB (meowdb.com)");
}

#[test]
fn the_hotkey_to_open_the_maps_meowdb_page_defaults_to_ctrl_shift_o_and_older_settings_get_it() {
    let older: Settings = serde_json::from_str(
        r#"{"version":1,"expanded":true,"hotkeys":{"togglePanel":"ctrl+shift+m","editMode":"ctrl+shift+e"}}"#,
    )
    .unwrap();

    assert_eq!(older.hotkeys.open_meowdb_page, "ctrl+shift+o");
    assert_eq!(Settings::default().hotkeys.open_meowdb_page, "ctrl+shift+o");
}
