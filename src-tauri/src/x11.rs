//! The game window, what is in front, the pointer and the game's pixels on Linux,
//! through the X server. Under Wine or Proton the game is an X11 window (XWayland's, on
//! a Wayland desktop), and Minerva's windows are X11 windows beside it (`main.rs`), so
//! one X server tells where the game is and draws both. Only reads the window manager's
//! properties, the pointer's position and buttons, and the game window's pixels: the X
//! server sends the game nothing for any of these, and nothing here opens its process,
//! sends it an event or changes a window of its. The only windows it changes are our own
//! (ADR 0001).

use crate::gamewin::{Game, looking_for};
use minerva_core::{Foreground, ScreenPoint, ScreenRect};
use std::sync::OnceLock;
use tauri::WebviewWindow;
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ConnectionExt as _, ImageFormat, ImageOrder, InputFocus, KeyButMask, MapState, Window,
};
use x11rb::rust_connection::RustConnection;

x11rb::atom_manager! {
    Atoms: AtomsCookie {
        _NET_ACTIVE_WINDOW,
        _NET_CLIENT_LIST,
        _NET_CURRENT_DESKTOP,
        _NET_FRAME_EXTENTS,
        _NET_WM_NAME,
        _NET_WM_PID,
        _NET_WM_STATE,
        _NET_WM_STATE_HIDDEN,
        _NET_WORKAREA,
        UTF8_STRING,
        WM_STATE,
    }
}

/// Our connection to the X server, made once and shared by every thread.
struct Display {
    conn: RustConnection,
    root: Window,
    atoms: Atoms,
}

/// The X server, or why there is none (no `DISPLAY`: a Wayland desktop without XWayland).
fn display() -> Result<&'static Display, String> {
    static DISPLAY: OnceLock<Result<Display, String>> = OnceLock::new();
    DISPLAY.get_or_init(connect).as_ref().map_err(Clone::clone)
}

fn connect() -> Result<Display, String> {
    let (conn, screen) = x11rb::connect(None).map_err(|e| format!("no X server to find the game on: {e}"))?;
    let root = conn.setup().roots[screen].root;
    let atoms = Atoms::new(&conn)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| format!("the X server's atoms: {e}"))?;
    Ok(Display { conn, root, atoms })
}

impl Display {
    /// A property's value, of any type; `None` when the window doesn't have it (or is
    /// gone).
    fn property(&self, window: Window, property: Atom) -> Option<x11rb::protocol::xproto::GetPropertyReply> {
        // Up to 64 KB: far more than a title or a window list holds.
        let reply = self.conn.get_property(false, window, property, AtomEnum::ANY, 0, 16_384).ok()?.reply().ok()?;
        (reply.type_ != u32::from(AtomEnum::NONE)).then_some(reply)
    }

    /// A property of 32-bit values (windows, cardinals).
    fn numbers(&self, window: Window, property: Atom) -> Vec<u32> {
        self.property(window, property).and_then(|reply| reply.value32().map(Iterator::collect)).unwrap_or_default()
    }

    /// The window's title as the window manager stores it: `_NET_WM_NAME` (UTF-8), or
    /// else `WM_NAME` (Latin-1).
    fn title(&self, window: Window) -> Option<String> {
        if let Some(reply) = self.property(window, self.atoms._NET_WM_NAME)
            && reply.type_ == self.atoms.UTF8_STRING
        {
            return Some(String::from_utf8_lossy(&reply.value).into_owned());
        }
        let reply = self.property(window, AtomEnum::WM_NAME.into())?;
        Some(reply.value.iter().map(|&b| char::from(b)).collect())
    }

    /// Whether the window is one of ours: our process set its `_NET_WM_PID`, as GTK
    /// does for every window it makes.
    fn is_ours(&self, window: Window) -> bool {
        self.numbers(window, self.atoms._NET_WM_PID).first() == Some(&std::process::id())
    }

    /// The window that has the keyboard, as the window manager says; `None` while
    /// it is none of the X server's (a Wayland app's, under XWayland).
    fn active(&self) -> Option<Window> {
        self.numbers(self.root, self.atoms._NET_ACTIVE_WINDOW).first().copied().filter(|&window| window != 0)
    }

    /// The application windows the window manager manages, or else the root's children.
    fn clients(&self) -> Vec<Window> {
        let clients = self.numbers(self.root, self.atoms._NET_CLIENT_LIST);
        if !clients.is_empty() {
            return clients;
        }
        let Some(tree) = self.conn.query_tree(self.root).ok().and_then(|tree| tree.reply().ok()) else {
            return Vec::new();
        };
        tree.children.into_iter().filter_map(|child| self.client_of(child, 4)).collect()
    }

