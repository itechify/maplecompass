//! Personal Bests outlive the launch: the core hands the shell every map's Visit bests
//! as plain data, the Visit under way's included, and takes them back at the next
//! launch, the same way as the saved world. Closing MapleCompass ends the Visit.

mod common;

use compass_core::{
    BESTS_VERSION, Compass, ExpBar, ExpBarReading, ExpBarRegion, GameWindow, PanelControl, PanelState,
    PersonalBestLine, Rect, SavedBests, ScreenRect, Settings, Size, TitleRegion,
};
use common::{FakeSource, index_with_title};

const S: u64 = 1000;
const MIN: u64 = 60 * S;
const DAY: u64 = 24 * 60 * MIN;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const ROCKY_ROAD: &str = "victoria-road__rocky-road-i--title-region";
const SUNSET_SKY_ID: &str = "010003070";
const CONSTRUCTION_SITE_ID: &str = "010003080";

/// The wall clock at capture time 0 of the first launch: 9 Oct 2026, 12:00 UTC.
const WALL_AT_START: u64 = 1_791_547_200_000;

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

/// The bests a launch hands out, as the shell would save them (JSON).
fn handed_out(compass: &Compass) -> SavedBests {
    let json = serde_json::to_string(&compass.bests()).unwrap();
    serde_json::from_str(&json).unwrap()
}

/// The next launch, its shell restoring `bests` at startup, its wall clock at
/// `wall_at_start` at capture time 0: the player's Session under way, on a map the
/// panel doesn't know.
fn relaunch(bests: SavedBests, wall_at_start: u64) -> Player {
    let mut compass = tracking();
    compass.restore_bests(bests);
    Player::start(compass, wall_at_start)
}

/// The player hunting on a 20,000 EXP level, the EXP bar read every 10 s, the shell
/// telling the wall clock before each reading. Readings at whole half minutes of capture
/// time are where the Grind Tracker's figures refresh.
struct Player {
    compass: Compass,
    exp: u64,
    at: u64,
    wall_at_start: u64,
    /// The shell's saves, if the test follows them.
    shell: Option<Shell>,
}

impl Player {
    /// The bar read at 1,000 [5.00%] at 0 s, then 10 EXP more every 10 s on a map the
    /// panel doesn't know, up to 30 s: the Session has started.
    fn start(compass: Compass, wall_at_start: u64) -> Self {
        Self::begin(compass, wall_at_start, None)
    }

    /// The first launch.
    fn first_launch() -> Self {
        Self::start(tracking(), WALL_AT_START)
    }

    /// The first launch, following the shell's saves from the start.
    fn saved_by_the_shell(compass: Compass) -> Self {
        let shell = Shell::new(&compass);
        Self::begin(compass, WALL_AT_START, Some(shell))
    }

    fn begin(compass: Compass, wall_at_start: u64, shell: Option<Shell>) -> Self {
        let mut player = Player { compass, exp: 1000, at: 0, wall_at_start, shell };
        player.read();
        player.hunt(30 * S, 10);
        player
    }

    /// The shell's loop, after an input.
    fn shell_loop(&mut self, input: &str) {
        if let Some(shell) = &mut self.shell {
            shell.after(&self.compass, &format!("{input} at {} ms", self.at));
        }
    }

    /// How many times the shell has saved the bests.
    fn saves(&self) -> u32 {
        self.shell.as_ref().map_or(0, |shell| shell.saves)
    }

    /// The wall clock now.
    fn wall_clock(&self) -> u64 {
        self.wall_at_start + self.at
    }

    fn read(&mut self) -> PanelState {
        self.compass.on_wall_clock(self.wall_clock());
        // The percentage the game shows on a 20,000 EXP level, rounded to hundredths.
        let percent_hundredths = ((self.exp * 20_000 + 20_000) / (2 * 20_000)) as u32;
        let bar = Some(ExpBar { exp: self.exp, percent_hundredths });
        let state = self.compass.on_exp_bar_reading(ExpBarReading { bar, at_ms: self.at }).clone();
        self.shell_loop("a reading");
        state
    }

