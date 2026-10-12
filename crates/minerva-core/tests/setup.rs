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
use minerva_core::{
    Action, Minerva, CurrentMap, GameWindow, MarkTarget, PanelControl, Rect, Request, ScreenRect, SelectionDirection,
    SetupReason, TestRead,
};

const KERNING: &str = "victoria-road__kerning-city--window";

/// The client area of a window sample shown at (812,110).
fn game(sample: &str) -> ScreenRect {
    let client = common::client_of(&common::load(sample));
    ScreenRect { x: 812, y: 110, width: client.width(), height: client.height() }
}

/// The game window as the shell reports it, with this client area on a 3440x1440 screen.
fn window(client: ScreenRect) -> Option<GameWindow> {
    let frame = ScreenRect { x: client.x - 1, y: client.y - 31, width: client.width + 2, height: client.height + 32 };
    Some(GameWindow { client, frame, screens: vec![ScreenRect { x: 0, y: 0, width: 3440, height: 1440 }] })
}

#[test]
fn a_fresh_install_asks_for_setup_and_reads_nothing_until_the_title_is_marked() {
    let mut minerva = Minerva::new(FakeSource::osms());
    let state = minerva.on_game_window(window(game(KERNING)));

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
    assert_eq!(state.title_region, None);
}

/// The player's mark around the two title lines, relative to the client area.
const MARK: Rect = Rect { x: 70, y: 40, width: 130, height: 48 };

#[test]
fn marking_the_title_starts_a_test_read_there_keeping_the_marked_corner_and_height() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));

    let state = minerva.act(Action::MarkTitleRegion(MARK));

    assert_eq!(state.setup.test_read, Some(TestRead::Reading));
    let region = state.title_region.expect("a region to capture");
    // The client is at (812,110) on screen.
    assert_eq!((region.x, region.y, region.height), (882, 150, 48));
    assert!(region.width >= MARK.width);
}

/// Mark `MARK` on a window sample and test-read the first frame captured from the
/// region the core asks for.
fn mark_and_test_read(minerva: &mut Minerva, sample: &str) -> Option<TestRead> {
    let game = game(sample);
    minerva.on_game_window(window(game));
    let region = minerva.act(Action::MarkTitleRegion(MARK)).title_region.expect("a region to capture");
    let client = common::client_of(&common::load(sample));
    minerva.on_frame(common::capture(&client, game, region, 0)).setup.test_read.clone()
}

fn recognised(id: &str, street_name: &str, map_name: &str) -> Option<TestRead> {
    let map = CurrentMap {
        id: id.into(),
        street_name: street_name.into(),
        map_name: map_name.into(),
        url: format!("https://meowdb.com/msclassic/maps/{id}"),
        mark: None,
    };
    Some(TestRead::Recognised { map })
}

#[test]
fn the_test_read_shows_the_real_map_the_title_snapped_to() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let read = mark_and_test_read(&mut minerva, KERNING);

    assert_eq!(read, recognised("010003000", "Victoria Road", "Kerning City"));
}

/// The minimap panel widens for long map names. A mark made around a short title still
/// captures a long one whole: the region extends to fit the longest name in the index.
#[test]
fn a_mark_narrower_than_a_long_map_name_still_reads_it_whole() {
    let mut minerva = Minerva::new(FakeSource::osms());

    // MARK spans client x 70..200; this map name runs to x ~423.
    let read = mark_and_test_read(&mut minerva, "victoria-road__construction-site-north-of-kerning-city--window");

    assert_eq!(read, recognised("010003080", "Victoria Road", "Construction Site North of Kerning City"));
}

/// ...and the world graphics beyond a narrow minimap (here, sky the colour of the
/// panel) aren't read as part of the title.
#[test]
fn the_extended_region_still_reads_a_short_title_on_a_narrow_minimap() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let read = mark_and_test_read(&mut minerva, "victoria-road__sunset-sky--window");

    assert_eq!(read, recognised("010003070", "Victoria Road", "Sunset Sky"));
}

#[test]
fn a_mark_that_misses_the_title_fails_its_test_read_and_can_be_marked_again() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));
    minerva.act(Action::MarkTitleRegion(MARK));

    let state = minerva.on_frame(common::black_frame(0));
    assert_eq!(state.setup.test_read, Some(TestRead::Failed));

    let state = minerva.act(Action::MarkTitleRegion(MARK));
    assert_eq!(state.setup.test_read, Some(TestRead::Reading));
}

/// What the shell does on restart: save the settings as JSON, load them into a new core.
fn restart(minerva: &Minerva) -> Minerva {
    let json = serde_json::to_string(&minerva.settings()).unwrap();
    let mut restarted = Minerva::new(FakeSource::osms());
    restarted.restore(serde_json::from_str(&json).unwrap());
    restarted
}

