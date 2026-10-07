//! The player drives MapleCompass with global hotkeys: Win+Alt+letter, because
//! MapleStory players commonly bind Ctrl and Shift (and a registered hotkey never
//! reaches the game). The shell registers what the panel state lists and hands every
//! press to the core, which decides what it does.

mod common;

use common::{FakeSource, arrive};
use compass_core::{
    Compass, ExpBar, ExpBarReading, ExpBarRegion, ExpTracking, GameWindow, Hotkeys, OpenInBrowser, PlacementMode, Rect, ScreenRect,
    Settings, SetupReason, Size, TitleRegion, TrackingState,
};

const SUNSET: &str = "victoria-road__sunset-sky--window";

fn win_alt(letter: &str) -> String {
    format!("super+alt+{letter}")
}

#[test]
fn the_hotkeys_are_win_alt_letters_with_no_ctrl_or_shift() {
    let compass = Compass::new(FakeSource::meowdb());

    assert_eq!(
        compass.panel_state().hotkeys,
        Hotkeys {
            toggle_panel: win_alt("c"),
            edit_mode: win_alt("e"),
            cycle_placement: win_alt("p"),
            open_meowdb_page: win_alt("o"),
            setup: win_alt("s"),
            choose_candidate: win_alt("n"),
            reset_session: win_alt("x"),
        }
    );
}

/// Settings saved by an earlier version, with its Ctrl+Shift hotkeys.
const CTRL_SHIFT_SETTINGS: &str = r#"{
  "version": 1,
  "expanded": true,
  "placement": "dockedLeft",
  "hotkeys": {
    "togglePanel": "ctrl+shift+m",
    "editMode": "ctrl+shift+e",
    "cyclePlacement": "ctrl+shift+d",
    "openMeowdbPage": "ctrl+shift+o",
    "setup": "ctrl+shift+s"
  }
}"#;

#[test]
fn old_ctrl_shift_hotkeys_are_replaced_by_the_win_alt_ones_and_the_rest_is_kept() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.restore(serde_json::from_str(CTRL_SHIFT_SETTINGS).unwrap());

    assert_eq!(state.hotkeys, Compass::new(FakeSource::meowdb()).panel_state().hotkeys);
    assert_eq!(state.placement_mode, PlacementMode::DockedLeft);
    assert!(compass.on_hotkey("ctrl+shift+m").is_none());
    assert!(compass.panel_state().expanded);
}

#[test]
fn win_alt_c_collapses_the_panel_and_expands_it_again() {
    let mut compass = Compass::new(FakeSource::meowdb());

    compass.on_hotkey("super+alt+c");
    assert!(!compass.panel_state().expanded);
    compass.on_hotkey("super+alt+c");
    assert!(compass.panel_state().expanded);
}

#[test]
fn win_alt_e_enters_and_leaves_edit_mode_and_win_alt_p_cycles_the_placement() {
    let mut compass = Compass::new(FakeSource::meowdb());

    compass.on_hotkey("super+alt+e");
    assert!(compass.panel_state().edit_mode);
    compass.on_hotkey("super+alt+p");
    assert_eq!(compass.panel_state().placement_mode, PlacementMode::DockedRight);
    compass.on_hotkey("super+alt+e");
    assert!(!compass.panel_state().edit_mode);
}

#[test]
fn win_alt_o_opens_the_shown_maps_meowdb_page_and_nothing_without_a_map() {
    let mut compass = Compass::new(FakeSource::meowdb());
    assert_eq!(compass.on_hotkey("super+alt+o"), None);

    arrive(&mut compass, SUNSET, 0);

    assert_eq!(
        compass.on_hotkey("super+alt+o"),
        Some(OpenInBrowser { url: "https://meowdb.com/msclassic/maps/010003070".into() })
    );
}

#[test]
fn win_alt_s_opens_setup_again() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.restore(Settings { title_region: Some(common::kerning_title_region()), ..Settings::default() });
    assert!(!compass.panel_state().setup.window_open);

    compass.on_hotkey("super+alt+s");

    let state = compass.panel_state();
    assert_eq!(state.setup.needed, Some(SetupReason::Requested));
    assert!(state.setup.window_open);
}

