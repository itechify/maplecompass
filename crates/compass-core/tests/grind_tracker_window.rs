//! The Grind Tracker has a window of its own, beside the panel. It opens once the EXP
//! bar is set, beside the panel with the tops level, and follows the panel there until
//! the player drags it somewhere; then it keeps that spot relative to the game window,
//! until they drag it back beside the panel. The player closes
//! it with its × and opens it again from the panel (or Ctrl+Shift+G); closed, it stays
//! closed across restarts, while the Session carries on underneath.
//!
//! Worked example used throughout, as in `panel.rs`: the game's client area at
//! (812,110), 1920x1200 physical px, on one 3440x1440 monitor; an expanded panel of
//! 340x72 (so at (2376,510)) and a Grind Tracker window of 300x150. The EXP Bar Region
//! is marked at client (1005,1144) 32 px tall, so it is captured at (1817,1254) 384x32.

mod common;

use common::FakeSource;
use compass_core::{
    Action, ClientOffset, Compass, ExpBarRegion, Foreground, GameWindow, PanelControl, PanelSizes, PanelState, Rect,
    ScreenRect, Settings, Size, TitleRegion,
};

const GAME: ScreenRect = ScreenRect { x: 812, y: 110, width: 1920, height: 1200 };
const SCREEN: ScreenRect = ScreenRect { x: 0, y: 0, width: 3440, height: 1440 };
const CLIENT: Size = Size { width: 1920, height: 1200 };

fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect { x, y, width, height }
}

/// The game window with this client area, framed as measured, on `SCREEN`.
fn game(client: ScreenRect) -> Option<GameWindow> {
    let frame = rect(client.x - 1, client.y - 31, client.width + 2, client.height + 32);
    Some(GameWindow { client, frame, screens: vec![SCREEN] })
}

fn sizes(expanded_height: u32) -> PanelSizes {
    PanelSizes { expanded: Size { width: 340, height: expanded_height }, collapsed: Size { width: 44, height: 44 } }
}

/// Settings with both regions marked on the game's client area.
fn tracking_settings() -> Settings {
    Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client: CLIENT }),
        exp_bar_region: Some(ExpBarRegion { marked: Rect { x: 1005, y: 1144, width: 240, height: 32 }, client: CLIENT }),
        ..Settings::default()
    }
}

/// A core restored from `settings`, with the game window found and the windows' sizes known.
fn started(settings: Settings) -> Compass {
    let mut compass = Compass::new(FakeSource::offline());
    compass.restore(settings);
    compass.set_panel_sizes(sizes(72));
    compass.set_grind_tracker_size(Size { width: 300, height: 150 });
    compass.on_game_window(game(GAME));
    compass
}

fn tracker(state: &PanelState) -> Option<ScreenRect> {
    state.grind_tracker.placement
}

/// What the shell does on restart: save the settings as JSON, load them into a new core.
fn restart(compass: &Compass) -> Compass {
    let json = serde_json::to_string(&compass.settings()).unwrap();
    started(serde_json::from_str(&json).unwrap())
}

#[test]
fn without_an_exp_bar_there_is_no_grind_tracker_window() {
    let compass = started(Settings { exp_bar_region: None, ..tracking_settings() });

    assert_eq!(tracker(compass.panel_state()), None);
}

#[test]
fn with_the_exp_bar_set_it_opens_beside_the_panel_tops_level() {
    let compass = started(tracking_settings());
    let state = compass.panel_state();

    assert!(state.grind_tracker.open);
    // The panel is in the game's right half: the window goes towards its middle, 8 px
    // left of the panel (2376), its top level with the panel's (510).
    assert_eq!(tracker(state), Some(rect(2068, 510, 300, 150)));
}

