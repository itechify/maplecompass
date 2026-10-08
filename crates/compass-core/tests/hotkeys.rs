//! The player drives MapleCompass with global hotkeys: Ctrl+Shift+letter, because
//! Windows and other apps already hold many Win+Alt letters (Win+Alt+F among them).
//! They are live only while the game or MapleCompass is in front, so other apps keep
//! their own Ctrl+Shift shortcuts. The shell registers what the panel state lists while
//! it says they are live, and hands every press to the core, which decides what it does.

mod common;

use common::{FakeSource, arrive};
use compass_core::{
    Compass, ExpBar, ExpBarReading, ExpBarRegion, ExpTracking, Foreground, GameWindow, Hotkeys, PanelControl,
    PlacementMode, Rect, Request, ScreenRect, Settings, SetupReason, Size, TitleRegion, TrackingState,
};

const SUNSET: &str = "victoria-road__sunset-sky--window";

fn ctrl_shift(letter: &str) -> String {
    format!("ctrl+shift+{letter}")
}

#[test]
fn the_hotkeys_are_ctrl_shift_letters() {
    let compass = Compass::new(FakeSource::meowdb());

    assert_eq!(
        compass.panel_state().hotkeys,
        Hotkeys {
            toggle_panel: ctrl_shift("c"),
            cycle_placement: ctrl_shift("p"),
            open_meowdb_page: ctrl_shift("m"),
            setup: ctrl_shift("s"),
            choose_candidate: ctrl_shift("n"),
            search: ctrl_shift("f"),
            reset_session: ctrl_shift("r"),
        }
    );
}

/// Settings saved by version 2, with its Win+Alt hotkeys.
const WIN_ALT_SETTINGS: &str = r#"{
  "version": 2,
  "expanded": true,
  "placementMode": "dockedLeft",
  "hotkeys": {
    "togglePanel": "super+alt+c",
    "editMode": "super+alt+e",
    "cyclePlacement": "super+alt+p",
    "openMeowdbPage": "super+alt+o",
    "setup": "super+alt+s",
    "chooseCandidate": "super+alt+n",
    "search": "super+alt+f"
  }
}"#;

#[test]
fn win_alt_hotkeys_from_version_2_are_replaced_by_the_ctrl_shift_ones_and_the_rest_is_kept() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.restore(serde_json::from_str(WIN_ALT_SETTINGS).unwrap());

    assert_eq!(state.hotkeys, Hotkeys::default());
    assert_eq!(state.placement_mode, PlacementMode::DockedLeft);
    assert!(compass.on_hotkey("super+alt+c").is_none());
    assert!(compass.panel_state().expanded);
    compass.on_hotkey("ctrl+shift+c");
    assert!(!compass.panel_state().expanded);
    assert_eq!(compass.settings().hotkeys, Hotkeys::default(), "and saved as Ctrl+Shift");
}

/// Settings saved as version 3 by a build of the EXP tracking branch, which kept the
/// Win+Alt hotkeys (and has fields this build doesn't know).
const VERSION_3_WIN_ALT_SETTINGS: &str = r#"{
  "version": 3,
  "expanded": true,
  "placementMode": "inside",
  "hotkeys": {
    "togglePanel": "super+alt+c",
    "editMode": "super+alt+e",
    "cyclePlacement": "super+alt+p",
    "openMeowdbPage": "super+alt+o",
    "setup": "super+alt+s",
    "chooseCandidate": "super+alt+n",
    "resetSession": "super+alt+x"
  },
  "expBarRegion": { "marked": { "x": 1016, "y": 1145, "width": 207, "height": 54 }, "client": { "width": 1920, "height": 1200 } }
}"#;

#[test]
fn win_alt_hotkeys_saved_as_version_3_are_replaced_by_the_ctrl_shift_ones_too() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.restore(serde_json::from_str(VERSION_3_WIN_ALT_SETTINGS).unwrap());

    assert_eq!(state.hotkeys, Hotkeys::default());
    assert!(state.hotkeys.all().iter().all(|hotkey| hotkey.starts_with("ctrl+shift+")));
    assert_eq!(compass.settings().hotkeys, Hotkeys::default(), "and saved as Ctrl+Shift");
}

/// Settings saved by version 1, with its own Ctrl+Shift letters.
const VERSION_1_SETTINGS: &str = r#"{
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
fn version_1_hotkeys_are_replaced_by_the_defaults_and_the_rest_is_kept() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.restore(serde_json::from_str(VERSION_1_SETTINGS).unwrap());

    assert_eq!(state.hotkeys, Hotkeys::default());
    assert_eq!(state.placement_mode, PlacementMode::DockedLeft);
    assert!(compass.on_hotkey("ctrl+shift+m").is_none());
    assert!(compass.panel_state().expanded);
}

