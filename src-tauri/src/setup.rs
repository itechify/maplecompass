//! First-run setup windows. The overlay can't take input (it is click-through and
//! never takes focus), so setup has its own normal window, plus a "region" window laid
//! over the game's client area where the player marks the Title Region. Both are
//! declared hidden in `tauri.conf.json`: creating windows inside commands can deadlock
//! on Windows. Every decision is the core's; this only shows windows and relays.
//!
//! Unlike the overlay (ADR 0001), the region window is always-on-top: it is our own,
//! shown only while the player marks the title, and the overlay's z-order keeper
//! ignores our own windows.

use crate::gamewin;
use crate::Command;
use compass_core::{Action, Hotkeys, PanelState, Recognition, Rect, ScreenRect, SelectionDirection, SetupState};
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
    pub view: Mutex<Option<serde_json::Value>>,
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

/// Shows and hides the setup window as the core says.
pub struct SetupWindows {
    app: AppHandle,
    setup: WebviewWindow,
    shown: bool,
    region_client: Option<ScreenRect>,
    settings_path: String,
    /// The last view sent to the setup window.
    emitted: Option<serde_json::Value>,
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
            settings_path: crate::settings::path(app)?.display().to_string(), emitted: None,
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
                    let _ = self.app.state::<Bridge>().commands.send(Command::Act(Action::CancelTitleSelection));
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
        // Only send changes: the loop wakes up 20 times a second.
        let Ok(view) = serde_json::to_value(&view) else { return };
        if self.emitted.as_ref() == Some(&view) {
            return;
        }
        if let Err(e) = self.app.emit_to(SETUP, "setup-state", &view) {
            eprintln!("emit failed: {e}");
        }
        let _ = self.app.emit_to(REGION, "selection-state", &state.setup.selection);
        *self.app.state::<Bridge>().view.lock().unwrap() = Some(view.clone());
        self.emitted = Some(view);
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
    Ok(bridge.view.lock().unwrap().clone())
}

/// Take the window under the mouse pointer as the game window.
#[tauri::command]
pub async fn pick_game_window(bridge: State<'_, Bridge>) -> Result<String, String> {
    let title = gamewin::window_under_cursor()?;
    bridge.picks.send(title.clone()).map_err(|e| e.to_string())?;
    Ok(title)
}

/// Begin marking; sync lays the window over the game client area. Its CSS px times
/// the device pixel ratio are then client px.
#[tauri::command]
pub async fn start_marking(bridge: State<'_, Bridge>) -> Result<(), String> {
    (*bridge.game.lock().unwrap()).ok_or("The game window isn't showing. Bring it back, then try again.")?;
    bridge.commands.send(Command::Act(Action::BeginTitleSelection)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn adjust_title_selection(
    bridge: State<'_, Bridge>, direction: SelectionDirection, resize: bool, coarse: bool,
) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::AdjustTitleSelection { direction, resize, coarse }))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn confirm_title_selection(app: AppHandle, bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::ConfirmTitleSelection)).map_err(|e| e.to_string())?;
    app.emit_to(SETUP, "title-marked", ()).map_err(|e| e.to_string())
}

/// The player dragged the Title Region (physical px, relative to the client area).
#[tauri::command]
pub async fn mark_title_region(
    app: AppHandle,
    bridge: State<'_, Bridge>,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    hide(&app, REGION);
    focus_setup(&app);
    bridge
        .commands
        .send(Command::Act(Action::MarkTitleRegion(Rect { x, y, width, height })))
        .map_err(|e| e.to_string())?;
    // The setup window moves on to the test read.
    app.emit_to(SETUP, "title-marked", ()).map_err(|e| e.to_string())
}

/// The player pressed Esc on the region window.
#[tauri::command]
pub async fn cancel_marking(app: AppHandle, bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::CancelTitleSelection)).map_err(|e| e.to_string())?;
    hide(&app, REGION);
    focus_setup(&app);
    app.emit_to(SETUP, "marking-cancelled", ()).map_err(|e| e.to_string())
}

/// Keep the marked Title Region (the core only does if its test read recognised a map).
#[tauri::command]
pub async fn finish_setup(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::FinishSetup)).map_err(|e| e.to_string())
}
