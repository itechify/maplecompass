//! The recognised map shows straight away; its MeowDB data follows. A map's page is
//! asked for only once the panel is sure of the map (ADR 0003: never ahead of time),
//! and a slow or unreachable MeowDB never hides which map the player is on.

mod common;

use common::{FakeSource, black_frame, requests_for, sample_frame};
use compass_core::{
    Compass, LevelRange, MEOWDB_AND_IMAGES_CREDIT, MEOWDB_CREDIT, MapData, MapDataStatus, Mob, MoreMobs, PanelState, Request,
};
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
        id: name.into(),
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
    // Its listed mobs' Mob Cards show images from maplestory.io.
    assert_eq!(state.credit, Some("NiaMeowDB (meowdb.com) · images: maplestory.io"));
    assert_eq!(MEOWDB_CREDIT, "NiaMeowDB (meowdb.com)");
}

/// Settings from before this hotkey existed get Ctrl+Shift+M.
#[test]
fn ctrl_shift_m_opens_the_shown_maps_page_even_with_settings_from_before_it_existed() {
    let older = r#"{"version":1,"expanded":true,"hotkeys":{"togglePanel":"ctrl+shift+m","editMode":"ctrl+shift+e"}}"#;
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.restore(serde_json::from_str(older).unwrap());
    compass.on_frame(sample_frame(SUNSET, 0));
    compass.on_frame(sample_frame(SUNSET, 333));

    let opened = compass.on_hotkey("ctrl+shift+m");

    assert_eq!(opened, Some(Request::OpenInBrowser { url: "https://meowdb.com/msclassic/maps/010003070".into() }));
}

/// When a saved copy was made: 2026-10-01 00:00 UTC, in ms since the Unix epoch.
const SAVED_AT: u64 = 1_790_812_800_000;

#[test]
fn when_meowdb_is_unreachable_a_saved_copy_of_the_page_is_shown_with_when_it_was_saved() {
    let mut compass = Compass::new(FakeSource::meowdb().serving_saved_copies(SAVED_AT));
    compass.on_frame(sample_frame(SUNSET, 0));

    let state = compass.on_frame(sample_frame(SUNSET, 333));

    assert_eq!(shown_map_name(state), Some("Sunset Sky"));
    let MapDataStatus::SavedCopy { data, saved_at_unix_ms } = &state.data else { panic!("{:?}", state.data) };
    assert_eq!(*saved_at_unix_ms, SAVED_AT);
    assert_eq!(data.mobs.len(), 5);
    assert_eq!(state.credit, Some(MEOWDB_AND_IMAGES_CREDIT));
}

/// The overlay gets the panel state as JSON: a saved copy is the map's data with its
/// status and date beside it, and a saved map index says so with its date.
#[test]
fn the_overlay_receives_a_saved_copy_with_its_date_beside_the_data() {
    let mut compass = Compass::new(FakeSource::meowdb().serving_saved_copies(SAVED_AT));
    compass.on_frame(sample_frame(SUNSET, 0));

    let json = serde_json::to_value(compass.on_frame(sample_frame(SUNSET, 333))).unwrap();

    assert_eq!(json["data"]["status"], "savedCopy");
    assert_eq!(json["data"]["savedAtUnixMs"], SAVED_AT);
    assert_eq!(json["data"]["mobs"].as_array().map(Vec::len), Some(5));
    assert!(json["data"]["summary"].is_object() && json["data"]["hiddenPortals"].is_array());
    assert!(json["data"].get("connectedMaps").is_none());
    assert_eq!(json["recognition"], serde_json::json!({ "status": "savedCopy", "savedAtUnixMs": SAVED_AT }));
}

/// A mob with this spawn share; the rest of its figures don't matter here.
fn mob_with_share(name: &str, level: u32, spawn_share_percent: u32) -> Mob {
    Mob { id: name.into(), name: name.into(), level, hp: 100, exp: 10, spawn_count: 1, spawn_share_percent }
}

/// The panel when the shown map's page gives these mobs.
fn panel_with_mobs(mobs: Vec<Mob>) -> MapData {
    let page = MapData { mobs, ..MapData::default() };
    let source = FakeSource::meowdb().with_pages(move |_| Poll::Ready(Ok(page.clone())));
    let mut compass = Compass::new(source);
    compass.on_frame(sample_frame(SUNSET, 0));
    let state = compass.on_frame(sample_frame(SUNSET, 333));
    let MapDataStatus::Ready(data) = &state.data else { panic!("{:?}", state.data) };
    data.clone()
}

#[test]
fn the_panel_lists_the_five_most_common_mobs_by_level_and_sums_up_the_rest() {
    let data = panel_with_mobs(vec![
        mob_with_share("Wild Boar", 25, 20),
        mob_with_share("Snail", 1, 2),
        mob_with_share("Stump", 5, 18),
        mob_with_share("Fire Boar", 32, 3),
        mob_with_share("Horny Mushroom", 22, 15),
        mob_with_share("Axe Stump", 17, 12),
        mob_with_share("Ghost Stump", 19, 25),
        mob_with_share("Ligator", 33, 5),
    ]);

    let listed: Vec<_> = data.listed_mobs.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(listed, ["Stump", "Axe Stump", "Ghost Stump", "Horny Mushroom", "Wild Boar"]);
    assert_eq!(
        data.more_mobs,
        Some(MoreMobs { count: 3, levels: LevelRange { lowest: 1, highest: 33 } })
    );
    // Every mob is still there for comparing maps, in level order.
    assert_eq!(data.mobs.len(), 8);
}

#[test]
fn with_five_mobs_or_fewer_the_panel_lists_them_all() {
    let data = panel_with_mobs(vec![
        mob_with_share("Stump", 5, 50),
        mob_with_share("Snail", 1, 50),
    ]);

    let listed: Vec<_> = data.listed_mobs.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(listed, ["Snail", "Stump"]);
    assert_eq!(data.more_mobs, None);
}

#[test]
fn when_mobs_tie_for_the_last_listed_place_the_lower_level_one_is_listed() {
    let data = panel_with_mobs(vec![
        mob_with_share("A", 10, 30),
        mob_with_share("B", 11, 30),
        mob_with_share("C", 12, 20),
        mob_with_share("D", 13, 10),
        mob_with_share("Late", 40, 5),
        mob_with_share("Early", 2, 5),
    ]);

    let listed: Vec<_> = data.listed_mobs.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(listed, ["Early", "A", "B", "C", "D"]);
    assert_eq!(
        data.more_mobs,
        Some(MoreMobs { count: 1, levels: LevelRange { lowest: 40, highest: 40 } })
    );
}
