//! The Personal Best (GLOSSARY): the Grind Tracker's Last 5 min EXP/h measured against
//! the best the player has had on the map they're on. A Visit's best is the highest
//! Last 5 min EXP/h shown at a refresh over a full 5 minutes of active play on the map;
//! the map's Personal Best is the best of its Visits'.

mod common;

use compass_core::{
    Compass, ExpBar, ExpBarReading, ExpBarRegion, GameWindow, PanelState, PersonalBestLine, Rect, ScreenRect,
    Settings, Size, TitleRegion,
};
use common::FakeSource;

const S: u64 = 1000;
const MIN: u64 = 60 * S;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";

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
