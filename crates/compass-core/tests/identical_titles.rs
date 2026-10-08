//! Some maps share one Minimap Title, so the title alone can't say which of them the
//! player is on. When every such map would show the same Map Summary and mobs, the
//! panel just shows it. Otherwise it prefers the one connected to the map the player
//! came from, and failing that lets the player choose with a hotkey.
//!
//! There are no samples of these titles, so the Kerning City sample stands in for
//! them: in each test's index its title belongs to the maps under test.

mod common;

use common::{
    FakeSource, arrive, black_frame, index_with_title, requests_for, sample_frame, saved_index,
    saved_page,
};
use compass_core::{
    Action, Compass, LevelRange, MapDataStatus, MapEntry, Outline, PanelState, SourceError, meowdb,
};
use std::task::Poll;

/// The titles MeowDB gives to more than one map (index saved 2026-10-07): Maple Road /
/// Mushroom Town and Hidden Street / Coconut Harvest, and no others (checked by hand
/// against the saved index; a fact about MeowDB's data, not something the core does).
const MUSHROOM_TOWNS: [&str; 2] = ["000000010", "000000020"];
const COCONUT_HARVESTS: [&str; 3] = ["090040200", "090040201", "090040202"];

const KERNING: &str = "victoria-road__kerning-city--window";
const SUNSET: &str = "victoria-road__sunset-sky--window";

fn shown_id(state: &PanelState) -> Option<&str> {
    state.map.as_ref().map(|m| m.id.as_str())
}

/// The Kerning City sample shows the title these maps share.
fn titled_like_kerning_city(ids: &[&str]) -> FakeSource {
    FakeSource::with_index(index_with_title("Victoria Road", "Kerning City", ids))
}

/// MeowDB's saved index, with whatever maps it titles `street` / `map` shown under the
/// Kerning City sample's title instead (and the real Kerning City left out).
fn saved_index_reading_as_kerning_city(street: &str, map: &str) -> FakeSource {
    let index = saved_index()
        .into_iter()
        .filter(|e| e.map_name != "Kerning City")
        .map(|e| {
            if e.street_name == street && e.map_name == map {
                MapEntry { street_name: "Victoria Road".into(), map_name: "Kerning City".into(), ..e }
            } else {
                e
            }
        })
        .collect();
    // Unreachable pages can't show the same, so every map with the title is offered.
    FakeSource::with_index(index).with_pages_offline()
}

#[test]
fn meowdbs_index_gives_the_mushroom_town_title_to_two_maps_and_coconut_harvest_to_three() {
    for (street, map, ids) in [
        ("Maple Road", "Mushroom Town", MUSHROOM_TOWNS.to_vec()),
        ("Hidden Street", "Coconut Harvest", COCONUT_HARVESTS.to_vec()),
    ] {
        let mut compass = Compass::new(saved_index_reading_as_kerning_city(street, map));

        let state = arrive(&mut compass, KERNING, 0);

        assert_eq!(candidates(&state), Some((ids, 0, false)), "{street} / {map}");
    }
}

#[test]
fn arriving_in_mushroom_town_shows_it_without_asking_since_both_mushroom_towns_show_the_same() {
    let source = titled_like_kerning_city(&MUSHROOM_TOWNS);
    let requests = source.map_requests.clone();
    let mut compass = Compass::new(source);
    compass.on_frame(sample_frame(KERNING, 0));

    let state = compass.on_frame(sample_frame(KERNING, 333));

    assert!(MUSHROOM_TOWNS.contains(&shown_id(state).unwrap()));
    assert_eq!(state.candidates, None);
    assert!(matches!(&state.data, MapDataStatus::Ready(data) if data.mobs.is_empty()));
    for id in MUSHROOM_TOWNS {
        assert!(requests_for(&requests, id) <= 1, "{id} asked for more than once");
    }
}

#[test]
fn maps_alike_but_for_their_hidden_portals_are_offered_as_candidates() {
    // Neither has mobs, but Orbis Tower <Hidden Room> has a Hidden Portal and Mushroom
    // Town has none.
    const HIDDEN_ROOM: &str = "020000099";
    let mut compass = Compass::new(titled_like_kerning_city(&[MUSHROOM_TOWNS[0], HIDDEN_ROOM]));
    compass.on_frame(sample_frame(KERNING, 0));

    let state = compass.on_frame(sample_frame(KERNING, 333));

    assert_eq!(candidates(state), Some((vec![MUSHROOM_TOWNS[0], HIDDEN_ROOM], 0, false)));
    // What sets them apart: no mobs either way, but one Hidden Portal against none.
    let outlines: Vec<_> = state.candidates.as_ref().unwrap().maps.iter().map(|m| m.outline.clone()).collect();
    let none_mobs_and = |hidden_portals| Some(Outline { mob_levels: None, first_mobs: vec![], hidden_portals });
    assert_eq!(outlines, [none_mobs_and(0), none_mobs_and(1)]);
}