/// A core whose setup was finished on the Kerning City sample with `MARK`.
fn set_up() -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_and_test_read(&mut minerva, KERNING);
    minerva.act(Action::FinishSetup);
    minerva
}

#[test]
fn finishing_setup_after_a_good_test_read_keeps_reading_the_marked_region() {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_and_test_read(&mut minerva, KERNING);
    let marked = minerva.panel_state().title_region;

    let state = minerva.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.setup.test_read, None);
    assert_eq!(state.title_region, marked);
}

#[test]
fn setup_cannot_be_finished_until_the_test_read_recognises_a_map() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));
    minerva.act(Action::MarkTitleRegion(MARK));
    minerva.on_frame(common::black_frame(0));

    let state = minerva.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
    assert_eq!(state.setup.test_read, Some(TestRead::Failed));
}

#[test]
fn the_title_region_survives_a_restart() {
    let minerva = set_up();

    let mut restarted = restart(&minerva);
    let state = restarted.on_game_window(window(game(KERNING)));

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.title_region, minerva.panel_state().title_region);
}

#[test]
fn the_title_region_moves_with_the_game_window() {
    let mut minerva = set_up();
    let before = minerva.panel_state().title_region.unwrap();

    let moved = ScreenRect { x: 100, y: 50, ..game(KERNING) };
    let state = minerva.on_game_window(window(moved));

    assert_eq!(state.setup.needed, None);
    // Still (70,40) from the client's top-left corner.
    assert_eq!(state.title_region, Some(ScreenRect { x: 170, y: 90, ..before }));
}

#[test]
fn a_new_game_resolution_asks_for_setup_again_and_stops_reading_the_old_spot() {
    let mut minerva = set_up();

    let resized = ScreenRect { width: 1280, height: 720, ..game(KERNING) };
    let state = minerva.on_game_window(window(resized));

    assert_eq!(state.setup.needed, Some(SetupReason::ResolutionChanged));
    assert_eq!(state.title_region, None);
}

#[test]
fn going_back_to_the_resolution_of_setup_reads_the_title_region_again() {
    let mut minerva = set_up();
    let region = minerva.panel_state().title_region;
    minerva.on_game_window(window(ScreenRect { width: 1280, height: 720, ..game(KERNING) }));

    let state = minerva.on_game_window(window(game(KERNING)));

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.title_region, region);
}

#[test]
fn setup_redone_at_the_new_resolution_is_kept_for_it() {
    let mut minerva = set_up();
    // The Kerning City client, 200 px narrower: the title stays put at the top-left.
    let narrower = ScreenRect { width: game(KERNING).width - 200, ..game(KERNING) };
    minerva.on_game_window(window(narrower));
    let state = minerva.act(Action::MarkTitleRegion(MARK));
    let region = state.title_region.unwrap();
    let client = common::client_of(&common::load(KERNING));
    minerva.on_frame(common::capture(&client, narrower, region, 1000));

    let state = minerva.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.title_region, Some(region));
}

#[test]
fn asking_to_set_up_again_keeps_reading_the_old_region_until_a_new_one_is_finished() {
    let mut minerva = set_up();
    let old = minerva.panel_state().title_region;

    let state = minerva.act(Action::RequestSetup);
    assert_eq!(state.setup.needed, Some(SetupReason::Requested));
    assert_eq!(state.title_region, old);

    // A taller mark this time, around the same title.
    let taller = Rect { y: 36, height: 56, ..MARK };
    let region = minerva.act(Action::MarkTitleRegion(taller)).title_region.unwrap();
    let client = common::client_of(&common::load(KERNING));
    minerva.on_frame(common::capture(&client, game(KERNING), region, 1000));
    let state = minerva.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, None);
    assert_eq!((region.y, region.height), (146, 56));
    assert_eq!(state.title_region, Some(region));
}

#[test]
fn cancelling_a_requested_setup_goes_back_to_the_saved_region() {
    let mut minerva = set_up();
    let old = minerva.panel_state().title_region;
    minerva.act(Action::RequestSetup);
    minerva.act(Action::MarkTitleRegion(Rect { x: 300, ..MARK }));

    let state = minerva.act(Action::CancelSetup);

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.setup.test_read, None);
    assert_eq!(state.title_region, old);
}

#[test]
fn cancelling_first_run_setup_leaves_it_still_needed() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));
    minerva.act(Action::MarkTitleRegion(MARK));

    let state = minerva.act(Action::CancelSetup);

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
    assert_eq!(state.setup.test_read, None);
    assert_eq!(state.title_region, None);
}

