//! Only one MapleCompass runs at a time. Starting it again while it runs shows the one
//! that is running: its panel, expanded, while there is a game window to show it on;
//! otherwise setup, so the player sees it is there.

mod common;

use common::FakeSource;
use compass_core::{
    Action, Compass, GameWindow, PanelSizes, PlacementMode, Rect, ScreenRect, Settings, Size, TitleRegion,
};

const GAME: ScreenRect = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };

/// A core set up before (a Title Region marked on this game window's size), so no
/// first-run setup is waiting, with or without the game window found.
fn set_up(game_found: bool) -> Compass {
    let client = Size { width: 1920, height: 1200 };
    let mut compass = Compass::new(FakeSource::offline());
    compass.set_panel_sizes(PanelSizes {
        expanded: Size { width: 380, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    compass.restore(Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client }),
        ..Settings::default()
    });
    let game = game_found.then(|| GameWindow { client: GAME, frame: GAME, screens: vec![GAME] });
    compass.on_game_window(game);
    compass
}

#[test]
fn launched_again_with_the_game_open_the_collapsed_panel_expands() {
    let mut compass = set_up(true);
    assert!(!compass.act(Action::ToggleExpanded).expanded);

    let state = compass.act(Action::LaunchedAgain);

    assert!(state.expanded);
    assert!(!state.setup.window_open, "the panel is enough to show it runs");
}

#[test]
fn launched_again_without_the_game_opens_setup() {
    let mut compass = set_up(false);
    assert!(!compass.panel_state().setup.window_open);

    let state = compass.act(Action::LaunchedAgain);

    assert!(state.setup.window_open);
}

#[test]
fn launched_again_detached_without_the_game_shows_the_panel_not_setup() {
    let mut compass = set_up(false);
    compass.restore(Settings {
        placement_mode: PlacementMode::Detached,
        expanded: false,
        title_region: compass.settings().title_region,
        ..Settings::default()
    });

    let state = compass.act(Action::LaunchedAgain);

    assert!(state.expanded, "a detached panel shows without the game");
    assert!(!state.setup.window_open);
}

#[test]
fn launched_again_during_first_run_setup_leaves_setup_as_it_is() {
    let mut compass = Compass::new(FakeSource::offline());
    compass.on_game_window(Some(GameWindow { client: GAME, frame: GAME, screens: vec![GAME] }));
    let before = compass.panel_state().setup.clone();
    assert!(before.window_open);

    let state = compass.act(Action::LaunchedAgain);

    assert_eq!(state.setup, before);
}
