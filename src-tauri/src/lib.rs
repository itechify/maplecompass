//! MapleCompass shell: a thin adapter around `compass-core`. It finds the game window,
//! captures the Title Region, feeds frames, hotkeys and drags to the core and renders
//! the panel state in a click-through, always-on-top overlay. All logic lives in the core.

pub mod capture;
pub mod gamewin;
mod hotkeys;
pub mod meowdb;
mod settings;

use compass_core::{
    Action, Compass, Frame, Hotkeys, PanelSizes, PanelState, ScreenRect, Settings, Size,
    default_title_region,
};
use gamewin::ClientArea;
use meowdb::MeowDbSource;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};
use tauri_plugin_opener::OpenerExt;

/// Capture and recognition rate: ~3 Hz keeps a map change within about a second.
const TICK: Duration = Duration::from_millis(333);
/// The overlay's size in CSS px (logical); `src/styles.css` lays out to fit. The
/// expanded panel's height follows its content, as the overlay reports it.
const EXPANDED: (f64, f64) = (380.0, 72.0);
const EXPANDED_HEIGHTS: std::ops::RangeInclusive<f64> = 44.0..=900.0;
const COLLAPSED: (f64, f64) = (44.0, 44.0);

/// What the compass loop is asked to do.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Command {
    /// Something the player did, for the core.
    Act(Action),
    /// Open the shown map's MeowDB page in the browser.
    OpenMeowDbPage,
    /// The expanded panel's content is this tall (CSS px).
    PanelHeight(f64),
}

/// The overlay reports how tall its expanded content is, so the window fits it.
#[tauri::command]
fn panel_height(height: f64, commands: tauri::State<'_, Sender<Command>>) {
    let _ = commands.send(Command::PanelHeight(height));
}

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
    hotkeys: &'a Hotkeys,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![panel_height])
        .setup(|app| {
            let overlay = app.get_webview_window("overlay").expect("overlay window in tauri.conf.json");
            // Keep our own panel out of our own (and any) screen capture.
            overlay.set_content_protected(true)?;
            // Clicks pass through to the game until the player enters edit mode.
            overlay.set_ignore_cursor_events(true)?;

            let settings_path = settings::path(app.handle())?;
            let settings = settings::load(&settings_path);
            let (commands, commands_rx) = mpsc::channel();
            hotkeys::install(app, &settings.hotkeys, commands.clone())?;
            app.manage(commands.clone());
            // The core ignores moves outside edit mode, so the shell's own repositioning
            // is harmless; in edit mode they are the player's drag.
            overlay.on_window_event(move |event| {
                if let WindowEvent::Moved(to) = event {
                    let _ = commands.send(Command::Act(Action::PanelMoved { x: to.x, y: to.y }));
                }
            });

            // MeowDB pages are cached on disk (ADR 0003). WebView2 shares the app cache
            // folder, hence the subfolder.
            let source = MeowDbSource::new(app.path().app_cache_dir()?.join("cache"))?;
            let handle = app.handle().clone();
            // Capture + OCR are blocking and CPU-bound: run them on their own thread.
            // Spawned from setup, so the process is already Per-Monitor-V2 DPI aware.
            std::thread::Builder::new().name("compass-loop".into()).spawn(move || {
                compass_loop(handle, overlay, source, commands_rx, settings, settings_path)
            })?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn compass_loop(
    app: AppHandle,
    overlay: WebviewWindow,
    source: MeowDbSource,
    commands: Receiver<Command>,
    settings: Settings,
    settings_path: PathBuf,
) {
    let hotkeys = settings.hotkeys.clone();
    let mut compass = Compass::new(source);
    compass.restore(settings);
    let mut saved = compass.settings();
    let mut click_through = true;
    let mut expanded = EXPANDED;
    let start = Instant::now();
    let mut next_tick = Instant::now();
    loop {
        // Wait for the player until the next capture is due, so hotkeys act at once.
        match commands.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
            Ok(Command::Act(action)) => {
                compass.act(action);
            }
            Ok(Command::OpenMeowDbPage) => open_meowdb_page(&app, compass.panel_state()),
            Ok(Command::PanelHeight(height)) => {
                expanded.1 = height.clamp(*EXPANDED_HEIGHTS.start(), *EXPANDED_HEIGHTS.end());
                set_panel_sizes(&mut compass, &overlay, expanded);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        // Checked after every wake-up, so a stream of drag moves can't starve capture.
        if Instant::now() >= next_tick {
            next_tick = Instant::now() + TICK;
            tick(&mut compass, &overlay, expanded, start);
        }

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

/// Find the game, capture its Title Region and feed both to the core.
fn tick(compass: &mut Compass, overlay: &WebviewWindow, expanded: (f64, f64), start: Instant) {
    set_panel_sizes(compass, overlay, expanded);
    let client = gamewin::find_game_client();
    compass.on_game_window(client.map(|c| ScreenRect { x: c.x, y: c.y, width: c.width, height: c.height }));
    if let Some(client) = client {
        match capture_title_frame(client, start.elapsed().as_millis() as u64) {
            Ok(frame) => {
                compass.on_frame(frame);
            }
            Err(e) => eprintln!("capture failed: {e}"),
        }
    }
}

/// Tell the core the panel's sizes in physical px, for the overlay's current display.
fn set_panel_sizes(compass: &mut Compass, overlay: &WebviewWindow, expanded: (f64, f64)) {
    if let Ok(scale) = overlay.scale_factor() {
        compass.set_panel_sizes(PanelSizes {
            expanded: physical(expanded, scale),
            collapsed: physical(COLLAPSED, scale),
        });
    }
}

/// Open the shown map's MeowDB page in the player's browser; without a map, nothing.
fn open_meowdb_page(app: &AppHandle, state: &PanelState) {
    let Some(map) = &state.map else { return };
    if let Err(e) = app.opener().open_url(&map.url, None::<&str>) {
        eprintln!("couldn't open {}: {e}", map.url);
    }
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
