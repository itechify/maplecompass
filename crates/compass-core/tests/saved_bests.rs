//! Personal Bests outlive the launch: the core hands the shell every map's Visit bests
//! as plain data, the Visit under way's included, and takes them back at the next
//! launch, the same way as the saved world. Closing MapleCompass ends the Visit.

mod common;

use common::player::{Player, WALL_AT_START};
use common::{CONSTRUCTION_SITE_ID, SUNSET_SKY_ID, sharing_a_title, tracking, tracking_with};
use compass_core::{BESTS_VERSION, Compass, PanelControl, PanelState, PersonalBestLine, SavedBests};

const S: u64 = 1000;
const MIN: u64 = 60 * S;
const DAY: u64 = 24 * 60 * MIN;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const ROCKY_ROAD: &str = "victoria-road__rocky-road-i--title-region";

/// The bests a launch hands out, as the shell would save them (JSON).
fn handed_out(compass: &Compass) -> SavedBests {
    let json = serde_json::to_string(&compass.bests()).unwrap();
    serde_json::from_str(&json).unwrap()
}

/// The first launch, its wall clock at [`WALL_AT_START`] at capture time 0.
fn first_launch() -> Player {
    Player::clocked(tracking())
}

/// The next launch, its shell restoring `bests` at startup, its wall clock at
/// `wall_at_start` at capture time 0: the player's Session under way, on a map the
/// panel doesn't know.
fn relaunch(bests: SavedBests, wall_at_start: u64) -> Player {
    let mut compass = tracking();
    compass.restore_bests(bests);
    Player::clocked_at(compass, wall_at_start)
}

fn line(state: &PanelState) -> Option<PersonalBestLine> {
    state.exp_tracking.personal_best
}

/// The line once earlier Visits had a best of `exp_per_hour` here, set at `set_at` by
/// the wall clock: "Your best EXP/h here: …", or with a share, "N% of your best EXP/h
/// here (…)".
fn earlier(exp_per_hour: u64, set_at: Option<u64>, share_hundredths: Option<i64>) -> Option<PersonalBestLine> {
    Some(PersonalBestLine {
        exp_per_hour,
        set_at_unix_ms: set_at,
        first: false,
        share_hundredths,
        new_best: None,
    })
}

#[test]
fn bests_handed_out_by_one_launch_show_the_same_line_and_hover_in_the_next() {
    // A first Visit to Sunset Sky at 7,200 EXP/h, its best set 5 minutes in; 2 minutes on
    // the Construction Site, and back.
    let mut first = first_launch();
    first.arrive(SUNSET);
    first.hunt(5 * MIN, 20);
    let set = first.wall_clock();
    first.hunt(MIN, 20);
    first.arrive(CONSTRUCTION_SITE);
    first.hunt(2 * MIN, 30);
    let back = first.arrive(SUNSET);
    assert_eq!(line(&back), earlier(7200, set, None));
    let saved = handed_out(&first.compass);
    // 5 minutes at 3,600 EXP/h: 50% of it.
    let five = first.hunt(5 * MIN, 10);
    assert_eq!(line(&five), earlier(7200, set, Some(5000)));

    // A day later, the next launch shows the same, set when it was.
    let mut next = relaunch(saved, WALL_AT_START + DAY);
    assert_eq!(line(&next.arrive(SUNSET)), line(&back));
    assert_eq!(line(&next.hunt(5 * MIN, 10)), line(&five));
}

#[test]
fn a_visit_cut_short_by_closing_maplecompass_keeps_its_best() {
    // A first Visit to Sunset Sky at 7,200 EXP/h, its best set 5 minutes in, closed a
    // minute later.
    let mut first = first_launch();
    first.arrive(SUNSET);
    first.hunt(5 * MIN, 20);
    let set = first.wall_clock();
    first.hunt(MIN, 20);

    // Closing ended the Visit: the next launch has its best as the earlier Visits'.
    let mut next = relaunch(handed_out(&first.compass), WALL_AT_START + DAY);
    assert_eq!(line(&next.arrive(SUNSET)), earlier(7200, set, None));

    // 10,800 EXP/h beats it, and that Visit is closed in turn: it is the best after.
    next.hunt(5 * MIN, 30);
    let beaten = next.wall_clock();
    let mut third = relaunch(handed_out(&next.compass), WALL_AT_START + 2 * DAY);
    assert_eq!(line(&third.arrive(SUNSET)), earlier(10_800, beaten, None));
}

/// A first Visit to Sunset Sky at 7,200 EXP/h for 6 minutes, closed then: the bests the
/// launch hands out.
fn a_best_of_7200_on_sunset_sky() -> SavedBests {
    let mut first = first_launch();
    first.arrive(SUNSET);
    first.hunt(6 * MIN, 20);
    handed_out(&first.compass)
}

