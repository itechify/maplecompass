//! MapleCompass shell: a thin adapter around `compass-core`. It finds the game window,
//! captures the Title Region, feeds frames, hotkeys and drags to the core and renders
//! the panel state in a click-through, always-on-top overlay. All logic lives in the core.

pub mod capture;
pub mod gamewin;
mod hotkeys;
pub mod meowdb;
mod settings;
mod setup;

use compass_core::{
    Action, Compass, Frame, Hotkeys, PanelSizes, PanelState, ScreenRect, Settings, Size,
    default_title_region,
};
use gamewin::ClientArea;
use meowdb::MeowDbSource;
use serde::Serialize;
use setup::{Bridge, SetupWindows};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};

/// Capture and recognition rate: ~3 Hz keeps a map change within about a second.
const TICK: Duration = Duration::from_millis(333);
/// The overlay's size in CSS px (logical); `src/styles.css` lays out to fit.
const EXPANDED: (f64, f64) = (340.0, 72.0);
const COLLAPSED: (f64, f64) = (44.0, 44.0);

/// Capture the default Title Region of the game's client area (the probe's guess; the
/// app captures the region the player marked at setup).
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
    hotkeys: &'a Hotkeys,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let overlay = app.get_webview_window("overlay").expect("overlay window in tauri.conf.json");
            // Keep our own panel out of our own (and any) screen capture.
            overlay.set_content_protected(true)?;
            // Clicks pass through to the game until the player enters edit mode.
            overlay.set_ignore_cursor_events(true)?;

            let settings_path = settings::path(app.handle())?;
            let settings = settings::load(&settings_path);
            let (actions, actions_rx) = mpsc::channel();
            let (picks, picks_rx) = mpsc::channel();
            app.manage(Bridge {
                actions: actions.clone(),
                picks,
                game: Mutex::new(None),
                needed: Mutex::new(None),
            });
            let setup_windows = SetupWindows::new(app.handle())?;
            hotkeys::install(app, &settings.hotkeys, actions.clone())?;
            // The core ignores moves outside edit mode, so the shell's own repositioning
            // is harmless; in edit mode they are the player's drag.
            overlay.on_window_event(move |event| {
                if let WindowEvent::Moved(to) = event {
                    let _ = actions.send(Action::PanelMoved { x: to.x, y: to.y });
                }
            });

            // MeowDB pages are cached on disk (ADR 0003). WebView2 shares the app cache
            // folder, hence the subfolder.
            let source = MeowDbSource::new(app.path().app_cache_dir()?.join("cache"))?;
            let handle = app.handle().clone();
            // Capture + OCR are blocking and CPU-bound: run them on their own thread.
            // Spawned from setup, so the process is already Per-Monitor-V2 DPI aware.
            let channels = Channels { actions: actions_rx, picks: picks_rx };
            std::thread::Builder::new().name("compass-loop".into()).spawn(move || {
                compass_loop(handle, overlay, setup_windows, source, channels, settings, settings_path)
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            setup::pick_game_window,
            setup::start_marking,
            setup::mark_title_region,
            setup::cancel_marking,
            setup::finish_setup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// What the compass loop hears from hotkeys, the overlay and the setup window.
struct Channels {
    actions: Receiver<Action>,
    /// Titles of game windows the player picked by hand.
    picks: Receiver<String>,
}

fn compass_loop(
    app: AppHandle,
    overlay: WebviewWindow,
    mut setup_windows: SetupWindows,
    source: MeowDbSource,
    channels: Channels,
    settings: Settings,
    settings_path: PathBuf,
) {
    let hotkeys = settings.hotkeys.clone();
    let mut compass = Compass::new(source);
    compass.restore(settings);
    let mut saved = compass.settings();
    let mut click_through = true;
    let mut game = None;
    let start = Instant::now();
    let mut next_tick = Instant::now();
    loop {
        // Wait for the player until the next capture is due, so hotkeys act at once.
        match channels.actions.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
            Ok(action) => {
                compass.act(action);
                if action == Action::RequestSetup {
                    setup_windows.open();
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(title) = channels.picks.try_recv() {
            compass.pick_game_window(title);
            next_tick = Instant::now(); // look for it at once
        }
        let game_title = compass.settings().game_window_title.unwrap_or_else(|| gamewin::GAME_TITLE.into());
        // Checked after every wake-up, so a stream of drag moves can't starve capture.
        if Instant::now() >= next_tick {
            next_tick = Instant::now() + TICK;
            game = tick(&mut compass, &overlay, &game_title, start);
        }
        setup_windows.sync(compass.panel_state(), game, &game_title, &hotkeys.setup);

        let state = compass.panel_state();
        let overlay_state = OverlayState { game_found: state.placement.is_some(), panel: state, hotkeys: &hotkeys };
        if let Err(e) = app.emit_to("overlay", "overlay-state", &overlay_state) {
            eprintln!("emit failed: {e}");
        }
        // Only edit mode takes the mouse; outside it every click reaches the game.
        if click_through != !state.edit_mode {
            click_through = !state.edit_mode;
            if let Err(e) = overlay.set_ignore_cursor_events(click_through) {
                eprintln!("click-through toggle failed: {e}");
            }
        }
        if let Some(placement) = state.placement {
            place_overlay(&overlay, placement, state.edit_mode);
        }

        // Save once a drag is over (edit mode left), not on every move during it.
        let current = compass.settings();
        if !compass.panel_state().edit_mode && current != saved {
            if let Err(e) = settings::save(&settings_path, &current) {
                eprintln!("saving settings failed: {e}");
            }
            saved = current;
        }
    }
}

/// Find the game, capture the Title Region the core asks for and feed both to the core.
/// Returns the game's client area, if found.
fn tick(compass: &mut Compass, overlay: &WebviewWindow, game_title: &str, start: Instant) -> Option<ClientArea> {
    if let Ok(scale) = overlay.scale_factor() {
        compass.set_panel_sizes(PanelSizes { expanded: physical(EXPANDED, scale), collapsed: physical(COLLAPSED, scale) });
    }
    let client = gamewin::find_game_client(game_title);
    let state = compass.on_game_window(client.map(|c| ScreenRect { x: c.x, y: c.y, width: c.width, height: c.height }));
    // None until setup is done, and after the game's resolution changes.
    if let Some(r) = state.title_region {
        match capture::capture_rgba(r.x, r.y, r.width as i32, r.height as i32) {
            Ok(rgba) => {
                let at_ms = start.elapsed().as_millis() as u64;
                compass.on_frame(Frame { width: r.width, height: r.height, rgba, at_ms });
            }
            Err(e) => eprintln!("capture failed: {e}"),
        }
    }
    client
}

fn physical((width, height): (f64, f64), scale: f64) -> Size {
    Size { width: (width * scale).round() as u32, height: (height * scale).round() as u32 }
}

/// Move and size the overlay to the core's placement. While the player drags in edit
/// mode the position is theirs; it is only re-anchored when the panel changes size.
fn place_overlay(overlay: &WebviewWindow, placement: ScreenRect, edit_mode: bool) {
    let size = PhysicalSize::new(placement.width, placement.height);
    let resized = overlay.inner_size().ok() != Some(size);
    if resized {
        let _ = overlay.set_size(size);
    }
    let to = PhysicalPosition::new(placement.x, placement.y);
    if (!edit_mode || resized) && overlay.outer_position().ok() != Some(to) {
        let _ = overlay.set_position(to);
    }
}
