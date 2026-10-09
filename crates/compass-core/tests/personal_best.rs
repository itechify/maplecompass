//! The Personal Best (GLOSSARY): the Grind Tracker's Last 5 min EXP/h measured against
//! the best the player has had on the map they're on. A Visit's best is the highest
//! Last 5 min EXP/h shown at a refresh over a full 5 minutes of active play on the map;
//! the map's Personal Best is the best of its Visits'.

mod common;

use common::player::Player;
use common::{CONSTRUCTION_SITE_ID, FakeSource, SUNSET_SKY_ID, saved_map_page, sharing_a_title, tracking, tracking_with};
use compass_core::{
    BestNamed, ExpBar, ExpBarReading, PanelControl, PanelState, PersonalBestLine, TrackingState, VisitBest, Whose,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Poll;

const S: u64 = 1000;
const MIN: u64 = 60 * S;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const ROCKY_ROAD: &str = "victoria-road__rocky-road-i--title-region";

/// Last 5 min's EXP/h as the Grind Tracker shows it.
fn rolling(state: &PanelState) -> Option<i64> {
    state.exp_tracking.rolling.map(|r| r.exp_per_hour)
}

/// The line on a first Visit to a map: "Your first best EXP/h here: …", with no wall
/// clock to date it by.
fn first_best(exp_per_hour: u64) -> Option<PersonalBestLine> {
    first_best_set(exp_per_hour, None)
}

/// The line on a first Visit to a map, its best set at `set_at_unix_ms` by the wall
/// clock. Forget would take that best.
fn first_best_set(exp_per_hour: u64, set_at_unix_ms: Option<u64>) -> Option<PersonalBestLine> {
    let best = VisitBest { exp_per_hour, set_at_unix_ms };
    Some(PersonalBestLine { named: BestNamed::FirstVisit { best }, forgets: Whose::ThisVisit })
}

#[test]
fn a_first_visits_best_is_dated_by_the_wall_clock_at_the_refresh_that_showed_it() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);

    // 3,600 EXP/h: the first best shows at the refresh 5 minutes in, and was set then.
    let five = player.hunt(5 * MIN, 10);
    let first_set = player.wall_clock();
    assert_eq!(rolling(&five), Some(3600));
    assert_eq!(five.exp_tracking.personal_best, first_best_set(3600, first_set));

    // The same rate shown again isn't a new best: it keeps when it was first set.
    let same = player.hunt(MIN, 10);
    assert_eq!(same.exp_tracking.personal_best, first_best_set(3600, first_set));

    // Faster, the best rises with each refresh, set at the last.
    let faster = player.hunt(5 * MIN, 20);
    let faster_set = player.wall_clock();
    assert_eq!(rolling(&faster), Some(7200));
    assert_eq!(faster.exp_tracking.personal_best, first_best_set(7200, faster_set));

    // Slower, it stays as it was, and when.
    let slower = player.hunt(5 * MIN, 10);
    assert_eq!(rolling(&slower), Some(3600));
    assert_eq!(slower.exp_tracking.personal_best, first_best_set(7200, faster_set));
}

#[test]
fn a_first_visit_shows_its_best_so_far_once_there_have_been_five_minutes_of_play_there() {
    let mut player = Player::new(tracking());
    // 10,800 EXP/h on the Construction Site for 6 minutes, then on to Sunset Sky.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(6 * MIN, 30);
    player.arrive(SUNSET);

    // 3,600 EXP/h there. Under 5 minutes in, Last 5 min still counts some of the
    // Construction Site's play: no best yet, and nothing to show.
    let under = player.hunt(4 * MIN + 30 * S, 10);
    assert_eq!(rolling(&under), Some(4320));
    assert_eq!(under.exp_tracking.personal_best, None);

    // 5 minutes there: Last 5 min is all Sunset Sky's, and this Visit's first best.
    let five = player.hunt(30 * S, 10);
    assert_eq!(rolling(&five), Some(3600));
    assert_eq!(five.exp_tracking.personal_best, first_best(3600));

    // Faster, the best so far rises with what Last 5 min shows...
    let faster = player.hunt(5 * MIN, 20);
    assert_eq!(rolling(&faster), Some(7200));
    assert_eq!(faster.exp_tracking.personal_best, first_best(7200));

    // ...and it stays when the rate falls back.
    let slower = player.hunt(5 * MIN, 10);
    assert_eq!(rolling(&slower), Some(3600));
    assert_eq!(slower.exp_tracking.personal_best, first_best(7200));
}

