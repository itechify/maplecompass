//! First-run setup windows. The overlay never takes focus on a click and has no room
//! for setup, so setup has its own normal window, plus a "region" window laid over the
//! game's client area where the player marks the Title Region or the EXP Bar Region.
//! Both are declared hidden in `tauri.conf.json`: creating windows inside commands can
//! deadlock on Windows. Every decision is the core's; this only shows windows and
//! relays.
//!
//! Unlike the overlay (ADR 0001), the region window is always-on-top: it is our own,
//! shown only while the player marks a region, and the overlay's z-order keeper
//! ignores our own windows.

use crate::gamewin;
use crate::view::{LastView, ViewSender};
use crate::Command;
use compass_core::{
    Action, Hotkeys, MarkTarget, PanelState, Recognition, Rect, ScreenRect, SelectionDirection, SetupState,
};
use serde::Serialize;
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow, WindowEvent};

pub const SETUP: &str = "setup";
pub const REGION: &str = "region";

/// What the setup commands share with the compass loop.
pub struct Bridge {
    pub commands: Sender<Command>,
    /// Titles of game windows the player picked by hand.
    pub picks: Sender<String>,
    /// The game's client area as last found, where the region window goes.
    pub game: Mutex<Option<ScreenRect>>,
    /// The last view sent to the setup window, for when it loads after it was sent.
    pub view: LastView,
}

/// What the setup window renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupView<'a> {
    setup: &'a SetupState,
    recognition: Recognition,
    /// The game client area's size, when the game window was found.
    game: Option<GameView>,
    /// The window title looked for.
    game_title: &'a str,
    hotkeys: &'a Hotkeys,
    settings_path: &'a str,
    /// Hotkeys another app has taken.
    unavailable_hotkeys: &'a [String],
}

#[derive(Serialize)]
struct GameView {
    width: u32,
    height: u32,
}

/// What the region window renders: the selection, and what it is being marked around.
#[derive(Serialize)]
struct SelectionView {
    selection: Option<Rect>,
    target: Option<MarkTarget>,
}

/// Shows and hides the setup window as the core says.
pub struct SetupWindows {
    app: AppHandle,
    setup: WebviewWindow,
    shown: bool,
    region_client: Option<ScreenRect>,
    settings_path: String,
    /// Sends the setup window its view.
    view: ViewSender,
}

impl SetupWindows {
    /// Closing setup tells the core, which decides what that means (on the first run,
    /// quitting).
    pub fn new(app: &AppHandle) -> tauri::Result<Self> {
        let setup = app.get_webview_window(SETUP).expect("setup window in tauri.conf.json");
        let handle = app.clone();
        setup.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide(&handle, SETUP);
                hide(&handle, REGION);
                let _ = handle.state::<Bridge>().commands.send(Command::Act(Action::CancelSetup));
            }
        });
        Ok(Self {
            app: app.clone(), setup, shown: false, region_client: None,
            settings_path: crate::settings::path(app)?.display().to_string(),
            view: ViewSender::new(SETUP, "setup-state"),
        })
    }

    /// Follow the core's setup state, and send the setup window what it renders.
    pub fn sync(&mut self, state: &PanelState, game: Option<ScreenRect>) {
        *self.app.state::<Bridge>().game.lock().unwrap() = game;
        match (self.shown, state.setup.window_open) {
            (false, true) => {
                let _ = self.setup.show();
                let _ = self.setup.unminimize();
                let _ = self.setup.set_focus();
            }
            (true, false) => {
                hide(&self.app, SETUP);
                hide(&self.app, REGION);
            }
            _ => {}
        }
        self.shown = state.setup.window_open;
        let mut client = game.filter(|_| state.setup.selection.is_some() && self.shown);
        if let Some(region) = self.app.get_webview_window(REGION) {
            if let Some(area) = client {
                let shown = (|| -> tauri::Result<()> {
                    if self.region_client != Some(area) {
                        region.set_position(PhysicalPosition::new(area.x, area.y))?;
                        region.set_size(PhysicalSize::new(area.width, area.height))?;
                    }
                    if self.region_client.is_none() {
                        region.show()?;
                        region.set_focus()?;
                    }
                    Ok(())
                })();
                if let Err(error) = shown {
                    let _ = region.hide();
                    focus_setup(&self.app);
                    let _ = self.app.state::<Bridge>().commands.send(Command::Act(Action::CancelSelection));
                    let _ = self.app.emit_to(SETUP, "marking-error", error.to_string());
                    client = None;
                }
            } else if self.region_client.is_some() {
                let _ = region.hide();
                if self.shown { focus_setup(&self.app); }
            }
        }
        self.region_client = client;
        let view = SetupView {
            setup: &state.setup,
            recognition: state.recognition,
            game: game.map(|c| GameView { width: c.width, height: c.height }),
            game_title: &state.game_window_title,
            hotkeys: &state.hotkeys,
            settings_path: &self.settings_path,
            unavailable_hotkeys: &state.unavailable_hotkeys,
        };
        if self.view.send(&self.app, &view, &self.app.state::<Bridge>().view) {
            let selection = SelectionView { selection: state.setup.selection, target: state.setup.selection_target };
            let _ = self.app.emit_to(REGION, "selection-state", &selection);
        }
    }
}

