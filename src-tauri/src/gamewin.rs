//! Game window locator and z-order keeper (Windows). Reads window-manager state only:
//! it never sends the game a message, opens its process or hooks it, and the only
//! window it ever changes is our own overlay (ADR 0001).

use compass_core::ScreenRect;

/// The game window's client area in physical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// The game window as found on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Game {
    /// The window handle as a number, only used to stack our overlay against it.
    pub handle: isize,
    pub client: ClientArea,
    /// The visible frame (caption and borders, not the invisible resize borders).
    pub frame: ScreenRect,
}

/// Exact title of the game window, unless the player picked another by hand.
pub const GAME_TITLE: &str = "MapleStory";

/// The title [`find_game`] looks for: [`GAME_TITLE`], or the window picked by hand.
static LOOK_FOR: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// Look for windows titled exactly `title` from now on.
pub fn look_for(title: &str) {
    let mut current = LOOK_FOR.write().unwrap_or_else(|e| e.into_inner());
    if *current != title {
        *current = title.to_string();
    }
}

/// The title [`find_game`] looks for.
pub fn looking_for() -> String {
    let current = LOOK_FOR.read().unwrap_or_else(|e| e.into_inner());
    if current.is_empty() { GAME_TITLE.to_string() } else { current.clone() }
}

#[cfg(windows)]
pub use win::{find_game, keep_above, monitor_work_areas, window_under_cursor};

/// The game's client area, or `None` when it isn't running or is minimised.
pub fn find_game_client() -> Option<ClientArea> {
    find_game().map(|game| game.client)
}

#[cfg(not(windows))]
pub fn find_game() -> Option<Game> {
    None
}

#[cfg(not(windows))]
pub fn window_under_cursor() -> Result<String, String> {
    Err("picking a window is only implemented on Windows so far".into())
}

#[cfg(not(windows))]
pub fn monitor_work_areas() -> Vec<ScreenRect> {
    Vec::new()
}

#[cfg(windows)]
mod win {
    use super::{ClientArea, Game, ScreenRect};
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{
        DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute,
    };
    use windows::Win32::Graphics::Gdi::{
        ClientToScreen, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GA_ROOT, GW_HWNDPREV, GWL_EXSTYLE, GetAncestor, GetClientRect, GetCursorPos,
        GetWindow, GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId, HWND_NOTOPMOST,
        HWND_TOP, HWND_TOPMOST, IsIconic, IsWindowVisible, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SetWindowPos, WS_EX_TOPMOST, WindowFromPoint,
    };
    use windows::core::BOOL;

