//! When Windows' move or size loop on the overlay starts and ends. The player's drag by
//! the header and resize by a corner grip run in that loop, and the page hears nothing
//! when the mouse is let go; Windows tells the window itself (`WM_ENTERSIZEMOVE`,
//! `WM_EXITSIZEMOVE`). Hearing them means subclassing our own window, nothing of the
//! game's (ADR 0001).

use crate::Command;
use std::sync::mpsc::Sender;
use tauri::WebviewWindow;

/// Send [`Command::SizeMoveStarted`] and [`Command::SizeMoveEnded`] as the overlay's move
/// or size loop starts and ends. Call it on the thread that owns the overlay (the main
/// thread, as in setup).
#[cfg(windows)]
pub fn watch(overlay: &WebviewWindow, commands: Sender<Command>) -> Result<(), String> {
    use windows::Win32::UI::Shell::SetWindowSubclass;
    let hwnd = overlay.hwnd().map_err(|e| e.to_string())?;
    // The overlay lasts as long as the app, and so does the sender it is given.
    let commands = Box::into_raw(Box::new(commands));
    if unsafe { SetWindowSubclass(hwnd, Some(on_message), 1, commands as usize) }.as_bool() {
        Ok(())
    } else {
        // Not subclassed: nothing will use it.
        drop(unsafe { Box::from_raw(commands) });
        Err("couldn't watch the overlay's move and size loop".into())
    }
}

#[cfg(windows)]
unsafe extern "system" fn on_message(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    commands: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE};
    let command = match msg {
        WM_ENTERSIZEMOVE => Some(Command::SizeMoveStarted),
        WM_EXITSIZEMOVE => Some(Command::SizeMoveEnded),
        _ => None,
    };
    if let Some(command) = command {
        // The sender `watch` leaked for this subclass.
        let commands = unsafe { &*(commands as *const Sender<Command>) };
        let _ = commands.send(command);
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(not(windows))]
pub fn watch(_overlay: &WebviewWindow, _commands: Sender<Command>) -> Result<(), String> {
    Err("watching the overlay's move and size loop is only implemented on Windows so far".into())
}
