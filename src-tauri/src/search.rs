//! The mob search window: a normal, focusable window of ours, declared hidden in
//! `tauri.conf.json` (creating windows inside commands can deadlock on Windows). It is
//! the one window that takes the keyboard (ADR 0001): shown and brought to the front
//! while the core says it's open, hidden when the core closes it, so Windows gives
//! focus back to the game. Nothing is sent to the game: no input, no messages.
//!
//! Every decision is the core's; this only shows the window and relays the player's
//! typing and keys.

use crate::view::{LastView, ViewSender};
use crate::{Command, gamewin};
use compass_core::{Action, PanelState, SearchState};
use serde::Serialize;
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Manager, State, WebviewWindow, WindowEvent};

pub const SEARCH: &str = "search";

/// What the search window renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchView<'a> {
    search: &'a SearchState,
}

/// The last view sent to the search window, for when it loads after it was sent.
#[derive(Default)]
pub struct LastSearchView(LastView);

/// Shows and hides the search window as the core says.
pub struct SearchWindow {
    app: AppHandle,
    window: WebviewWindow,
    shown: bool,
    view: ViewSender,
}

impl SearchWindow {
    /// Closing the window, or leaving it for another (the game), is Esc: the window
    /// has the keyboard only while it's open and in front.
    pub fn new(app: &AppHandle, commands: Sender<Command>) -> Self {
        let window = app.get_webview_window(SEARCH).expect("search window in tauri.conf.json");
        window.on_window_event(move |event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = commands.send(Command::Act(Action::SearchEscape));
            }
            // Not when focus only moved within our windows: the window reports losing
            // focus when its own webview takes it, as it does on opening.
            WindowEvent::Focused(false) if !gamewin::foreground_is_ours() => {
                let _ = commands.send(Command::Act(Action::SearchEscape));
            }
            _ => {}
        });
        app.manage(LastSearchView::default());
        Self { app: app.clone(), window, shown: false, view: ViewSender::new(SEARCH, "search-state") }
    }

    /// Follow the core's search state, and send the search window what it renders.
    pub fn sync(&mut self, state: &PanelState) {
        match (self.shown, state.search.window_open) {
            (false, true) => {
                let _ = self.window.show();
                let _ = self.window.unminimize();
                bring_to_front(&self.window);
            }
            (true, false) => {
                let _ = self.window.hide();
            }
            _ => {}
        }
        self.shown = state.search.window_open;
        self.view.send(&self.app, &SearchView { search: &state.search }, &self.app.state::<LastSearchView>().0);
    }

    /// Bring the open search window back in front, if the game came in front of it.
    pub fn raise(&self) {
        if self.shown {
            let _ = self.window.unminimize();
            bring_to_front(&self.window);
        }
    }
}

/// Make our search window the foreground window, so the player's typing goes to it.
/// Only `SetForegroundWindow` on our own window, which Windows allows us right after
/// the player pressed our hotkey. Not tao's `set_focus`: when that call is refused,
/// it synthesises an Alt key press (`SendInput`), which would go to the game.
fn bring_to_front(window: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        // SAFETY: a plain Win32 call on our own window handle.
        let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd) };
    }
    #[cfg(not(windows))]
    let _ = window;
}

/// What the search window renders, as last sent (`None` before the first).
#[tauri::command]
pub fn search_view(last: State<'_, LastSearchView>) -> Option<serde_json::Value> {
    last.0.get()
}

// The player's input. Plain (not async) commands run in the order they come, so a
// quick type-then-Enter reaches the core as typed.

/// The search box now holds `query`.
#[tauri::command]
pub fn search_query(query: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::SearchQuery(query));
}

/// Up or Down: move the highlight.
#[tauri::command]
pub fn search_move(down: bool, commands: State<'_, Sender<Command>>) {
    let action = if down { Action::SearchNext } else { Action::SearchPrevious };
    let _ = commands.send(Command::Act(action));
}

/// Enter: pick the highlighted mob, or clear the search when nothing is typed.
#[tauri::command]
pub fn search_enter(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Act(Action::SearchEnter));
}

/// Esc: close the window, keeping the search.
#[tauri::command]
pub fn search_escape(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Act(Action::SearchEscape));
}

/// The Taxis checkbox: count taxi rides in Distance, or stop counting them.
#[tauri::command]
pub fn search_toggle_taxis(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Act(Action::ToggleTaxis));
}

/// The Teleport Scrolls checkbox: count Teleport Scrolls in Distance, or stop.
#[tauri::command]
pub fn search_toggle_teleport_scrolls(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Act(Action::ToggleTeleportScrolls));
}