    /// The first visible, uncloaked, not minimised top-level window titled exactly
    /// "MapleStory" (or as picked by hand). `None` when the game isn't running or is
    /// minimised.
    pub fn find_game() -> Option<Game> {
        let title = super::looking_for();
        let mut search: (&str, Vec<HWND>) = (&title, Vec::new());
        unsafe {
            let _ = EnumWindows(Some(collect), LPARAM(&mut search as *mut (&str, Vec<HWND>) as isize));
        }
        search.1.into_iter().find_map(game)
    }

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let (title, found) = unsafe { &mut *(lparam.0 as *mut (&str, Vec<HWND>)) };
        if is_game_window(hwnd, title) {
            found.push(hwnd);
        }
        BOOL(1) // keep enumerating
    }

    /// The title of the top-level window under the mouse pointer, for picking the game
    /// window by hand. Only reads window-manager state: the pointer position, which
    /// window is there, its root and its stored caption (ADR 0001).
    pub fn window_under_cursor() -> Result<String, String> {
        let mut at = POINT::default();
        unsafe { GetCursorPos(&mut at) }.map_err(|e| format!("no pointer position: {e}"))?;
        let root = unsafe { GetAncestor(WindowFromPoint(at), GA_ROOT) };
        if root.is_invalid() {
            return Err("no window under the pointer".into());
        }
        if is_ours(root) {
            return Err("that is MapleCompass itself".into());
        }
        match caption(root) {
            Some(title) if !title.trim().is_empty() => Ok(title),
            _ => Err("the window under the pointer has no title".into()),
        }
    }

    /// The caption stored by the window manager; no WM_GETTEXT is sent to another
    /// process's window.
    fn caption(hwnd: HWND) -> Option<String> {
        let mut buf = [0u16; 256];
        let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
        (n > 0).then(|| String::from_utf16_lossy(&buf[..n as usize]))
    }

    fn is_game_window(hwnd: HWND, title: &str) -> bool {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                return false;
            }
            if caption(hwnd).as_deref() != Some(title) {
                return false;
            }
            let mut cloaked: u32 = 0;
            let cloaked_ok = DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                &mut cloaked as *mut u32 as *mut _,
                size_of::<u32>() as u32,
            );
            cloaked_ok.is_err() || cloaked == 0
        }
    }

    fn game(hwnd: HWND) -> Option<Game> {
        unsafe {
            if IsIconic(hwnd).as_bool() {
                return None;
            }
            let mut client_rect = RECT::default();
            GetClientRect(hwnd, &mut client_rect).ok()?;
            let mut origin = POINT { x: 0, y: 0 };
            if !ClientToScreen(hwnd, &mut origin).as_bool() {
                return None;
            }
            let width = (client_rect.right - client_rect.left) as u32;
            let height = (client_rect.bottom - client_rect.top) as u32;
            if width == 0 || height == 0 {
                return None;
            }
            let client = ClientArea { x: origin.x, y: origin.y, width, height };
            // GetWindowRect would include the invisible resize borders (~7 px).
            let mut frame = RECT::default();
            let frame = match DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut frame as *mut RECT as *mut _,
                size_of::<RECT>() as u32,
            ) {
                Ok(()) => rect(frame),
                Err(_) => ScreenRect { x: client.x, y: client.y, width, height },
            };
            Some(Game { handle: hwnd.0 as isize, client, frame })
        }
    }

    fn rect(r: RECT) -> ScreenRect {
        ScreenRect { x: r.left, y: r.top, width: (r.right - r.left) as u32, height: (r.bottom - r.top) as u32 }
    }

    /// The work area (screen minus taskbar) of every monitor.
    pub fn monitor_work_areas() -> Vec<ScreenRect> {
        let mut areas: Vec<ScreenRect> = Vec::new();
        unsafe {
            let _ = EnumDisplayMonitors(None, None, Some(work_area), LPARAM(&mut areas as *mut Vec<ScreenRect> as isize));
        }
        areas
    }

    unsafe extern "system" fn work_area(monitor: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
        let areas = unsafe { &mut *(lparam.0 as *mut Vec<ScreenRect>) };
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            areas.push(rect(info.rcWork));
        }
        BOOL(1)
    }

    fn is_topmost(hwnd: HWND) -> bool {
        unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 }
    }

    fn is_ours(hwnd: HWND) -> bool {
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        pid == unsafe { GetCurrentProcessId() }
    }

    /// Keep our `overlay` just above the game in the z-order, without making it
    /// topmost (unless the game itself is): an app in front of the game then covers
    /// the overlay too. Only our own window is moved, only when the order is wrong,
    /// and never relative to the game's own windows (Windows refuses that when the
    /// game runs elevated). Returns whether it had to move the overlay.
    ///
    /// No owner/parent link to the game (that would attach input queues across the
    /// processes, ADR 0001) and no event hook: the caller polls this.
    pub fn keep_above(overlay: HWND, game: isize) -> windows::core::Result<bool> {
        let game = HWND(game as *mut _);
        unsafe {
            // Walk up from the game. Invisible windows (the game's IME windows sit
            // right above it) and our own helper windows don't count: the overlay
            // must come before any other visible window.
            let mut above = GetWindow(game, GW_HWNDPREV).ok();
            let mut first_other = None;
            for _ in 0..1024 {
                let Some(w) = above else { break };
                if w == overlay {
                    return Ok(false);
                }
                if IsWindowVisible(w).as_bool() && !is_ours(w) {
                    first_other = Some(w);
                    break;
                }
                above = GetWindow(w, GW_HWNDPREV).ok();
            }
            let game_topmost = is_topmost(game);
            // Inserting after that window puts ours between it and the game. If it is
            // topmost and the game isn't, the game leads its band, so go to the top of
            // the band: HWND_TOP, or HWND_NOTOPMOST to bring a topmost overlay down.
            let after = match first_other {
                Some(w) if is_topmost(w) == game_topmost => w,
                _ if game_topmost => HWND_TOPMOST,
                _ if is_topmost(overlay) => HWND_NOTOPMOST,
                _ => HWND_TOP,
            };
            SetWindowPos(overlay, Some(after), 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE)?;
            Ok(true)
        }
    }
}
