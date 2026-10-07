//! MapleCompass shell: a thin adapter around `compass-core`. It finds the game window,
//! captures the Title Region, feeds frames to the core and renders the panel state in
//! a click-through, always-on-top overlay. All logic lives in the core.

pub mod capture;
pub mod gamewin;

use compass_core::{Compass, Frame, PanelState, default_title_region};
use gamewin::ClientArea;
use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

/// Capture and recognition rate: ~3 Hz keeps a map change within about a second.
const TICK: Duration = Duration::from_millis(333);
/// Gap between the overlay and the game client's right edge (physical px).
const OVERLAY_MARGIN: i32 = 16;

/// Capture the default Title Region of the game's client area.
pub fn capture_title_frame(client: ClientArea, at_ms: u64) -> Result<Frame, String> {
    let r = default_title_region(client.width, client.height);
    let rgba = capture::capture_rgba(
        client.x + r.x as i32,
        client.y + r.y as i32,
        r.width as i32,
        r.height as i32,
    )?;
    Ok(Frame { width: r.width, height: r.height, rgba, at_ms })
}

/// What the overlay webview renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OverlayState<'a> {
    game_found: bool,
    panel: &'a PanelState,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let overlay = app.get_webview_window("overlay").expect("overlay window in tauri.conf.json");
            // Keep our own panel out of our own (and any) screen capture.
            overlay.set_content_protected(true)?;
            // Clicks pass through to the game.
            overlay.set_ignore_cursor_events(true)?;
            let handle = app.handle().clone();
            // Capture + OCR are blocking and CPU-bound: run them on their own thread.
            // Spawned from setup, so the process is already Per-Monitor-V2 DPI aware.
            std::thread::Builder::new()
                .name("compass-loop".into())
                .spawn(move || compass_loop(handle, overlay))?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn compass_loop(app: AppHandle, overlay: WebviewWindow) {
    let mut compass = Compass::new();
    let start = Instant::now();
    loop {
        let tick = Instant::now();
        let client = gamewin::find_game_client();
        if let Some(client) = client {
            match capture_title_frame(client, start.elapsed().as_millis() as u64) {
                Ok(frame) => {
                    compass.on_frame(frame);
                }
                Err(e) => eprintln!("capture failed: {e}"),
            }
            place_overlay(&overlay, client);
        }
        let state = OverlayState { game_found: client.is_some(), panel: compass.panel_state() };
        if let Err(e) = app.emit_to("overlay", "overlay-state", &state) {
            eprintln!("emit failed: {e}");
        }
        std::thread::sleep(TICK.saturating_sub(tick.elapsed()));
    }
}

/// Stopgap placement until the core computes it (#9, #10): inside the game window,
/// vertically centred on its right edge, clear of the minimap and Quest Helper.
fn place_overlay(overlay: &WebviewWindow, client: ClientArea) {
    let Ok(size) = overlay.outer_size() else { return };
    let x = client.x + client.width as i32 - size.width as i32 - OVERLAY_MARGIN;
    let y = client.y + (client.height as i32 - size.height as i32) / 2;
    let to = PhysicalPosition::new(x, y);
    if overlay.outer_position().ok() != Some(to) {
        let _ = overlay.set_position(to);
    }
}
