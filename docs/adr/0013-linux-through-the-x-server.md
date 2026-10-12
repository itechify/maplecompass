---
status: accepted
---

# Linux through the X server, beside the game under Wine or Proton

On Linux, MapleStory Classic World runs under Wine or Proton (Steam, Heroic, Lutris). Wine makes the game an X11 window: XWayland's, on a Wayland desktop. Wine's Wayland driver is opt-in, and we don't support it. So Minerva makes its own windows X11 windows too (`GDK_BACKEND=x11`, set in `main.rs`), and the shell asks one X server where the game is, what is in front and what the game shows (`src-tauri/src/x11.rs`). Everything ADR 0001 says still holds. Only its Windows calls change:

- **Finding the game:** the window manager's list of application windows (`_NET_CLIENT_LIST`), each one's stored title (`_NET_WM_NAME`, or else `WM_NAME`), whether it is mapped, and whether it is minimised (`_NET_WM_STATE_HIDDEN`). Its client area comes from `GetGeometry` and `TranslateCoordinates`. A window is ours when its `_NET_WM_PID` is our process, which GTK sets on every window it makes.
- **What is in front:** `_NET_ACTIVE_WINDOW` on the root window, in place of `GetForegroundWindow`. While a Wayland app is in front, the X server knows no active window, and that counts as another app.
- **Capture:** `GetImage` of the game's window. The root window can't serve, because under XWayland the X server holds no picture of the whole screen, only each X11 window's. The X server copies the pixels it holds and sends the game nothing; `PrintWindow`, by contrast, sends the game `WM_PRINT`. As a side effect, a window covering the game is never in the capture, and neither is one of ours.
- **Hotkeys:** a passive grab on the root window (`XGrabKey`, through Tauri's global-shortcut plugin). The X server tells us when the player presses one of our combinations while an X11 window has the keyboard: under XWayland, the game or one of ours. The hotkeys are only live then anyway. We never use XRecord, XInput's raw events or `/dev/input`.
- **The pointer:** `QueryPointer`, in place of `GetCursorPos`, for Item Lookup. Also for our own drags (below).
- **Our windows:** the overlay windows and the region-marking window are utility windows (`_NET_WM_WINDOW_TYPE_UTILITY`), kept above (`_NET_WM_STATE_ABOVE`). Window managers float utility windows, a tiling one included, and put them where we say. A window that takes no input (`WM_HINTS` input off, from `focusable: false`) is our `WS_EX_NOACTIVATE`: a click on it leaves the X keyboard focus with the game. Out of sight is hidden, not the bottom of the z-order. Showing such a window activates nothing. Every window manager raises a window as it shows, and one that keeps no window above the others would still draw its border around an empty window.
- **Taking the keyboard** for the search view and the Item Card's fields: we ask the window manager to activate our window (`_NET_ACTIVE_WINDOW`, GTK's `present`) and move the X keyboard focus to it (`SetInputFocus`), in place of `SetForegroundWindow`. The window manager may refuse both. Then the player's click on the field gives it the keyboard. Giving the keyboard back hides our window and shows it again, as on Windows.
- **The player's drags and resizes** of our windows: the shell runs the move or size loop itself (`sizemove.rs`). While the player holds the button down on our window, the pointer is ours until they let go, and the shell moves or sizes the window after it. Window managers run their own loops in different ways, and none of them says when one ends. Tao's own resizing from a borderless window's edges is turned off: it would start the window manager's loop.

We never send the game input (no XTest) and never send an event to any of its windows. We never set an owner or transient link to its window (`WM_TRANSIENT_FOR`). We never read its memory, its process (`/proc`, `ptrace`) or its files.

## Why not native Wayland

A Wayland client can't place a window over another app's window, and can't tell which window is in front. It can only capture another window through the desktop's screen-sharing portal, which asks the player and shows that the screen is shared. That is no overlay. So Minerva needs XWayland, which every mainstream Wayland desktop runs.

## Consequences

- GNOME, KDE Plasma, Xfce, Cinnamon and other window managers that follow the EWMH standard keep the panel above the game, and don't give it the keyboard on a click.
- **Hyprland** floats our utility windows, but ignores `_NET_WM_STATE_ABOVE`. When the player clicks the game's window and it floats, it rises over the panel. A window rule that pins Minerva's windows keeps them above everything (the site's FAQ gives it). A click on our window makes it Hyprland's active window. But with input off, Hyprland leaves the X keyboard focus with the game, so the game keeps the typing, and Minerva counts one of its own windows in front. Hyprland ignores an app's request to activate its window unless `misc:focus_on_activate` is on. So the search view opened by its hotkey gets the keyboard once the player clicks its box. Opened by a click on 🔍, it gets it at once.
- **Steam Deck:** Game Mode (gamescope) shows one app at a time, so the panel can't show over the game. Desktop Mode works.
- Minerva doesn't run on a Wayland desktop without XWayland, nor beside a game under Wine's Wayland driver.
- The Linux packages (`.deb`, `.rpm`, AppImage) are built on GitHub's oldest Ubuntu runner when a release is published (`docs/releasing.md`). A build on a newer system would need a newer C library than many players have.
