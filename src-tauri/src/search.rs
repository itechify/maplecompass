//! Mob search in the overlay. The overlay is always clickable but never activated by a
//! click; only while the search view is open does it take the keyboard (ADR 0001): it
//! becomes focusable and comes to the front when the view opens, and gives focus back
//! when it closes, so Windows returns it to the game. Nothing is sent to the game: no
//! input, no messages.
//!
//! Every decision is the core's; this only moves focus to and from our own overlay and
//! relays the player's typing, keys and clicks.

use crate::Command;
use compass_core::{Action, PanelControl};
use std::sync::mpsc::Sender;
use tauri::{State, WebviewWindow};

/// Make our overlay the foreground window, so the player's typing goes to its query
/// box. Only `SetForegroundWindow` on our own window, which Windows allows us right
/// after the player pressed our hotkey or clicked the overlay (our process received the
/// last input). Not tao's `set_focus`: when that call is refused, it synthesises an Alt
/// key press (`SendInput`), which would go to the game.
pub fn take_focus(overlay: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = overlay.hwnd() {
        // SAFETY: a plain Win32 call on our own window handle.
        let taken = unsafe { windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd) };
        if !taken.as_bool() {
            eprintln!("Windows refused the overlay the foreground: the game keeps the keyboard");
        }
    }
    #[cfg(not(windows))]
    let _ = overlay;
}

/// The search view closed: if our overlay still has the foreground, hide it and show it
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

fn act(commands: &Sender<Command>, action: Action) {
    let _ = commands.send(Command::Act(action));
}

/// One of the icons at the top of the panel (🔍, help & setup, placement and the rest):
/// the same as its hotkey.
#[tauri::command]
pub fn panel_press(control: PanelControl, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Press(control));
}

/// The query box now holds `query`.
#[tauri::command]
pub fn search_query(query: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::SearchQuery(query));
}

/// Up or Down: move the highlight.
#[tauri::command]
pub fn search_move(down: bool, commands: State<'_, Sender<Command>>) {
    act(&commands, if down { Action::SearchNext } else { Action::SearchPrevious });
}

/// Enter: pick the highlighted mob, or the highlighted map.
#[tauri::command]
pub fn search_enter(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::SearchEnter);
}

/// A click on a suggested mob or a listed map (its MeowDB ID): pick it.
#[tauri::command]
pub fn search_pick(id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::SearchPick(id));
}

/// Esc or the back arrow: back to the normal view, with nothing changed.
#[tauri::command]
pub fn search_escape(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::SearchEscape);
}

/// The Hunting section's 🔍: the Hunting Target mob's map list, to pick another map.
#[tauri::command]
pub fn hunting_reopen(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::ReopenMapList);
}

/// The Hunting section's ×: clear the Hunting Target.
#[tauri::command]
pub fn hunting_clear(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::ClearHuntingTarget);
}

/// The Taxis checkbox: count taxi rides in Distance, or stop counting them.
#[tauri::command]
pub fn search_toggle_taxis(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::ToggleTaxis);
}

/// The Teleport Scrolls checkbox: count Teleport Scrolls in Distance, or stop.
#[tauri::command]
pub fn search_toggle_teleport_scrolls(commands: State<'_, Sender<Command>>) {
    act(&commands, Action::ToggleTeleportScrolls);
}
