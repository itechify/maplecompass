//! MapleCompass shell: a thin adapter around `compass-core`. It finds the game window,
//! captures the Title Region, feeds frames, hotkeys and drags to the core and renders
//! the panel state in an overlay that sits just above the game (or, detached, in an
//! ordinary window). All logic lives in the core.

pub mod capture;
pub mod gamewin;
mod hotkeys;
pub mod meowdb;
mod settings;

use compass_core::{
    Action, Compass, Frame, GameWindow, Hotkeys, Layer, PanelSizes, PanelState, ScreenRect,
    Settings, Size, default_title_region,
};
use gamewin::ClientArea;
use meowdb::MeowDbSource;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};
use tauri_plugin_opener::OpenerExt;

/// Capture and recognition rate: ~3 Hz keeps a map change within about a second.
const TICK: Duration = Duration::from_millis(333);
/// Game window and z-order check rate: a docked panel follows a moving game window
/// and the overlay is back above the game within ~50 ms of the game coming to front.
const WINDOW_TICK: Duration = Duration::from_millis(50);
/// Settings are saved once they have stopped changing for this long (after a drag).
const SAVE_DELAY: Duration = Duration::from_millis(500);
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
            // Clicks pass through to the game until the panel is draggable.
            overlay.set_ignore_cursor_events(true)?;

            let settings_path = settings::path(app.handle())?;
            let settings = settings::load(&settings_path);
            let (commands, commands_rx) = mpsc::channel();
            hotkeys::install(app, &settings.hotkeys, commands.clone())?;
            app.manage(commands.clone());
            // The core only takes moves of a draggable panel, so the shell's own
            // repositioning is harmless; otherwise they are the player's drag.
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
    let mut window = OverlayWindow::new(overlay);
    let mut saved = compass.settings();
    let mut pending = (saved.clone(), Instant::now());
    let mut emitted: Option<PanelState> = None;
    let mut game = None;
    let mut expanded = EXPANDED;
    let start = Instant::now();
    let mut next_capture = Instant::now();
    let mut next_window_check = Instant::now();
    loop {
        // Wait for the player until the next check is due, so hotkeys act at once.
        let due = next_capture.min(next_window_check);
        match commands.recv_timeout(due.saturating_duration_since(Instant::now())) {
            Ok(Command::Act(action)) => {
                compass.act(action);
            }
            Ok(Command::OpenMeowDbPage) => open_meowdb_page(&app, compass.panel_state()),
            Ok(Command::PanelHeight(height)) => {
                expanded.1 = height.clamp(*EXPANDED_HEIGHTS.start(), *EXPANDED_HEIGHTS.end());
                set_panel_sizes(&mut compass, &window.overlay, expanded);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        // Checked after every wake-up, so a stream of drag moves can't starve them.
        if Instant::now() >= next_window_check {
            next_window_check = Instant::now() + WINDOW_TICK;
            game = find_game(&mut compass, &window.overlay, expanded);
        }
        if Instant::now() >= next_capture {
            next_capture = Instant::now() + TICK;
            if let Some(game) = game {
                match capture_title_frame(game.client, start.elapsed().as_millis() as u64) {
                    Ok(frame) => {
                        compass.on_frame(frame);
                    }
                    Err(e) => eprintln!("capture failed: {e}"),
                }
            }
        }

        let state = compass.panel_state();
        if emitted.as_ref() != Some(state) {
            let overlay_state = OverlayState { panel: state, hotkeys: &hotkeys };
            if let Err(e) = app.emit_to("overlay", "overlay-state", &overlay_state) {
                eprintln!("emit failed: {e}");
            }
            emitted = Some(state.clone());
        }
        window.show(state);

        // Save once the settings settle, not on every move of a drag.
        let current = compass.settings();
        if current != pending.0 {
            pending = (current, Instant::now());
        }
        if pending.0 != saved && pending.1.elapsed() >= SAVE_DELAY {
            if let Err(e) = settings::save(&settings_path, &pending.0) {
                eprintln!("saving settings failed: {e}");
            }
            saved = pending.0.clone();
        }
    }
}

/// Find the game window and the monitors, and feed them to the core.
fn find_game(compass: &mut Compass, overlay: &WebviewWindow, expanded: (f64, f64)) -> Option<gamewin::Game> {
    set_panel_sizes(compass, overlay, expanded);
    let game = gamewin::find_game();
    compass.on_game_window(game.map(|g| GameWindow {
        client: screen_rect(g.client),
        frame: g.frame,
        screens: gamewin::monitor_work_areas(),
    }));
    game
}

fn screen_rect(a: ClientArea) -> ScreenRect {
    ScreenRect { x: a.x, y: a.y, width: a.width, height: a.height }
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

/// CSS px to physical px, rounded up so the panel's content is never clipped.
fn physical((width, height): (f64, f64), scale: f64) -> Size {
    Size { width: (width * scale).ceil() as u32, height: (height * scale).ceil() as u32 }
}

/// The overlay window, kept in line with the panel state. Window properties are only
/// set when they change.
struct OverlayWindow {
    overlay: WebviewWindow,
    layer: Option<Layer>,
    click_through: bool,
    /// Tells the z-order keeper to hold the overlay just above the game.
    above_game: Arc<AtomicBool>,
}

impl OverlayWindow {
    fn new(overlay: WebviewWindow) -> Self {
        let above_game = Arc::new(AtomicBool::new(false));
        let (keeper_overlay, keeper_flag) = (overlay.clone(), above_game.clone());
        // Its own thread: title recognition can hold the compass loop for a few hundred
        // ms, and the overlay should be back above the game well before then.
        let spawned = std::thread::Builder::new()
            .name("z-order".into())
            .spawn(move || keep_above_game(keeper_overlay, keeper_flag));
        if let Err(e) = spawned {
            eprintln!("z-order keeper not started: {e}");
        }
        Self { overlay, layer: None, click_through: true, above_game }
    }

    fn show(&mut self, state: &PanelState) {
        let overlay = &self.overlay;
        self.above_game.store(state.layer == Layer::AboveGame, Ordering::Relaxed);
        let layer_changed = self.layer != Some(state.layer);
        if layer_changed {
            self.layer = Some(state.layer);
            // Detached, it is an ordinary window: it can be activated and found in the
            // taskbar. Otherwise it never takes focus from the game. Never topmost:
            // above the game it is kept just above it in the z-order instead.
            let free = state.layer == Layer::Free;
            let _ = overlay.set_always_on_top(false);
            let _ = overlay.set_focusable(free);
            let _ = overlay.set_skip_taskbar(!free);
        }
        // Clicks pass through unless the player can drag the panel. Hidden (the game
        // is gone) the webview draws nothing, so nothing shows and nothing is in the way.
        if self.click_through == state.draggable {
            self.click_through = !state.draggable;
            if let Err(e) = overlay.set_ignore_cursor_events(self.click_through) {
                eprintln!("click-through toggle failed: {e}");
            }
        }
        if let Some(placement) = state.placement {
            place(overlay, placement, state.draggable && !layer_changed);
        }
    }
}

/// While `above_game` is set, check every `WINDOW_TICK` that the overlay is still just
/// above the game in the z-order, and restack it if not (the game came to the front).
fn keep_above_game(overlay: WebviewWindow, above_game: Arc<AtomicBool>) {
    let mut last_error = None;
    loop {
        std::thread::sleep(WINDOW_TICK);
        if !above_game.load(Ordering::Relaxed) {
            continue;
        }
        #[cfg(windows)]
        if let (Some(game), Ok(hwnd)) = (gamewin::find_game(), overlay.hwnd()) {
            // Polled 20 times a second: report a failure once, not every time.
            let error = gamewin::keep_above(hwnd, game.handle).err().map(|e| e.to_string());
            if error.is_some() && error != last_error {
                eprintln!("keeping the overlay above the game failed: {}", error.as_deref().unwrap_or_default());
            }
            last_error = error;
        }
        #[cfg(not(windows))]
        let _ = (&overlay, &mut last_error);
    }
}

/// Move and size the overlay to the core's placement. While the player can drag it,
/// the position is theirs; it is only re-anchored when the panel changes size.
fn place(overlay: &WebviewWindow, placement: ScreenRect, player_moves_it: bool) {
    let size = PhysicalSize::new(placement.width, placement.height);
    let resized = overlay.inner_size().ok() != Some(size);
    if resized {
        let _ = overlay.set_size(size);
    }
    let to = PhysicalPosition::new(placement.x, placement.y);
    if (!player_moves_it || resized) && overlay.outer_position().ok() != Some(to) {
        let _ = overlay.set_position(to);
    }
}
