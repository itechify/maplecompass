//! Mob search: the player presses the search hotkey, types part of a mob's name, picks
//! the mob, and sees every map it spawns on, ranked by Density Tier and then Density.
//! The search window is ours and takes the keyboard while it's open (ADR 0001); the top
//! results stay on the panel as the player travels.

mod common;

use common::world::{SEARCH, WorldMap, crawled, map, mob, orange, search_for, tick_through, world};
use common::{FakeSource, arrive};
use compass_core::{Action, Compass, DensityTier, MEOWDB_CREDIT, Mob, PanelState, SearchState};

fn mushroom_world() -> Vec<WorldMap> {
    vec![
        map("100000001", "Mushroom Field", 100, vec![mob("9", "Orange Mushroom", 8, 10), mob("4", "Shroom", 2, 5)]),
        map("100000002", "Green Hill", 100, vec![mob("13", "Green Mushroom", 15, 8)]),
        map("100000003", "Ant Tunnel", 100, vec![mob("2110200", "Horny Mushroom", 22, 12), mob("3", "Blue Snail", 4, 6)]),
        map("100000004", "Mushroom Hill", 100, vec![mob("9", "Orange Mushroom", 8, 4)]),
    ]
}

#[test]
fn the_search_hotkey_leaves_an_open_search_window_as_it_is() {
    let mut compass = crawled(mushroom_world());
    compass.on_hotkey(SEARCH);
    compass.search_query("mush");
    let before = compass.panel_state().clone();

    compass.on_hotkey(SEARCH);

    assert_eq!(*compass.panel_state(), before);
}

/// The suggestions as "Name (level)", in order.
fn suggestions(state: &PanelState) -> Vec<String> {
    state.search.suggestions.iter().map(|m| format!("{} ({})", m.name, m.level)).collect()
}

/// The name of the highlighted suggestion.
fn highlighted(state: &PanelState) -> Option<&str> {
    state.search.highlighted.map(|i| state.search.suggestions[i].name.as_str())
}

#[test]
fn typing_suggests_every_known_mob_whose_name_contains_it_whatever_the_case_with_its_level() {
    let mut compass = crawled(mushroom_world());
    compass.on_hotkey(SEARCH);

    let state = compass.search_query("MUSH");

    assert_eq!(state.search.query, "MUSH");
    assert_eq!(suggestions(state), ["Green Mushroom (15)", "Horny Mushroom (22)", "Orange Mushroom (8)"]);
    assert_eq!(highlighted(state), Some("Green Mushroom"), "the first is highlighted");

    let state = compass.search_query("room");
    assert_eq!(suggestions(state).len(), 4, "anywhere in the name: Shroom too");
    let state = compass.search_query("dragon");
    assert!(suggestions(state).is_empty());
    assert_eq!(highlighted(state), None);
}

/// The picked mob's name, if any.
fn picked(state: &PanelState) -> Option<&str> {
    state.search.picked.as_ref().map(|m| m.name.as_str())
}

#[test]
fn the_highlight_moves_through_the_suggestions_and_enter_picks_it_and_closes_the_window() {
    let mut compass = crawled(mushroom_world());
    compass.on_hotkey(SEARCH);
    compass.search_query("mush");

    assert_eq!(highlighted(compass.act(Action::SearchNext)), Some("Horny Mushroom"));
    assert_eq!(highlighted(compass.act(Action::SearchNext)), Some("Orange Mushroom"));
    assert_eq!(highlighted(compass.act(Action::SearchNext)), Some("Orange Mushroom"), "stays on the last");
    assert_eq!(highlighted(compass.act(Action::SearchPrevious)), Some("Horny Mushroom"));
    let state = compass.act(Action::SearchEnter);

    assert_eq!(picked(state), Some("Horny Mushroom"));
    assert_eq!(state.search.picked.as_ref().map(|m| m.level), Some(22));
    assert!(!state.search.window_open, "focus goes back to the game");
}

