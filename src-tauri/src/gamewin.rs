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

/// Exact title of the game window.
pub const GAME_TITLE: &str = "MapleStory";

#[cfg(windows)]
pub use win::{find_game, keep_above, monitor_work_areas};

/// The game's client area, or `None` when it isn't running or is minimised.
pub fn find_game_client() -> Option<ClientArea> {
    find_game().map(|game| game.client)
}

#[cfg(not(windows))]
pub fn find_game() -> Option<Game> {
    None
}

#[cfg(not(windows))]
pub fn monitor_work_areas() -> Vec<ScreenRect> {
    Vec::new()
}

#[cfg(windows)]
mod win {
    use super::{ClientArea, GAME_TITLE, Game, ScreenRect};
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{
        DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute,
    };
    use windows::Win32::Graphics::Gdi::{
        ClientToScreen, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_HWNDPREV, GWL_EXSTYLE, GetClientRect, GetWindow, GetWindowLongPtrW,
        GetWindowTextW, GetWindowThreadProcessId, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST, IsIconic, IsWindowVisible,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, WS_EX_TOPMOST,
    };
    use windows::core::BOOL;

    /// The first visible, uncloaked, not minimised top-level window titled exactly
    /// "MapleStory". `None` when the game isn't running or is minimised.
    pub fn find_game() -> Option<Game> {
        let mut found: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(collect), LPARAM(&mut found as *mut Vec<HWND> as isize));
        }
        found.into_iter().find_map(game)
    }

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let found = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        if is_game_window(hwnd) {
            found.push(hwnd);
        }
        BOOL(1) // keep enumerating
    }

    fn is_game_window(hwnd: HWND) -> bool {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                return false;
            }
            // Reads the caption stored by the window manager; no WM_GETTEXT is sent
            // to another process's window.
            let mut buf = [0u16; 64];
            let n = GetWindowTextW(hwnd, &mut buf);
            if n <= 0 || String::from_utf16_lossy(&buf[..n as usize]) != GAME_TITLE {
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
