//! Game window locator (Windows). Reads window-manager state only: it never sends the
//! game a message, opens its process or hooks it (ADR 0001).

/// The game window's client area in physical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Exact title of the game window.
pub const GAME_TITLE: &str = "MapleStory";

#[cfg(windows)]
pub use win::find_game_client;

#[cfg(not(windows))]
pub fn find_game_client() -> Option<ClientArea> {
    None
}

#[cfg(windows)]
mod win {
    use super::{ClientArea, GAME_TITLE};
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClientRect, GetWindowTextW, IsIconic, IsWindowVisible,
    };
    use windows::core::BOOL;

    /// The first visible, uncloaked top-level window titled exactly "MapleStory".
    /// `None` when the game isn't running or is minimised.
    pub fn find_game_client() -> Option<ClientArea> {
        let mut found: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(collect), LPARAM(&mut found as *mut Vec<HWND> as isize));
        }
        found.into_iter().find_map(client_area)
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

    fn client_area(hwnd: HWND) -> Option<ClientArea> {
        unsafe {
            if IsIconic(hwnd).as_bool() {
                return None;
            }
            let mut rect = RECT::default();
            GetClientRect(hwnd, &mut rect).ok()?;
            let mut origin = POINT { x: 0, y: 0 };
            if !ClientToScreen(hwnd, &mut origin).as_bool() {
                return None;
            }
            let (width, height) = ((rect.right - rect.left) as u32, (rect.bottom - rect.top) as u32);
            (width > 0 && height > 0).then_some(ClientArea { x: origin.x, y: origin.y, width, height })
        }
    }
}