#[test]
fn the_highlight_stays_at_the_top_going_up_and_a_new_query_starts_it_at_the_top_again() {
    let mut compass = crawled(mushroom_world());
    compass.on_hotkey(SEARCH);
    compass.search_query("mush");

    assert_eq!(highlighted(compass.act(Action::SearchPrevious)), Some("Green Mushroom"));
    compass.act(Action::SearchNext);
    assert_eq!(highlighted(compass.search_query("mushr")), Some("Green Mushroom"));
}

#[test]
fn esc_closes_the_search_window_and_keeps_the_current_search() {
    let mut compass = crawled(mushroom_world());
    search_for(&mut compass, "orange");

    assert_eq!(compass.on_hotkey(SEARCH), None);
    let state = compass.panel_state();
    assert!(state.search.window_open);
    assert_eq!(state.search.query, "", "it opens with nothing typed");
    compass.search_query("horny");
    let state = compass.act(Action::SearchEscape);

    assert!(!state.search.window_open);
    assert_eq!(picked(state), Some("Orange Mushroom"));
}

#[test]
fn enter_with_nothing_typed_clears_the_search_and_closes_the_window() {
    let mut compass = crawled(mushroom_world());
    search_for(&mut compass, "orange");

    let state = search_for(&mut compass, "  ");

    assert!(!state.search.window_open);
    assert_eq!(picked(&state), None);
}

#[test]
fn enter_on_a_query_that_matches_no_mob_does_nothing() {
    let mut compass = crawled(mushroom_world());
    search_for(&mut compass, "orange");

    let state = search_for(&mut compass, "dragon");

    assert!(state.search.window_open, "the player can fix the typing");
    assert_eq!(picked(&state), Some("Orange Mushroom"));
}

#[test]
fn search_keys_do_nothing_while_the_search_window_is_closed() {
    let mut compass = crawled(mushroom_world());
    search_for(&mut compass, "orange");
    let before = compass.panel_state().clone();

    compass.act(Action::SearchEnter);
    compass.act(Action::SearchNext);
    let state = compass.act(Action::SearchEscape);

    assert_eq!(*state, before);
}

/// A mob with this Spawn Share (whole percent).
fn sharing(share: u32, mut mob: Mob) -> Mob {
    mob.spawn_share_percent = share;
    mob
}

/// Each result as "Map Name: Density (hundredths), Tier".
fn ranked(state: &PanelState) -> Vec<String> {
    state.search.results.iter().map(|r| format!("{}: {}, {:?}", r.map_name, r.density_hundredths, r.tier)).collect()
}

#[test]
fn every_map_the_picked_mob_spawns_on_is_a_result_with_its_density_spawn_share_and_tier() {
    let mut compass = crawled(vec![
        map("100000001", "Mushroom Field", 150, vec![sharing(40, orange(10)), mob("4", "Shroom", 2, 15)]),
        map("100000002", "Green Hill", 100, vec![mob("13", "Green Mushroom", 15, 8)]),
        map("100000003", "Mushroom Hill", 100, vec![sharing(100, orange(4))]),
    ]);

    let state = search_for(&mut compass, "orange");

    let results = &state.search.results;
    assert_eq!(results.len(), 2);
    let field = &results[0];
    assert_eq!(field.map_id, "100000001");
    assert_eq!(field.street_name, "Victoria Road");
    assert_eq!(field.map_name, "Mushroom Field");
    assert_eq!(field.density_hundredths, 1_500, "10 spawns at a 1.5x Mob Rate");
    assert_eq!(field.spawn_share_percent, 40);
    assert_eq!(field.tier, DensityTier::High);
    assert_eq!(ranked(&state)[1], "Mushroom Hill: 400, Low");
    assert_eq!(state.search.credit, Some(MEOWDB_CREDIT));
}