#[test]
fn hotkeys_the_player_rebound_are_the_ones_that_act() {
    let mut compass = Compass::new(FakeSource::meowdb());
    let hotkeys = Hotkeys { toggle_panel: "super+alt+j".into(), ..Hotkeys::default() };
    compass.restore(Settings { hotkeys, ..Settings::default() });

    compass.on_hotkey("super+alt+c");
    assert!(compass.panel_state().expanded);
    compass.on_hotkey("Super+Alt+J");
    assert!(!compass.panel_state().expanded);
}

#[test]
fn a_hotkey_another_app_has_taken_is_shown_as_unavailable() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.hotkeys_unavailable(vec!["super+alt+e".into()]);

    assert_eq!(state.unavailable_hotkeys, ["super+alt+e"]);
}

/// A core with EXP tracking on (both regions kept, on a 1920x1200 game window) and
/// these hotkeys.
fn tracking_with(hotkeys: Hotkeys) -> Compass {
    let client = Size { width: 1920, height: 1200 };
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.restore(Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client }),
        exp_bar_region: Some(ExpBarRegion { marked: Rect { x: 1005, y: 1144, width: 240, height: 32 }, client }),
        hotkeys,
        ..Settings::default()
    });
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    compass.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    compass
}

/// Start a Session: the EXP bar reads `160 [3.81%]`, then `266 [6.33%]`.
fn start_session(compass: &mut Compass) {
    for (at_ms, exp, percent_hundredths) in [(0, 160, 381), (10_000, 266, 633)] {
        compass.on_exp_bar_reading(ExpBarReading { bar: Some(ExpBar { exp, percent_hundredths }), at_ms });
    }
    assert_eq!(compass.panel_state().exp_tracking.state, TrackingState::WarmingUp);
}

fn waiting() -> ExpTracking {
    ExpTracking { state: TrackingState::Waiting, ..ExpTracking::default() }
}

#[test]
fn win_alt_x_resets_the_session_back_to_waiting() {
    let mut compass = tracking_with(Hotkeys::default());
    start_session(&mut compass);

    assert_eq!(compass.on_hotkey("super+alt+x"), None);

    assert_eq!(compass.panel_state().exp_tracking, waiting());
}

/// Settings saved before the reset hotkey existed, with a hotkey the player rebound.
const SETTINGS_WITHOUT_RESET_SESSION: &str = r#"{
  "version": 3,
  "hotkeys": {
    "togglePanel": "super+alt+j",
    "editMode": "super+alt+e",
    "cyclePlacement": "super+alt+p",
    "openMeowdbPage": "super+alt+o",
    "setup": "super+alt+s",
    "chooseCandidate": "super+alt+n"
  }
}"#;

#[test]
fn settings_from_before_the_reset_hotkey_get_win_alt_x_and_keep_the_rest() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.restore(serde_json::from_str(SETTINGS_WITHOUT_RESET_SESSION).unwrap());

    assert_eq!(state.hotkeys.reset_session, win_alt("x"));
    assert_eq!(state.hotkeys.toggle_panel, win_alt("j"));
}

#[test]
fn a_rebound_reset_hotkey_is_kept_and_is_the_one_that_resets() {
    let json = SETTINGS_WITHOUT_RESET_SESSION.replace(r#""chooseCandidate""#, r#""resetSession": "super+alt+z", "chooseCandidate""#);
    let saved: Settings = serde_json::from_str(&json).unwrap();
    let mut compass = tracking_with(saved.hotkeys);
    assert_eq!(compass.panel_state().hotkeys.reset_session, win_alt("z"));
    start_session(&mut compass);

    compass.on_hotkey("super+alt+x");
    assert_eq!(compass.panel_state().exp_tracking.state, TrackingState::WarmingUp);
    compass.on_hotkey("Super+Alt+Z");
    assert_eq!(compass.panel_state().exp_tracking, waiting());
}