#[test]
fn ctrl_shift_c_collapses_the_panel_and_expands_it_again() {
    let mut compass = Compass::new(FakeSource::meowdb());

    compass.on_hotkey("ctrl+shift+c");
    assert!(!compass.panel_state().expanded);
    compass.on_hotkey("ctrl+shift+c");
    assert!(compass.panel_state().expanded);
}

#[test]
fn ctrl_shift_p_cycles_the_placement() {
    let mut compass = Compass::new(FakeSource::meowdb());

    compass.on_hotkey("ctrl+shift+p");

    assert_eq!(compass.panel_state().placement_mode, PlacementMode::DockedRight);
}

/// Settings saved before edit mode was retired, with its hotkey.
const WITH_EDIT_MODE: &str = r#"{
  "version": 4,
  "placementMode": "detached",
  "hotkeys": {
    "togglePanel": "ctrl+shift+x",
    "editMode": "ctrl+shift+e",
    "cyclePlacement": "ctrl+shift+p",
    "openMeowdbPage": "ctrl+shift+m",
    "setup": "ctrl+shift+s",
    "chooseCandidate": "ctrl+shift+n",
    "search": "ctrl+shift+f"
  }
}"#;

#[test]
fn settings_with_the_retired_edit_mode_hotkey_still_load_and_it_is_neither_registered_nor_saved() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.restore(serde_json::from_str(WITH_EDIT_MODE).unwrap()).clone();

    assert_eq!(state.placement_mode, PlacementMode::Detached);
    assert_eq!(state.hotkeys.toggle_panel, "ctrl+shift+x", "the rest is kept");
    assert!(!state.hotkeys.all().contains(&"ctrl+shift+e"));
    let saved = serde_json::to_string(&compass.settings()).unwrap();
    assert!(!saved.contains("editMode"), "{saved}");
}

#[test]
fn ctrl_shift_e_does_nothing_any_more() {
    let mut compass = Compass::new(FakeSource::meowdb());
    let before = compass.panel_state().clone();

    assert_eq!(compass.on_hotkey("ctrl+shift+e"), None);

    assert_eq!(*compass.panel_state(), before);
}

#[test]
fn ctrl_shift_m_opens_the_shown_maps_meowdb_page_and_nothing_without_a_map() {
    let mut compass = Compass::new(FakeSource::meowdb());
    assert_eq!(compass.on_hotkey("ctrl+shift+m"), None);

    arrive(&mut compass, SUNSET, 0);

    assert_eq!(
        compass.on_hotkey("ctrl+shift+m"),
        Some(Request::OpenInBrowser { url: "https://meowdb.com/msclassic/maps/010003070".into() })
    );
}

#[test]
fn ctrl_shift_s_opens_setup_again() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.restore(Settings { title_region: Some(common::kerning_title_region()), ..Settings::default() });
    assert!(!compass.panel_state().setup.window_open);

    compass.on_hotkey("ctrl+shift+s");

    let state = compass.panel_state();
    assert_eq!(state.setup.needed, Some(SetupReason::Requested));
    assert!(state.setup.window_open);
}

#[test]
fn hotkeys_the_player_rebound_are_the_ones_that_act() {
    let mut compass = Compass::new(FakeSource::meowdb());
    let hotkeys = Hotkeys { toggle_panel: "ctrl+shift+x".into(), ..Hotkeys::default() };
    compass.restore(Settings { hotkeys, ..Settings::default() });

    compass.on_hotkey("ctrl+shift+c");
    assert!(compass.panel_state().expanded);
    compass.on_hotkey("Ctrl+Shift+X");
    assert!(!compass.panel_state().expanded);
}

#[test]
fn a_hotkey_another_app_has_taken_is_shown_as_unavailable() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.hotkeys_unavailable(vec!["ctrl+shift+n".into()]);

    assert_eq!(state.unavailable_hotkeys, ["ctrl+shift+n"]);
}

#[test]
fn ctrl_shift_f_is_the_search_hotkey_even_with_a_settings_file_from_before_search() {
    let before_search = r#"{"version":4,"hotkeys":{"togglePanel":"ctrl+shift+x","chooseCandidate":"ctrl+shift+n"}}"#;
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.restore(serde_json::from_str(before_search).unwrap()).clone();

    assert_eq!(state.hotkeys.search, ctrl_shift("f"));
    assert_eq!(state.hotkeys.toggle_panel, ctrl_shift("x"), "the player's own bindings are kept");
    assert_eq!(compass.settings().hotkeys.search, ctrl_shift("f"), "and saved with the rest");
}

