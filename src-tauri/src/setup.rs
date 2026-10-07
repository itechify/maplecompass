//! First-run setup windows. The overlay can't take input (it is click-through and
//! never takes focus), so setup has its own normal window, plus a "region" window laid
//! over the game's client area where the player drags the Title Region. Both are
//! declared hidden in `tauri.conf.json`: creating windows inside commands can deadlock
//! on Windows. Every decision is the core's; this only shows windows and relays.

use crate::gamewin::{self, ClientArea};
use compass_core::{Action, PanelState, Recognition, Rect, SetupReason, SetupState};
use serde::Serialize;
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow, WindowEvent};

pub const SETUP: &str = "setup";
pub const REGION: &str = "region";

/// What the setup commands share with the compass loop.
pub struct Bridge {
    pub actions: Sender<Action>,
    /// Titles of game windows the player picked by hand.
    pub picks: Sender<String>,
    /// The game's client area as last found, where the region window goes.
    pub game: Mutex<Option<ClientArea>>,
    /// Why setup is needed, as last reported by the core.
    pub needed: Mutex<Option<SetupReason>>,
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
}

#[derive(Serialize)]
struct GameView {
    width: u32,
    height: u32,
}

/// Shows the setup window when the core says setup is needed, and hides it when done.
pub struct SetupWindows {
    app: AppHandle,
    setup: WebviewWindow,
    shown_for: Option<SetupReason>,
}

impl SetupWindows {
    /// Closing setup cancels it; closing it on the first run quits, since there is
    /// nothing to read without a Title Region.
    pub fn new(app: &AppHandle) -> tauri::Result<Self> {
        let setup = app.get_webview_window(SETUP).expect("setup window in tauri.conf.json");
        let handle = app.clone();
        setup.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let bridge = handle.state::<Bridge>();
                if *bridge.needed.lock().unwrap() == Some(SetupReason::FirstRun) {
                    handle.exit(0);
                    return;
                }
                hide(&handle, SETUP);
                hide(&handle, REGION);
                let _ = bridge.actions.send(Action::CancelSetup);
            }
        });
        Ok(Self { app: app.clone(), setup, shown_for: None })
    }

    /// Bring setup up (the re-setup hotkey), even if it was needed before and closed.
    pub fn open(&self) {
        let _ = self.setup.show();
        let _ = self.setup.unminimize();
        let _ = self.setup.set_focus();
    }

    /// Follow the core's setup state, and send the setup window what it renders.
    pub fn sync(&mut self, state: &PanelState, game: Option<ClientArea>, game_title: &str, setup_hotkey: &str) {
        *self.app.state::<Bridge>().needed.lock().unwrap() = state.setup.needed;
        *self.app.state::<Bridge>().game.lock().unwrap() = game;
        match (self.shown_for, state.setup.needed) {
            (None, Some(_)) => self.open(),
            (Some(_), None) => {
                hide(&self.app, SETUP);
                hide(&self.app, REGION);
            }
            _ => {}
        }
        self.shown_for = state.setup.needed;
        let view = SetupView {
            setup: &state.setup,
            recognition: state.recognition,
            game: game.map(|c| GameView { width: c.width, height: c.height }),
            game_title,
            setup_hotkey,
        };
        if let Err(e) = self.app.emit_to(SETUP, "setup-state", &view) {
            eprintln!("emit failed: {e}");
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
        .actions
        .send(Action::MarkTitleRegion(Rect { x, y, width, height }))
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
    bridge.actions.send(Action::FinishSetup).map_err(|e| e.to_string())
}
