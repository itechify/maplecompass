//! The highlighted result: search mode shows the full route to one of the picked mob's
//! maps at a time. The arrow keys move the highlight through the results whenever no mob
//! suggestions are listed (as when search mode starts on a search, with nothing typed
//! yet).

mod common;

use common::arrive;
use common::world::{SEARCH, WorldMap, map, mob, orange, search_for, world};
use compass_core::{Action, Compass, PanelState};

const SUNSET: &str = "victoria-road__sunset-sky--window";
const KERNING: &str = "victoria-road__kerning-city--window";

/// Sunset Sky leads to the Near Field, the Near Field to the Far Field, and the Far
/// Field to Kerning City; Orange Mushrooms on both fields, Blue Snails on the Far Field.
fn fields() -> Vec<WorldMap> {
    vec![
        map("100000000", "Sunset Sky", 100, vec![]).to(&["100000001"]),
        map("100000001", "Near Field", 100, vec![orange(10)]).to(&["100000002"]),
        map("100000002", "Far Field", 100, vec![orange(10), mob("3", "Blue Snail", 2, 5)]).to(&["100000003"]),
        map("100000003", "Kerning City", 100, vec![]).to(&["100000002"]),
    ]
}

/// A core over `maps`, crawled, with the player arrived on Sunset Sky.
fn on_sunset_sky(maps: Vec<WorldMap>) -> Compass {
    let mut compass = Compass::new(world(maps));
    compass.tick(0);
    compass.tick(50);
    arrive(&mut compass, SUNSET, 100);
    compass
}

/// The highlighted result's map name.
fn highlighted(state: &PanelState) -> Option<&str> {
    state.search.highlighted_result.map(|at| state.search.results[at].map_name.as_str())
}

/// The highlighted result's route, as its map names.
fn highlighted_route(state: &PanelState) -> Vec<&str> {
    let at = state.search.highlighted_result.expect("a highlighted result");
    state.search.results[at].route.iter().map(|stop| stop.map_name.as_str()).collect()
}

#[test]
fn a_picked_mobs_first_result_is_highlighted_with_its_route() {
    let mut compass = on_sunset_sky(fields());

    let state = search_for(&mut compass, "orange");

    assert_eq!(highlighted(&state), Some("Near Field"));
    assert_eq!(highlighted_route(&state), ["Sunset Sky", "Near Field"]);
}

#[test]
fn with_nothing_typed_the_arrows_move_the_highlight_through_the_results() {
    let mut compass = on_sunset_sky(fields());
    search_for(&mut compass, "orange");
    compass.on_hotkey(SEARCH);

    let state = compass.act(Action::SearchNext).clone();
    assert_eq!(highlighted(&state), Some("Far Field"));
    assert_eq!(highlighted_route(&state), ["Sunset Sky", "Near Field", "Far Field"]);

    let state = compass.act(Action::SearchNext).clone();
    assert_eq!(highlighted(&state), Some("Far Field"), "it stops at the last");

    let state = compass.act(Action::SearchPrevious).clone();
    assert_eq!(highlighted(&state), Some("Near Field"));
    let state = compass.act(Action::SearchPrevious).clone();
    assert_eq!(highlighted(&state), Some("Near Field"), "it stops at the first");
}

#[test]
fn outside_search_mode_the_arrows_do_nothing() {
    let mut compass = on_sunset_sky(fields());
    search_for(&mut compass, "orange");
    compass.on_hotkey(SEARCH);
    compass.act(Action::SearchEscape);

    let state = compass.act(Action::SearchNext).clone();

    assert_eq!(highlighted(&state), Some("Near Field"));
}

#[test]
fn while_mobs_are_suggested_the_arrows_move_through_them_and_not_the_results() {
    let mut compass = on_sunset_sky(fields());
    search_for(&mut compass, "orange");
    compass.on_hotkey(SEARCH);
    compass.search_query("a");
    assert_eq!(compass.panel_state().search.suggestions.len(), 2);

    let state = compass.act(Action::SearchNext).clone();

    assert_eq!(state.search.highlighted, Some(1));
    assert_eq!(highlighted(&state), Some("Near Field"));
}

#[test]
fn the_highlight_stays_on_its_map_when_a_map_change_reorders_the_results() {
    let mut compass = on_sunset_sky(fields());
    search_for(&mut compass, "orange");
    compass.on_hotkey(SEARCH);
    compass.act(Action::SearchNext);
    compass.act(Action::SearchEscape);

    let state = arrive(&mut compass, KERNING, 1_000);

    assert_eq!(state.search.results[0].map_name, "Far Field", "the Far Field is now nearer");
    assert_eq!(highlighted(&state), Some("Far Field"));
    assert_eq!(highlighted_route(&state), ["Kerning City", "Far Field"]);
}

#[test]
fn picking_another_mob_highlights_its_first_result() {
    let mut compass = on_sunset_sky(fields());
    search_for(&mut compass, "orange");
    compass.on_hotkey(SEARCH);
    compass.act(Action::SearchNext);
    compass.act(Action::SearchEscape);
    search_for(&mut compass, "orange");
    let state = compass.panel_state().clone();
    assert_eq!(highlighted(&state), Some("Near Field"), "picked afresh");

    let state = search_for(&mut compass, "snail");

    assert_eq!(highlighted(&state), Some("Far Field"));
}

#[test]
fn with_no_results_or_no_search_nothing_is_highlighted() {
    let mut compass = on_sunset_sky(vec![
        map("100000000", "Sunset Sky", 100, vec![]),
        event_field(),
    ]);
    assert_eq!(compass.panel_state().search.highlighted_result, None);

    let state = search_for(&mut compass, "orange");

    assert!(state.search.results.is_empty());
    assert_eq!(state.search.highlighted_result, None);
}

/// An Event map with Orange Mushrooms: never a result.
fn event_field() -> WorldMap {
    let mut field = map("100000001", "Event Field", 100, vec![orange(10)]);
    field.entry.region = "Event".into();
    field
}