#[test]
fn a_game_window_picked_by_hand_is_looked_for_from_then_on_even_after_a_restart() {
    let mut minerva = Minerva::new(FakeSource::osms());
    assert_eq!(minerva.panel_state().game_window_title, "MapleStory");

    let state = minerva.pick_game_window("MapleStory Classic".into());
    assert_eq!(state.game_window_title, "MapleStory Classic");

    assert_eq!(restart(&minerva).panel_state().game_window_title, "MapleStory Classic");
}

#[test]
fn an_empty_settings_file_has_a_re_setup_hotkey_and_needs_setup() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(serde_json::from_str("{}").unwrap());

    let state = minerva.on_game_window(window(game(KERNING)));

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
    assert_eq!(state.hotkeys.setup, "ctrl+shift+s");
}

// The setup window: the core says when it shows, and what closing it does.

#[test]
fn the_setup_window_opens_on_first_run_and_closing_it_then_quits() {
    let mut minerva = Minerva::new(FakeSource::osms());
    let state = minerva.on_game_window(window(game(KERNING)));
    assert!(state.setup.window_open);
    assert!(!state.quit);

    // Nothing can be read without a Title Region.
    let state = minerva.act(Action::CancelSetup);
    assert!(state.quit);
}

#[test]
fn closing_a_requested_setup_just_closes_it() {
    let mut minerva = set_up();
    assert!(!minerva.panel_state().setup.window_open);
    assert!(minerva.act(Action::RequestSetup).setup.window_open);

    let state = minerva.act(Action::CancelSetup);

    assert!(!state.setup.window_open);
    assert!(!state.quit);
}

#[test]
fn finishing_setup_closes_its_window() {
    let minerva = set_up();

    assert!(!minerva.panel_state().setup.window_open);
}

#[test]
fn a_new_game_resolution_opens_setup_and_closing_it_leaves_it_needed_without_quitting() {
    let mut minerva = set_up();
    let resized = ScreenRect { width: 1280, height: 720, ..game(KERNING) };
    assert!(minerva.on_game_window(window(resized)).setup.window_open);

    let state = minerva.act(Action::CancelSetup);
    assert!(!state.setup.window_open);
    assert!(!state.quit);
    assert_eq!(state.setup.needed, Some(SetupReason::ResolutionChanged));

    // The re-setup hotkey opens it again.
    assert!(minerva.act(Action::RequestSetup).setup.window_open);
}

#[test]
fn a_recognised_test_read_credits_osms_for_the_names_it_shows() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));
    assert_eq!(minerva.act(Action::MarkTitleRegion(MARK)).setup.credit, None);

    mark_and_test_read(&mut minerva, KERNING);

    assert_eq!(minerva.panel_state().setup.credit, Some("Map names from OSMS Data Explorer"));
}

#[test]
fn about_and_credits_opens_the_minerva_site_in_the_browser_and_changes_nothing_else() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));
    let before = minerva.panel_state().clone();

    assert_eq!(
        minerva.press(PanelControl::OpenSite),
        Some(Request::OpenInBrowser { url: "https://itechify.github.io/minerva/".into() })
    );
    assert_eq!(minerva.panel_state(), &before);
}

/// The width of the region the core captures after marking `MARK` and reading one frame
/// (which loads the map list).
fn region_width(source: FakeSource) -> u32 {
    let mut minerva = Minerva::new(source);
    mark_and_test_read(&mut minerva, KERNING);
    minerva.panel_state().title_region.expect("a region to capture").width
}

#[test]
fn the_region_extends_only_as_far_as_the_longest_name_in_the_map_list_needs() {
    let mut short_names = common::osms::map_names();
    short_names.maps.retain(|m| m.name.trim().len() <= 12);

    assert!(region_width(FakeSource::with_world(short_names)) < region_width(FakeSource::osms()));
}

#[test]
fn the_region_never_extends_past_the_game_window() {
    let mut minerva = Minerva::new(FakeSource::osms());
    // A small 400x300 window: the region can reach at most 330 px right of x 70.
    minerva.on_game_window(window(ScreenRect { x: 812, y: 110, width: 400, height: 300 }));

    let state = minerva.act(Action::MarkTitleRegion(MARK));

    assert_eq!(state.title_region, Some(ScreenRect { x: 882, y: 150, width: 330, height: 48 }));
}

#[test]
fn keyboard_marking_starts_with_a_visible_box_without_committing_a_read() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));

    let state = minerva.act(Action::BeginSelection(MarkTarget::Title));

    assert_eq!(state.setup.selection, Some(Rect { x: 0, y: 16, width: 560, height: 96 }));
    assert_eq!(state.setup.test_read, None);
    assert_eq!(state.title_region, None);
}