/// The line once earlier Visits had `best` here: "Your best EXP/h here: …", or with a
/// share, "N% of your best EXP/h here (…)"; with this Visit's best once it has beaten
/// it, and the "new best" tag. Forget would take the higher of the two: this Visit's
/// once it has beaten the earlier Visits'.
fn earlier_best(best: VisitBest, share_hundredths: Option<i64>, new_best: Option<VisitBest>) -> Option<PersonalBestLine> {
    let forgets = if new_best.is_some() { Whose::ThisVisit } else { Whose::EarlierVisits };
    Some(PersonalBestLine { named: BestNamed::EarlierVisits { best, share_hundredths, new_best }, forgets })
}

/// As [`earlier_best`], with no wall clock to date the bests.
fn best(exp_per_hour: u64, share_hundredths: Option<i64>, new_best: Option<u64>) -> Option<PersonalBestLine> {
    let undated = |exp_per_hour| VisitBest { exp_per_hour, set_at_unix_ms: None };
    earlier_best(undated(exp_per_hour), share_hundredths, new_best.map(undated))
}

/// A first Visit to Sunset Sky at 7,200 EXP/h for 6 minutes, then 2 minutes on the
/// Construction Site, and back to Sunset Sky.
fn back_on_sunset_sky_after_a_best_of_7200() -> (Player, PanelState) {
    let (player, back, _) = back_after_a_best_of_7200(Player::new(tracking()));
    (player, back)
}

