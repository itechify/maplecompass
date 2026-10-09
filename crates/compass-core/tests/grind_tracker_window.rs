//! The Grind Tracker has a window of its own, beside the panel. It opens once the EXP
//! bar is set, just below the panel, and follows the panel there until the player drags
//! it somewhere; then it keeps that spot relative to the game window. The player closes
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
    Action, ClientOffset, Compass, ExpBarRegion, GameWindow, PanelControl, PanelSizes, PanelState, Rect, ScreenRect,
    Settings, Size, TitleRegion,
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
fn with_the_exp_bar_set_it_opens_just_below_the_panel() {
    let compass = started(tracking_settings());
    let state = compass.panel_state();

    assert!(state.grind_tracker.open);
    // Right edges in line (2716), 8 px below the panel's bottom (582).
    assert_eq!(tracker(state), Some(rect(2416, 590, 300, 150)));
}

#[test]
fn until_dragged_it_follows_the_expanded_panel() {
    let mut compass = started(tracking_settings());

    // The panel grows with a map's data: the window moves down with its bottom.
    let state = compass.set_panel_sizes(sizes(450));
    assert_eq!(tracker(state), Some(rect(2416, 968, 300, 150)));

    // Collapsing the panel leaves it where the expanded panel would end.
    let state = compass.press_and_state(PanelControl::TogglePanel);
    assert!(!state.expanded);
    assert_eq!(tracker(&state), Some(rect(2416, 968, 300, 150)));

    // Docked, it goes below the docked panel, outside the game.
    compass.press(PanelControl::TogglePanel);
    let state = compass.press_and_state(PanelControl::CyclePlacement);
    let panel = state.placement.unwrap();
    assert_eq!(tracker(&state), Some(rect(panel.x + 40, panel.y + 458, 300, 150)));
}

#[test]
fn below_a_panel_reaching_the_screen_s_bottom_it_goes_above_it() {
    let mut compass = started(tracking_settings());

    // Too tall to fit below its spot, the panel moves up to (2376,410), down to 1310.
    let state = compass.set_panel_sizes(sizes(900));
    assert_eq!(state.placement, Some(rect(2376, 410, 340, 900)));

    // 150 px more below that would reach past the screen (1440).
    assert_eq!(tracker(state), Some(rect(2416, 252, 300, 150)));
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
    assert_eq!(tracker(compass.panel_state()), Some(rect(2416, 590, 300, 150)));

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
    assert_eq!(tracker(compass.panel_state()), Some(rect(2416, 590, 300, 150)));
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