#[test]
fn keyboard_marking_can_move_resize_and_test_the_previous_mark() {
    let mut minerva = set_up();
    minerva.act(Action::RequestSetup);
    assert_eq!(minerva.act(Action::BeginSelection(MarkTarget::Title)).setup.selection, Some(MARK));
    for (direction, resize, coarse) in [
        (SelectionDirection::Right, false, true),
        (SelectionDirection::Down, false, false),
        (SelectionDirection::Left, true, true),
        (SelectionDirection::Down, true, true),
    ] {
        minerva.act(Action::AdjustSelection { direction, resize, coarse });
    }
    assert_eq!(minerva.panel_state().setup.selection, Some(Rect { x: 80, y: 41, width: 120, height: 58 }));

    let state = minerva.act(Action::ConfirmSelection);

    assert_eq!(state.setup.selection, None);
    assert_eq!(state.setup.test_read, Some(TestRead::Reading));
    let region = state.title_region.unwrap();
    assert_eq!((region.x, region.y, region.height), (892, 151, 58));
}

#[test]
fn keyboard_selection_stays_inside_the_client_and_never_shrinks_below_eight_pixels() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(ScreenRect { x: 100, y: 50, width: 40, height: 30 }));
    minerva.act(Action::BeginSelection(MarkTarget::Title));
    for direction in [SelectionDirection::Left, SelectionDirection::Up] {
        for _ in 0..10 {
            minerva.act(Action::AdjustSelection { direction, resize: true, coarse: true });
        }
    }
    for direction in [SelectionDirection::Right, SelectionDirection::Down] {
        for _ in 0..10 {
            let state = minerva.act(Action::AdjustSelection { direction, resize: false, coarse: true });
            let r = state.setup.selection.unwrap();
            assert!(r.x + r.width <= 40 && r.y + r.height <= 30);
        }
    }
    assert_eq!(minerva.panel_state().setup.selection, Some(Rect { x: 32, y: 22, width: 8, height: 8 }));
    for direction in [SelectionDirection::Right, SelectionDirection::Down] {
        minerva.act(Action::AdjustSelection { direction, resize: true, coarse: true });
    }
    assert_eq!(minerva.panel_state().setup.selection, Some(Rect { x: 32, y: 22, width: 8, height: 8 }));
    for direction in [SelectionDirection::Left, SelectionDirection::Up] {
        for _ in 0..10 {
            minerva.act(Action::AdjustSelection { direction, resize: false, coarse: true });
        }
    }
    assert_eq!(minerva.panel_state().setup.selection, Some(Rect { x: 0, y: 0, width: 8, height: 8 }));
}

#[test]
fn cancelling_keyboard_marking_keeps_the_existing_test_read() {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_and_test_read(&mut minerva, KERNING);
    let read = minerva.panel_state().setup.test_read.clone();
    let region = minerva.panel_state().title_region;
    minerva.act(Action::BeginSelection(MarkTarget::Title));
    minerva.act(Action::AdjustSelection { direction: SelectionDirection::Right, resize: false, coarse: true });

    let state = minerva.act(Action::CancelSelection);

    assert_eq!(state.setup.selection, None);
    assert_eq!(state.setup.test_read, read);
    assert_eq!(state.title_region, region);
    assert!(state.setup.window_open);
}

#[test]
fn a_selection_is_discarded_if_the_game_disappears_or_changes_size() {
    let mut minerva = set_up();
    minerva.act(Action::RequestSetup);
    minerva.act(Action::BeginSelection(MarkTarget::Title));
    assert_eq!(minerva.on_game_window(None).setup.selection, None);
    assert_eq!(minerva.act(Action::ConfirmSelection).setup.test_read, None);
    minerva.on_game_window(window(game(KERNING)));
    minerva.act(Action::BeginSelection(MarkTarget::Title));

    let state = minerva.on_game_window(window(ScreenRect { width: 1280, height: 720, ..game(KERNING) }));

    assert_eq!(state.setup.selection, None);
    assert_eq!(state.setup.needed, Some(SetupReason::ResolutionChanged));
    assert_eq!(state.title_region, None);
}

#[test]
fn a_pointer_mark_is_bounded_before_it_is_test_read() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(ScreenRect { x: 100, y: 50, width: 40, height: 30 }));

    let state = minerva.act(Action::MarkTitleRegion(Rect { x: 999, y: 999, width: 100, height: 4 }));

    assert_eq!(state.setup.test_read, Some(TestRead::Reading));
    assert_eq!(state.title_region, Some(ScreenRect { x: 100, y: 72, width: 40, height: 8 }));
}