#[test]
fn every_hotkey_the_panel_lists_is_one_to_register_the_search_one_included() {
    let compass = Compass::new(FakeSource::meowdb());

    let to_register = compass.panel_state().hotkeys.all();

    assert_eq!(
        to_register,
        ["ctrl+shift+c", "ctrl+shift+p", "ctrl+shift+m", "ctrl+shift+s", "ctrl+shift+n", "ctrl+shift+r", "ctrl+shift+f"]
    );
}

#[test]
fn a_rebound_search_hotkey_another_app_has_taken_is_shown_as_unavailable() {
    let mut compass = Compass::new(FakeSource::meowdb());
    let hotkeys = Hotkeys { search: "ctrl+shift+q".into(), ..Hotkeys::default() };
    compass.restore(Settings { hotkeys, ..Settings::default() });

    let state = compass.hotkeys_unavailable(vec!["ctrl+shift+q".into()]);

    assert_eq!(state.hotkeys.search, "ctrl+shift+q");
    assert_eq!(state.hotkeys.all().last(), Some(&"ctrl+shift+q"));
    assert_eq!(state.unavailable_hotkeys, ["ctrl+shift+q"]);
}

#[test]
fn the_hotkeys_are_live_only_while_the_game_or_maplecompass_is_in_front() {
    let mut compass = Compass::new(FakeSource::meowdb());

    assert!(compass.on_foreground(Foreground::Game).hotkeys_live);
    assert!(!compass.on_foreground(Foreground::OtherApp).hotkeys_live, "other apps keep their Ctrl+Shift keys");
    assert!(compass.on_foreground(Foreground::MapleCompass).hotkeys_live, "e.g. the overlay with the search view open, or setup");
}

#[test]
fn before_the_foreground_is_known_the_hotkeys_are_not_live() {
    let compass = Compass::new(FakeSource::meowdb());

    assert!(!compass.panel_state().hotkeys_live);
}

#[test]
fn a_hotkey_reported_unavailable_stays_listed_while_the_hotkeys_are_not_live() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.on_foreground(Foreground::Game);
    compass.hotkeys_unavailable(vec!["ctrl+shift+n".into()]);

    let state = compass.on_foreground(Foreground::OtherApp);

    assert_eq!(state.unavailable_hotkeys, ["ctrl+shift+n"]);
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
fn ctrl_shift_r_resets_the_session_back_to_waiting() {
    let mut compass = tracking_with(Hotkeys::default());
    start_session(&mut compass);

    assert_eq!(compass.on_hotkey("ctrl+shift+r"), None);

    assert_eq!(compass.panel_state().exp_tracking, waiting());
}

#[test]
fn the_reset_icon_at_the_top_of_the_panel_resets_the_session_too() {
    let mut compass = tracking_with(Hotkeys::default());
    start_session(&mut compass);

    assert_eq!(compass.press(PanelControl::ResetSession), None);

    assert_eq!(compass.panel_state().exp_tracking, waiting());
}

/// Settings saved before the reset hotkey existed, with a hotkey the player rebound.
const SETTINGS_WITHOUT_RESET_SESSION: &str = r#"{
  "version": 4,
  "hotkeys": {
    "togglePanel": "ctrl+shift+x",
    "cyclePlacement": "ctrl+shift+p",
    "openMeowdbPage": "ctrl+shift+m",
    "setup": "ctrl+shift+s",
    "chooseCandidate": "ctrl+shift+n",
    "search": "ctrl+shift+f"
  }
}"#;

#[test]
fn settings_from_before_the_reset_hotkey_get_ctrl_shift_r_and_keep_the_rest() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.restore(serde_json::from_str(SETTINGS_WITHOUT_RESET_SESSION).unwrap());

    assert_eq!(state.hotkeys.reset_session, ctrl_shift("r"));
    assert_eq!(state.hotkeys.toggle_panel, ctrl_shift("x"));
}

#[test]
fn a_rebound_reset_hotkey_is_kept_and_is_the_one_that_resets() {
    let json = SETTINGS_WITHOUT_RESET_SESSION.replace(r#""search""#, r#""resetSession": "ctrl+shift+z", "search""#);
    let saved: Settings = serde_json::from_str(&json).unwrap();
    let mut compass = tracking_with(saved.hotkeys);
    assert_eq!(compass.panel_state().hotkeys.reset_session, ctrl_shift("z"));
    start_session(&mut compass);

    compass.on_hotkey("ctrl+shift+r");
    assert_eq!(compass.panel_state().exp_tracking.state, TrackingState::WarmingUp);
    compass.on_hotkey("Ctrl+Shift+Z");
    assert_eq!(compass.panel_state().exp_tracking, waiting());
}
