//! Distance: how many map changes each of the picked mob's maps is from the map the
//! panel shows, through portals. Within a Density Tier the nearest maps come first, and
//! a headline names the nearest map with the mob at all.
//!
//! Each test builds a small world. The map the player stands on is titled like one of
//! the minimap samples (Sunset Sky, Kerning City), so arriving with that sample puts
//! the panel on it.

mod common;

use common::arrive;
use common::world::{WorldMap, map, mob, orange, search_for, tick_through, world};
use compass_core::{Action, Compass, Distance, PanelState, PortalKind};

const SUNSET: &str = "victoria-road__sunset-sky--window";
const KERNING: &str = "victoria-road__kerning-city--window";

/// The map the player arrives on with the Sunset Sky sample.
fn sunset_sky(id: &str) -> WorldMap {
    map(id, "Sunset Sky", 100, vec![])
}

/// A core over `maps`, crawled, with the player arrived on Sunset Sky and the Orange
/// Mushroom picked.
fn searched_from_sunset_sky(maps: Vec<WorldMap>) -> PanelState {
    let mut compass = Compass::new(world(maps));
    compass.tick(0);
    compass.tick(50);
    arrive(&mut compass, SUNSET, 100);
    search_for(&mut compass, "orange")
}

/// Each result as "Map Name: Distance".
fn distances(state: &PanelState) -> Vec<String> {
    state.search.results.iter().map(|r| format!("{}: {}", r.map_name, shown(r.distance))).collect()
}

fn shown(distance: Distance) -> String {
    match distance {
        Distance::MapChanges(n) => n.to_string(),
        Distance::NoRoute => "no route".into(),
        Distance::QuestOnly => "quest only".into(),
        Distance::Unknown => "?".into(),
    }
}

#[test]
fn distance_counts_the_map_changes_through_portals_and_hidden_portals() {
    let state = searched_from_sunset_sky(vec![
        sunset_sky("100000000").to(&["100000001"]),
        map("100000001", "Mushroom Field", 100, vec![orange(10)]).exit(PortalKind::Hidden, "100000002"),
        map("100000002", "Hidden Mushrooms", 100, vec![orange(10)]).to(&["100000003"]),
        map("100000003", "Deep Mushrooms", 100, vec![orange(10)]),
    ]);

    assert_eq!(distances(&state), ["Mushroom Field: 1", "Hidden Mushrooms: 2", "Deep Mushrooms: 3"]);
}

#[test]
fn one_way_exits_are_only_taken_the_way_they_go() {
    // The Ledge drops down to Sunset Sky, and Sunset Sky down to the Pit: neither leads
    // back up.
    let state = searched_from_sunset_sky(vec![
        map("100000001", "Ledge", 100, vec![orange(10)]).to(&["100000000"]),
        sunset_sky("100000000").to(&["100000002"]),
        map("100000002", "Pit", 100, vec![orange(10)]),
    ]);

    assert_eq!(distances(&state), ["Pit: 1", "Ledge: no route"]);
}

#[test]
fn quest_portals_are_never_counted_and_a_map_only_they_reach_is_quest_only() {
    let state = searched_from_sunset_sky(vec![
        sunset_sky("100000000")
            .exit(PortalKind::Quest, "100000003")
            .exit(PortalKind::Quest, "100000004")
            .to(&["100000001"]),
        map("100000001", "Road", 100, vec![]).to(&["100000002"]),
        map("100000002", "Long Way Round", 100, vec![]).to(&["100000003"]),
        map("100000003", "Mushroom Field", 100, vec![orange(10)]),
        map("100000004", "Quest Mushrooms", 100, vec![orange(10)]),
    ]);

    assert_eq!(distances(&state), ["Mushroom Field: 3", "Quest Mushrooms: quest only"]);
}

#[test]
fn doors_count_as_map_changes() {
    let state = searched_from_sunset_sky(vec![
        sunset_sky("100000000").exit(PortalKind::Door, "100000001"),
        map("100000001", "Mushroom Shop", 100, vec![orange(10)]),
    ]);

    assert_eq!(distances(&state), ["Mushroom Shop: 1"]);
}