#[test]
fn until_dragged_it_stays_beside_the_panel_s_top_however_tall_the_panel_grows() {
    let mut compass = started(tracking_settings());

    // The panel grows with a map's data, down from its top, which stays: so does the
    // window.
    let state = compass.set_panel_sizes(sizes(450));
    assert_eq!(state.placement, Some(rect(2376, 510, 340, 450)));
    assert_eq!(tracker(state), Some(rect(2068, 510, 300, 150)));

    // Collapsed, it goes beside the icon (2672,510).
    let state = compass.press_and_state(PanelControl::TogglePanel);
    assert!(!state.expanded);
    assert_eq!(state.placement, Some(rect(2672, 510, 44, 44)));
    assert_eq!(tracker(&state), Some(rect(2364, 510, 300, 150)));

    // Docked, it goes beside the docked panel (2733,110), away from the game.
    compass.press(PanelControl::TogglePanel);
    let state = compass.press_and_state(PanelControl::CyclePlacement);
    assert_eq!(state.placement, Some(rect(2733, 110, 340, 450)));
    assert_eq!(tracker(&state), Some(rect(3081, 110, 300, 150)));
}

#[test]
fn without_room_on_its_side_of_the_panel_it_goes_on_the_other() {
    let mut compass = started(tracking_settings());
    // Docked left of a game window near the screen's left edge: the panel at (9,110),
    // and no room left of it for the window, which goes right of it, over the game. There
    // it would cover the Title Region (420..550, 150..198), so it moves the shortest way
    // off it: down.
    let near_the_edge = rect(350, 110, 1920, 1200);
    compass.on_game_window(game(near_the_edge));
    compass.press(PanelControl::CyclePlacement);
    let state = compass.press_and_state(PanelControl::CyclePlacement);

    assert_eq!(state.placement, Some(rect(9, 110, 340, 72)));
    assert_eq!(tracker(&state), Some(rect(357, 198, 300, 150)));
}

#[test]
fn dragged_it_keeps_its_spot_relative_to_the_game_window() {
    let mut compass = started(tracking_settings());

    let state = compass.act(Action::GrindTrackerMoved { x: 1000, y: 300 });
    assert_eq!(tracker(state), Some(rect(1000, 300, 300, 150)));

    // The panel growing doesn't move it now.
    assert_eq!(tracker(compass.set_panel_sizes(sizes(450))), Some(rect(1000, 300, 300, 150)));

    // The game window moving takes it along.
    let moved = rect(GAME.x + 100, GAME.y + 50, GAME.width, GAME.height);
    assert_eq!(tracker(compass.on_game_window(game(moved))), Some(rect(1100, 350, 300, 150)));

    assert_eq!(compass.settings().grind_tracker_at, Some(ClientOffset { x: 188, y: 190 }));
    let restarted = restart(&compass);
    assert_eq!(tracker(restarted.panel_state()), Some(rect(1000, 300, 300, 150)));
}

#[test]
fn dragged_back_beside_the_panel_it_follows_the_panel_again() {
    let mut compass = started(tracking_settings());
    compass.act(Action::GrindTrackerMoved { x: 1000, y: 300 });

    // Let go within an eighth of its width (37 px) of its spot beside the panel
    // (2068,510): it goes there, and follows the panel from then on.
    let state = compass.act(Action::GrindTrackerMoved { x: 2068 + 30, y: 510 - 20 });
    assert_eq!(tracker(state), Some(rect(2068, 510, 300, 150)));
    assert_eq!(compass.settings().grind_tracker_at, None);
    compass.press(PanelControl::TogglePanel);
    assert_eq!(tracker(compass.panel_state()), Some(rect(2364, 510, 300, 150)));

    // A little further, it stays where it was let go.
    compass.press(PanelControl::TogglePanel);
    let state = compass.act(Action::GrindTrackerMoved { x: 2068 + 40, y: 510 });
    assert_eq!(tracker(state), Some(rect(2108, 510, 300, 150)));
}

#[test]
fn dragged_onto_the_exp_bar_it_moves_off_it() {
    let mut compass = started(tracking_settings());

    // Over the top of the EXP Bar Region (1817..2201, 1254..1286): our windows show in
    // the capture, so it moves the shortest way off it, here above it.
    let state = compass.act(Action::GrindTrackerMoved { x: 1800, y: 1180 });

    assert_eq!(tracker(state), Some(rect(1800, 1104, 300, 150)));
}

