//! The player's drags and resizes of our borderless windows (the overlay and the Grind
//! Tracker's), as Windows runs them. The page starts a drag from the header and a resize
//! from a corner grip, and Tauri's own borders around a resizable borderless window start
//! a resize from any edge; all of them by sending the window a mouse press on its frame
//! (`WM_NCLBUTTONDOWN`). Windows then runs its move or size loop (`WM_ENTERSIZEMOVE` to
//! `WM_EXITSIZEMOVE`), and the page hears nothing when the mouse is let go. Hearing these
//! means subclassing our own windows, nothing of the game's (ADR 0001).

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

/// What Windows tells of a drag or resize of one of our windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeMove {
    /// The player pressed the mouse to drag or resize it.
    Pressed(Gesture),
    /// Windows' move or size loop on it started: a drag or resize is running.
    LoopStarted,
    /// Windows' move or size loop on it ended: the player let go of the mouse.
    LoopEnded,
}

/// What a watched window's subclass needs: where to send what it hears, as which
/// command, and whether the player may resize the window now (`None`: never).
struct Watcher {
    commands: Sender<Command>,
    command: fn(SizeMove) -> Command,
    resizable: Option<&'static AtomicBool>,
}

/// Whether the player can resize the overlay now: only the expanded panel, never the
/// collapsed icon.
static RESIZABLE: AtomicBool = AtomicBool::new(false);

pub fn set_resizable(resizable: bool) {
    RESIZABLE.store(resizable, Ordering::Relaxed);
}

/// Whether the player can resize the overlay now: [`set_resizable`].
pub fn overlay_resizable() -> Option<&'static AtomicBool> {
    Some(&RESIZABLE)
}

/// Send `command(SizeMove::Pressed(_))` as the player starts a drag or resize of
/// `window`, and `command(SizeMove::LoopStarted)` and `command(SizeMove::LoopEnded)` as
/// Windows' loop that runs it starts and ends. A window that isn't `resizable` (now) is
/// only ever dragged. Call it on the thread that owns the window (the main thread, as in
/// setup).
#[cfg(windows)]
pub fn watch(
    window: &WebviewWindow,
    commands: Sender<Command>,
    command: fn(SizeMove) -> Command,
    resizable: Option<&'static AtomicBool>,
) -> Result<(), String> {
    use windows::Win32::UI::Shell::SetWindowSubclass;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    // Our windows last as long as the app, and so does what their subclass is given.
    let watcher = Box::into_raw(Box::new(Watcher { commands, command, resizable }));
    if unsafe { SetWindowSubclass(hwnd, Some(on_message), 1, watcher as usize) }.as_bool() {
        Ok(())
    } else {
        // Not subclassed: nothing will use it.
        drop(unsafe { Box::from_raw(watcher) });
        Err(format!("couldn't watch the {} window's drags and resizes", window.label()))
    }
}

#[cfg(windows)]
unsafe extern "system" fn on_message(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    watcher: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::{LRESULT, WPARAM};
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{
        HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT,
        WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_NCLBUTTONDOWN,
    };
    // What `watch` leaked for this subclass.
    let watcher = unsafe { &*(watcher as *const Watcher) };
    let send = |what| {
        let _ = watcher.commands.send((watcher.command)(what));
    };
    let mut wparam = wparam;
    match msg {
        WM_NCLBUTTONDOWN => match wparam.0 as u32 {
            HTCAPTION => send(SizeMove::Pressed(Gesture::Drag)),
            hit @ (HTLEFT | HTTOPLEFT | HTBOTTOMLEFT | HTRIGHT | HTTOPRIGHT | HTBOTTOMRIGHT | HTTOP | HTBOTTOM) => {
                // The panel's height follows its content: the player sets its width only,
                // by its left or right edge (a corner counts as its side). Nor do they
                // resize the collapsed icon.
                let side = match hit {
                    HTLEFT | HTTOPLEFT | HTBOTTOMLEFT => HTLEFT,
                    HTRIGHT | HTTOPRIGHT | HTBOTTOMRIGHT => HTRIGHT,
                    _ => return LRESULT(0),
                };
                if !watcher.resizable.is_some_and(|resizable| resizable.load(Ordering::Relaxed)) {
                    return LRESULT(0);
                }
                send(SizeMove::Pressed(Gesture::Resize));
                wparam = WPARAM(side as usize);
            }
            _ => {}
        },
        WM_ENTERSIZEMOVE => send(SizeMove::LoopStarted),
        WM_EXITSIZEMOVE => send(SizeMove::LoopEnded),
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(not(windows))]
pub fn watch(
    _window: &WebviewWindow,
    _commands: Sender<Command>,
    _command: fn(SizeMove) -> Command,
    _resizable: Option<&'static AtomicBool>,
) -> Result<(), String> {
    Err("watching drags and resizes is only implemented on Windows so far".into())
}
