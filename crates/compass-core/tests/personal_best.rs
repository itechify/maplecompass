//! The Personal Best (GLOSSARY): the Grind Tracker's Last 5 min EXP/h measured against
//! the best the player has had on the map they're on. A Visit's best is the highest
//! Last 5 min EXP/h shown at a refresh over a full 5 minutes of active play on the map;
//! the map's Personal Best is the best of its Visits'.

mod common;

use compass_core::{
    Compass, ExpBar, ExpBarReading, ExpBarRegion, GameWindow, MapData, PanelControl, PanelState, PersonalBestLine,
    Rect, ScreenRect, Settings, Size, SourceError, TitleRegion, TrackingState, meowdb,
};
use common::{FakeSource, index_with_title, saved_page};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Poll;

const S: u64 = 1000;
const MIN: u64 = 60 * S;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const ROCKY_ROAD: &str = "victoria-road__rocky-road-i--title-region";
const SUNSET_SKY_ID: &str = "010003070";
const CONSTRUCTION_SITE_ID: &str = "010003080";

/// A core with EXP tracking on: both regions kept, on a 1920x1200 game window.
fn tracking() -> Compass {
    tracking_with(FakeSource::meowdb())
}

/// As [`tracking`], with maps and their pages from `source`.
fn tracking_with(source: FakeSource) -> Compass {
    let client = Size { width: 1920, height: 1200 };
    let mut compass = Compass::new(source);
    compass.restore(Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client }),
        exp_bar_region: Some(ExpBarRegion { marked: Rect { x: 1005, y: 1144, width: 240, height: 32 }, client }),
        ..Settings::default()
    });
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    compass.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    compass
}

/// The percentage the game shows for `exp` on a 20,000 EXP level, rounded to hundredths.
fn percent(exp: u64) -> u32 {
    ((exp * 20_000 + 20_000) / (2 * 20_000)) as u32
}

/// The player hunting on a 20,000 EXP level, the EXP bar read every 10 s. Readings at
/// whole half minutes of capture time are where the Grind Tracker's figures refresh.
struct Player {
    compass: Compass,
    exp: u64,
    at: u64,
}

impl Player {
    /// The bar read at 1,000 [5.00%] at 0 s, then 10 EXP more every 10 s on a map the
    /// panel doesn't know, up to 30 s: the Session has started.
    fn new(compass: Compass) -> Self {
        let mut player = Player { compass, exp: 1000, at: 0 };
        player.read();
        player.hunt(30 * S, 10);
        player
    }

    fn read(&mut self) -> PanelState {
        let bar = Some(ExpBar { exp: self.exp, percent_hundredths: percent(self.exp) });
        self.compass.on_exp_bar_reading(ExpBarReading { bar, at_ms: self.at }).clone()
    }

    /// The panel confirms the map of a sample: a Visit starts now.
    fn arrive(&mut self, sample: &str) -> PanelState {
        common::arrive(&mut self.compass, sample, self.at)
    }

    /// Hunt for `ms` more, `gain` EXP every 10 s (`gain` × 360 EXP/h): the panel state at
    /// the last reading.
    fn hunt(&mut self, ms: u64, gain: u64) -> PanelState {
        let mut state = self.compass.panel_state().clone();
        for _ in 0..ms / (10 * S) {
            self.at += 10 * S;
            self.exp += gain;
            state = self.read();
        }
        state
    }

    /// The bar doesn't read whole for `ms` more, a frame every 10 s: the panel state at
    /// the last.
    fn unread(&mut self, ms: u64) -> PanelState {
        let mut state = self.compass.panel_state().clone();
        for _ in 0..ms / (10 * S) {
            self.at += 10 * S;
            state = self.compass.on_exp_bar_reading(ExpBarReading { bar: None, at_ms: self.at }).clone();
        }
        state
    }

    /// Choose the next of the Candidates with the hotkey.
    fn choose_next(&mut self) -> PanelState {
        self.compass.on_hotkey("ctrl+shift+n");
        self.compass.panel_state().clone()
    }
}

/// Last 5 min's EXP/h as the Grind Tracker shows it.
fn rolling(state: &PanelState) -> Option<i64> {
    state.exp_tracking.rolling.map(|r| r.exp_per_hour)
}

