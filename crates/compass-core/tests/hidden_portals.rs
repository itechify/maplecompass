//! Once the panel has a map's MeowDB page, it lists the map's Hidden Portals: the exits
//! the player can't see, where each leads and roughly where it is. Pages are the ones
//! saved on 2026-10-07 (personal-use test data, ADR 0003), served by a fake map data
//! source.

mod common;

use common::{FakeSource, sample_frame, saved_index, saved_page};
use compass_core::{Compass, HiddenPortal, MapDataStatus, MapEntry, PanelState};

const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const SUNSET: &str = "victoria-road__sunset-sky--window";
const NORTHERN_TOP: &str = "hidden-street__northern-top-of-construction-site--window";

const CONSTRUCTION_SITE_ID: &str = "010003080";
const KERNING_CITY_ID: &str = "010003000";
const HENESYS_HUNTING_GROUND_I: &str = "010001010";
const FIELD_EAST_OF_AMHERST: &str = "000001020";
const THREE_WAY_ROAD_SPLIT: &str = "010000020";
const ORBIS_TOWER_8F: &str = "020000098";
const ORBIS_TOWER_HIDDEN_ROOM: &str = "020000099";

/// Two confident reads of a sample: the panel switches to its map.
fn arrive(compass: &mut Compass, sample: &str, at_ms: u64) -> PanelState {
    compass.on_frame(sample_frame(sample, at_ms));
    compass.on_frame(sample_frame(sample, at_ms + 333)).clone()
}

/// Arrive on map `id`, which has no minimap sample: the index here files the Sunset Sky
/// title under that ID.
fn arrive_on(id: &str, source: impl FnOnce(Vec<MapEntry>) -> FakeSource) -> PanelState {
    let index = saved_index()
        .into_iter()
        .map(|m| if m.map_name == "Sunset Sky" { MapEntry { id: id.into(), ..m } } else { m })
        .collect();
    let mut compass = Compass::new(source(index));
    arrive(&mut compass, SUNSET, 0)
}

/// Where each of the shown Hidden Portals leads, and roughly where it is.
fn destinations(state: &PanelState) -> Vec<(&str, &str)> {
    hidden_portals(state).iter().map(|p| (p.to_map_name.as_str(), p.position.as_str())).collect()
}

fn hidden_portals(state: &PanelState) -> &[HiddenPortal] {
    match &state.data {
        MapDataStatus::Ready(data) => &data.hidden_portals,
        other => panic!("no map data: {other:?}"),
    }
}

#[test]
fn the_construction_site_north_of_kerning_city_has_a_hidden_portal_up_on_the_right_to_a_hidden_street() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = arrive(&mut compass, CONSTRUCTION_SITE, 0);

    assert_eq!(
        hidden_portals(&state),
        [HiddenPortal {
            name: "hiden00".into(),
            to_map_id: "010003081".into(),
            to_map_name: "Northern Top of Construction Site".into(),
            to_street_name: "Hidden Street".into(),
            to_hidden_street: true,
            quest: false,
            position: "right, upper".into(),
        }]
    );
}

/// Kerning City has 14 invisible exits (type 1): its building doors and the way down to
/// The Swamp of Despair. On a Town they are Doors, drawn into the scenery, not hidden.
#[test]
fn the_doors_of_a_town_are_not_hidden_portals() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = arrive(&mut compass, KERNING, 0);

    assert_eq!(hidden_portals(&state), []);
}

/// Henesys Hunting Ground I has two visible exits and one walk-into exit (type 3) to
/// Hunting Ground II, which counts as visible until it's checked in-game. Sunset Sky's
/// other portals are teleports within the map. Northern Top's only exit is visible.
#[test]
fn visible_exits_walk_into_exits_and_teleports_within_the_map_are_not_hidden_portals() {
    let state = arrive_on(HENESYS_HUNTING_GROUND_I, FakeSource::with_index);
    assert_eq!(hidden_portals(&state), []);

    for sample in [SUNSET, NORTHERN_TOP] {
        let mut compass = Compass::new(FakeSource::meowdb());
        let state = arrive(&mut compass, sample, 0);
        assert_eq!(hidden_portals(&state), [], "{sample}");
    }
}