/// As [`back_on_sunset_sky_after_a_best_of_7200`], for `player`: and when the 7,200 was
/// set by the wall clock, at the refresh 5 minutes into that first Visit, the first to
/// show it.
fn back_after_a_best_of_7200(mut player: Player) -> (Player, PanelState, Option<u64>) {
    player.arrive(SUNSET);
    player.hunt(5 * MIN, 20);
    let set = player.wall_clock();
    player.hunt(MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(2 * MIN, 30);
    let back = player.arrive(SUNSET);
    (player, back, set)
}

#[test]
fn the_best_from_earlier_visits_keeps_when_it_was_set() {
    let (mut player, back, set) = back_after_a_best_of_7200(Player::clocked(tracking()));
    let earlier = |share_hundredths| earlier_best(VisitBest { exp_per_hour: 7200, set_at_unix_ms: set }, share_hundredths, None);
    assert!(set.is_some());
    assert_eq!(back.exp_tracking.personal_best, earlier(None));

    // 5 minutes at 3,600 EXP/h, 50% of it: the best is as it was, and when.
    let five = player.hunt(5 * MIN, 10);
    assert_eq!(five.exp_tracking.personal_best, earlier(Some(5000)));
}

#[test]
fn once_this_visit_beats_the_best_the_line_carries_this_visits_best_and_when_it_was_set() {
    let (mut player, _, set) = back_after_a_best_of_7200(Player::clocked(tracking()));
    let line = |share_hundredths, new_best| {
        earlier_best(VisitBest { exp_per_hour: 7200, set_at_unix_ms: set }, share_hundredths, new_best)
    };

    // 3,600 EXP/h for 5 minutes: this Visit's best so far is under 7,200, so the line
    // carries only the earlier one.
    let under = player.hunt(5 * MIN, 10);
    assert_eq!(under.exp_tracking.personal_best, line(Some(5000), None));

    // 10,800 EXP/h for 5 minutes beats it: this Visit's best is the 10,800 the last
    // refresh showed, and set then.
    let beaten = player.hunt(5 * MIN, 30);
    let this_visit = Some(VisitBest { exp_per_hour: 10_800, set_at_unix_ms: player.wall_clock() });
    assert_eq!(rolling(&beaten), Some(10_800));
    assert_eq!(beaten.exp_tracking.personal_best, line(Some(15_000), this_visit));

    // Back down to 3,600 EXP/h: it stays as it was, and when, for the rest of the Visit.
    let slower = player.hunt(5 * MIN, 10);
    assert_eq!(slower.exp_tracking.personal_best, line(Some(5000), this_visit));
}

#[test]
fn coming_back_shows_the_best_until_five_minutes_there_then_the_rate_as_a_share_of_it() {
    let (mut player, back) = back_on_sunset_sky_after_a_best_of_7200();
    assert_eq!(back.exp_tracking.personal_best, best(7200, None, None));

    // 3,600 EXP/h: under 5 minutes, Last 5 min isn't all Sunset Sky's yet.
    let under = player.hunt(4 * MIN + 30 * S, 10);
    assert_eq!(under.exp_tracking.personal_best, best(7200, None, None));

    // 5 minutes there: Last 5 min's 3,600 EXP/h is 50% of 7,200.
    let five = player.hunt(30 * S, 10);
    assert_eq!(rolling(&five), Some(3600));
    assert_eq!(five.exp_tracking.personal_best, best(7200, Some(5000), None));
}

#[test]
fn beating_the_best_shows_the_share_past_100_percent_and_a_new_best_tag_for_the_rest_of_the_visit() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();

    // 10,800 EXP/h for 5 minutes: 150% of 7,200, a new best.
    let beaten = player.hunt(5 * MIN, 30);
    assert_eq!(rolling(&beaten), Some(10_800));
    assert_eq!(beaten.exp_tracking.personal_best, best(7200, Some(15_000), Some(10_800)));

    // Back down to 3,600 EXP/h, 50%: still a new best this Visit.
    let slower = player.hunt(5 * MIN, 10);
    assert_eq!(rolling(&slower), Some(3600));
    assert_eq!(slower.exp_tracking.personal_best, best(7200, Some(5000), Some(10_800)));
}

#[test]
fn the_best_compared_with_is_fixed_on_arrival_and_a_new_one_counts_from_the_next_visit() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();

    // 10,800 EXP/h, then 9,000: both measured against 7,200 all Visit, never against
    // the 10,800 set in it.
    player.hunt(5 * MIN, 30);
    let after = player.hunt(5 * MIN, 25);
    assert_eq!(rolling(&after), Some(9000));
    assert_eq!(after.exp_tracking.personal_best, best(7200, Some(12_500), Some(10_800)));

    // The next Visit is measured against it: 9,000 EXP/h is 83.33% of 10,800.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 30);
    let next = player.arrive(SUNSET);
    assert_eq!(next.exp_tracking.personal_best, best(10_800, None, None));
    let five = player.hunt(5 * MIN, 25);
    assert_eq!(five.exp_tracking.personal_best, best(10_800, Some(8333), None));
}

/// MeowDB's saved index and pages, but Sunset Sky's page is held back until `page_in`
/// lets it through.
fn sunset_sky_held_back(page_in: &Arc<AtomicBool>) -> FakeSource {
    let page_in = page_in.clone();
    FakeSource::meowdb().with_pages(move |id| {
        if id == SUNSET_SKY_ID && !page_in.load(Ordering::SeqCst) { Poll::Pending } else { Poll::Ready(saved_map_page(id)) }
    })
}

