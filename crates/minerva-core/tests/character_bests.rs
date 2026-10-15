//! Personal Bests per Character (GLOSSARY): each best is stamped with the Character, level
//! and job the Status Bar showed at the refresh that set it, and kept with that Character's.
//! The Grind Tracker races the current Character's best on the map, or the Unassigned ones
//! while no Character is read. Bests saved before Characters existed are Unassigned.

mod common;

use common::player::{Player, WALL_AT_START};
use common::{CONSTRUCTION_SITE_ID, SUNSET_SKY_ID, tracking};
use minerva_core::{Minerva, PanelControl, PanelState, SavedBests, VisitBest};

const S: u64 = 1000;
const MIN: u64 = 60 * S;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";

/// The bests `minerva` hands out, as the shell saves them: JSON.
fn handed_out(minerva: &Minerva) -> serde_json::Value {
    serde_json::to_value(minerva.bests()).unwrap()
}

/// The Best row's EXP/h: the map's Personal Best from earlier Visits.
fn best_row(state: &PanelState) -> Option<u64> {
    state.exp_tracking.personal_best
}

#[test]
fn bests_saved_before_characters_read_in_as_unassigned_with_nothing_lost() {
    // A file as version 1 saved it, with no Characters.
    let version_1 = format!(
        r#"{{"version": 1, "maps": [
            {{"id": "{SUNSET_SKY_ID}", "visits": [
                {{"expPerHour": 7200, "setAtUnixMs": {WALL_AT_START}}},
                {{"expPerHour": 3600, "setAtUnixMs": null}}
            ]}},
            {{"id": "{CONSTRUCTION_SITE_ID}", "visits": [{{"expPerHour": 5000, "setAtUnixMs": null}}]}}
        ]}}"#
    );
    let saved: SavedBests = serde_json::from_str(&version_1).unwrap();
    let mut minerva = tracking();
    minerva.restore_bests(saved);

    // Every Visit is Unassigned, and the shell saves them in the new shape.
    let expected = serde_json::json!({
        "version": 2,
        "unassigned": [
            { "id": SUNSET_SKY_ID, "visits": [
                { "expPerHour": 7200, "setAtUnixMs": WALL_AT_START },
                { "expPerHour": 3600, "setAtUnixMs": null }
            ]},
            { "id": CONSTRUCTION_SITE_ID, "visits": [{ "expPerHour": 5000, "setAtUnixMs": null }] }
        ],
        "characters": []
    });
    assert_eq!(handed_out(&minerva), expected);

    // With no Character read, the Grind Tracker races them.
    let mut player = Player::new(minerva);
    assert_eq!(best_row(&player.arrive(SUNSET)), Some(7200));
    player.hunt(MIN, 10);
    assert_eq!(best_row(&player.arrive(CONSTRUCTION_SITE)), Some(5000));
}

/// What the bests `minerva` hands out keep for `character` (`None`: Unassigned) on `map`:
/// each Visit's (EXP/h, level), highest first.
fn kept(minerva: &Minerva, character: Option<&str>, map: &str) -> Vec<(u64, Option<u32>)> {
    let bests = minerva.bests();
    let maps = bests.maps_of(character);
    maps.iter().find(|m| m.id == map).map_or_else(Vec::new, |m| m.visits.iter().map(|v| (v.exp_per_hour, v.level)).collect())
}

/// Egbert, a level 13 Magician, a Visit to Sunset Sky at 7,200 EXP/h for 6 minutes, then on
/// to the Construction Site.
fn egbert_after_a_best_of_7200_on_sunset_sky() -> Player {
    let mut player = Player::clocked(tracking());
    player.status(Some("Egbert"), 13);
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player
}

#[test]
fn a_best_is_stamped_with_the_character_level_and_job_the_status_bar_showed_when_it_was_set() {
    let mut player = Player::clocked(tracking());
    player.status(Some("Egbert"), 13);
    player.arrive(SUNSET);
    player.hunt(5 * MIN, 20);
    let set = player.wall_clock().unwrap();

    let expected = serde_json::json!({
        "version": 2,
        "unassigned": [],
        "characters": [{ "name": "Egbert", "maps": [{ "id": SUNSET_SKY_ID, "visits": [
            { "expPerHour": 7200, "setAtUnixMs": set, "level": 13, "job": "Magician" }
        ]}]}]
    });
    assert_eq!(handed_out(&player.minerva), expected);
}

