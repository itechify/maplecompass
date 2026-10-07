//! The player drives MapleCompass with global hotkeys: Win+Alt+letter, because
//! MapleStory players commonly bind Ctrl and Shift (and a registered hotkey never
//! reaches the game). The shell registers what the panel state lists and hands every
//! press to the core, which decides what it does.

mod common;

use common::{FakeSource, arrive};
use compass_core::{Compass, Hotkeys, OpenInBrowser, PlacementMode, Settings, SetupReason};

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
    let hotkeys = Hotkeys { toggle_panel: "super+alt+x".into(), ..Hotkeys::default() };
    compass.restore(Settings { hotkeys, ..Settings::default() });

    compass.on_hotkey("super+alt+c");
    assert!(compass.panel_state().expanded);
    compass.on_hotkey("Super+Alt+X");
    assert!(!compass.panel_state().expanded);
}

#[test]
fn a_hotkey_another_app_has_taken_is_shown_as_unavailable() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = compass.hotkeys_unavailable(vec!["super+alt+e".into()]);

    assert_eq!(state.unavailable_hotkeys, ["super+alt+e"]);
}
