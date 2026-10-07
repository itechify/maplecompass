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

/// Exact title of the game window, unless the player picked another by hand.
pub const GAME_TITLE: &str = "MapleStory";

#[cfg(windows)]
pub use win::{find_game_client, window_under_cursor};

#[cfg(not(windows))]
pub fn find_game_client(_title: &str) -> Option<ClientArea> {
    None
}

#[cfg(not(windows))]
pub fn window_under_cursor() -> Result<String, String> {
    Err("picking a window is only implemented on Windows so far".into())
}

#[cfg(windows)]
mod win {
    use super::ClientArea;
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GA_ROOT, GetAncestor, GetClientRect, GetCursorPos, GetWindowTextW,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, WindowFromPoint,
    };
    use windows::core::BOOL;

    /// The first visible, uncloaked top-level window titled exactly `title`. `None`
    /// when the game isn't running or is minimised.
    pub fn find_game_client(title: &str) -> Option<ClientArea> {
        let mut search: (&str, Vec<HWND>) = (title, Vec::new());
        unsafe {
            let _ = EnumWindows(Some(collect), LPARAM(&mut search as *mut (&str, Vec<HWND>) as isize));
        }
        search.1.into_iter().find_map(client_area)
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
        unsafe {
            let mut at = POINT::default();
            GetCursorPos(&mut at).map_err(|e| format!("no pointer position: {e}"))?;
            let root = GetAncestor(WindowFromPoint(at), GA_ROOT);
            if root.is_invalid() {
                return Err("no window under the pointer".into());
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(root, Some(&mut pid));
            if pid == std::process::id() {
                return Err("that is MapleCompass itself".into());
            }
            match caption(root) {
                Some(title) if !title.trim().is_empty() => Ok(title),
                _ => Err("the window under the pointer has no title".into()),
            }
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