#[test]
fn dragged_off_the_screen_it_stays_on_it() {
    let mut compass = started(tracking_settings());

    let state = compass.act(Action::GrindTrackerMoved { x: 3300, y: 1400 });

    assert_eq!(tracker(state), Some(rect(3140, 1290, 300, 150)));
}

#[test]
fn its_close_button_closes_it_and_closed_it_stays_closed_after_a_restart() {
    let mut compass = started(tracking_settings());

    assert_eq!(compass.press(PanelControl::CloseGrindTracker), None);
    let state = compass.panel_state();
    assert!(!state.grind_tracker.open);
    assert_eq!(tracker(state), None);

    let restarted = restart(&compass);
    assert!(!restarted.panel_state().grind_tracker.open);
    assert_eq!(tracker(restarted.panel_state()), None);
}

#[test]
fn the_panel_s_grind_tracker_button_and_ctrl_shift_g_open_and_close_it() {
    let mut compass = started(tracking_settings());
    compass.press(PanelControl::CloseGrindTracker);

    assert_eq!(compass.press(PanelControl::ToggleGrindTracker), None);
    assert_eq!(tracker(compass.panel_state()), Some(rect(2068, 510, 300, 150)));

    assert_eq!(compass.on_hotkey("ctrl+shift+g"), None);
    assert_eq!(tracker(compass.panel_state()), None);
    compass.on_hotkey("Ctrl+Shift+G");
    assert!(compass.panel_state().grind_tracker.open);
}

#[test]
fn closing_it_leaves_the_session_running() {
    let mut compass = started(tracking_settings());
    let reading = |exp, percent_hundredths, at_ms| compass_core::ExpBarReading {
        bar: Some(compass_core::ExpBar { exp, percent_hundredths }),
        at_ms,
    };
    compass.on_exp_bar_reading(reading(160, 381, 0));
    compass.on_exp_bar_reading(reading(266, 633, 10_000));
    let before = compass.panel_state().exp_tracking.clone();

    compass.press(PanelControl::CloseGrindTracker);
    compass.press(PanelControl::ToggleGrindTracker);

    assert_eq!(compass.panel_state().exp_tracking, before);
}

#[test]
fn it_hides_without_the_game_window_and_while_setup_is_open() {
    let mut compass = started(tracking_settings());

    assert_eq!(tracker(compass.on_game_window(None)), None);
    assert!(compass.panel_state().grind_tracker.open, "hidden, not closed");
    compass.on_game_window(game(GAME));

    compass.press(PanelControl::Setup);
    assert!(compass.panel_state().setup.window_open);
    assert_eq!(tracker(compass.panel_state()), None);
    compass.act(Action::CancelSetup);
    assert_eq!(tracker(compass.panel_state()), Some(rect(2068, 510, 300, 150)));
}

#[test]
fn it_hides_while_another_app_is_in_front_of_the_game() {
    let mut compass = started(tracking_settings());
    let shown = tracker(compass.panel_state());
    assert!(shown.is_some());

    assert_eq!(tracker(compass.on_foreground(Foreground::OtherApp)), None);
    assert!(compass.panel_state().grind_tracker.open, "hidden, not closed");

    assert_eq!(tracker(compass.on_foreground(Foreground::Game)), shown);
    compass.on_foreground(Foreground::OtherApp);
    assert_eq!(tracker(compass.on_foreground(Foreground::MapleCompass)), shown);
}

#[test]
fn turning_tracking_off_hides_it() {
    let mut compass = started(tracking_settings());

    compass.act(Action::ClearExpBarRegion);

    assert_eq!(tracker(compass.panel_state()), None);
}

/// Pressing a control, then the panel state after it.
trait PressAndState {
    fn press_and_state(&mut self, control: PanelControl) -> PanelState;
}

impl PressAndState for Compass {
    fn press_and_state(&mut self, control: PanelControl) -> PanelState {
        self.press(control);
        self.panel_state().clone()
    }
}
