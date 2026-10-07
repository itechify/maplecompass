//! The panel stays out of the way: it collapses to an icon, and in edit mode the player
//! drags it to a spot inside the game window that it keeps as the game window moves.
//!
//! Worked example used throughout: the game's client area at (812,110), 1920x1200
//! physical px (measured on the dev machine); an expanded panel of 340x72 and a
//! collapsed icon of 44x44.

mod common;

use common::FakeSource;
use compass_core::{Action, Compass, PanelSizes, ScreenRect, Settings, Size};

fn compass() -> Compass {
    // Layout never asks for the map index, so any source will do.
    let mut compass = Compass::new(FakeSource::offline());
    compass.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    compass
}

const GAME: ScreenRect = ScreenRect { x: 812, y: 110, width: 1920, height: 1200 };

fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect { x, y, width, height }
}

#[test]
fn the_panel_starts_expanded_on_the_right_edge_of_the_game_vertically_centred() {
    let mut compass = compass();
    let state = compass.on_game_window(Some(GAME));
    assert!(state.expanded);
    // 16 px in from the client's right edge (2732), centred on its height.
    assert_eq!(state.placement, Some(rect(2376, 674, 340, 72)));
}

#[test]
fn the_collapse_hotkey_shrinks_the_panel_to_an_icon_and_expands_it_again() {
    let mut compass = compass();
    compass.on_game_window(Some(GAME));

    let state = compass.act(Action::ToggleExpanded);
    assert!(!state.expanded);
    // The icon also sits on the right edge, vertically centred.
    assert_eq!(state.placement, Some(rect(2672, 688, 44, 44)));

    let state = compass.act(Action::ToggleExpanded);
    assert!(state.expanded);
    assert_eq!(state.placement, Some(rect(2376, 674, 340, 72)));
}

#[test]
fn a_panel_dragged_in_edit_mode_keeps_its_spot_relative_to_the_game_window() {
    let mut compass = compass();
    compass.on_game_window(Some(GAME));

    assert!(compass.act(Action::ToggleEditMode).edit_mode);
    // 188 px right of and 190 px below the client's top-left corner.
    let state = compass.act(Action::PanelMoved { x: 1000, y: 300 });
    assert_eq!(state.placement, Some(rect(1000, 300, 340, 72)));
    assert!(!compass.act(Action::ToggleEditMode).edit_mode);

    let moved_game = ScreenRect { x: 100, y: 50, ..GAME };
    let state = compass.on_game_window(Some(moved_game));
    assert_eq!(state.placement, Some(rect(288, 240, 340, 72)));
}

#[test]
fn outside_edit_mode_moving_the_overlay_window_does_not_move_the_panel() {
    let mut compass = compass();
    compass.on_game_window(Some(GAME));

    let state = compass.act(Action::PanelMoved { x: 1000, y: 300 });
    assert!(!state.edit_mode);
    assert_eq!(state.placement, Some(rect(2376, 674, 340, 72)));
}

#[test]
fn a_dragged_panel_collapses_to_an_icon_at_its_top_right_corner() {
    let mut compass = compass();
    compass.on_game_window(Some(GAME));
    compass.act(Action::ToggleEditMode);
    compass.act(Action::PanelMoved { x: 1000, y: 300 });
    compass.act(Action::ToggleEditMode);

    // The expanded panel spans x 1000..1340, so the icon ends at 1340 too.
    let state = compass.act(Action::ToggleExpanded);
    assert_eq!(state.placement, Some(rect(1296, 300, 44, 44)));
}

#[test]
fn the_panel_stays_inside_the_game_window_when_the_window_shrinks() {
    let mut compass = compass();
    compass.on_game_window(Some(GAME));
    compass.act(Action::ToggleEditMode);
    // Near the bottom-right corner of the 1920x1200 client.
    compass.act(Action::PanelMoved { x: 2376, y: 1200 });
    compass.act(Action::ToggleEditMode);

    let smaller_game = ScreenRect { width: 1280, height: 720, ..GAME };
    let state = compass.on_game_window(Some(smaller_game));
    // Flush with the 1280x720 client's bottom-right corner at (2092, 830).
    assert_eq!(state.placement, Some(rect(1752, 758, 340, 72)));
}

/// What the shell does on restart: save the settings as JSON, load them into a new core.
fn restart(compass: &Compass) -> Compass {
    let json = serde_json::to_string(&compass.settings()).unwrap();
    let mut restarted = self::compass();
    restarted.restore(serde_json::from_str(&json).unwrap());
    restarted
}

#[test]
fn the_dragged_spot_and_collapsed_state_survive_a_restart() {
    let mut compass = compass();
    compass.on_game_window(Some(GAME));
    compass.act(Action::ToggleEditMode);
    compass.act(Action::PanelMoved { x: 1000, y: 300 });
    compass.act(Action::ToggleEditMode);
    compass.act(Action::ToggleExpanded);

    let mut restarted = restart(&compass);
    let moved_game = ScreenRect { x: 100, y: 50, ..GAME };
    let state = restarted.on_game_window(Some(moved_game));
    assert!(!state.expanded);
    assert!(!state.edit_mode);
    // Icon top-right at client (528, 190), as dragged.
    assert_eq!(state.placement, Some(rect(584, 240, 44, 44)));
}

#[test]
fn an_empty_settings_file_gives_the_default_panel_and_hotkeys() {
    let settings: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(settings.hotkeys.toggle_panel, "ctrl+shift+m");
    assert_eq!(settings.hotkeys.edit_mode, "ctrl+shift+e");

    let mut compass = compass();
    compass.restore(settings);
    let state = compass.on_game_window(Some(GAME));
    assert!(state.expanded);
    assert_eq!(state.placement, Some(rect(2376, 674, 340, 72)));
}

#[test]
fn without_a_game_window_the_panel_has_no_placement() {
    let mut compass = compass();
    compass.on_game_window(Some(GAME));
    assert_eq!(compass.on_game_window(None).placement, None);
}
