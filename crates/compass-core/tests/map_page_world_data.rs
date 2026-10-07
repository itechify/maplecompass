//! A map's MeowDB page also gives what mob search needs about the world (ADR 0004):
//! each mob's MeowDB ID, the map's Mob Rate, every exit to another map with its portal
//! kind, and the map's nearest town. Pages are the ones saved on 2026-10-07
//! (personal-use test data, ADR 0003), served by a fake map data source.

mod common;

use common::{FakeSource, arrive, saved_index, saved_page};
use compass_core::{Compass, MapData, MapDataStatus, MapEntry, PanelState, PortalKind};

const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const SUNSET: &str = "victoria-road__sunset-sky--window";

const CONSTRUCTION_SITE_ID: &str = "010003080";
const KERNING_CITY_ID: &str = "010003000";
const HENESYS_HUNTING_GROUND_I: &str = "010001010";
const HENESYS: &str = "010001000";

/// Arrive on map `id`, which has no minimap sample: the index here files the Sunset Sky
/// title under that ID.
fn arrive_on(id: &str) -> PanelState {
    let index = saved_index()
        .into_iter()
        .map(|m| if m.map_name == "Sunset Sky" { MapEntry { id: id.into(), ..m } } else { m })
        .collect();
    let mut compass = Compass::new(FakeSource::with_index(index));
    arrive(&mut compass, SUNSET, 0)
}

fn data(state: &PanelState) -> &MapData {
    match &state.data {
        MapDataStatus::Ready(data) => data,
        other => panic!("no map data: {other:?}"),
    }
}

/// Where each of the map's portals to other maps leads, and its kind, in MeowDB's order.
fn portals(state: &PanelState) -> Vec<(&str, PortalKind)> {
    data(state).portals.iter().map(|p| (p.to_map_id.as_str(), p.kind)).collect()
}

#[test]
fn henesys_hunting_ground_i_has_a_mob_rate_of_1_5_and_henesys_as_its_nearest_town() {
    let state = arrive_on(HENESYS_HUNTING_GROUND_I);

    assert_eq!(data(&state).mob_rate_percent, 150);
    assert_eq!(data(&state).nearest_town.as_deref(), Some(HENESYS));
}

#[test]
fn each_mob_comes_with_its_meowdb_id() {
    let state = arrive_on(HENESYS_HUNTING_GROUND_I);

    let mut ids: Vec<_> = data(&state).mobs.iter().map(|m| (m.name.as_str(), m.id.as_str())).collect();
    ids.sort();
    assert_eq!(
        ids,
        [
            ("Blue Snail", "3"),
            ("Green Mushroom", "13"),
            ("Orange Mushroom", "9"),
            ("Red Snail", "5"),
            ("Shroom", "4"),
            ("Slime", "7"),
            ("Stump", "6"),
        ]
    );
}

#[test]
fn mob_rates_other_than_whole_numbers_are_kept_to_the_hundredth() {
    // Sunset Sky 0.5, the Construction Site 1.2, the Northern Top 2, Dead Mine IV 0.8.
    for (id, percent) in [("010003070", 50), (CONSTRUCTION_SITE_ID, 120), ("010003081", 200), ("020001067", 80)] {
        assert_eq!(data(&arrive_on(id)).mob_rate_percent, percent, "{id}");
    }
}

/// Henesys Hunting Ground I's two visible exits and its walk-into exit (type 3) to
/// Hunting Ground II, which counts as visible.
#[test]
fn every_exit_to_another_map_comes_with_its_portal_kind() {
    let state = arrive_on(HENESYS_HUNTING_GROUND_I);

    assert_eq!(
        portals(&state),
        [("010001020", PortalKind::Visible), (HENESYS, PortalKind::Visible), ("010001011", PortalKind::Visible)]
    );
}

#[test]
fn the_construction_sites_invisible_exit_is_a_hidden_portal() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = arrive(&mut compass, CONSTRUCTION_SITE, 0);

    assert_eq!(
        portals(&state),
        [
            ("010003070", PortalKind::Visible),
            ("010004030", PortalKind::Visible),
            ("010003081", PortalKind::Hidden),
        ]
    );
}

#[test]
fn a_towns_invisible_exits_are_doors_and_its_nearest_town_is_itself() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = arrive(&mut compass, KERNING, 0);

    let portals = portals(&state);
    assert!(portals.contains(&("010003001", PortalKind::Door)), "{portals:?}");
    assert!(portals.contains(&("010003090", PortalKind::Door)), "{portals:?}");
    assert!(portals.contains(&("010003070", PortalKind::Visible)), "{portals:?}");
    assert!(!portals.iter().any(|(_, kind)| *kind == PortalKind::Hidden));
    assert_eq!(data(&state).nearest_town.as_deref(), Some(KERNING_CITY_ID));
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

/// No saved page has a Quest Portal (MeowDB portal type 11), so this one is made up.
#[test]
fn a_type_11_exit_is_a_quest_portal() {
    let page = construction_site_with(r#"\"type\":1,"#, r#"\"type\":11,"#);
    let mut compass = Compass::new(FakeSource::meowdb().with_page(CONSTRUCTION_SITE_ID, page));

    let state = arrive(&mut compass, CONSTRUCTION_SITE, 0);

    assert_eq!(portals(&state)[2], ("010003081", PortalKind::Quest));
}

/// Made up: no saved page has an exit back into its own map, or to MeowDB's dead end.
#[test]
fn an_exit_into_the_same_map_or_to_the_dead_end_leads_to_no_other_map() {
    let own_map = r#"\"toMapId\":\"010003080\""#;
    let dead_end = r#"\"toMapId\":\"999999999\""#;
    for to in [own_map, dead_end] {
        let page = construction_site_with(r#"\"toMapId\":\"010003081\""#, to);
        let mut compass = Compass::new(FakeSource::meowdb().with_page(CONSTRUCTION_SITE_ID, page));

        let state = arrive(&mut compass, CONSTRUCTION_SITE, 0);

        assert_eq!(
            portals(&state),
            [("010003070", PortalKind::Visible), ("010004030", PortalKind::Visible)],
            "{to}"
        );
    }
}

#[test]
fn a_page_without_its_mob_rate_or_nearest_town_is_reported_as_a_format_change() {
    let page = saved_page(CONSTRUCTION_SITE_ID).unwrap();
    for field in ["mobRate", "returnMap"] {
        let renamed = page.replace(&format!(r#"\"{field}\""#), r#"\"renamed\""#);
        let mut compass = Compass::new(FakeSource::meowdb().with_page(CONSTRUCTION_SITE_ID, renamed));

        let state = arrive(&mut compass, CONSTRUCTION_SITE, 0);

        assert_eq!(state.data, MapDataStatus::FormatChanged, "{field}");
    }
}
