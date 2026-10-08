//! Ships: a ride between regions (Ellinia ↔ Orbis) is always one map change, so maps
//! in Ossyria have a Distance from Victoria Island and the other way round. Each result
//! has its route (the maps in order, each with how it is reached) and says which kinds
//! of travel besides portals that route uses.
//!
//! The ships run between the real stations (by their MeowDB IDs), so the hand-built
//! worlds here use those IDs for the stations.

mod common;

use common::world::{WorldMap, map, orange, search_for, tick_through};
use common::{FakeSource, arrive, index_with_title, saved_index};
use compass_core::{Compass, Distance, MapData, MapEntry, Mob, PanelState, Portal, PortalKind, SearchResult, Step};
use std::task::Poll;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const KERNING: &str = "victoria-road__kerning-city--window";

const ELLINIA: &str = "010002000";
const ELLINIA_STATION: &str = "010002090";
const ORBIS: &str = "020000000";
const ORBIS_TICKETING_BOOTH: &str = "020000010";

/// A core over `maps`, crawled, with the player arrived on the map named Sunset Sky
/// and the Orange Mushroom picked.
fn searched_from_sunset_sky(maps: Vec<WorldMap>) -> PanelState {
    let mut compass = Compass::new(common::world::world(maps));
    compass.tick(0);
    compass.tick(50);
    arrive(&mut compass, SUNSET, 100);
    search_for(&mut compass, "orange")
}

/// Each result as "Map: Distance".
fn distances(state: &PanelState) -> Vec<String> {
    state
        .search
        .results
        .iter()
        .map(|r| match r.distance {
            Distance::MapChanges(n) => format!("{}: {n}", r.map_name),
            other => format!("{}: {other:?}", r.map_name),
        })
        .collect()
}

#[test]
fn a_ship_ride_from_ellinia_to_orbis_is_one_map_change() {
    let state = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&[ELLINIA]),
        map(ELLINIA, "Ellinia", 100, vec![]).to(&[ELLINIA_STATION]),
        map(ELLINIA_STATION, "Ellinia Station", 100, vec![]).to(&[ELLINIA]),
        map(ORBIS_TICKETING_BOOTH, "Orbis Ticketing Booth", 100, vec![]).to(&[ORBIS]),
        map(ORBIS, "Orbis", 100, vec![]).to(&["200000001"]),
        map("200000001", "Cloud Park", 100, vec![orange(10)]),
    ]);

    assert_eq!(distances(&state), ["Cloud Park: 5"]);
}

/// A result's route as "Start, portal: Next, ship: Then…".
fn route(result: &SearchResult) -> String {
    result
        .route
        .iter()
        .map(|stop| match stop.by {
            None => stop.map_name.clone(),
            Some(step) => format!("{step:?}: {}", stop.map_name),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn a_result_reached_by_ship_has_its_route_and_says_it_takes_a_ship() {
    let state = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&[ELLINIA]),
        map(ELLINIA, "Ellinia", 100, vec![]).to(&[ELLINIA_STATION]),
        map(ELLINIA_STATION, "Ellinia Station", 100, vec![]).to(&[ELLINIA]),
        map(ORBIS_TICKETING_BOOTH, "Orbis Ticketing Booth", 100, vec![]).to(&[ORBIS]),
        map(ORBIS, "Orbis", 100, vec![]).to(&["200000001"]),
        map("200000001", "Cloud Park", 100, vec![orange(10)]),
    ]);

    let result = &state.search.results[0];
    assert_eq!(
        route(result),
        "Sunset Sky, Portal: Ellinia, Portal: Ellinia Station, Ship: Orbis Ticketing Booth, \
         Portal: Orbis, Portal: Cloud Park"
    );
    assert_eq!(result.travel, [Step::Ship]);
}

