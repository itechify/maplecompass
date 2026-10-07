//! Mob search in the overlay. In search mode the overlay is the one window of ours that
//! takes the keyboard (ADR 0001): it becomes focusable and comes to the front when search
//! mode starts, and gives focus back when it ends, so Windows returns it to the game.
//! Nothing is sent to the game: no input, no messages.
//!
//! Every decision is the core's; this only moves focus to and from our own overlay and
//! relays the player's typing, keys and clicks.

use crate::Command;
use compass_core::Action;
use std::sync::mpsc::Sender;
use tauri::{State, WebviewWindow};

/// Make our overlay the foreground window, so the player's typing goes to its query
/// box. Only `SetForegroundWindow` on our own window, which Windows allows us right
/// after the player pressed our hotkey. Not tao's `set_focus`: when that call is
/// refused, it synthesises an Alt key press (`SendInput`), which would go to the game.
pub fn take_focus(overlay: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = overlay.hwnd() {
        // SAFETY: a plain Win32 call on our own window handle.
        let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd) };
    }
    #[cfg(not(windows))]
    let _ = overlay;
}

/// Search mode ended: if our overlay still has the foreground, hide it and show it
/// again without activating it, so Windows activates the window below it (the game),
/// as when any window in front of the game closes. Only our own window is hidden and
/// shown; nothing is called on the game's.
pub fn give_focus_back(overlay: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = overlay.hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SW_HIDE, SW_SHOWNOACTIVATE, ShowWindow};
        // SAFETY: plain Win32 calls on our own window handle.
        unsafe {
            if GetForegroundWindow() == hwnd {
                let _ = ShowWindow(hwnd, SW_HIDE);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
    }
    #[cfg(not(windows))]
    let _ = overlay;
}

// The player's input. Plain (not async) commands run in the order they come, so a
// quick type-then-Enter reaches the core as typed.

/// The query box now holds `query`.
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

/// Esc: leave search mode, keeping the search.
#[tauri::command]
pub fn search_escape(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Act(Action::SearchEscape));
}

/// The mob search section's ×: clear the search, and leave search mode.
#[tauri::command]
pub fn search_clear(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Act(Action::ClearSearch));
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
