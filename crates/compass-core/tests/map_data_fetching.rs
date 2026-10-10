//! The recognised map shows with its data from OSMS's file set, which the core holds
//! whole: no page is asked for, and no map waits for one. The panel lists the most
//! common mobs, credits both sites, and still opens the map's MeowDB page.

mod common;

use common::osms::{self, SUNSET_SKY, map, mob};
use common::{FakeSource, arrive, sample_frame};
use compass_core::{Compass, LevelRange, MapData, MapDataStatus, MoreMobs, OSMS_AND_MEOWDB_CREDIT, PanelState, Request};

const SUNSET: &str = "victoria-road__sunset-sky--window";
const KERNING: &str = "victoria-road__kerning-city--window";

fn shown_map_name(state: &PanelState) -> Option<&str> {
    state.map.as_ref().map(|m| m.map_name.as_str())
}

#[test]
fn the_recognised_map_shows_at_once_with_its_data() {
    let mut compass = Compass::new(FakeSource::osms());
    compass.on_frame(sample_frame(SUNSET, 0));

    let state = compass.on_frame(sample_frame(SUNSET, 333));

    assert_eq!(shown_map_name(state), Some("Sunset Sky"));
    assert!(matches!(&state.data, MapDataStatus::Ready(data) if data.mobs.len() == 5));
}

#[test]
fn before_any_map_is_shown_there_is_no_data_and_no_credit() {
    let mut compass = Compass::new(FakeSource::osms());

    let state = compass.on_frame(sample_frame(SUNSET, 0));

    assert_eq!(state.data, MapDataStatus::NoMap);
    assert_eq!(state.credit, None);
}

/// The panel when the shown map, Sunset Sky, has these mobs: (ID, name, level, spawn
/// points).
fn sunset_with(mobs: &[(u64, &str, u32, u32)]) -> MapData {
    let mut world = osms::game().with(map(SUNSET_SKY, "Victoria Road", "Sunset Sky"));
    for &(id, name, level, points) in mobs {
        world = world.with_mob(mob(id, name, level, 100, 10)).changing(SUNSET_SKY, |m| m.spawns(id, points));
    }
    let mut compass = Compass::new(FakeSource::with_world(world));
    let state = arrive(&mut compass, SUNSET, 0);
    let MapDataStatus::Ready(data) = &state.data else { panic!("{:?}", state.data) };
    data.clone()
}

#[test]
fn mobs_are_sorted_by_level_then_name_whatever_order_osms_gives() {
    let data = sunset_with(&[(101, "Stump", 5, 1), (102, "Ice Sentinel", 30, 1), (103, "Fire Sentinel", 30, 1), (104, "Snail", 1, 1)]);

    let order: Vec<_> = data.mobs.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(order, ["Snail", "Stump", "Fire Sentinel", "Ice Sentinel"]);
}

/// Of 100 spawn points, so each mob's Spawn Share is its count.
#[test]
fn the_panel_lists_the_five_most_common_mobs_by_level_and_sums_up_the_rest() {
    let data = sunset_with(&[
        (101, "Wild Boar", 25, 20),
        (102, "Snail", 1, 2),
        (103, "Stump", 5, 18),
        (104, "Fire Boar", 32, 3),
        (105, "Horny Mushroom", 22, 15),
        (106, "Axe Stump", 17, 12),
        (107, "Ghost Stump", 19, 25),
        (108, "Ligator", 33, 5),
    ]);

    let listed: Vec<_> = data.listed_mobs.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(listed, ["Stump", "Axe Stump", "Ghost Stump", "Horny Mushroom", "Wild Boar"]);
    assert_eq!(data.more_mobs, Some(MoreMobs { count: 3, levels: LevelRange { lowest: 1, highest: 33 } }));
    // Every mob is still there for comparing maps, in level order.
    assert_eq!(data.mobs.len(), 8);
}

#[test]
fn with_five_mobs_or_fewer_the_panel_lists_them_all() {
    let data = sunset_with(&[(101, "Stump", 5, 1), (102, "Snail", 1, 1)]);

    let listed: Vec<_> = data.listed_mobs.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(listed, ["Snail", "Stump"]);
    assert_eq!(data.more_mobs, None);
}

/// Of 20 spawn points: A and B 6 each (30%), C 4 (20%), D 2 (10%), Late and Early 1
/// each (5%).
#[test]
fn when_mobs_tie_for_the_last_listed_place_the_lower_level_one_is_listed() {
    let data = sunset_with(&[
        (101, "A", 10, 6),
        (102, "B", 11, 6),
        (103, "C", 12, 4),
        (104, "D", 13, 2),
        (105, "Late", 40, 1),
        (106, "Early", 2, 1),
    ]);

    let listed: Vec<_> = data.listed_mobs.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(listed, ["Early", "A", "B", "C", "D"]);
    assert_eq!(data.more_mobs, Some(MoreMobs { count: 1, levels: LevelRange { lowest: 40, highest: 40 } }));
}

/// The data is OSMS Data Explorer's; the title and the mobs' names open NiaMeowDB's pages.
#[test]
fn the_shown_map_credits_osms_for_its_data_and_meowdb_for_its_pages() {
    let mut compass = Compass::new(FakeSource::osms());

    let state = arrive(&mut compass, KERNING, 0);

    assert_eq!(state.credit, Some(OSMS_AND_MEOWDB_CREDIT));
    assert_eq!(OSMS_AND_MEOWDB_CREDIT, "Data: OSMS Data Explorer (osmsdataexplorer.com) · Pages: NiaMeowDB (meowdb.com)");
}

#[test]
fn the_shown_map_still_links_to_its_meowdb_page() {
    let mut compass = Compass::new(FakeSource::osms());

    let state = arrive(&mut compass, SUNSET, 0);

    assert_eq!(state.map.as_ref().map(|m| m.url.as_str()), Some("https://meowdb.com/msclassic/maps/010003070"));
}

/// Settings from before this hotkey existed get Ctrl+Shift+M.
#[test]
fn ctrl_shift_m_opens_the_shown_maps_meowdb_page_even_with_settings_from_before_it_existed() {
    let older = r#"{"version":1,"expanded":true,"hotkeys":{"togglePanel":"ctrl+shift+m","editMode":"ctrl+shift+e"}}"#;
    let mut compass = Compass::new(FakeSource::osms());
    compass.restore(serde_json::from_str(older).unwrap());
    arrive(&mut compass, SUNSET, 0);

    let opened = compass.on_hotkey("ctrl+shift+m");

    assert_eq!(opened, Some(Request::OpenInBrowser { url: "https://meowdb.com/msclassic/maps/010003070".into() }));
}

/// The overlay gets the panel state as JSON: what the panel renders, and nothing of the
/// world graph.
#[test]
fn the_overlay_receives_the_shown_maps_data_without_its_portal_graph() {
    let mut compass = Compass::new(FakeSource::osms());

    let json = serde_json::to_value(arrive(&mut compass, SUNSET, 0)).unwrap();

    assert_eq!(json["data"]["status"], "ready");
    assert_eq!(json["data"]["mobs"].as_array().map(Vec::len), Some(5));
    assert!(json["data"]["summary"].is_object() && json["data"]["hiddenPortals"].is_array());
    assert!(json["data"].get("connectedMaps").is_none() && json["data"].get("portals").is_none());
    assert_eq!(json["recognition"], serde_json::json!({ "status": "ready" }));
}