/// Each result as "Map Name: Distance, Tier".
fn ranked(state: &PanelState) -> Vec<String> {
    state.search.results.iter().map(|r| format!("{}: {}, {:?}", r.map_name, shown(r.distance), r.tier)).collect()
}

#[test]
fn within_a_tier_maps_go_by_distance_then_density_then_no_route_then_quest_only() {
    let state = searched_from_sunset_sky(vec![
        sunset_sky("100000000").exit(PortalKind::Quest, "100000006").to(&["100000001", "100000002", "100000007"]),
        map("100000001", "Near Sparse", 100, vec![orange(8)]).to(&["100000003", "100000004"]),
        map("100000002", "Near Dense", 100, vec![orange(9)]),
        map("100000003", "Far Densest", 100, vec![orange(10)]),
        map("100000004", "Far Thin", 100, vec![orange(1)]),
        map("100000005", "Unreachable Dense", 100, vec![orange(10)]),
        map("100000006", "Quest Dense", 100, vec![orange(10)]),
        map("100000007", "Near Thin", 100, vec![orange(2)]),
        map("100000008", "Unreachable Thin", 100, vec![orange(1)]),
    ]);

    assert_eq!(
        ranked(&state),
        [
            "Near Dense: 1, High",
            "Near Sparse: 1, High",
            "Far Densest: 2, High",
            "Unreachable Dense: no route, High",
            "Quest Dense: quest only, High",
            "Near Thin: 1, Low",
            "Far Thin: 2, Low",
            "Unreachable Thin: no route, Low",
        ]
    );
}

/// The headline: the nearest map with the mob, and its Distance.
fn nearest(state: &PanelState) -> Option<String> {
    state.search.nearest.as_ref().map(|r| format!("{}: {}", r.map_name, shown(r.distance)))
}

#[test]
fn the_headline_names_the_nearest_map_with_the_mob_whatever_its_tier() {
    let state = searched_from_sunset_sky(vec![
        sunset_sky("100000000").exit(PortalKind::Quest, "100000004").to(&["100000001"]),
        map("100000001", "Road", 100, vec![]).to(&["100000002"]),
        map("100000002", "Thin Field", 100, vec![orange(1)]).to(&["100000003"]),
        map("100000003", "Dense Field", 100, vec![orange(10)]),
        map("100000004", "Quest Mushrooms", 100, vec![orange(10)]),
        map("100000005", "Lost Mushrooms", 100, vec![orange(10)]),
    ]);

    assert_eq!(state.search.results[0].map_name, "Dense Field");
    assert_eq!(nearest(&state).as_deref(), Some("Thin Field: 2"));
}

#[test]
fn of_maps_equally_near_the_headline_names_the_denser() {
    let state = searched_from_sunset_sky(vec![
        sunset_sky("100000000").to(&["100000001", "100000002", "100000003"]),
        map("100000001", "A Field", 100, vec![orange(4)]),
        map("100000002", "Busy Field", 100, vec![orange(6)]),
        map("100000003", "Calm Field", 100, vec![orange(5)]),
    ]);

    assert_eq!(nearest(&state).as_deref(), Some("Busy Field: 1"));
}

#[test]
fn on_a_map_with_the_mob_the_headline_names_that_map() {
    let state = searched_from_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![orange(1)]).to(&["100000001"]),
        map("100000001", "Dense Field", 100, vec![orange(10)]),
    ]);

    assert_eq!(nearest(&state).as_deref(), Some("Sunset Sky: 0"));
}

#[test]
fn with_no_map_reached_there_is_no_headline() {
    let state = searched_from_sunset_sky(vec![
        sunset_sky("100000000").exit(PortalKind::Quest, "100000001"),
        map("100000001", "Quest Mushrooms", 100, vec![orange(10)]),
        map("100000002", "Lost Mushrooms", 100, vec![orange(10)]),
    ]);

    assert_eq!(nearest(&state), None);
}

