//! A map's MeowDB page also gives what mob search needs about the world (ADR 0004):
//! each mob's MeowDB ID, the map's Mob Rate, every exit to another map with its portal
//! kind, and the map's nearest town. They show in Density, Distance and routes. Pages
//! are the ones saved on 2026-10-07 (personal-use test data, ADR 0003), crawled from
//! the saved map index and served by a fake map data source.
//!
//! On those pages Kerning City leads to Sunset Sky, Sunset Sky to the Construction
//! Site, and the Construction Site's invisible exit (type 1, off a town, so a Hidden
//! Portal) is the only way up to its Northern Top.

mod common;

use common::world::{search_for, tick_through};
use common::{FakeSource, arrive, saved_index, saved_page};
use compass_core::{
    Action, Compass, Distance, MapData, MapDataStatus, MapEntry, PanelState, SearchResult, Step,
};

const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const SUNSET: &str = "victoria-road__sunset-sky--window";

const CONSTRUCTION_SITE_ID: &str = "010003080";
const HENESYS_HUNTING_GROUND_I: &str = "010001010";

/// Every saved page crawled, the player arrived on Kerning City, and `mob` searched for.
fn searched_from_kerning(source: FakeSource, mob: &str) -> PanelState {
    let mut compass = Compass::new(source);
    // The pages answer at once, a few each tick: the whole index in under 3 s.
    tick_through(&mut compass, 0, 3_000);
    arrive(&mut compass, KERNING, 3_100);
    search_for(&mut compass, mob)
}

/// The result on the map named `map_name`.
fn result<'a>(state: &'a PanelState, map_name: &str) -> &'a SearchResult {
    let found = state.search.results.iter().find(|r| r.map_name == map_name);
    found.unwrap_or_else(|| panic!("no result on {map_name}: {:?}", state.search.results))
}

fn data(state: &PanelState) -> &MapData {
    match &state.data {
        MapDataStatus::Ready(data) => data,
        other => panic!("no map data: {other:?}"),
    }
}

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

/// Orange Mushrooms: 5 on Henesys Hunting Ground I (Mob Rate 1.5), 8 on Sunset Sky
/// (0.5), 9 on the Construction Site (1.2) and 6 on its Northern Top (2); Flyeyes: 18 on
/// Dead Mine IV (0.8).
#[test]
fn each_maps_mob_rate_counts_in_density_to_the_hundredth() {
    let state = searched_from_kerning(FakeSource::meowdb(), "orange");

    for (map, density) in [
        ("Henesys Hunting Ground I", 750),
        ("Sunset Sky", 400),
        ("Construction Site North of Kerning City", 1_080),
        ("Northern Top of Construction Site", 1_200),
    ] {
        assert_eq!(result(&state, map).density_hundredths, density, "{map}");
    }
    let state = searched_from_kerning(FakeSource::meowdb(), "flyeye");
    assert_eq!(result(&state, "Dead Mine IV").density_hundredths, 1_440);
}

#[test]
fn every_exit_counts_in_distance_and_a_hidden_portal_too() {
    let state = searched_from_kerning(FakeSource::meowdb(), "orange");

    for (map, changes) in [
        ("Sunset Sky", 1),
        ("Construction Site North of Kerning City", 2),
        ("Northern Top of Construction Site", 3),
    ] {
        assert_eq!(result(&state, map).distance, Distance::MapChanges(changes), "{map}");
    }
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
fn a_map_only_a_type_11_exit_leads_to_is_quest_only() {
    let page = construction_site_with(r#"\"type\":1,"#, r#"\"type\":11,"#);

    let state = searched_from_kerning(FakeSource::meowdb().with_page(CONSTRUCTION_SITE_ID, page), "orange");

    assert_eq!(result(&state, "Northern Top of Construction Site").distance, Distance::QuestOnly);
}

/// Made up: no saved page has an exit back into its own map, or to MeowDB's dead end.
#[test]
fn an_exit_into_the_same_map_or_to_the_dead_end_leads_nowhere() {
    let own_map = r#"\"toMapId\":\"010003080\""#;
    let dead_end = r#"\"toMapId\":\"999999999\""#;
    for to in [own_map, dead_end] {
        let page = construction_site_with(r#"\"toMapId\":\"010003081\""#, to);

        let state = searched_from_kerning(FakeSource::meowdb().with_page(CONSTRUCTION_SITE_ID, page), "orange");

        let northern_top = result(&state, "Northern Top of Construction Site");
        assert_eq!(northern_top.distance, Distance::NoRoute, "{to}");
        assert_eq!(result(&state, "Construction Site North of Kerning City").distance, Distance::MapChanges(2));
    }
}

/// Every saved page has a named-scroll town as its nearest town, so the Construction
/// Site's is made up: Henesys Hunting Ground I, a map no named scroll goes to.
#[test]
fn a_nearest_town_teleport_scroll_goes_to_the_town_the_maps_page_names() {
    const RETURN_MAP: &str = r#"\"returnMap\":\"010003000\""#;
    let page = saved_page(CONSTRUCTION_SITE_ID).unwrap();
    assert!(page.contains(RETURN_MAP));
    let page = page.replace(RETURN_MAP, &RETURN_MAP.replace("010003000", HENESYS_HUNTING_GROUND_I));
    let mut compass = Compass::new(FakeSource::meowdb().with_page(CONSTRUCTION_SITE_ID, page));
    tick_through(&mut compass, 0, 3_000);
    arrive(&mut compass, KERNING, 3_100);
    let state = search_for(&mut compass, "orange");
    assert_eq!(result(&state, "Henesys Hunting Ground I").distance, Distance::NoRoute, "scrolls off");

    let state = compass.act(Action::ToggleTeleportScrolls);

    let hunting_ground = result(state, "Henesys Hunting Ground I");
    assert_eq!(hunting_ground.distance, Distance::MapChanges(3));
    assert_eq!(hunting_ground.route.last().unwrap().by, Some(Step::TeleportScroll));
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