    /// The application window in `window`, a top-level window: itself, or the one in the
    /// frame a window manager drew around it (the window with `WM_STATE`), up to `depth`
    /// levels down.
    fn client_of(&self, window: Window, depth: u32) -> Option<Window> {
        if self.property(window, self.atoms.WM_STATE).is_some() {
            return Some(window);
        }
        if depth == 0 {
            return None;
        }
        let tree = self.conn.query_tree(window).ok()?.reply().ok()?;
        tree.children.into_iter().rev().find_map(|child| self.client_of(child, depth - 1))
    }

    /// Whether `window` is the game's, titled `title`: showing, not minimised, and not
    /// ours.
    fn is_game_window(&self, window: Window, title: &str) -> bool {
        let Some(attributes) = self.conn.get_window_attributes(window).ok().and_then(|a| a.reply().ok()) else {
            return false;
        };
        attributes.map_state == MapState::VIEWABLE
            && !self.is_ours(window)
            && self.title(window).as_deref() == Some(title)
            && !self.numbers(window, self.atoms._NET_WM_STATE).contains(&self.atoms._NET_WM_STATE_HIDDEN)
    }

    fn game(&self, window: Window) -> Option<Game> {
        let geometry = self.conn.get_geometry(window).ok()?.reply().ok()?;
        let origin = self.conn.translate_coordinates(window, self.root, 0, 0).ok()?.reply().ok()?;
        let (width, height) = (u32::from(geometry.width), u32::from(geometry.height));
        if width == 0 || height == 0 {
            return None;
        }
        let client = ScreenRect { x: origin.dst_x.into(), y: origin.dst_y.into(), width, height };
        // The window manager's frame around it, if it draws one.
        let frame = match self.numbers(window, self.atoms._NET_FRAME_EXTENTS)[..] {
            [left, right, top, bottom] => ScreenRect {
                x: client.x - left as i32,
                y: client.y - top as i32,
                width: width + left + right,
                height: height + top + bottom,
            },
            _ => client,
        };
        Some(Game { handle: window as isize, client, frame })
    }
}

/// The first showing, not minimised window titled exactly "MapleStory" (or as picked by
/// hand). `None` when the game isn't running or is minimised.
pub fn find_game() -> Option<Game> {
    let x = display().ok()?;
    let title = looking_for();
    x.clients().into_iter().filter(|&window| x.is_game_window(window, &title)).find_map(|window| x.game(window))
}

/// Where the mouse pointer is on screen (physical px), for Item Lookup. Under XWayland
/// the X server follows it only over X11 windows, such as the game.
pub fn cursor_position() -> Option<ScreenPoint> {
    let x = display().ok()?;
    let pointer = x.conn.query_pointer(x.root).ok()?.reply().ok()?;
    Some(ScreenPoint { x: pointer.root_x.into(), y: pointer.root_y.into() })
}

/// Where the mouse pointer is on screen (physical px), and whether its left button is
/// down, for the shell's own drags of our windows (`sizemove.rs`). While the player holds
/// the button down on our window, the pointer is ours until they let go.
pub fn pointer() -> Option<(ScreenPoint, bool)> {
    let x = display().ok()?;
    let pointer = x.conn.query_pointer(x.root).ok()?.reply().ok()?;
    let at = ScreenPoint { x: pointer.root_x.into(), y: pointer.root_y.into() };
    Some((at, pointer.mask.contains(KeyButMask::BUTTON1)))
}

/// The title of the top-level window under the mouse pointer, for picking the game
/// window by hand. Only reads the pointer position, which window is there and its
/// stored title.
pub fn window_under_cursor() -> Result<String, String> {
    let x = display()?;
    let pointer = x
        .conn
        .query_pointer(x.root)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| format!("no pointer position: {e}"))?;
    let window = (pointer.child != 0)
        .then(|| x.client_of(pointer.child, 4).unwrap_or(pointer.child))
        .ok_or("no window under the pointer")?;
    if x.is_ours(window) {
        return Err("that is Minerva itself".into());
    }
    match x.title(window) {
        Some(title) if !title.trim().is_empty() => Ok(title),
        _ => Err("the window under the pointer has no title".into()),
    }
}

