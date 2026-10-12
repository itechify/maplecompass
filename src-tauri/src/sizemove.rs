//! The player's drags and resizes of our borderless windows (the overlay and the Grind
//! Tracker's), as Windows runs them. The page starts a drag from the header and a resize
//! from a corner grip ([`start_drag`], [`start_resize`]), and Tauri's own borders around a
//! resizable borderless window start a resize from any edge; all of them by sending the
//! window a mouse press on its frame (`WM_NCLBUTTONDOWN`). Windows then runs its move or
//! size loop (`WM_ENTERSIZEMOVE` to `WM_EXITSIZEMOVE`), and the page hears nothing when
//! the mouse is let go. Hearing these means subclassing our own windows, nothing of the
//! game's (ADR 0001).
//!
//! On Linux the shell runs that loop itself: while the player holds the mouse button down
//! on our window, which keeps the pointer to itself until they let go, it moves or sizes
//! the window after the pointer. Window managers run their own move loops differently, and
//! none of them says when one ends.

use crate::Command;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use tauri::WebviewWindow;

/// A drag or resize of the overlay by the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    Drag,
    Resize,
}

/// What Windows tells of a drag or resize of one of our windows (on Linux, the shell's own
/// loop).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeMove {
    /// The player pressed the mouse to drag or resize it.
    Pressed(Gesture),
    /// Windows' move or size loop on it started: a drag or resize is running.
    LoopStarted,
    /// Windows' move or size loop on it ended: the player let go of the mouse.
    LoopEnded,
}

/// The side of the panel a corner grip resizes it by: the player sets its width only.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum Side {
    West,
    East,
}

/// What a watched window's subclass needs: where to send what it hears, as which
/// command, and whether the player may resize the window now (`None`: never).
#[derive(Clone)]
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

/// The page's press on the window's header (or the card's bar): the player drags it.
#[tauri::command]
pub fn start_drag(window: WebviewWindow) {
    start(window, None);
}

/// The page's press on a corner grip of the expanded panel: the player resizes it by its
/// `side`.
#[tauri::command]
pub fn start_resize(window: WebviewWindow, side: Side) {
    start(window, Some(side));
}

/// Start Windows' move loop on the window, or its size loop by `side`, as tao does: the
/// window's subclass hears it ([`watch`]).
#[cfg(windows)]
fn start(window: WebviewWindow, side: Option<Side>) {
    use tauri_runtime::ResizeDirection;
    let webview: &tauri::Webview = window.as_ref();
    let started = match side {
        None => window.start_dragging(),
        Some(Side::West) => webview.window().start_resize_dragging(ResizeDirection::West),
        Some(Side::East) => webview.window().start_resize_dragging(ResizeDirection::East),
    };
    if let Err(e) = started {
        crate::errors::error!("couldn't start a drag of the {} window: {e}", window.label());
    }
}

#[cfg(target_os = "linux")]
use linux::start;
#[cfg(target_os = "linux")]
pub use linux::watch;

#[cfg(target_os = "linux")]
mod linux {
    use super::{Gesture, Side, SizeMove, Watcher};
    use crate::Command;
    use crate::errors::error;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::Sender;
    use std::sync::{LazyLock, Mutex};
    use std::time::Duration;
    use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};

    /// How often the loop follows the pointer: as smooth as most screens draw.
    const FOLLOW_TICK: Duration = Duration::from_millis(8);

    /// The watched windows, by label.
    static WATCHERS: LazyLock<Mutex<HashMap<String, Watcher>>> = LazyLock::new(Default::default);

    /// Send `command(SizeMove::Pressed(_))` as the player starts a drag or resize of
    /// `window`, and `command(SizeMove::LoopStarted)` and `command(SizeMove::LoopEnded)` as
    /// the shell's loop that runs it starts and ends. A window that isn't `resizable` (now)
    /// is only ever dragged.
    pub fn watch(
        window: &WebviewWindow,
        commands: Sender<Command>,
        command: fn(SizeMove) -> Command,
        resizable: Option<&'static AtomicBool>,
    ) -> Result<(), String> {
        let watcher = Watcher { commands, command, resizable };
        WATCHERS.lock().unwrap_or_else(|e| e.into_inner()).insert(window.label().to_string(), watcher);
        Ok(())
    }

    /// Run the move loop on the window, or its size loop by `side`, on a thread of its own
    /// until the player lets go of the mouse.
    pub fn start(window: WebviewWindow, side: Option<Side>) {
        let watcher = WATCHERS.lock().unwrap_or_else(|e| e.into_inner()).get(window.label()).cloned();
        let Some(watcher) = watcher else {
            error!("the {} window's drags aren't watched", window.label());
            return;
        };
        if side.is_some() && !watcher.resizable.is_some_and(|resizable| resizable.load(Ordering::Relaxed)) {
            return;
        }
        let name = format!("drag-{}", window.label());
        let spawned = std::thread::Builder::new().name(name).spawn(move || follow_pointer(&window, &watcher, side));
        if let Err(e) = spawned {
            error!("couldn't follow a drag: {e}");
        }
    }

    /// Move the window after the pointer (or size it, by `side`) while the left button is
    /// down. Only reads the pointer's position and buttons, on our own connection to the
    /// X server, and moves our own window (ADR 0001).
    fn follow_pointer(window: &WebviewWindow, watcher: &Watcher, side: Option<Side>) {
        let send = |what| {
            let _ = watcher.commands.send((watcher.command)(what));
        };
        send(SizeMove::Pressed(if side.is_some() { Gesture::Resize } else { Gesture::Drag }));
        // Let go already: like a press whose loop never starts.
        let Some((from, true)) = crate::x11::pointer() else { return };
        let (Ok(at), Ok(size), Ok(scale)) = (window.outer_position(), window.inner_size(), window.scale_factor()) else {
            return;
        };
        // The overlay is the one window the player resizes, never under the panel's least.
        let least = (f64::from(minerva_core::MIN_PANEL_WIDTH) * scale).ceil() as i32;
        send(SizeMove::LoopStarted);
        let mut last = None;
        while let Some((to, true)) = crate::x11::pointer() {
            let (dx, dy) = (to.x - from.x, to.y - from.y);
            let (x, width) = match side {
                None => (at.x + dx, size.width as i32),
                Some(Side::East) => (at.x, (size.width as i32 + dx).max(least)),
                Some(Side::West) => {
                    let width = (size.width as i32 - dx).max(least);
                    (at.x + size.width as i32 - width, width)
                }
            };
            let y = if side.is_some() { at.y } else { at.y + dy };
            if last != Some((x, y, width)) {
                last = Some((x, y, width));
                if side.is_some() {
                    let _ = window.set_size(PhysicalSize::new(width as u32, size.height));
                }
                let _ = window.set_position(PhysicalPosition::new(x, y));
            }
            std::thread::sleep(FOLLOW_TICK);
        }
        send(SizeMove::LoopEnded);
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn watch(
    _window: &WebviewWindow,
    _commands: Sender<Command>,
    _command: fn(SizeMove) -> Command,
    _resizable: Option<&'static AtomicBool>,
) -> Result<(), String> {
    Err("watching drags and resizes is only implemented on Windows and Linux so far".into())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn start(_window: WebviewWindow, _side: Option<Side>) {}