#[test]
fn each_hidden_portal_has_a_rough_position_on_the_map() {
    let cases = [
        (FIELD_EAST_OF_AMHERST, vec![("Tomato Field", "far right, middle")]),
        (
            THREE_WAY_ROAD_SPLIT,
            vec![("The Pig Beach", "left, lower"), ("Beach Hunting Ground", "centre, lower")],
        ),
        (ORBIS_TOWER_8F, vec![("Orbis Tower <Hidden Room>", "centre, middle")]),
    ];
    for (id, expected) in cases {
        let state = arrive_on(id, FakeSource::with_index);
        assert_eq!(destinations(&state), expected, "{id}");
    }
}

#[test]
fn only_destinations_on_a_hidden_street_are_labelled_hidden_street() {
    let labelled = |id| {
        let state = arrive_on(id, FakeSource::with_index);
        hidden_portals(&state).iter().map(|p| (p.to_map_name.clone(), p.to_hidden_street)).collect::<Vec<_>>()
    };

    assert_eq!(labelled(FIELD_EAST_OF_AMHERST), [("Tomato Field".into(), false)]);
    assert_eq!(
        labelled(THREE_WAY_ROAD_SPLIT),
        [("The Pig Beach".into(), true), ("Beach Hunting Ground".into(), true)]
    );
}

/// The Construction Site's page, with `from` changed to `to` in its invisible exit to
/// the Northern Top.
fn construction_site_with(from: &str, to: &str) -> String {
    const HIDDEN_EXIT: &str =
        r#"\"name\":\"hiden00\",\"type\":1,\"x\":-3415,\"y\":-733,\"toMapId\":\"010003081\""#;
    let page = saved_page(CONSTRUCTION_SITE_ID).unwrap();
    assert!(page.contains(HIDDEN_EXIT) && HIDDEN_EXIT.contains(from));
    page.replace(HIDDEN_EXIT, &HIDDEN_EXIT.replace(from, to))
}

/// No saved page has a script portal (type 11), so this one is made up: the
/// Construction Site's invisible exit, turned into a script portal.
#[test]
fn a_script_portal_is_a_quest_portal() {
    let page = construction_site_with(r#"\"type\":1,"#, r#"\"type\":11,"#);
    let mut compass = Compass::new(FakeSource::meowdb().with_page(CONSTRUCTION_SITE_ID, page));

    let state = arrive(&mut compass, CONSTRUCTION_SITE, 0);

    let quest: Vec<_> = hidden_portals(&state).iter().map(|p| (p.to_map_name.as_str(), p.quest)).collect();
    assert_eq!(quest, [("Northern Top of Construction Site", true)]);
}

/// Made up, like the script portal: two of Kerning City's Doors turned into a hidden
/// portal (type 10) and a script portal (type 11). Only invisible exits are Doors.
#[test]
fn a_towns_hidden_and_script_portals_are_still_hidden_portals() {
    let page = saved_page(KERNING_CITY_ID)
        .unwrap()
        .replace(r#"\"name\":\"in05\",\"type\":1,"#, r#"\"name\":\"in05\",\"type\":10,"#)
        .replace(r#"\"name\":\"in02\",\"type\":1,"#, r#"\"name\":\"in02\",\"type\":11,"#);
    let mut compass = Compass::new(FakeSource::meowdb().with_page(KERNING_CITY_ID, page));

    let state = arrive(&mut compass, KERNING, 0);

    let shown: Vec<_> = hidden_portals(&state).iter().map(|p| (p.to_map_name.as_str(), p.quest)).collect();
    assert_eq!(shown, [("Kerning City Pharmacy", true), ("Kerning City Hair Salon", false)]);
}

/// Made up: no saved page has an exit back into its own map, or to MeowDB's dead end.
#[test]
fn an_exit_into_the_same_map_or_to_the_dead_end_is_not_a_hidden_portal() {
    let own_map = r#"\"toMapId\":\"010003080\""#;
    let dead_end = r#"\"toMapId\":\"999999999\""#;
    for to in [own_map, dead_end] {
        let page = construction_site_with(r#"\"toMapId\":\"010003081\""#, to);
        let mut compass = Compass::new(FakeSource::meowdb().with_page(CONSTRUCTION_SITE_ID, page));

        let state = arrive(&mut compass, CONSTRUCTION_SITE, 0);

        assert_eq!(hidden_portals(&state), [], "{to}");
    }
}

/// The Hidden Room has no minimap, so MeowDB gives it no bounds to place its exit by.
#[test]
fn on_a_map_without_a_minimap_a_hidden_portals_position_is_unknown() {
    let state = arrive_on(ORBIS_TOWER_HIDDEN_ROOM, FakeSource::with_index);

    assert_eq!(destinations(&state), [("Orbis Tower <8th Floor>", "position unknown")]);
}