#[test]
fn a_map_is_high_from_two_thirds_of_the_densest_maps_density_and_medium_from_one_third() {
    // Densities 3.00, 2.00 (exactly 2/3), 1.99, 1.00 (exactly 1/3) and 0.99.
    let mut compass = crawled(vec![
        map("100000001", "Two Thirds", 200, vec![orange(1)]),
        map("100000002", "Just Under One Third", 99, vec![orange(1)]),
        map("100000003", "Densest", 300, vec![orange(1)]),
        map("100000004", "One Third", 100, vec![orange(1)]),
        map("100000005", "Just Under Two Thirds", 199, vec![orange(1)]),
    ]);

    let state = search_for(&mut compass, "orange");

    assert_eq!(
        ranked(&state),
        [
            "Densest: 300, High",
            "Two Thirds: 200, High",
            "Just Under Two Thirds: 199, Medium",
            "One Third: 100, Medium",
            "Just Under One Third: 99, Low",
        ]
    );
}

#[test]
fn results_are_ordered_by_tier_then_density_with_spawn_count_and_mob_rate_both_counting() {
    let mut compass = crawled(vec![
        map("100000001", "Slow Refill", 50, vec![orange(20)]),
        map("100000002", "Few But Fast", 200, vec![orange(4)]),
        map("100000003", "Many", 100, vec![orange(12)]),
        map("100000004", "Sparse", 100, vec![orange(2)]),
        map("100000005", "Fast Refill", 160, vec![orange(10)]),
    ]);

    let state = search_for(&mut compass, "orange");

    assert_eq!(
        ranked(&state),
        ["Fast Refill: 1600, High", "Many: 1200, High", "Slow Refill: 1000, Medium", "Few But Fast: 800, Medium", "Sparse: 200, Low"]
    );
}

#[test]
fn event_maps_are_left_out_and_dont_set_the_densest_map() {
    let mut compass = crawled(vec![
        map("100000001", "Mushroom Field", 100, vec![orange(10)]),
        map("100000002", "Mushroom Festival", 200, vec![orange(30)]).in_region("Event"),
        map("100000003", "Hidden Mushrooms", 100, vec![orange(4)]).in_region("Hidden Street"),
    ]);

    let state = search_for(&mut compass, "orange");

    assert_eq!(ranked(&state), ["Mushroom Field: 1000, High", "Hidden Mushrooms: 400, Medium"]);
}

const KERNING: &str = "victoria-road__kerning-city--window";
const SUNSET: &str = "victoria-road__sunset-sky--window";
const HENESYS_HUNTING_GROUND_I: &str = "010001010";

#[test]
fn a_mob_found_only_on_event_maps_has_no_results_and_search_says_why() {
    let mut compass = crawled(vec![
        map("100000001", "Snail Field", 100, vec![mob("3", "Blue Snail", 4, 5)]),
        map("100000002", "Mushroom Festival", 200, vec![orange(30)]).in_region("Event"),
    ]);

    let state = search_for(&mut compass, "orange");

    assert!(state.search.results.is_empty());
    assert!(state.search.only_on_event_maps);
    let state = search_for(&mut compass, "snail");
    assert!(!state.search.only_on_event_maps, "the Blue Snail has results");
}

/// The panel's results, as "Map Name".
fn top(state: &PanelState) -> Vec<&str> {
    state.search.top_results.iter().map(|r| r.map_name.as_str()).collect()
}

#[test]
fn the_panel_gets_the_top_five_results_and_how_many_more_there_are() {
    let maps = (1..=7).map(|n| map(&format!("10000000{n}"), &format!("Field {n}"), 100, vec![orange(10 - n)])).collect();
    let mut compass = crawled(maps);

    let state = search_for(&mut compass, "orange");

    assert_eq!(state.search.results.len(), 7);
    assert_eq!(top(&state), ["Field 1", "Field 2", "Field 3", "Field 4", "Field 5"]);
    assert_eq!(state.search.more_results, 2);
}

#[test]
fn with_five_results_or_fewer_the_panel_gets_them_all() {
    let mut compass = crawled(mushroom_world());

    let state = search_for(&mut compass, "orange");

    assert_eq!(top(&state), ["Mushroom Field", "Mushroom Hill"]);
    assert_eq!(state.search.more_results, 0);
}

