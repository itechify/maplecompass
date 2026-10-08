//! First-run setup windows. The overlay can't take input (it is click-through and
//! never takes focus), so setup has its own normal window, plus a "region" window laid
//! over the game's client area where the player drags the Title Region. Both are
//! declared hidden in `tauri.conf.json`: creating windows inside commands can deadlock
//! on Windows. Every decision is the core's; this only shows windows and relays.
//!
//! Unlike the overlay (ADR 0001), the region window is always-on-top: it is our own,
//! shown only while the player marks the title, and the overlay's z-order keeper
//! ignores our own windows.

use crate::gamewin;
use crate::Command;
use compass_core::{Action, PanelState, Recognition, Rect, ScreenRect, SetupState};
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
    setup_hotkey: &'a str,
    /// Hotkeys that do nothing: another app has them, or they are invalid.
    unavailable_hotkeys: &'a [String],
    /// Those of them that don't name a key combination.
    invalid_hotkeys: &'a [String],
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
        Ok(Self { app: app.clone(), setup, shown: false, emitted: None })
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
        let view = SetupView {
            setup: &state.setup,
            recognition: state.recognition,
            game: game.map(|c| GameView { width: c.width, height: c.height }),
            game_title: &state.game_window_title,
            setup_hotkey: &state.hotkeys.setup,
            unavailable_hotkeys: &state.unavailable_hotkeys,
            invalid_hotkeys: &state.invalid_hotkeys,
        };
        // Only send changes: the loop wakes up 20 times a second.
        let Ok(view) = serde_json::to_value(&view) else { return };
        if self.emitted.as_ref() == Some(&view) {
            return;
        }
        if let Err(e) = self.app.emit_to(SETUP, "setup-state", &view) {
            eprintln!("emit failed: {e}");
        }
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

/// Lay the region window over the game's client area so the player can drag the
/// Title Region on it. Its CSS px times the device pixel ratio are then client px.
#[tauri::command]
pub async fn start_marking(app: AppHandle, bridge: State<'_, Bridge>) -> Result<(), String> {
    let client = (*bridge.game.lock().unwrap()).ok_or("The game window isn't showing.")?;
    let region = app.get_webview_window(REGION).ok_or("no region window")?;
    let fail = |e: tauri::Error| e.to_string();
    region.set_position(PhysicalPosition::new(client.x, client.y)).map_err(fail)?;
    region.set_size(PhysicalSize::new(client.width, client.height)).map_err(fail)?;
    region.emit("start-marking", ()).map_err(fail)?;
    region.show().map_err(fail)?;
    region.set_focus().map_err(fail)?;
    Ok(())
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
pub async fn cancel_marking(app: AppHandle) -> Result<(), String> {
    hide(&app, REGION);
    focus_setup(&app);
    Ok(())
}

/// Keep the marked Title Region (the core only does if its test read recognised a map).
#[tauri::command]
pub async fn finish_setup(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.commands.send(Command::Act(Action::FinishSetup)).map_err(|e| e.to_string())
}
