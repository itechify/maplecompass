//! First-run setup: the player marks the Title Region around the minimap title, a test
//! read shows the real map it snapped to, and the region is kept relative to the game
//! window until the game's resolution changes or the player asks to set up again.
//!
//! Worked example used throughout: the Kerning City window sample, whose client area
//! (1916x1197 physical px) sits at (812,110) on screen. Its title lines span client
//! x 75..~200, y 45..80 on a light-blue panel band at y 32..99; the player marks
//! (70,40) 130x48 around them.

mod common;

use common::FakeSource;
use compass_core::{Action, Compass, CurrentMap, Rect, ScreenRect, SetupReason, TestRead};

const KERNING: &str = "victoria-road__kerning-city--window";

/// The game window as the shell reports it: the sample's client area at (812,110).
fn game(sample: &str) -> ScreenRect {
    let client = common::client_of(&common::load(sample));
    ScreenRect { x: 812, y: 110, width: client.width(), height: client.height() }
}

#[test]
fn a_fresh_install_asks_for_setup_and_reads_nothing_until_the_title_is_marked() {
    let mut compass = Compass::new(FakeSource::meowdb());
    let state = compass.on_game_window(Some(game(KERNING)));

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
    assert_eq!(state.title_region, None);
}

/// The player's mark around the two title lines, relative to the client area.
const MARK: Rect = Rect { x: 70, y: 40, width: 130, height: 48 };

#[test]
fn marking_the_title_starts_a_test_read_there_keeping_the_marked_corner_and_height() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_game_window(Some(game(KERNING)));

    let state = compass.act(Action::MarkTitleRegion(MARK));

    assert_eq!(state.setup.test_read, Some(TestRead::Reading));
    let region = state.title_region.expect("a region to capture");
    // The client is at (812,110) on screen.
    assert_eq!((region.x, region.y, region.height), (882, 150, 48));
    assert!(region.width >= MARK.width);
}

/// Mark `MARK` on a window sample and test-read the first frame captured from the
/// region the core asks for.
fn mark_and_test_read(compass: &mut Compass, sample: &str) -> Option<TestRead> {
    let game = game(sample);
    compass.on_game_window(Some(game));
    let region = compass.act(Action::MarkTitleRegion(MARK)).title_region.expect("a region to capture");
    let client = common::client_of(&common::load(sample));
    compass.on_frame(common::capture(&client, game, region, 0)).setup.test_read.clone()
}

fn recognised(id: &str, street_name: &str, map_name: &str) -> Option<TestRead> {
    let map = CurrentMap { id: id.into(), street_name: street_name.into(), map_name: map_name.into() };
    Some(TestRead::Recognised { map })
}

#[test]
fn the_test_read_shows_the_real_map_the_title_snapped_to() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let read = mark_and_test_read(&mut compass, KERNING);

    assert_eq!(read, recognised("010003000", "Victoria Road", "Kerning City"));
}

/// The minimap panel widens for long map names. A mark made around a short title still
/// captures a long one whole: the region extends to fit the longest name in the index.
#[test]
fn a_mark_narrower_than_a_long_map_name_still_reads_it_whole() {
    let mut compass = Compass::new(FakeSource::meowdb());

    // MARK spans client x 70..200; this map name runs to x ~423.
    let read = mark_and_test_read(&mut compass, "victoria-road__construction-site-north-of-kerning-city--window");

    assert_eq!(read, recognised("010003080", "Victoria Road", "Construction Site North of Kerning City"));
}

/// ...and the world graphics beyond a narrow minimap (here, sky the colour of the
/// panel) aren't read as part of the title.
#[test]
fn the_extended_region_still_reads_a_short_title_on_a_narrow_minimap() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let read = mark_and_test_read(&mut compass, "victoria-road__sunset-sky--window");

    assert_eq!(read, recognised("010003070", "Victoria Road", "Sunset Sky"));
}

#[test]
fn a_mark_that_misses_the_title_fails_its_test_read_and_can_be_marked_again() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_game_window(Some(game(KERNING)));
    compass.act(Action::MarkTitleRegion(MARK));

    let state = compass.on_frame(common::black_frame(0));
    assert_eq!(state.setup.test_read, Some(TestRead::Failed));

    let state = compass.act(Action::MarkTitleRegion(MARK));
    assert_eq!(state.setup.test_read, Some(TestRead::Reading));
}

/// What the shell does on restart: save the settings as JSON, load them into a new core.
fn restart(compass: &Compass) -> Compass {
    let json = serde_json::to_string(&compass.settings()).unwrap();
    let mut restarted = Compass::new(FakeSource::meowdb());
    restarted.restore(serde_json::from_str(&json).unwrap());
    restarted
}

/// A core whose setup was finished on the Kerning City sample with `MARK`.
fn set_up() -> Compass {
    let mut compass = Compass::new(FakeSource::meowdb());
    mark_and_test_read(&mut compass, KERNING);
    compass.act(Action::FinishSetup);
    compass
}

#[test]
fn finishing_setup_after_a_good_test_read_keeps_reading_the_marked_region() {
    let mut compass = Compass::new(FakeSource::meowdb());
    mark_and_test_read(&mut compass, KERNING);
    let marked = compass.panel_state().title_region;

    let state = compass.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.setup.test_read, None);
    assert_eq!(state.title_region, marked);
}

#[test]
fn setup_cannot_be_finished_until_the_test_read_recognises_a_map() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_game_window(Some(game(KERNING)));
    compass.act(Action::MarkTitleRegion(MARK));
    compass.on_frame(common::black_frame(0));

    let state = compass.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
    assert_eq!(state.setup.test_read, Some(TestRead::Failed));
}

#[test]
fn the_title_region_survives_a_restart() {
    let compass = set_up();

    let mut restarted = restart(&compass);
    let state = restarted.on_game_window(Some(game(KERNING)));

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.title_region, compass.panel_state().title_region);
}

/// The width of the region the core captures after marking `MARK` and reading one frame
/// (which loads the map index).
fn region_width(source: FakeSource) -> u32 {
    let mut compass = Compass::new(source);
    mark_and_test_read(&mut compass, KERNING);
    compass.panel_state().title_region.expect("a region to capture").width
}

#[test]
fn the_region_extends_only_as_far_as_the_longest_name_in_the_index_needs() {
    let short_names = FakeSource::with_index(
        common::saved_index().into_iter().filter(|m| m.map_name.len() <= 12).collect(),
    );

    assert!(region_width(short_names) < region_width(FakeSource::meowdb()));
}

#[test]
fn the_region_never_extends_past_the_game_window() {
    let mut compass = Compass::new(FakeSource::meowdb());
    // A small 400x300 window: the region can reach at most 330 px right of x 70.
    compass.on_game_window(Some(ScreenRect { x: 812, y: 110, width: 400, height: 300 }));

    let state = compass.act(Action::MarkTitleRegion(MARK));

    assert_eq!(state.title_region, Some(ScreenRect { x: 882, y: 150, width: 330, height: 48 }));
}