#[test]
fn nothing_is_recorded_on_a_map_without_a_map_summary() {
    let page_in = Arc::new(AtomicBool::new(false));
    let mut player = Player::new(tracking_with(sunset_sky_held_back(&page_in)));

    // A town has none: no line there, however long the play.
    player.arrive(KERNING);
    assert_eq!(player.hunt(6 * MIN, 30).exp_tracking.personal_best, None);

    // Nor is there one while the map's page hasn't come.
    let sunset = player.arrive(SUNSET);
    assert_eq!(sunset.data.solo_exp_ceiling(), None);
    assert_eq!(player.hunt(6 * MIN, 30).exp_tracking.personal_best, None);

    // Back with its page in, nothing came of the Visit before: a first Visit's line,
    // once there have been 5 minutes there.
    player.arrive(CONSTRUCTION_SITE);
    page_in.store(true, Ordering::SeqCst);
    player.hunt(MIN, 10);
    let back = player.arrive(SUNSET);
    assert_eq!(back.data.solo_exp_ceiling(), Some(223_928));
    assert_eq!(back.exp_tracking.personal_best, None);
    assert_eq!(player.hunt(5 * MIN, 10).exp_tracking.personal_best, first_best(3600));
}

#[test]
fn the_five_minutes_count_from_arriving_even_when_the_maps_page_comes_later() {
    let page_in = Arc::new(AtomicBool::new(false));
    let mut player = Player::new(tracking_with(sunset_sky_held_back(&page_in)));
    // 10,800 EXP/h on the Construction Site for 6 minutes, then on to Sunset Sky, whose
    // page hasn't come.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(6 * MIN, 30);
    assert_eq!(player.arrive(SUNSET).data.solo_exp_ceiling(), None);

    // 3,600 EXP/h there, the bar read all along; the page comes 2 minutes in.
    player.hunt(2 * MIN, 10);
    page_in.store(true, Ordering::SeqCst);
    let paged = player.compass.tick(player.at).clone();
    assert_eq!(paged.data.solo_exp_ceiling(), Some(223_928));
    assert_eq!(paged.exp_tracking.personal_best, None);

    // Under 5 minutes from arriving, Last 5 min still counts some of the Construction
    // Site's play: nothing yet.
    let under = player.hunt(2 * MIN + 30 * S, 10);
    assert_eq!(under.exp_tracking.personal_best, None);

    // 5 minutes from arriving, though only 3 from the page: Sunset Sky's first best.
    let five = player.hunt(30 * S, 10);
    assert_eq!(rolling(&five), Some(3600));
    assert_eq!(five.exp_tracking.personal_best, first_best(3600));
}

fn shown_map(state: &PanelState) -> Option<&str> {
    state.map.as_ref().map(|m| m.id.as_str())
}

#[test]
fn nothing_is_recorded_while_candidates_wait_for_a_choice() {
    let mut player = Player::new(tracking_with(sharing_a_title()));
    let arrived = player.arrive(KERNING);
    assert_eq!(shown_map(&arrived), Some(CONSTRUCTION_SITE_ID));
    assert_eq!(arrived.candidates.map(|c| c.chosen), Some(false));

    // 10,800 EXP/h, then 3,600, with nothing chosen: no line.
    assert_eq!(player.hunt(6 * MIN, 30).exp_tracking.personal_best, None);
    assert_eq!(player.hunt(5 * MIN, 10).exp_tracking.personal_best, None);

    // The player chooses the Construction Site: the hotkey shows Sunset Sky, then it.
    player.choose_next();
    let chosen = player.choose_next();
    assert_eq!(shown_map(&chosen), Some(CONSTRUCTION_SITE_ID));
    // Its first best is Last 5 min as it shows now: the 10,800 shown before the choice
    // never counted.
    assert_eq!(chosen.exp_tracking.personal_best, first_best(3600));
}