#[test]
fn the_ship_from_orbis_takes_the_player_back_to_victoria_island() {
    let state = searched_from_sunset_sky(vec![
        map("200000002", "Sunset Sky", 100, vec![]).to(&[ORBIS]),
        map(ORBIS, "Orbis", 100, vec![]).to(&[ORBIS_TICKETING_BOOTH]),
        map(ORBIS_TICKETING_BOOTH, "Orbis Ticketing Booth", 100, vec![]).to(&[ORBIS]),
        map(ELLINIA_STATION, "Ellinia Station", 100, vec![]).to(&[ELLINIA]),
        map(ELLINIA, "Ellinia", 100, vec![]).to(&["100000001"]),
        map("100000001", "Mushroom Field", 100, vec![orange(10)]),
    ]);

    let result = &state.search.results[0];
    assert_eq!(result.distance, Distance::MapChanges(5));
    assert_eq!(
        route(result),
        "Sunset Sky, Portal: Orbis, Portal: Orbis Ticketing Booth, Ship: Ellinia Station, \
         Portal: Ellinia, Portal: Mushroom Field"
    );
    assert_eq!(result.travel, [Step::Ship]);
}

/// A core over MeowDB's real map index (the saved copy), whose maps have these pages
/// (and the rest none), crawled, with the player arrived on the Kerning City sample and
/// the Orange Mushroom picked.
fn searched_in_the_real_index(index: Vec<MapEntry>, pages: Vec<(&'static str, MapData)>) -> PanelState {
    let source = FakeSource::with_index(index).with_pages(move |id| {
        Poll::Ready(Ok(pages.iter().find(|(page_id, _)| *page_id == id).map(|(_, p)| p.clone()).unwrap_or_default()))
    });
    let mut compass = Compass::new(source);
    let state = tick_through(&mut compass, 0, 5_000);
    assert!(state.search.crawl.done, "the whole index is crawled");
    arrive(&mut compass, KERNING, 5_100);
    search_for(&mut compass, "orange")
}

/// A page with visible exits to `to` and these mobs.
fn page(to: &[&str], mobs: Vec<Mob>) -> MapData {
    let portals = to
        .iter()
        .enumerate()
        .map(|(i, to)| Portal { name: format!("portal{i}"), to_map_id: (*to).into(), kind: PortalKind::Visible })
        .collect();
    MapData { portals, mobs, mob_rate_percent: 100, ..MapData::default() }
}

#[test]
fn the_ships_run_between_the_real_stations_in_meowdbs_map_index() {
    // Made-up portals from Kerning City to Ellinia and from Orbis to Orbis Tower 8F, to
    // keep the route short; the rest are the real stations' exits.
    let state = searched_in_the_real_index(
        saved_index(),
        vec![
            ("010003000", page(&[ELLINIA], vec![])),
            (ELLINIA, page(&[ELLINIA_STATION], vec![])),
            (ELLINIA_STATION, page(&[ELLINIA], vec![])),
            (ORBIS_TICKETING_BOOTH, page(&[ORBIS], vec![])),
            (ORBIS, page(&[ORBIS_TICKETING_BOOTH, "020000098"], vec![])),
            ("020000098", page(&[], vec![orange(10)])),
        ],
    );

    let result = &state.search.results[0];
    assert_eq!(result.map_name, "Orbis Tower <8th Floor>");
    assert_eq!(result.distance, Distance::MapChanges(5));
    assert_eq!(
        route(result),
        "Kerning City, Portal: Ellinia, Portal: Ellinia Station, Ship: Orbis Ticketing Booth, \
         Portal: Orbis, Portal: Orbis Tower <8th Floor>"
    );
    assert_eq!(result.travel, [Step::Ship]);
}

#[test]
fn the_ship_from_the_real_orbis_station_reaches_victoria_island() {
    // The player is in the Orbis Weapon Store, given the Kerning City title so the
    // sample arrives there.
    let state = searched_in_the_real_index(
        index_with_title("Victoria Road", "Kerning City", &["020000001"]),
        vec![
            ("020000001", page(&[ORBIS], vec![])),
            (ORBIS, page(&[ORBIS_TICKETING_BOOTH], vec![])),
            (ORBIS_TICKETING_BOOTH, page(&[ORBIS], vec![])),
            (ELLINIA_STATION, page(&[ELLINIA], vec![])),
            (ELLINIA, page(&["010002010"], vec![])),
            ("010002010", page(&[], vec![orange(10)])),
        ],
    );

    let result = &state.search.results[0];
    assert_eq!(result.distance, Distance::MapChanges(5));
    assert_eq!(
        route(result),
        "Kerning City, Portal: Orbis, Portal: Orbis Ticketing Booth, Ship: Ellinia Station, \
         Portal: Ellinia, Portal: The Field South of Ellinia"
    );
    assert_eq!(result.travel, [Step::Ship]);
}

#[test]
fn shanks_sails_from_southperry_to_lith_harbor_and_no_ship_sails_back() {
    const SOUTHPERRY: &str = "000000060";
    const LITH_HARBOR: &str = "010000000";
    let state = searched_from_sunset_sky(vec![
        map(SOUTHPERRY, "Sunset Sky", 100, vec![orange(10)]),
        map(LITH_HARBOR, "Lith Harbor", 100, vec![orange(10)]),
    ]);
    assert_eq!(distances(&state), ["Sunset Sky: 0", "Lith Harbor: 1"]);
    assert_eq!(state.search.results[1].travel, [Step::Ship]);

    let state = searched_from_sunset_sky(vec![
        map(SOUTHPERRY, "Southperry", 100, vec![orange(10)]),
        map(LITH_HARBOR, "Sunset Sky", 100, vec![orange(10)]),
    ]);
    assert_eq!(distances(&state), ["Sunset Sky: 0", "Southperry: NoRoute"]);
}

#[test]
fn a_result_reached_by_portals_alone_has_its_route_and_no_travel_marker() {
    let state = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Road", 100, vec![]).to(&["100000002"]),
        map("100000002", "Mushroom Field", 100, vec![orange(10)]),
    ]);

    let result = &state.search.results[0];
    assert_eq!(route(result), "Sunset Sky, Portal: Road, Portal: Mushroom Field");
    assert_eq!(result.travel, []);
}

