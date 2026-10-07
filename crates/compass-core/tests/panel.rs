//! The panel stays out of the way: it collapses to an icon, and in edit mode the player
//! drags it to a spot inside the game window that it keeps as the game window moves.
//! It can also dock against the game window's left or right edge, or detach into a
//! free window anywhere.
//!
//! Worked example used throughout: the game's client area at (812,110), 1920x1200
//! physical px (measured on the dev machine), on one 3440x1440 monitor; an expanded
//! panel of 340x72 and a collapsed icon of 44x44. As measured on Windows 11, the game's
//! visible frame adds 1 px at the sides and bottom of the client area and 31 px on top.

mod common;

use common::FakeSource;
use compass_core::{
    Action, Compass, GameWindow, Layer, PanelSizes, PlacementMode, ScreenRect, Settings, Size,
};

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

const SCREEN: ScreenRect = ScreenRect { x: 0, y: 0, width: 3440, height: 1440 };

fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect { x, y, width, height }
}

/// The game window with this client area, framed as measured, on `SCREEN`.
fn game(client: ScreenRect) -> Option<GameWindow> {
    let frame = rect(client.x - 1, client.y - 31, client.width + 2, client.height + 32);
    Some(GameWindow { client, frame, screens: vec![SCREEN] })
}

#[test]
fn the_panel_starts_expanded_on_the_right_edge_of_the_game_vertically_centred() {
    let mut compass = compass();
    let state = compass.on_game_window(game(GAME));
    assert!(state.expanded);
    // 16 px in from the client's right edge (2732), centred on its height.
    assert_eq!(state.placement, Some(rect(2376, 674, 340, 72)));
}

#[test]
fn the_collapse_hotkey_shrinks_the_panel_to_an_icon_and_expands_it_again() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));

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
    compass.on_game_window(game(GAME));

    assert!(compass.act(Action::ToggleEditMode).edit_mode);
    // 188 px right of and 190 px below the client's top-left corner.
    let state = compass.act(Action::PanelMoved { x: 1000, y: 300 });
    assert_eq!(state.placement, Some(rect(1000, 300, 340, 72)));
    assert!(!compass.act(Action::ToggleEditMode).edit_mode);

    let moved_game = ScreenRect { x: 100, y: 50, ..GAME };
    let state = compass.on_game_window(game(moved_game));
    assert_eq!(state.placement, Some(rect(288, 240, 340, 72)));
}

#[test]
fn outside_edit_mode_moving_the_overlay_window_does_not_move_the_panel() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));

    let state = compass.act(Action::PanelMoved { x: 1000, y: 300 });
    assert!(!state.edit_mode);
    assert_eq!(state.placement, Some(rect(2376, 674, 340, 72)));
}

#[test]
fn a_dragged_panel_collapses_to_an_icon_at_its_top_right_corner() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));
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
    compass.on_game_window(game(GAME));
    compass.act(Action::ToggleEditMode);
    // Near the bottom-right corner of the 1920x1200 client.
    compass.act(Action::PanelMoved { x: 2376, y: 1200 });
    compass.act(Action::ToggleEditMode);

    let smaller_game = ScreenRect { width: 1280, height: 720, ..GAME };
    let state = compass.on_game_window(game(smaller_game));
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
    compass.on_game_window(game(GAME));
    compass.act(Action::ToggleEditMode);
    compass.act(Action::PanelMoved { x: 1000, y: 300 });
    compass.act(Action::ToggleEditMode);
    compass.act(Action::ToggleExpanded);

    let mut restarted = restart(&compass);
    let moved_game = ScreenRect { x: 100, y: 50, ..GAME };
    let state = restarted.on_game_window(game(moved_game));
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
    assert_eq!(settings.hotkeys.cycle_placement, "ctrl+shift+d");

    let mut compass = compass();
    compass.restore(settings);
    let state = compass.on_game_window(game(GAME));
    assert!(state.expanded);
    assert_eq!(state.placement_mode, PlacementMode::Inside);
    assert_eq!(state.placement, Some(rect(2376, 674, 340, 72)));
}

#[test]
fn without_a_game_window_the_panel_has_no_placement() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));
    assert_eq!(compass.on_game_window(None).placement, None);
}

// Docked and detached placements. The placement hotkey cycles
// inside -> docked right -> docked left -> detached -> inside.

#[test]
fn docked_right_the_panel_sits_against_the_game_windows_right_edge_level_with_its_top() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));

    let state = compass.act(Action::CyclePlacement);
    // Flush with the frame's right edge (client right 2732 + 1 px border), level with
    // the top of the client area.
    assert_eq!(state.placement, Some(rect(2733, 110, 340, 72)));
}

#[test]
fn docked_left_the_panel_sits_against_the_game_windows_left_edge_level_with_its_top() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));
    compass.act(Action::CyclePlacement);

    let state = compass.act(Action::CyclePlacement);
    assert_eq!(state.placement_mode, PlacementMode::DockedLeft);
    // Ends at the frame's left edge, 811 (client left 812 - 1 px border).
    assert_eq!(state.placement, Some(rect(471, 110, 340, 72)));
}

#[test]
fn a_docked_panel_without_room_on_screen_falls_back_inside_the_game_window() {
    let mut compass = compass();
    compass.act(Action::CyclePlacement);
    // The frame now ends at 3421, 19 px short of the screen's right edge.
    let near_the_edge = ScreenRect { x: 1500, ..GAME };

    let state = compass.on_game_window(game(near_the_edge));
    assert_eq!(state.placement_mode, PlacementMode::DockedRight);
    // The default inside spot: 16 px in from the client's right edge (3420), centred.
    assert_eq!(state.placement, Some(rect(3064, 674, 340, 72)));
}