#[test]
fn bests_saved_in_another_shape_are_left_and_the_app_starts_with_none() {
    let saved = SavedBests { version: BESTS_VERSION + 1, ..a_best_of_7200_on_sunset_sky() };

    let mut next = relaunch(saved, WALL_AT_START + DAY);
    assert_eq!(line(&next.arrive(SUNSET)), None);
    // 5 minutes at 3,600 EXP/h: a first Visit's best.
    let five = next.hunt(5 * MIN, 10);
    assert_eq!(line(&five).map(|l| (l.exp_per_hour, l.first)), Some((3600, true)));
}

/// The line on a first Visit to a map, its best so far set at `set_at` by the wall clock:
/// "Your first best EXP/h here: …".
fn first_visit(exp_per_hour: u64, set_at: Option<u64>) -> Option<PersonalBestLine> {
    Some(PersonalBestLine { exp_per_hour, set_at_unix_ms: set_at, first: true, share_hundredths: None, new_best: None })
}

#[test]
fn bests_of_no_exp_in_a_hand_edited_file_are_left_out() {
    // The file edited by hand: Sunset Sky's only best, and the Construction Site's
    // second, are 0 EXP/h.
    let edited = format!(
        r#"{{"version": {BESTS_VERSION}, "maps": [
            {{"id": "{SUNSET_SKY_ID}", "visits": [{{"expPerHour": 0, "setAtUnixMs": null}}]}},
            {{"id": "{CONSTRUCTION_SITE_ID}", "visits": [
                {{"expPerHour": 7200, "setAtUnixMs": {WALL_AT_START}}},
                {{"expPerHour": 0, "setAtUnixMs": {WALL_AT_START}}}
            ]}}
        ]}}"#
    );
    let mut next = relaunch(serde_json::from_str(&edited).unwrap(), WALL_AT_START + DAY);
    let handed = handed_out(&next.compass);
    assert!(handed.maps.iter().all(|m| m.id != SUNSET_SKY_ID));
    assert_eq!(saved_rates(&handed, CONSTRUCTION_SITE_ID), [7200]);

    // Sunset Sky has no best: a first Visit, with its own best from 5 minutes there.
    assert_eq!(line(&next.arrive(SUNSET)), None);
    let five = next.hunt(5 * MIN, 10);
    assert_eq!(line(&five), first_visit(3600, next.wall_clock()));

    // On the Construction Site, 3,600 EXP/h is 50% of the 7,200; forgetting it leaves no
    // best from earlier Visits, and the line reads as on a first Visit.
    next.arrive(CONSTRUCTION_SITE);
    let five = next.hunt(5 * MIN, 10);
    let set = next.wall_clock();
    assert_eq!(line(&five), earlier(7200, Some(WALL_AT_START), Some(5000)));
    assert_eq!(line(&next.press(PanelControl::ForgetBest)), first_visit(3600, set));
}

/// The EXP/h of each Visit's best a map's saved bests hold, in their order.
fn saved_rates(bests: &SavedBests, map: &str) -> Vec<u64> {
    let map = bests.maps.iter().find(|m| m.id == map);
    map.map_or_else(Vec::new, |m| m.visits.iter().map(|v| v.exp_per_hour).collect())
}

#[test]
fn each_map_keeps_only_its_10_best_visits() {
    // 11 Visits to Sunset Sky, 5 minutes each, at 3,600 to 7,200 EXP/h in steps of 360
    // (the slowest the third), each but the last ended by stepping onto the Construction
    // Site. The last is under way when the bests are handed out.
    let mut first = Player::saved_by_the_shell(tracking());
    for (i, gain) in [15, 11, 10, 20, 18, 12, 19, 13, 17, 14, 16].into_iter().enumerate() {
        if i > 0 {
            first.arrive(CONSTRUCTION_SITE);
        }
        first.arrive(SUNSET);
        first.hunt(5 * MIN, gain);
    }

    // The slowest Visit didn't make the 10, highest first.
    let kept: Vec<u64> = (11..=20).rev().map(|gain| gain * 360).collect();
    assert_eq!(saved_rates(&handed_out(&first.compass), SUNSET_SKY_ID), kept);

    // A twelfth Visit at 3,240 EXP/h doesn't make them either, ended or not, so the
    // shell has nothing to save.
    first.arrive(CONSTRUCTION_SITE);
    first.arrive(SUNSET);
    let saves = first.saves();
    first.hunt(5 * MIN, 9);
    assert_eq!(saved_rates(&handed_out(&first.compass), SUNSET_SKY_ID), kept);
    first.arrive(CONSTRUCTION_SITE);
    assert_eq!(saved_rates(&handed_out(&first.compass), SUNSET_SKY_ID), kept);
    assert_eq!(first.saves(), saves);
}

#[test]
fn with_no_bests_saved_the_app_starts_with_none() {
    let mut next = relaunch(SavedBests::default(), WALL_AT_START + DAY);
    assert_eq!(line(&next.arrive(SUNSET)), None);
}