/// The line on a first Visit to a map: "Your first best EXP/h here: …".
fn first_best(exp_per_hour: u64) -> Option<PersonalBestLine> {
    Some(PersonalBestLine { exp_per_hour, first: true, share_hundredths: None, new_best: false })
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

/// The line once earlier Visits had a best here: "Your best EXP/h here: …", or with a
/// share, "N% of your best EXP/h here (…)".
fn best(exp_per_hour: u64, share_hundredths: Option<i64>, new_best: bool) -> Option<PersonalBestLine> {
    Some(PersonalBestLine { exp_per_hour, first: false, share_hundredths, new_best })
}

/// A first Visit to Sunset Sky at 7,200 EXP/h for 6 minutes, then 2 minutes on the
/// Construction Site, and back to Sunset Sky.
fn back_on_sunset_sky_after_a_best_of_7200() -> (Player, PanelState) {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(2 * MIN, 30);
    let back = player.arrive(SUNSET);
    (player, back)
}

#[test]
fn coming_back_shows_the_best_until_five_minutes_there_then_the_rate_as_a_share_of_it() {
    let (mut player, back) = back_on_sunset_sky_after_a_best_of_7200();
    assert_eq!(back.exp_tracking.personal_best, best(7200, None, false));

    // 3,600 EXP/h: under 5 minutes, Last 5 min isn't all Sunset Sky's yet.
    let under = player.hunt(4 * MIN + 30 * S, 10);
    assert_eq!(under.exp_tracking.personal_best, best(7200, None, false));

    // 5 minutes there: Last 5 min's 3,600 EXP/h is 50% of 7,200.
    let five = player.hunt(30 * S, 10);
    assert_eq!(rolling(&five), Some(3600));
    assert_eq!(five.exp_tracking.personal_best, best(7200, Some(5000), false));
}

#[test]
fn beating_the_best_shows_the_share_past_100_percent_and_a_new_best_tag_for_the_rest_of_the_visit() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();

    // 10,800 EXP/h for 5 minutes: 150% of 7,200, a new best.
    let beaten = player.hunt(5 * MIN, 30);
    assert_eq!(rolling(&beaten), Some(10_800));
    assert_eq!(beaten.exp_tracking.personal_best, best(7200, Some(15_000), true));

    // Back down to 3,600 EXP/h, 50%: still a new best this Visit.
    let slower = player.hunt(5 * MIN, 10);
    assert_eq!(rolling(&slower), Some(3600));
    assert_eq!(slower.exp_tracking.personal_best, best(7200, Some(5000), true));
}

#[test]
fn the_best_compared_with_is_fixed_on_arrival_and_a_new_one_counts_from_the_next_visit() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();

    // 10,800 EXP/h, then 9,000: both measured against 7,200 all Visit, never against
    // the 10,800 set in it.
    player.hunt(5 * MIN, 30);
    let after = player.hunt(5 * MIN, 25);
    assert_eq!(rolling(&after), Some(9000));
    assert_eq!(after.exp_tracking.personal_best, best(7200, Some(12_500), true));

    // The next Visit is measured against it: 9,000 EXP/h is 83.33% of 10,800.
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 30);
    let next = player.arrive(SUNSET);
    assert_eq!(next.exp_tracking.personal_best, best(10_800, None, false));
    let five = player.hunt(5 * MIN, 25);
    assert_eq!(five.exp_tracking.personal_best, best(10_800, Some(8333), false));
}

/// Map `id`'s saved MeowDB page, read; unreachable without one.
fn saved_map_page(id: &str) -> Result<MapData, SourceError> {
    match saved_page(id) {
        Some(html) => meowdb::read_map_page(&html, id),
        None => Err(SourceError::Unreachable(format!("no saved page for {id}"))),
    }
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

/// The Construction Site and Sunset Sky sharing the Kerning City sample's title: the
/// Construction Site, first in the map index, shows first.
fn sharing_a_title() -> FakeSource {
    FakeSource::with_index(index_with_title("Victoria Road", "Kerning City", &[CONSTRUCTION_SITE_ID, SUNSET_SKY_ID]))
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
    assert_eq!(site.exp_tracking.personal_best, best(3600, None, false));
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
    assert_eq!(reset.exp_tracking.personal_best, best(7200, None, true));
    // The new Session's 5 minutes are on this map too: 3,600 EXP/h is 50% of 7,200.
    player.hunt(5 * MIN + 10 * S, 10);
    assert_eq!(player.hunt(20 * S, 10).exp_tracking.personal_best, best(7200, Some(5000), true));

    // Another character's bar, 157 [0.94%], read three times: a Session of their own.
    for _ in 0..3 {
        player.at += 10 * S;
        let bar = Some(ExpBar { exp: 157, percent_hundredths: 94 });
        player.compass.on_exp_bar_reading(ExpBarReading { bar, at_ms: player.at });
    }
    let switched = player.compass.panel_state().clone();
    assert!(switched.exp_tracking.character_switched);
    assert_eq!(switched.exp_tracking.personal_best, best(7200, None, true));

    // This Visit's best joins Sunset Sky's when it ends, whoever plays.
    player.arrive(CONSTRUCTION_SITE);
    let back = player.arrive(SUNSET);
    assert_eq!(back.exp_tracking.personal_best, best(10_800, None, false));
}

#[test]
fn with_the_bar_unreadable_or_paused_the_line_holds_like_the_other_figures() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    let held = best(7200, Some(5000), false);
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
fn the_overlay_gets_the_line_as_the_grind_tracker_renders_it() {
    let (mut player, _) = back_on_sunset_sky_after_a_best_of_7200();
    let beaten = serde_json::to_value(player.hunt(5 * MIN, 30)).unwrap();
    assert_eq!(
        beaten["expTracking"]["personalBest"],
        serde_json::json!({ "expPerHour": 7200, "first": false, "shareHundredths": 15_000, "newBest": true })
    );
}