fn hide(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.hide();
    }
}

fn focus_setup(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETUP) {
        let _ = window.set_focus();
    }
}

/// What the setup window renders, as last sent (`None` before the first).
#[tauri::command]
pub async fn setup_view(bridge: State<'_, Bridge>) -> Result<Option<serde_json::Value>, String> {
    Ok(bridge.view.get())
}

/// Take the window under the mouse pointer as the game window.
#[tauri::command]
pub async fn pick_game_window(bridge: State<'_, Bridge>) -> Result<String, String> {
    let title = gamewin::window_under_cursor()?;
    bridge.picks.send(title.clone()).map_err(|e| e.to_string())?;
    Ok(title)
}

/// Begin marking the Title Region or the EXP Bar Region; sync lays the window over the
/// game client area. Its CSS px times the device pixel ratio are then client px.
#[tauri::command]
pub async fn start_marking(bridge: State<'_, Bridge>, target: MarkTarget) -> Result<(), String> {
    (*bridge.game.lock().unwrap()).ok_or("The game window isn't showing. Bring it back, then try again.")?;
    bridge.commands.send(Command::Act(Action::BeginSelection(target))).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn adjust_selection(
    bridge: State<'_, Bridge>, direction: SelectionDirection, resize: bool, coarse: bool,
) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::AdjustSelection { direction, resize, coarse }))
        .map_err(|e| e.to_string())
}

/// Enter on the region window: test-read the selection, around `target`.
#[tauri::command]
pub async fn confirm_selection(app: AppHandle, bridge: State<'_, Bridge>, target: MarkTarget) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::ConfirmSelection)).map_err(|e| e.to_string())?;
    app.emit_to(SETUP, marked_event(target), ()).map_err(|e| e.to_string())
}

/// The player dragged the Title Region or the EXP Bar Region (physical px, relative to
/// the client area).
#[tauri::command]
pub async fn mark_region(
    app: AppHandle,
    bridge: State<'_, Bridge>,
    target: MarkTarget,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    hide(&app, REGION);
    focus_setup(&app);
    let marked = Rect { x, y, width, height };
    let action = match target {
        MarkTarget::Title => Action::MarkTitleRegion(marked),
        MarkTarget::ExpBar => Action::MarkExpBarRegion(marked),
    };
    bridge.commands.send(Command::Act(action)).map_err(|e| e.to_string())?;
    // The setup window moves on to the test read.
    app.emit_to(SETUP, marked_event(target), ()).map_err(|e| e.to_string())
}

/// What tells the setup window a region was marked, and so its test read is on.
fn marked_event(target: MarkTarget) -> &'static str {
    match target {
        MarkTarget::Title => "title-marked",
        MarkTarget::ExpBar => "exp-bar-marked",
    }
}

/// Drop the EXP Bar Region being marked (the player skips it, or keeps the one before).
#[tauri::command]
pub async fn discard_exp_bar_mark(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::DiscardExpBarMark)).map_err(|e| e.to_string())
}

/// Turn EXP tracking off: forget the EXP Bar Region.
#[tauri::command]
pub async fn clear_exp_bar_region(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::ClearExpBarRegion)).map_err(|e| e.to_string())
}

/// The player pressed Esc on the region window.
#[tauri::command]
pub async fn cancel_marking(app: AppHandle, bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::CancelSelection)).map_err(|e| e.to_string())?;
    hide(&app, REGION);
    focus_setup(&app);
    app.emit_to(SETUP, "marking-cancelled", ()).map_err(|e| e.to_string())
}

/// Finish setup (the core only does once the test reads of what was marked succeeded).
#[tauri::command]
pub async fn finish_setup(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::FinishSetup)).map_err(|e| e.to_string())
}