#[test]
fn choosing_a_candidate_past_another_keeps_this_visits_best_for_the_one_chosen() {
    let mut player = Player::new(tracking_with(sharing_a_title()));
    player.arrive(KERNING);
    player.hunt(6 * MIN, 10);

    // Choosing the Construction Site goes past Sunset Sky.
    player.choose_next();
    assert_eq!(player.choose_next().exp_tracking.personal_best, first_best(3600));

    // On Rocky Road I, whose page doesn't settle which map the title is on the way back.
    player.arrive(ROCKY_ROAD);
    player.hunt(MIN, 10);
    let back = player.arrive(KERNING);
    assert_eq!(back.candidates.map(|c| c.chosen), Some(false));

    // Sunset Sky has no best from the moment it showed on the way past...
    let sunset = player.choose_next();
    assert_eq!(shown_map(&sunset), Some(SUNSET_SKY_ID));
    assert_eq!(sunset.exp_tracking.personal_best, None);
    // ...and the Construction Site has the Visit's.
    let site = player.choose_next();
    assert_eq!(shown_map(&site), Some(CONSTRUCTION_SITE_ID));
    assert_eq!(site.exp_tracking.personal_best, best(3600, None, None));
}

#[test]
fn nothing_is_recorded_while_the_exp_bar_cannot_be_read() {
    let page_in = Arc::new(AtomicBool::new(false));
    let mut player = Player::new(tracking_with(sunset_sky_held_back(&page_in)));
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 30);

    // The bar goes unread, and Sunset Sky's page comes meanwhile: the figures are measured
    // afresh for its Map Summary, but nothing is known of the EXP.
    let unreadable = player.unread(30 * S);
    assert_eq!(unreadable.exp_tracking.state, TrackingState::Unreadable);
    page_in.store(true, Ordering::SeqCst);
    let paged = player.compass.tick(player.at).clone();
    assert_eq!(paged.data.solo_exp_ceiling(), Some(223_928));

    // Off to the Construction Site before it reads again, then back: nothing came of the
    // Visit.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 10);
    let back = player.arrive(SUNSET);
    assert_eq!(back.exp_tracking.personal_best, None);
}

#[test]
fn a_best_is_last_5_min_as_shown_at_a_refresh_never_a_peak_between() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    assert_eq!(player.hunt(6 * MIN, 10).exp_tracking.personal_best, first_best(3600));

    // 10 s into a half minute, a burst of 310 EXP: Last 5 min would read 7,200 EXP/h
    // (600 EXP in 5 minutes), but the figures shown hold till the half minute is out.
    let burst = player.hunt(10 * S, 310);
    assert_eq!(rolling(&burst), Some(3600));
    assert_eq!(burst.exp_tracking.personal_best, first_best(3600));

    // Nothing more till then: Last 5 min shows 6,960 EXP/h (580 EXP), and that is the
    // best, as long as the burst counts in it.
    let shown = player.hunt(20 * S, 0);
    assert_eq!(rolling(&shown), Some(6960));
    assert_eq!(shown.exp_tracking.personal_best, first_best(6960));
    let later = player.hunt(5 * MIN, 10);
    assert_eq!(rolling(&later), Some(3600));
    assert_eq!(later.exp_tracking.personal_best, first_best(6960));
}

#[test]
fn a_session_reset_or_a_character_switch_leaves_the_bests_as_they_were() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    // A new best this Visit: 10,800 EXP/h.
    player.hunt(5 * MIN, 30);

    // The player starts the Session afresh: the best and this Visit's new one stay.
    player.compass.press(PanelControl::ResetSession);
    let reset = player.compass.panel_state().clone();
    assert_eq!(reset.exp_tracking.state, TrackingState::Waiting);
    assert_eq!(reset.exp_tracking.personal_best, best(7200, None, Some(10_800)));
    // The new Session's 5 minutes are on this map too: 3,600 EXP/h is 50% of 7,200.
    player.hunt(5 * MIN + 10 * S, 10);
    assert_eq!(player.hunt(20 * S, 10).exp_tracking.personal_best, best(7200, Some(5000), Some(10_800)));

    // Another character's bar, 157 [0.94%], read three times: a Session of their own.
    for _ in 0..3 {
        player.at += 10 * S;
        let bar = Some(ExpBar { exp: 157, percent_hundredths: 94 });
        player.compass.on_exp_bar_reading(ExpBarReading { bar, at_ms: player.at });
    }
    let switched = player.compass.panel_state().clone();
    assert!(switched.exp_tracking.character_switched);
    assert_eq!(switched.exp_tracking.personal_best, best(7200, None, Some(10_800)));

    // This Visit's best joins Sunset Sky's when it ends, whoever plays.
    player.arrive(CONSTRUCTION_SITE);
    let back = player.arrive(SUNSET);
    assert_eq!(back.exp_tracking.personal_best, best(10_800, None, None));
}

