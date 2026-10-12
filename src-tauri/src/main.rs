// Prevents an additional console window on Windows in release. DO NOT REMOVE!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // On Linux our windows are X11 windows, as the game's are under Wine or Proton
    // (XWayland's on a Wayland desktop): a Wayland window can't place itself over another
    // app's window, nor tell which window is in front (`x11.rs`).
    // SAFETY: nothing else runs yet to read the environment.
    #[cfg(target_os = "linux")]
    unsafe {
        std::env::set_var("GDK_BACKEND", "x11")
    };
    minerva_lib::run()
}