#[test]
fn distances_are_measured_again_when_the_player_changes_map() {
    let mut compass = Compass::new(world(vec![
        sunset_sky("100000000").to(&["100000001"]),
        map("100000001", "Kerning City", 100, vec![]).to(&["100000000", "100000002"]),
        map("100000002", "Mushroom Field", 100, vec![orange(10)]),
    ]));
    compass.tick(0);
    compass.tick(50);
    arrive(&mut compass, SUNSET, 100);
    let state = search_for(&mut compass, "orange");
    assert_eq!(distances(&state), ["Mushroom Field: 2"]);

    let state = arrive(&mut compass, KERNING, 1_000);

    assert_eq!(state.map.as_ref().map(|m| m.map_name.as_str()), Some("Kerning City"));
    assert_eq!(distances(&state), ["Mushroom Field: 1"]);
    assert_eq!(nearest(&state).as_deref(), Some("Mushroom Field: 1"));
}

#[test]
fn on_a_title_several_maps_share_distance_is_measured_from_the_candidate_shown() {
    // Two maps titled Kerning City, told apart by their mobs: the first is next to the
    // Mushroom Field, the second two map changes from it.
    let mut compass = Compass::new(world(vec![
        map("100000000", "Kerning City", 100, vec![]).to(&["100000002"]),
        map("100000001", "Kerning City", 100, vec![mob("3", "Blue Snail", 4, 1)]).to(&["100000003"]),
        map("100000002", "Mushroom Field", 100, vec![orange(10)]),
        map("100000003", "Road", 100, vec![]).to(&["100000002"]),
    ]));
    compass.tick(0);
    compass.tick(50);
    let state = arrive(&mut compass, KERNING, 100);
    assert!(state.candidates.is_some(), "the player may choose");
    let state = search_for(&mut compass, "orange");
    assert_eq!(distances(&state), ["Mushroom Field: 1"]);

    let state = compass.act(Action::ChooseCandidate);

    assert_eq!(state.map.as_ref().map(|m| m.id.as_str()), Some("100000001"));
    assert_eq!(distances(state), ["Mushroom Field: 2"]);
}

#[test]
fn distances_are_worked_out_again_as_the_crawl_adds_maps() {
    // Crawled in index order, one page every 2 s: the Road to the Mushroom Field last.
    let mut compass = Compass::new(
        world(vec![
            sunset_sky("100000000").to(&["100000002"]),
            map("100000001", "Mushroom Field", 100, vec![orange(10)]),
            map("100000002", "Road", 100, vec![]).to(&["100000001"]),
        ])
        .crawling_uncached(),
    );
    tick_through(&mut compass, 0, 100);
    arrive(&mut compass, SUNSET, 100);
    tick_through(&mut compass, 500, 2_100);
    let state = search_for(&mut compass, "orange");
    assert_eq!(state.search.crawl.known, 2);
    assert_eq!(distances(&state), ["Mushroom Field: no route"]);

    let state = tick_through(&mut compass, 2_100, 4_100);

    assert_eq!(state.search.crawl.known, 3);
    assert_eq!(distances(&state), ["Mushroom Field: 2"]);
}

#[test]
fn with_no_map_recognised_every_distance_is_unknown_and_maps_go_by_tier_then_density() {
    let mut compass = Compass::new(world(vec![
        sunset_sky("100000000").to(&["100000001"]),
        map("100000001", "Near Thin", 100, vec![orange(2)]),
        map("100000002", "Far Medium", 100, vec![orange(5)]),
        map("100000003", "Far Medium Denser", 100, vec![orange(6)]),
        map("100000004", "Densest", 100, vec![orange(10)]),
    ]));
    compass.tick(0);
    compass.tick(50);

    let state = search_for(&mut compass, "orange");

    assert_eq!(
        ranked(&state),
        ["Densest: ?, High", "Far Medium Denser: ?, Medium", "Far Medium: ?, Medium", "Near Thin: ?, Low"]
    );
    assert_eq!(nearest(&state), None);
}