#[test]
fn of_two_equally_short_routes_the_one_by_portals_is_taken() {
    // Ellinia Station leads straight to the Orbis Ticketing Booth by ship, and by a
    // (made-up) portal too.
    let state = searched_from_sunset_sky(vec![
        map(ELLINIA_STATION, "Sunset Sky", 100, vec![]).to(&[ORBIS_TICKETING_BOOTH]),
        map(ORBIS_TICKETING_BOOTH, "Orbis Ticketing Booth", 100, vec![orange(10)]),
    ]);

    let result = &state.search.results[0];
    assert_eq!(route(result), "Sunset Sky, Portal: Orbis Ticketing Booth");
    assert_eq!(result.travel, []);
}

#[test]
fn on_the_map_with_the_mob_the_route_is_that_map_alone() {
    let state = searched_from_sunset_sky(vec![map("100000000", "Sunset Sky", 100, vec![orange(10)])]);

    assert_eq!(route(&state.search.results[0]), "Sunset Sky");
}

#[test]
fn a_map_with_no_route_or_only_a_quest_route_has_no_route_shown() {
    let state = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]).exit(PortalKind::Quest, "100000001"),
        map("100000001", "Quest Mushrooms", 100, vec![orange(10)]),
        map("100000002", "Lost Mushrooms", 100, vec![orange(10)]),
    ]);

    assert_eq!(distances(&state), ["Lost Mushrooms: NoRoute", "Quest Mushrooms: QuestOnly"]);
    assert!(state.search.results.iter().all(|r| r.route.is_empty() && r.travel.is_empty()));
}

#[test]
fn with_no_map_recognised_no_route_is_shown() {
    let mut compass = Compass::new(common::world::world(vec![
        map(ELLINIA_STATION, "Ellinia Station", 100, vec![]),
        map(ORBIS_TICKETING_BOOTH, "Orbis Ticketing Booth", 100, vec![orange(10)]),
    ]));
    compass.tick(0);
    compass.tick(50);

    let state = search_for(&mut compass, "orange");

    assert_eq!(distances(&state), ["Orbis Ticketing Booth: Unknown"]);
    assert!(state.search.results[0].route.is_empty());
    assert!(state.search.results[0].travel.is_empty());
}