/// Every monitor, less what the desktop's panels take when the window manager says.
pub fn monitor_work_areas() -> Vec<ScreenRect> {
    let Ok(x) = display() else { return Vec::new() };
    let Some(monitors) = x.conn.randr_get_monitors(x.root, true).ok().and_then(|m| m.reply().ok()) else {
        return Vec::new();
    };
    // One work area per desktop, over all the monitors.
    let desktop = x.numbers(x.root, x.atoms._NET_CURRENT_DESKTOP).first().copied().unwrap_or(0) as usize;
    let work_area = match x.numbers(x.root, x.atoms._NET_WORKAREA).get(desktop * 4..desktop * 4 + 4) {
        Some(&[x, y, width, height]) => Some(ScreenRect { x: x as i32, y: y as i32, width, height }),
        _ => None,
    };
    monitors
        .monitors
        .iter()
        .map(|m| ScreenRect { x: m.x.into(), y: m.y.into(), width: m.width.into(), height: m.height.into() })
        .map(|monitor| work_area.and_then(|area| intersection(monitor, area)).unwrap_or(monitor))
        .collect()
}

fn intersection(a: ScreenRect, b: ScreenRect) -> Option<ScreenRect> {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width as i32).min(b.x + b.width as i32);
    let bottom = (a.y + a.height as i32).min(b.y + b.height as i32);
    (right > left && bottom > top).then(|| ScreenRect { x: left, y: top, width: (right - left) as u32, height: (bottom - top) as u32 })
}

/// Which window is in front: the `game` (its window, if it was found), one of ours, or
/// another app's (a Wayland app's too, which the X server doesn't know).
pub fn foreground(game: Option<isize>) -> Foreground {
    let Ok(x) = display() else { return Foreground::OtherApp };
    match x.active() {
        Some(active) if game == Some(active as isize) => Foreground::Game,
        Some(active) if x.is_ours(active) => Foreground::Minerva,
        _ => Foreground::OtherApp,
    }
}

/// Whether the window in front is one of ours (e.g. focus only moved inside our window,
/// into its webview), rather than the game or another app.
pub fn foreground_is_ours() -> bool {
    display().is_ok_and(|x| x.active().is_some_and(|active| x.is_ours(active)))
}

/// Capture `w`x`h` physical pixels at screen `(x, y)`, inside the game's client area, as
/// tightly packed RGBA, top-down rows. Copied from the game window itself: under
/// XWayland the X server has no picture of the whole screen, only of each X11 window's.
/// The X server copies what the game last drew and tells it nothing, as with any
/// screenshot tool's window capture.
pub fn capture_rgba(x: i32, y: i32, w: i32, h: i32) -> Result<Vec<u8>, String> {
    if w <= 0 || h <= 0 {
        return Err(format!("empty capture {w}x{h}"));
    }
    let display = display()?;
    let game = find_game().ok_or("no game window to capture")?;
    let (left, top) = (x - game.client.x, y - game.client.y);
    if left < 0 || top < 0 || left + w > game.client.width as i32 || top + h > game.client.height as i32 {
        return Err(format!("{w}x{h} at ({x}, {y}) isn't inside the game window"));
    }
    let image = display
        .conn
        .get_image(ImageFormat::Z_PIXMAP, game.handle as Window, left as i16, top as i16, w as u16, h as u16, !0)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| format!("GetImage: {e}"))?;
    to_rgba(display.conn.setup(), &image, w as usize, h as usize)
}

/// The X server's picture in RGBA. Only 32 bits a pixel with 8 bits a colour, the layout
/// X servers give 24- and 32-bit windows, is read; any other is refused.
fn to_rgba(
    setup: &x11rb::protocol::xproto::Setup,
    image: &x11rb::protocol::xproto::GetImageReply,
    w: usize,
    h: usize,
) -> Result<Vec<u8>, String> {
    let format = setup.pixmap_formats.iter().find(|f| f.depth == image.depth).ok_or("no pixel format")?;
    let visual = setup
        .roots
        .iter()
        .flat_map(|screen| &screen.allowed_depths)
        .flat_map(|depth| &depth.visuals)
        .find(|visual| visual.visual_id == image.visual)
        .ok_or("no visual")?;
    let masks = [visual.red_mask, visual.green_mask, visual.blue_mask];
    if format.bits_per_pixel != 32 || masks.iter().any(|mask| mask.count_ones() != 8) {
        return Err(format!("unsupported {}-bit pixels at depth {}", format.bits_per_pixel, image.depth));
    }
    let pad = usize::from(format.scanline_pad);
    let stride = (w * 32).div_ceil(pad) * pad / 8;
    if image.data.len() < stride * h {
        return Err(format!("GetImage returned {} bytes for {w}x{h}", image.data.len()));
    }
    let big_endian = setup.image_byte_order == ImageOrder::MSB_FIRST;
    let mut rgba = Vec::with_capacity(w * h * 4);
    for row in image.data.chunks_exact(stride).take(h) {
        for px in row[..w * 4].chunks_exact(4) {
            let px = [px[0], px[1], px[2], px[3]];
            let value = if big_endian { u32::from_be_bytes(px) } else { u32::from_le_bytes(px) };
            for mask in masks {
                rgba.push(((value & mask) >> mask.trailing_zeros()) as u8);
            }
            rgba.push(0xFF);
        }
    }
    Ok(rgba)
}

