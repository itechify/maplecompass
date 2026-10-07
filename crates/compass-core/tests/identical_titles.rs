//! Some maps share one Minimap Title, so the title alone can't say which of them the
//! player is on. When every such map would show the same Map Summary and mobs, the
//! panel just shows it. Otherwise it prefers the one connected to the map the player
//! came from, and failing that lets the player choose with a hotkey.
//!
//! There are no samples of these titles, so the Kerning City sample stands in for
//! them: in each test's index its title belongs to the maps under test.

mod common;

use common::{
    FakeSource, black_frame, index_with_title, requests_for, sample_frame, saved_index, saved_page,
};
use compass_core::{Action, Candidates, Compass, MapDataStatus, PanelState, SourceError, meowdb};
use std::task::Poll;

/// The titles MeowDB gives to more than one map (index saved 2026-10-07).
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

#[test]
fn the_only_titles_meowdb_gives_to_more_than_one_map_are_mushroom_town_and_coconut_harvest() {
    let index = saved_index();
    let mut shared: Vec<(String, Vec<&str>)> = Vec::new();
    for entry in &index {
        let title = format!("{} / {}", entry.street_name, entry.map_name);
        let ids: Vec<&str> =
            index.iter().filter(|e| e.street_name == entry.street_name && e.map_name == entry.map_name).map(|e| e.id.as_str()).collect();
        if ids.len() > 1 && !shared.iter().any(|(t, _)| *t == title) {
            shared.push((title, ids));
        }
    }

    assert_eq!(
        shared,
        [
            ("Maple Road / Mushroom Town".to_string(), MUSHROOM_TOWNS.to_vec()),
            ("Hidden Street / Coconut Harvest".to_string(), COCONUT_HARVESTS.to_vec()),
        ]
    );
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

fn coconut_candidates(shown: usize, chosen: bool) -> Option<Candidates> {
    Some(Candidates { ids: COCONUT_HARVESTS.map(String::from).to_vec(), shown, chosen })
}

#[test]
fn arriving_with_nothing_to_go_on_in_one_of_maps_that_differ_shows_the_first_and_offers_the_others() {
    let source = coconut_harvests_that_differ();
    let requests = source.map_requests.clone();
    let mut compass = Compass::new(source);
    compass.on_frame(sample_frame(KERNING, 0));

    let state = compass.on_frame(sample_frame(KERNING, 333));

    assert_eq!(shown_id(state), Some("090040200"));
    assert_eq!(state.candidates, coconut_candidates(0, false));
    assert_eq!(mob_count(state), Some(5));
    for id in COCONUT_HARVESTS {
        assert!(requests_for(&requests, id) <= 1, "{id} asked for more than once");
    }
}

#[test]
fn the_choose_candidate_hotkey_shows_each_candidate_in_turn_with_its_own_data() {
    let mut compass = Compass::new(coconut_harvests_that_differ());
    compass.on_frame(sample_frame(KERNING, 0));
    compass.on_frame(sample_frame(KERNING, 333));

    let state = compass.act(Action::ChooseCandidate);
    assert_eq!(shown_id(state), Some("090040201"));
    assert_eq!(state.candidates, coconut_candidates(1, true));
    assert_eq!(mob_count(state), Some(6));

    let state = compass.act(Action::ChooseCandidate);
    assert_eq!(shown_id(state), Some("090040202"));
    assert_eq!(mob_count(state), Some(2));

    let state = compass.act(Action::ChooseCandidate);
    assert_eq!(shown_id(state), Some("090040200"));
    assert_eq!(state.candidates, coconut_candidates(0, true));
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
    assert_eq!(state.candidates, coconut_candidates(1, true));

    // Off to Sunset Sky and back: the choice is gone, and the first is shown again.
    compass.on_frame(sample_frame(SUNSET, 1333));
    let state = compass.on_frame(sample_frame(SUNSET, 1666));
    assert_eq!(shown_id(state), Some(SUNSET_SKY));
    assert_eq!(state.candidates, None);
    compass.on_frame(sample_frame(KERNING, 2000));
    let state = compass.on_frame(sample_frame(KERNING, 2333));
    assert_eq!(shown_id(state), Some("090040200"));
    assert_eq!(state.candidates, coconut_candidates(0, false));
}

#[test]
fn the_choose_candidate_hotkey_does_nothing_when_there_is_nothing_to_choose() {
    let mut compass = Compass::new(titled_like_kerning_city(&MUSHROOM_TOWNS));
    compass.on_frame(sample_frame(KERNING, 0));
    let before = compass.on_frame(sample_frame(KERNING, 333)).clone();

    let state = compass.act(Action::ChooseCandidate);

    assert_eq!(*state, before);
}