/// The candidates' IDs, which of them the panel shows, and whether the player chose it.
fn candidates(state: &PanelState) -> Option<(Vec<&str>, usize, bool)> {
    let c = state.candidates.as_ref()?;
    Some((c.maps.iter().map(|m| m.id.as_str()).collect(), c.shown, c.chosen))
}

#[test]
fn maps_sharing_a_title_are_fetched_one_at_a_time_and_the_panel_shows_loading_meanwhile() {
    // Each page arrives on the second time the core checks on it.
    let source = titled_like_kerning_city(&MUSHROOM_TOWNS).with_slow_pages(2);
    let requests = source.map_requests.clone();
    let mut compass = Compass::new(source);
    compass.on_frame(sample_frame(KERNING, 0));

    let state = compass.on_frame(sample_frame(KERNING, 333));
    assert_eq!(state.data, MapDataStatus::Loading);
    assert_eq!(state.candidates, None);
    assert_eq!(requests_for(&requests, MUSHROOM_TOWNS[1]), 0);

    let state = compass.on_frame(black_frame(666));
    assert_eq!(state.data, MapDataStatus::Loading);

    let state = compass.on_frame(black_frame(1000));
    assert!(matches!(&state.data, MapDataStatus::Ready(data) if data.mobs.is_empty()));
    assert_eq!(state.candidates, None);
}

const SUNSET_SKY: &str = "010003070";
const CONSTRUCTION_SITE: &str = "010003080";
const DEAD_MINE: &str = "020001067";

/// The Coconut Harvests, if they differed: each answers with another map's saved page
/// (Sunset Sky's 5 mobs, Construction Site's 6, Dead Mine IV's 2).
fn coconut_harvests_that_differ() -> FakeSource {
    titled_like_kerning_city(&COCONUT_HARVESTS).with_pages(|id| {
        let stand_in = match id {
            "090040200" => SUNSET_SKY,
            "090040201" => CONSTRUCTION_SITE,
            "090040202" => DEAD_MINE,
            other => other,
        };
        Poll::Ready(match saved_page(stand_in) {
            Some(html) => meowdb::read_map_page(&html, stand_in),
            None => Err(SourceError::Unreachable(format!("no saved page for {id}"))),
        })
    })
}

fn mob_count(state: &PanelState) -> Option<usize> {
    match &state.data {
        MapDataStatus::Ready(data) => Some(data.mobs.len()),
        _ => None,
    }
}

fn coconut_candidates(shown: usize, chosen: bool) -> Option<(Vec<&'static str>, usize, bool)> {
    Some((COCONUT_HARVESTS.to_vec(), shown, chosen))
}

/// Each candidate's mob levels and first (lowest-level) mobs.
fn mobs_at_a_glance(state: &PanelState) -> Vec<Option<(LevelRange, Vec<String>)>> {
    let maps = &state.candidates.as_ref().expect("candidates").maps;
    maps.iter()
        .map(|m| m.outline.as_ref().and_then(|o| Some((o.mob_levels?, o.first_mobs.clone()))))
        .collect()
}

#[test]
fn each_candidate_is_listed_with_its_mob_levels_and_first_mobs_to_tell_them_apart() {
    let mut compass = Compass::new(coconut_harvests_that_differ());
    compass.on_frame(sample_frame(KERNING, 0));

    let state = compass.on_frame(sample_frame(KERNING, 333));

    let names = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
    assert_eq!(
        mobs_at_a_glance(state),
        [
            Some((LevelRange { lowest: 2, highest: 14 }, names(&["Blue Snail", "Stump", "Orange Mushroom"]))),
            Some((LevelRange { lowest: 4, highest: 19 }, names(&["Red Snail", "Orange Mushroom", "Dark Stump"]))),
            Some((LevelRange { lowest: 41, highest: 58 }, names(&["Flyeye", "Minor Zombie"]))),
        ]
    );
}

#[test]
fn arriving_with_nothing_to_go_on_in_one_of_maps_that_differ_shows_the_first_and_offers_the_others() {
    let source = coconut_harvests_that_differ();
    let requests = source.map_requests.clone();
    let mut compass = Compass::new(source);
    compass.on_frame(sample_frame(KERNING, 0));

    let state = compass.on_frame(sample_frame(KERNING, 333));

    assert_eq!(shown_id(state), Some("090040200"));
    assert_eq!(candidates(state), coconut_candidates(0, false));
    assert_eq!(mob_count(state), Some(5));
    for id in COCONUT_HARVESTS {
        assert!(requests_for(&requests, id) <= 1, "{id} asked for more than once");
    }
}