/// Make our borderless `window` (the panel's, and the others laid over the game) a
/// utility window that stays above the others while it shows: window managers float such
/// windows (a tiling one too), put them where we say, and don't give them the keyboard
/// as they show. One that takes no input (`focusable: false`) doesn't get the keyboard
/// from a click either: the game keeps it. A window manager reads a window's type as it
/// shows, so a window showing already is hidden and shown again. On the main thread,
/// before the window is used.
///
/// The shell sizes these windows to what they show, so to GTK they are all resizable: GTK
/// holds a window that isn't at its webview's natural size, 200 px tall at the least,
/// which would leave a strip over the game taking its clicks. The player still never
/// resizes one by its edges (`without_edge_resizing`).
pub fn as_utility_window(window: &WebviewWindow) {
    use gtk::prelude::{GtkWindowExt, WidgetExt};
    let Ok(gtk) = window.gtk_window() else { return };
    let shows = gtk.is_visible();
    if shows {
        gtk.hide();
    }
    gtk.set_type_hint(gtk::gdk::WindowTypeHint::Utility);
    gtk.set_keep_above(true);
    // Tao sets the window's resizable flag from `tauri.conf.json` as it is first placed,
    // in a handler that runs before this one.
    gtk.connect_configure_event(|gtk, _| {
        if !gtk.is_resizable() {
            gtk.set_resizable(true);
        }
        false
    });
    if shows {
        gtk.show_all();
    }
    without_edge_resizing(gtk);
}

/// The player resizes the panel by the page's grips only (`sizemove.rs`), and our other
/// windows not at all. Tao takes a press within 5 px of a resizable borderless window's
/// edges before the page hears it, and starts the window manager's resize from it, in any
/// direction, which nobody hears the end of (and it shows a resize pointer there). Those
/// are tao's handlers of the press and of the pointer's moves on the window itself; tao's
/// others there only report the mouse as window events, which Minerva doesn't use. The
/// page has the mouse from the window's webview, which they leave alone. Tao connects them
/// on its first turn of the main loop, after setup, so they go once the main loop is idle.
fn without_edge_resizing(gtk: gtk::ApplicationWindow) {
    use gtk::glib::gobject_ffi::{G_SIGNAL_MATCH_ID, g_signal_handlers_disconnect_matched, g_signal_lookup};
    use gtk::glib::object::ObjectType;
    use gtk::glib::translate::IntoGlib;
    use gtk::glib::types::StaticType;
    gtk::glib::idle_add_local_once(move || {
        for signal in [c"button-press-event", c"motion-notify-event"] {
            // SAFETY: the signal is looked up by name on GtkWidget, and only handlers
            // connected to it on our own window are disconnected.
            unsafe {
                let id = g_signal_lookup(signal.as_ptr(), gtk::Widget::static_type().into_glib());
                let (closure, func, data) = (std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
                g_signal_handlers_disconnect_matched(gtk.as_ptr().cast(), G_SIGNAL_MATCH_ID, id, 0, closure, func, data);
            }
        }
    });
}

/// Give our `window` the keyboard: ask the window manager to activate it, as GTK does
/// (`_NET_ACTIVE_WINDOW`), and move the X server's keyboard focus to it, which a window
/// manager that already counts it in front (after the player's click on it) only lets
/// stand. Only our own window is activated; the game hears that it lost the keyboard, as
/// from any app the player switches to.
pub fn take_focus(window: &WebviewWindow) -> Result<(), String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    window.set_focus().map_err(|e| e.to_string())?;
    let handle = window.window_handle().map_err(|e| e.to_string())?;
    let ours = match handle.as_raw() {
        RawWindowHandle::Xlib(handle) => handle.window as Window,
        RawWindowHandle::Xcb(handle) => handle.window.get(),
        _ => return Err("not an X11 window".into()),
    };
    let x = display()?;
    x.conn.set_input_focus(InputFocus::PARENT, ours, x11rb::CURRENT_TIME).map_err(|e| e.to_string())?;
    x.conn.flush().map_err(|e| e.to_string())
}