#[test]
fn a_docked_panel_can_sit_on_the_monitor_beside_the_game() {
    let mut compass = compass();
    compass.act(Action::CyclePlacement);
    // The frame ends exactly at the main screen's right edge, 3440; a 1920x1080
    // monitor continues to the right of it.
    let at_the_edge = ScreenRect { x: 1519, ..GAME };
    let frame = rect(1518, 79, 1922, 1232);
    let second = rect(3440, 0, 1920, 1080);
    let window = GameWindow { client: at_the_edge, frame, screens: vec![SCREEN, second] };

    let state = compass.on_game_window(Some(window));
    assert_eq!(state.placement, Some(rect(3440, 110, 340, 72)));
}

#[test]
fn a_docked_panel_follows_the_game_window_as_it_moves_and_resizes() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));
    compass.act(Action::CyclePlacement);
    compass.act(Action::CyclePlacement);

    let moved = rect(1200, 300, 1280, 720);
    let state = compass.on_game_window(game(moved));
    // Docked left of the frame at 1199.
    assert_eq!(state.placement, Some(rect(859, 300, 340, 72)));
}

#[test]
fn a_docked_panel_collapses_to_an_icon_against_the_same_edge() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));
    compass.act(Action::CyclePlacement);
    compass.act(Action::CyclePlacement);

    let state = compass.act(Action::ToggleExpanded);
    // The icon still ends at the frame's left edge, 811.
    assert_eq!(state.placement, Some(rect(767, 110, 44, 44)));
}

#[test]
fn the_placement_hotkey_cycles_through_every_placement_and_back_inside() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));

    let modes: Vec<_> =
        (0..4).map(|_| compass.act(Action::CyclePlacement).placement_mode).collect();
    use PlacementMode::*;
    assert_eq!(modes, [DockedRight, DockedLeft, Detached, Inside]);
    assert_eq!(compass.panel_state().placement, Some(rect(2376, 674, 340, 72)));
}

/// Inside -> docked right -> docked left -> detached.
fn detach(compass: &mut Compass) {
    for _ in 0..3 {
        compass.act(Action::CyclePlacement);
    }
}

#[test]
fn a_detached_panel_stays_where_it_was_when_the_game_window_moves() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));

    detach(&mut compass);
    let state = compass.on_game_window(game(ScreenRect { x: 100, y: 50, ..GAME }));
    assert_eq!(state.placement_mode, PlacementMode::Detached);
    // Where it was last, docked left of the game, until the player moves it.
    assert_eq!(state.placement, Some(rect(471, 110, 340, 72)));
}

#[test]
fn a_detached_panel_moved_to_another_monitor_stays_there_across_a_restart() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));
    detach(&mut compass);

    // Dragged onto a monitor left of the main one; no edit mode needed when detached.
    let state = compass.act(Action::PanelMoved { x: -1500, y: 200 });
    assert_eq!(state.placement, Some(rect(-1500, 200, 340, 72)));

    let mut restarted = restart(&compass);
    let state = restarted.on_game_window(game(GAME));
    assert_eq!(state.placement_mode, PlacementMode::Detached);
    assert_eq!(state.placement, Some(rect(-1500, 200, 340, 72)));
}

#[test]
fn the_chosen_placement_survives_a_restart() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));
    compass.act(Action::CyclePlacement);

    let mut restarted = restart(&compass);
    let state = restarted.on_game_window(game(GAME));
    assert_eq!(state.placement_mode, PlacementMode::DockedRight);
    assert_eq!(state.placement, Some(rect(2733, 110, 340, 72)));
}

// Z-order and the mouse. Inside and docked, the overlay belongs to the game: it sits just
// above it, so whatever covers the game covers the overlay too, and it disappears with
// the game. Detached, it is an ordinary window.

#[test]
fn inside_or_docked_the_overlay_sits_above_the_game_and_disappears_without_it() {
    let mut compass = compass();
    let state = compass.on_game_window(game(GAME));
    assert!(state.game_found);
    assert_eq!(state.layer, Layer::AboveGame);

    compass.act(Action::CyclePlacement);
    assert_eq!(compass.panel_state().layer, Layer::AboveGame);

    // Minimised or closed: the shell reports no game window.
    let state = compass.on_game_window(None);
    assert!(!state.game_found);
    assert_eq!(state.layer, Layer::Hidden);
}

#[test]
fn a_detached_panel_is_an_ordinary_window_that_stays_when_the_game_is_gone() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));
    detach(&mut compass);

    let state = compass.on_game_window(None);
    assert!(!state.game_found);
    assert_eq!(state.layer, Layer::Free);
    assert!(state.draggable);
    assert_eq!(state.placement, Some(rect(471, 110, 340, 72)));
}

#[test]
fn clicks_pass_through_an_inside_panel_until_edit_mode() {
    let mut compass = compass();
    let state = compass.on_game_window(game(GAME));
    assert!(!state.draggable);

    assert!(compass.act(Action::ToggleEditMode).draggable);
}

#[test]
fn a_docked_panel_cannot_be_dragged_off_its_edge_even_in_edit_mode() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));
    compass.act(Action::CyclePlacement);

    let state = compass.act(Action::ToggleEditMode);
    assert!(!state.draggable);
    let state = compass.act(Action::PanelMoved { x: 1000, y: 300 });
    assert_eq!(state.placement, Some(rect(2733, 110, 340, 72)));
}

#[test]
fn a_hidden_overlay_never_takes_the_mouse() {
    let mut compass = compass();
    compass.on_game_window(game(GAME));
    compass.act(Action::ToggleEditMode);

    let state = compass.on_game_window(None);
    assert_eq!(state.layer, Layer::Hidden);
    assert!(!state.draggable);
}