#[test]
fn coming_from_a_map_connected_to_just_one_of_them_shows_that_one_and_fetches_only_its_page() {
    // Kerning City shares its title with Northern Top of Construction Site here. Sunset
    // Sky's MeowDB page connects it to Kerning City (and Construction Site), not to
    // Northern Top.
    const KERNING_CITY: &str = "010003000";
    const NORTHERN_TOP: &str = "010003081";
    let source = titled_like_kerning_city(&[KERNING_CITY, NORTHERN_TOP]);
    let requests = source.map_requests.clone();
    let mut compass = Compass::new(source);
    compass.on_frame(sample_frame(SUNSET, 0));
    compass.on_frame(sample_frame(SUNSET, 333));

    compass.on_frame(sample_frame(KERNING, 666));
    let state = compass.on_frame(sample_frame(KERNING, 1000));

    assert_eq!(shown_id(state), Some(KERNING_CITY));
    assert_eq!(state.candidates, None);
    assert!(matches!(state.data, MapDataStatus::Ready(_)));
    assert_eq!(requests_for(&requests, NORTHERN_TOP), 0);
}

/// MeowDB unreachable, with saved copies of every page: the map the player came from
/// still settles which map it is.
#[test]
fn coming_from_a_connected_map_settles_it_serving_saved_copies_too() {
    const KERNING_CITY: &str = "010003000";
    const NORTHERN_TOP: &str = "010003081";
    let mut compass = Compass::new(
        titled_like_kerning_city(&[KERNING_CITY, NORTHERN_TOP]).serving_saved_copies(1_790_812_800_000),
    );
    compass.on_frame(sample_frame(SUNSET, 0));
    compass.on_frame(sample_frame(SUNSET, 333));

    compass.on_frame(sample_frame(KERNING, 666));
    let state = compass.on_frame(sample_frame(KERNING, 1000));

    assert_eq!(shown_id(state), Some(KERNING_CITY));
    assert_eq!(state.candidates, None);
    assert!(matches!(state.data, MapDataStatus::SavedCopy { .. }));
}

#[test]
fn the_choose_candidate_hotkey_shows_each_candidate_in_turn_with_its_own_data() {
    let mut compass = Compass::new(coconut_harvests_that_differ());
    compass.on_frame(sample_frame(KERNING, 0));
    compass.on_frame(sample_frame(KERNING, 333));

    let state = compass.act(Action::ChooseCandidate);
    assert_eq!(shown_id(state), Some("090040201"));
    assert_eq!(candidates(state), coconut_candidates(1, true));
    assert_eq!(mob_count(state), Some(6));

    let state = compass.act(Action::ChooseCandidate);
    assert_eq!(shown_id(state), Some("090040202"));
    assert_eq!(mob_count(state), Some(2));

    let state = compass.act(Action::ChooseCandidate);
    assert_eq!(shown_id(state), Some("090040200"));
    assert_eq!(candidates(state), coconut_candidates(0, true));
}

#[test]
fn a_chosen_candidate_holds_through_unreadable_frames_until_the_map_changes() {
    let mut compass = Compass::new(coconut_harvests_that_differ());
    compass.on_frame(sample_frame(KERNING, 0));
    compass.on_frame(sample_frame(KERNING, 333));
    compass.act(Action::ChooseCandidate);

    compass.on_frame(black_frame(666));
    let state = compass.on_frame(sample_frame(KERNING, 1000));
    assert_eq!(shown_id(state), Some("090040201"));
    assert_eq!(candidates(state), coconut_candidates(1, true));

    // Off to Sunset Sky and back: the choice is gone, and the first is shown again.
    compass.on_frame(sample_frame(SUNSET, 1333));
    let state = compass.on_frame(sample_frame(SUNSET, 1666));
    assert_eq!(shown_id(state), Some(SUNSET_SKY));
    assert_eq!(state.candidates, None);
    compass.on_frame(sample_frame(KERNING, 2000));
    let state = compass.on_frame(sample_frame(KERNING, 2333));
    assert_eq!(shown_id(state), Some("090040200"));
    assert_eq!(candidates(state), coconut_candidates(0, false));
}

/// Settings from before this hotkey existed (or from before Ctrl+Shift) get Ctrl+Shift+N.
#[test]
fn ctrl_shift_n_shows_the_next_candidate_even_with_settings_from_before_it_existed() {
    let older = r#"{"version":1,"expanded":true,"hotkeys":{"togglePanel":"ctrl+shift+m","openMeowdbPage":"ctrl+shift+o"}}"#;
    let mut compass = Compass::new(coconut_harvests_that_differ());
    compass.restore(serde_json::from_str(older).unwrap());
    arrive(&mut compass, KERNING, 0);

    compass.on_hotkey("ctrl+shift+n");

    assert_eq!(candidates(compass.panel_state()), coconut_candidates(1, true));
}

#[test]
fn the_choose_candidate_hotkey_does_nothing_when_there_is_nothing_to_choose() {
    let mut compass = Compass::new(titled_like_kerning_city(&MUSHROOM_TOWNS));
    compass.on_frame(sample_frame(KERNING, 0));
    let before = compass.on_frame(sample_frame(KERNING, 333)).clone();

    let state = compass.act(Action::ChooseCandidate);

    assert_eq!(*state, before);
}