    /// The panel confirms the map of a sample: a Visit starts now.
    fn arrive(&mut self, sample: &str) -> PanelState {
        let state = common::arrive(&mut self.compass, sample, self.at);
        self.shell_loop(sample);
        state
    }

    /// The bar doesn't read whole for `ms` more, a frame every 10 s.
    fn unread(&mut self, ms: u64) {
        for _ in 0..ms / (10 * S) {
            self.at += 10 * S;
            self.compass.on_exp_bar_reading(ExpBarReading { bar: None, at_ms: self.at });
            self.shell_loop("the bar unread");
        }
    }

    /// The player presses a control on the panel or the Grind Tracker.
    fn press(&mut self, control: PanelControl) {
        self.compass.press(control);
        self.shell_loop(&format!("{control:?}"));
    }

    /// Choose the next of the Candidates with the hotkey.
    fn choose_next(&mut self) -> PanelState {
        self.compass.on_hotkey("ctrl+shift+n");
        self.shell_loop("choosing the next Candidate");
        self.compass.panel_state().clone()
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

/// The shell's saves, as its loop makes them: whenever the bests' revision has changed
/// since the last.
struct Shell {
    revision: u64,
    saved: SavedBests,
    saves: u32,
}

impl Shell {
    /// Just after startup, with what was restored.
    fn new(compass: &Compass) -> Self {
        Self { revision: compass.bests_revision(), saved: compass.bests(), saves: 0 }
    }

    /// The shell's loop after `input`: it saves if the revision changed, which it must
    /// exactly when the bests did.
    fn after(&mut self, compass: &Compass, input: &str) {
        let (revision, bests) = (compass.bests_revision(), compass.bests());
        let changed = revision != self.revision;
        assert_eq!(changed, bests != self.saved, "{input}: saved {:?}, now {bests:?}", self.saved);
        if changed {
            *self = Self { revision, saved: bests, saves: self.saves + 1 };
        }
    }
}

fn line(state: &PanelState) -> Option<PersonalBestLine> {
    state.exp_tracking.personal_best
}

/// The line once earlier Visits had a best of `exp_per_hour` here, set at `set_at` by
/// the wall clock: "Your best EXP/h here: …", or with a share, "N% of your best EXP/h
/// here (…)".
fn earlier(exp_per_hour: u64, set_at: u64, share_hundredths: Option<i64>) -> Option<PersonalBestLine> {
    Some(PersonalBestLine {
        exp_per_hour,
        set_at_unix_ms: Some(set_at),
        first: false,
        share_hundredths,
        new_best: None,
    })
}

#[test]
fn bests_handed_out_by_one_launch_show_the_same_line_and_hover_in_the_next() {
    // A first Visit to Sunset Sky at 7,200 EXP/h, its best set 5 minutes in; 2 minutes on
    // the Construction Site, and back.
    let mut first = Player::first_launch();
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
    let mut first = Player::first_launch();
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
    let mut first = Player::first_launch();
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

/// The Construction Site and Sunset Sky sharing the Kerning City sample's title: the
/// Construction Site, first in the map index, shows first.
fn sharing_a_title() -> FakeSource {
    FakeSource::with_index(index_with_title("Victoria Road", "Kerning City", &[CONSTRUCTION_SITE_ID, SUNSET_SKY_ID]))
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

    // With no rate since a reset, choosing Sunset Sky gives it no best of its own, but
    // the Visit's best goes with the choice; choosing the Construction Site brings it back.
    player.press(PanelControl::ResetSession);
    player.choose_next();
    assert_eq!((player.saves(), site_rates(&player)), (joined + 1, vec![14_400]));
    player.choose_next();
    assert_eq!((player.saves(), site_rates(&player)), (joined + 2, vec![14_400, 7200]));
}