#[test]
fn each_characters_bests_are_kept_apart_and_the_grind_tracker_races_the_one_playing() {
    let mut player = egbert_after_a_best_of_7200_on_sunset_sky();
    assert_eq!(best_row(&player.arrive(SUNSET)), Some(7200));

    // Bob, at the character select screen and back on Sunset Sky: he has no best there.
    player.status(Some("Bob"), 40);
    player.hunt(MIN, 10);
    player.arrive(CONSTRUCTION_SITE);
    assert_eq!(best_row(&player.arrive(SUNSET)), None);
    // 3,600 EXP/h: his own first best, which Egbert's doesn't star or race.
    let five = player.hunt(6 * MIN, 10);
    assert_eq!((best_row(&five), five.exp_tracking.new_best), (None, false));
    assert_eq!(kept(&player.minerva, Some("Bob"), SUNSET_SKY_ID), [(3600, Some(40))]);
    assert_eq!(kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID), [(7200, Some(13))]);

    // Back as Egbert: his best again.
    player.status(Some("Egbert"), 13);
    player.hunt(MIN, 10);
    player.arrive(CONSTRUCTION_SITE);
    assert_eq!(best_row(&player.arrive(SUNSET)), Some(7200));
}

#[test]
fn the_best_row_races_the_unassigned_bests_until_a_name_is_read_then_the_characters_at_once() {
    let mut minerva = tracking();
    let saved = serde_json::json!({
        "version": 2,
        "unassigned": [{ "id": SUNSET_SKY_ID, "visits": [{ "expPerHour": 9000, "setAtUnixMs": null }] }],
        "characters": [{ "name": "Egbert", "maps": [{ "id": SUNSET_SKY_ID, "visits": [
            { "expPerHour": 7200, "setAtUnixMs": null, "level": 13, "job": "Magician" }
        ]}]}]
    });
    minerva.restore_bests(serde_json::from_value(saved).unwrap());
    let mut player = Player::new(minerva);
    // The Status Bar's name can't be read.
    player.status(None, 13);
    assert_eq!(best_row(&player.arrive(SUNSET)), Some(9000));

    player.status(Some("Egbert"), 13);
    assert_eq!(best_row(&player.hunt(10 * S, 10)), Some(7200));
}

#[test]
fn a_best_set_while_no_character_is_named_is_unassigned_and_the_player_can_move_it_to_one() {
    let mut player = Player::saved_by_the_shell(tracking());
    player.status(None, 13);
    player.arrive(SUNSET);
    player.hunt(5 * MIN, 20);
    let set = player.wall_clock();
    player.arrive(CONSTRUCTION_SITE);
    assert_eq!(kept(&player.minerva, None, SUNSET_SKY_ID), [(7200, Some(13))]);

    // Egbert named: he has no best on Sunset Sky, and the window offers him and Unassigned.
    player.status(Some("Egbert"), 13);
    player.hunt(10 * S, 10);
    assert_eq!(best_row(&player.arrive(SUNSET)), None);
    let window = player.press(PanelControl::TogglePersonalBests).personal_bests;
    let choices: Vec<(Option<&str>, bool, usize)> =
        window.characters.iter().map(|c| (c.name.as_deref(), c.current, c.maps.len())).collect();
    assert_eq!(choices, [(Some("Egbert"), true, 0), (None, false, 1)]);
    let listed = &window.characters[1].maps[0].visits[0];
    assert_eq!((listed.exp_per_hour, listed.set_at_unix_ms, listed.level), (7200, set, Some(13)));

    // Moved to Egbert, it is his best at once, and Unassigned has none left to show.
    let visit = VisitBest { exp_per_hour: 7200, set_at_unix_ms: set, level: Some(13), job: None };
    let moved = player.move_visit(SUNSET_SKY_ID, visit, "Egbert");
    assert_eq!(best_row(&moved), Some(7200));
    assert_eq!(kept(&player.minerva, Some("Egbert"), SUNSET_SKY_ID), [(7200, Some(13))]);
    assert_eq!(kept(&player.minerva, None, SUNSET_SKY_ID), []);
    let names: Vec<Option<&str>> = moved.personal_bests.characters.iter().map(|c| c.name.as_deref()).collect();
    assert_eq!(names, [Some("Egbert")]);
}