#[test]
fn the_picked_mob_and_its_results_stay_as_the_player_changes_map_until_the_search_is_cleared() {
    // MeowDB's saved pages: Orange Mushrooms live in Henesys Hunting Ground I.
    let mut compass = Compass::new(FakeSource::meowdb());
    tick_through(&mut compass, 0, 5_000);
    let searched = search_for(&mut compass, "orange mushroom").search;
    assert!(searched.results.iter().any(|r| r.map_id == HENESYS_HUNTING_GROUND_I), "{:?}", searched.results);

    // The same maps, at a Distance from wherever the player is (tests/distance.rs).
    let maps = |search: &SearchState| {
        let mut maps: Vec<_> = search.results.iter().map(|r| (r.map_id.clone(), r.density_hundredths, r.tier)).collect();
        maps.sort();
        maps
    };

    let state = arrive(&mut compass, KERNING, 10_000);
    assert_eq!(state.map.as_ref().map(|m| m.map_name.as_str()), Some("Kerning City"));
    assert_eq!(state.search.picked, searched.picked);
    assert_eq!(maps(&state.search), maps(&searched));
    let state = arrive(&mut compass, SUNSET, 20_000);
    assert_eq!(state.map.as_ref().map(|m| m.map_name.as_str()), Some("Sunset Sky"));
    assert_eq!(maps(&state.search), maps(&searched));

    let state = search_for(&mut compass, "");
    assert_eq!(picked(&state), None);
    assert!(state.search.results.is_empty());
}

#[test]
fn mid_crawl_the_results_cover_the_maps_known_so_far_and_grow_as_the_crawl_goes_on() {
    let maps = vec![
        map("100000001", "Mushroom Field", 100, vec![orange(10)]),
        map("100000002", "Green Hill", 100, vec![mob("13", "Green Mushroom", 15, 8)]),
        map("100000003", "Mushroom Hill", 100, vec![orange(4)]),
        map("100000004", "Mushroom Grove", 100, vec![orange(20)]),
    ];
    let mut compass = Compass::new(world(maps).crawling_uncached());
    // One page every 2 s: by 2.1 s, the first two are known.
    tick_through(&mut compass, 0, 2_100);

    let state = search_for(&mut compass, "orange");
    assert_eq!(ranked(&state), ["Mushroom Field: 1000, High"]);
    let progress = state.search.crawl;
    assert_eq!((progress.known, progress.total, progress.done), (2, 4, false));

    let state = tick_through(&mut compass, 2_100, 4_100);
    assert_eq!(ranked(&state), ["Mushroom Field: 1000, High", "Mushroom Hill: 400, Medium"]);
    let state = tick_through(&mut compass, 4_100, 6_100);
    assert_eq!(ranked(&state), ["Mushroom Grove: 2000, High", "Mushroom Field: 1000, Medium", "Mushroom Hill: 400, Low"]);
    assert!(state.search.crawl.done);
}

#[test]
fn mid_crawl_the_suggestions_cover_the_mobs_known_so_far() {
    let maps = vec![
        map("100000001", "Mushroom Field", 100, vec![orange(10)]),
        map("100000002", "Green Hill", 100, vec![mob("13", "Green Mushroom", 15, 8)]),
    ];
    let mut compass = Compass::new(world(maps).crawling_uncached());
    tick_through(&mut compass, 0, 100);
    compass.on_hotkey(SEARCH);

    let state = compass.search_query("mushroom");
    assert_eq!(suggestions(state), ["Orange Mushroom (8)"]);

    let state = tick_through(&mut compass, 100, 2_100);
    assert_eq!(suggestions(&state), ["Green Mushroom (15)", "Orange Mushroom (8)"]);
}

#[test]
fn without_a_search_there_are_no_results_and_no_credit() {
    let mut compass = crawled(mushroom_world());
    search_for(&mut compass, "orange");

    let state = search_for(&mut compass, "");

    assert!(state.search.results.is_empty());
    assert_eq!(state.search.credit, None);
}

#[test]
fn the_search_hotkey_opens_the_search_window_with_nothing_typed() {
    let mut compass = crawled(mushroom_world());
    assert!(!compass.panel_state().search.window_open);

    compass.on_hotkey(SEARCH);

    let search = &compass.panel_state().search;
    assert!(search.window_open);
    assert_eq!(search.query, "");
    assert!(search.suggestions.is_empty());
}