#[test]
fn the_shell_saves_the_bests_whenever_they_change_and_only_then() {
    let mut player = Player::saved_by_the_shell(tracking_with(sharing_a_title()));

    // 10,800 EXP/h while the Candidates wait for a choice: no best, nothing to save.
    player.arrive(KERNING);
    player.hunt(6 * MIN, 30);
    assert_eq!(player.saves(), 0);

    // The hotkey shows Sunset Sky, which takes this Visit's best; on to the Construction
    // Site, which takes it from Sunset Sky.
    player.choose_next();
    assert_eq!(player.saves(), 1);
    let site = player.choose_next();
    assert_eq!(site.map.map(|m| m.id), Some(CONSTRUCTION_SITE_ID.to_string()));
    assert_eq!(player.saves(), 2);

    // The same rate again is no new best; 14,400 EXP/h raises it at refreshes.
    player.hunt(2 * MIN, 30);
    assert_eq!(player.saves(), 2);
    player.hunt(5 * MIN, 40);
    let raised = player.saves();
    assert!(raised > 2);

    // A reset, the bar unread, leaving the map, and play on a map without a Map Summary
    // change no best.
    player.press(PanelControl::ResetSession);
    player.unread(MIN);
    player.arrive(ROCKY_ROAD);
    player.hunt(6 * MIN, 30);
    assert_eq!(player.saves(), raised);

    // Back, choosing the Construction Site past Sunset Sky: a slower Visit's best joins
    // its others.
    player.arrive(KERNING);
    player.choose_next();
    player.choose_next();
    player.hunt(6 * MIN, 20);
    let joined = player.saves();
    assert!(joined > raised);
    let site_rates = |player: &Player| saved_rates(&player.compass.bests(), CONSTRUCTION_SITE_ID);
    assert_eq!(site_rates(&player), [14_400, 7200]);

    // With no rate since a reset, choosing Sunset Sky gives it no best of its own: the
    // Visit's best stays the Construction Site's, so closing MapleCompass now keeps it,
    // and it joins the Construction Site's others when the Visit ends there.
    player.press(PanelControl::ResetSession);
    let sunset = player.choose_next();
    assert_eq!(sunset.map.map(|m| m.id), Some(SUNSET_SKY_ID.to_string()));
    assert_eq!((player.saves(), site_rates(&player)), (joined, vec![14_400, 7200]));
    player.arrive(ROCKY_ROAD);
    assert_eq!((player.saves(), site_rates(&player)), (joined, vec![14_400, 7200]));
}

#[test]
fn a_forget_and_its_undo_change_the_bests_the_shell_saves() {
    let mut player = Player::saved_by_the_shell(tracking());
    let sunset_rates = |player: &Player| saved_rates(&player.compass.bests(), SUNSET_SKY_ID);
    let saving = |player: &mut Player, control| {
        let saves = player.saves();
        player.press(control);
        assert_eq!(player.saves(), saves + 1, "{control:?} saved");
        sunset_rates(player)
    };

    // A first Visit's only best, forgotten: Sunset Sky has none to save. Undo brings it
    // back.
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    assert_eq!(sunset_rates(&player), [7200]);
    assert_eq!(saving(&mut player, PanelControl::ForgetBest), [] as [u64; 0]);
    assert_eq!(saving(&mut player, PanelControl::UndoForget), [7200]);

    // Back after the Construction Site: forgetting the earlier Visit's best, Sunset Sky's
    // only one, leaves it none to save. Undo brings it back.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 30);
    player.arrive(SUNSET);
    assert_eq!(saving(&mut player, PanelControl::ForgetBest), [] as [u64; 0]);
    assert!(player.compass.bests().maps.iter().all(|m| m.id != SUNSET_SKY_ID));
    assert_eq!(saving(&mut player, PanelControl::UndoForget), [7200]);

    // At 3,600 EXP/h: forgetting the earlier Visit's best leaves this Visit's. Undo brings
    // it back.
    player.hunt(5 * MIN, 10);
    assert_eq!(sunset_rates(&player), [7200, 3600]);
    assert_eq!(saving(&mut player, PanelControl::ForgetBest), [3600]);
    assert_eq!(saving(&mut player, PanelControl::UndoForget), [7200, 3600]);

    // 10,800 EXP/h beats it: forgetting this Visit's best leaves the earlier one.
    player.hunt(5 * MIN, 30);
    assert_eq!(sunset_rates(&player), [10_800, 7200]);
    assert_eq!(saving(&mut player, PanelControl::ForgetBest), [7200]);
    assert_eq!(saving(&mut player, PanelControl::UndoForget), [10_800, 7200]);

    // Forgotten when MapleCompass closes, it stays forgotten.
    player.press(PanelControl::ForgetBest);
    let set = player.compass.bests().maps[0].visits[0].set_at_unix_ms.unwrap();
    let mut next = relaunch(handed_out(&player.compass), WALL_AT_START + DAY);
    assert_eq!(line(&next.arrive(SUNSET)), earlier(7200, Some(set), None));
}
