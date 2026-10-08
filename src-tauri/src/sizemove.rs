//! The player's drags and resizes of the overlay, as Windows runs them. The page starts a
//! drag from the header and a resize from a corner grip, and Tauri's own borders around a
//! borderless window start a resize from any edge; all of them by sending the overlay a
//! mouse press on its frame (`WM_NCLBUTTONDOWN`). Windows then runs its move or size loop
//! (`WM_ENTERSIZEMOVE` to `WM_EXITSIZEMOVE`), and the page hears nothing when the mouse is
//! let go. Hearing these means subclassing our own window, nothing of the game's (ADR 0001).

use crate::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use tauri::WebviewWindow;

/// A drag or resize of the overlay by the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    Drag,
    Resize,
}

/// Whether the player can resize the overlay now: only the expanded panel, never the
/// collapsed icon.
static RESIZABLE: AtomicBool = AtomicBool::new(false);

pub fn set_resizable(resizable: bool) {
    RESIZABLE.store(resizable, Ordering::Relaxed);
}

/// Send [`Command::GestureStarted`] as the player starts a drag or resize of the overlay,
/// and [`Command::SizeMoveStarted`] and [`Command::SizeMoveEnded`] as Windows' loop that
/// runs it starts and ends. Call it on the thread that owns the overlay (the main thread,
/// as in setup).
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
        Err("couldn't watch the overlay's drags and resizes".into())
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
    use windows::Win32::Foundation::{LRESULT, WPARAM};
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{
        HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT,
        WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_NCLBUTTONDOWN,
    };
    // The sender `watch` leaked for this subclass.
    let send = |command| {
        let _ = unsafe { &*(commands as *const Sender<Command>) }.send(command);
    };
    let mut wparam = wparam;
    match msg {
        WM_NCLBUTTONDOWN => match wparam.0 as u32 {
            HTCAPTION => send(Command::GestureStarted(Gesture::Drag)),
            hit @ (HTLEFT | HTTOPLEFT | HTBOTTOMLEFT | HTRIGHT | HTTOPRIGHT | HTBOTTOMRIGHT | HTTOP | HTBOTTOM) => {
                // The panel's height follows its content: the player sets its width only,
                // by its left or right edge (a corner counts as its side). Nor do they
                // resize the collapsed icon.
                let side = match hit {
                    HTLEFT | HTTOPLEFT | HTBOTTOMLEFT => HTLEFT,
                    HTRIGHT | HTTOPRIGHT | HTBOTTOMRIGHT => HTRIGHT,
                    _ => return LRESULT(0),
                };
                if !RESIZABLE.load(Ordering::Relaxed) {
                    return LRESULT(0);
                }
                send(Command::GestureStarted(Gesture::Resize));
                wparam = WPARAM(side as usize);
            }
            _ => {}
        },
        WM_ENTERSIZEMOVE => send(Command::SizeMoveStarted),
        WM_EXITSIZEMOVE => send(Command::SizeMoveEnded),
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(not(windows))]
pub fn watch(_overlay: &WebviewWindow, _commands: Sender<Command>) -> Result<(), String> {
    Err("watching the overlay's drags and resizes is only implemented on Windows so far".into())
}