#[test]
fn with_the_bar_unreadable_or_paused_the_line_holds_like_the_other_figures() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    let held = best(7200, Some(5000), None);
    assert_eq!(player.hunt(5 * MIN, 10).exp_tracking.personal_best, held);

    // The bar can't be read for 10 minutes: everything shown holds.
    let unreadable = player.unread(10 * MIN);
    assert_eq!(unreadable.exp_tracking.state, TrackingState::Unreadable);
    assert_eq!(rolling(&unreadable), Some(3600));
    assert_eq!(unreadable.exp_tracking.personal_best, held);

    // It reads again, unchanged: the Session is paused at Last 5 min's 3,600 EXP/h, and
    // the line with it, however long the break.
    let paused = player.hunt(10 * S, 0);
    assert_eq!(paused.exp_tracking.state, TrackingState::Paused);
    assert_eq!(rolling(&paused), Some(3600));
    assert_eq!(paused.exp_tracking.personal_best, held);
    let later = player.hunt(10 * MIN, 0);
    assert_eq!(later.exp_tracking.state, TrackingState::Paused);
    assert_eq!(later.exp_tracking.personal_best, held);
}

#[test]
fn the_overlay_gets_the_line_as_the_grind_tracker_renders_it_with_whose_best_forget_takes() {
    // A first Visit's best so far: Forget would take it.
    let mut first = Player::clocked(tracking());
    first.arrive(SUNSET);
    let five = serde_json::to_value(first.hunt(5 * MIN, 10)).unwrap();
    assert_eq!(
        five["expTracking"]["personalBest"],
        serde_json::json!({
            "kind": "firstVisit", "best": { "expPerHour": 3600, "setAtUnixMs": first.wall_clock() },
            "forgets": "thisVisit",
        })
    );

    // Earlier Visits' 7,200 beaten: Forget would take this Visit's 10,800...
    let (mut player, _, set) = back_after_a_best_of_7200(Player::clocked(tracking()));
    let beaten = serde_json::to_value(player.hunt(5 * MIN, 30)).unwrap();
    let beaten_at = player.wall_clock();
    assert_eq!(
        beaten["expTracking"]["personalBest"],
        serde_json::json!({
            "kind": "earlierVisits", "best": { "expPerHour": 7200, "setAtUnixMs": set }, "shareHundredths": 15_000,
            "newBest": { "expPerHour": 10_800, "setAtUnixMs": beaten_at }, "forgets": "thisVisit",
        })
    );
    assert_eq!(beaten["expTracking"]["forgetUndoable"], false);

    // ...and with it forgotten, the 7,200, the Forget undoable.
    let forgot = serde_json::to_value(player.press(PanelControl::ForgetBest)).unwrap();
    assert_eq!(
        forgot["expTracking"]["personalBest"],
        serde_json::json!({
            "kind": "earlierVisits", "best": { "expPerHour": 7200, "setAtUnixMs": set }, "shareHundredths": null,
            "newBest": null, "forgets": "earlierVisits",
        })
    );
    assert_eq!(forgot["expTracking"]["forgetUndoable"], true);
}

/// Sunset Sky's best Visits: 10,800 EXP/h on the first, 7,200 on the second; then back on
/// it a third time.
fn back_on_sunset_sky_after_bests_of_10800_and_7200() -> (Player, PanelState) {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 30);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(2 * MIN, 30);
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(2 * MIN, 30);
    let back = player.arrive(SUNSET);
    (player, back)
}

#[test]
fn forgetting_the_best_this_visit_has_not_beaten_leaves_the_next_best_visit_to_compare_with() {
    let (mut player, back) = back_on_sunset_sky_after_bests_of_10800_and_7200();
    assert_eq!(back.exp_tracking.personal_best, best(10_800, None, None));

    // 3,600 EXP/h for 5 minutes: 33.3% of the 10,800, which this Visit hasn't beaten.
    let five = player.hunt(5 * MIN, 10);
    assert_eq!(five.exp_tracking.personal_best, best(10_800, Some(3333), None));

    // Forget takes the 10,800: the 7,200 is the best now, and 3,600 is 50% of it.
    let forgot = player.press(PanelControl::ForgetBest);
    assert_eq!(forgot.exp_tracking.personal_best, best(7200, Some(5000), None));
}

#[test]
fn forgetting_this_visits_best_once_it_beat_the_best_counts_only_windows_that_start_after() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();

    // A bad jump: 36,000 EXP/h for 5 minutes, 500% of 7,200, a new best.
    let jump = player.hunt(5 * MIN, 100);
    assert_eq!(jump.exp_tracking.personal_best, best(7200, Some(50_000), Some(36_000)));

    // Forget takes this Visit's 36,000: the 7,200 is left, and the window starts over.
    let forgot = player.press(PanelControl::ForgetBest);
    assert_eq!(forgot.exp_tracking.personal_best, best(7200, None, None));

    // 3,600 EXP/h for 2 minutes: Last 5 min still has 3 minutes of the jump, 23,040 EXP/h,
    // and it isn't recorded again.
    let after = player.hunt(2 * MIN, 10);
    assert_eq!(rolling(&after), Some(23_040));
    assert_eq!(after.exp_tracking.personal_best, best(7200, None, None));

    // 5 minutes since the Forget: Last 5 min's 3,600 EXP/h is 50% of 7,200.
    let five = player.hunt(3 * MIN, 10);
    assert_eq!(rolling(&five), Some(3600));
    assert_eq!(five.exp_tracking.personal_best, best(7200, Some(5000), None));

    // 10,800 EXP/h for 5 minutes beats it: a new best since the Forget.
    let beaten = player.hunt(5 * MIN, 30);
    assert_eq!(beaten.exp_tracking.personal_best, best(7200, Some(15_000), Some(10_800)));

    // That is this Visit's best when it ends: the 36,000 never joins Sunset Sky's.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 30);
    let next = player.arrive(SUNSET);
    assert_eq!(next.exp_tracking.personal_best, best(10_800, None, None));
}

#[test]
fn forgetting_a_maps_only_best_from_an_earlier_visit_reads_as_a_first_visit() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    let under = player.hunt(4 * MIN, 10);
    assert_eq!(under.exp_tracking.personal_best, best(7200, None, None));

    // Forget takes the 7,200, Sunset Sky's only best: under 5 minutes here, a first
    // Visit has nothing to show...
    let forgot = player.press(PanelControl::ForgetBest);
    assert_eq!(forgot.exp_tracking.personal_best, None);

    // ...and from 5 minutes, its best so far.
    let five = player.hunt(MIN, 10);
    assert_eq!(five.exp_tracking.personal_best, first_best(3600));
}

#[test]
fn forgetting_a_first_visits_best_leaves_the_map_none_until_a_window_after_the_forget() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    assert_eq!(player.hunt(6 * MIN, 30).exp_tracking.personal_best, first_best(10_800));

    // Forget takes the 10,800, the map's only best.
    let forgot = player.press(PanelControl::ForgetBest);
    assert_eq!(forgot.exp_tracking.personal_best, None);

    // 3,600 EXP/h: nothing until 5 minutes since the Forget, then this Visit's best anew.
    assert_eq!(player.hunt(4 * MIN + 30 * S, 10).exp_tracking.personal_best, None);
    assert_eq!(player.hunt(30 * S, 10).exp_tracking.personal_best, first_best(3600));
}

#[test]
fn undo_brings_back_this_visits_forgotten_best_as_it_was() {
    let (mut player, _, _) = back_after_a_best_of_7200(Player::clocked(tracking()));
    let beaten = player.hunt(5 * MIN, 30);
    assert!(!beaten.exp_tracking.forget_undoable);

    // Forget takes this Visit's 10,800, and says so with an Undo.
    let forgot = player.press(PanelControl::ForgetBest);
    assert!(forgot.exp_tracking.forget_undoable);
    assert_ne!(forgot.exp_tracking.personal_best, beaten.exp_tracking.personal_best);

    // Undo: the line is as it was, share and dates and all, and the note goes.
    let undone = player.press(PanelControl::UndoForget);
    assert!(!undone.exp_tracking.forget_undoable);
    assert_eq!(undone.exp_tracking.personal_best, beaten.exp_tracking.personal_best);
}

#[test]
fn undo_brings_back_an_earlier_visits_forgotten_best_as_the_one_compared_with() {
    let (mut player, _) = back_on_sunset_sky_after_bests_of_10800_and_7200();
    assert_eq!(player.hunt(5 * MIN, 10).exp_tracking.personal_best, best(10_800, Some(3333), None));

    let forgot = player.press(PanelControl::ForgetBest);
    assert!(forgot.exp_tracking.forget_undoable);
    assert_eq!(forgot.exp_tracking.personal_best, best(7200, Some(5000), None));

    let undone = player.press(PanelControl::UndoForget);
    assert!(!undone.exp_tracking.forget_undoable);
    assert_eq!(undone.exp_tracking.personal_best, best(10_800, Some(3333), None));
}

#[test]
fn undo_keeps_a_higher_best_set_since_the_forget() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    player.hunt(5 * MIN, 30);
    player.press(PanelControl::ForgetBest);

    // 14,400 EXP/h for 5 minutes since the Forget: a new best above the 10,800 forgotten.
    let since = player.hunt(5 * MIN, 40);
    assert_eq!(since.exp_tracking.personal_best, best(7200, Some(20_000), Some(14_400)));

    // Undo brings the 10,800 back under it: the higher stays this Visit's best.
    let undone = player.press(PanelControl::UndoForget);
    assert_eq!(undone.exp_tracking.personal_best, best(7200, Some(20_000), Some(14_400)));
}

#[test]
fn leaving_the_map_ends_the_undo_and_the_forget_stands() {
    let (mut player, _) = back_on_sunset_sky_after_bests_of_10800_and_7200();
    player.hunt(5 * MIN, 10);
    assert!(player.press(PanelControl::ForgetBest).exp_tracking.forget_undoable);

    // Off to the Construction Site: the note goes, and Undo does nothing.
    let away = player.arrive(CONSTRUCTION_SITE);
    assert!(!away.exp_tracking.forget_undoable);
    player.hunt(MIN, 30);
    player.press(PanelControl::UndoForget);

    // Back on Sunset Sky, the 10,800 is gone for good: this Visit's 3,600 joined the 7,200.
    let back = player.arrive(SUNSET);
    assert!(!back.exp_tracking.forget_undoable);
    assert_eq!(back.exp_tracking.personal_best, best(7200, None, None));
}

#[test]
fn a_forgotten_best_is_not_recorded_again_from_a_session_whose_reset_is_undone() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    player.hunt(5 * MIN, 100);

    // The player resets the Session, forgets this Visit's 36,000, then undoes the reset:
    // Last 5 min is the jump again, but it started before the Forget.
    player.press(PanelControl::ResetSession);
    player.press(PanelControl::ForgetBest);
    let undone = player.press(PanelControl::UndoReset);
    assert_eq!(rolling(&undone), Some(36_000));
    assert_eq!(undone.exp_tracking.personal_best, best(7200, None, None));
}
