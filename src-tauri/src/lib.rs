//! MapleCompass shell: a thin adapter around `compass-core`. It finds the game window,
//! captures the Title Region, feeds frames, hotkeys and drags to the core and renders
//! the panel state in an overlay that sits just above the game (or, detached, in an
//! ordinary window). All logic lives in the core.

pub mod capture;
pub mod gamewin;
mod files;
mod hotkeys;
pub mod meowdb;
mod search;
mod settings;
mod setup;

use compass_core::{
    Action, Compass, Frame, GameWindow, Layer, PanelSizes, PanelState, ScreenRect, Size, TitleReader,
    TitleReading,
    default_title_region,
};
use meowdb::MeowDbSource;
use search::SearchWindow;
use setup::{Bridge, SetupWindows};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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
#[derive(Debug, Clone)]
pub(crate) enum Command {
    /// Something the player did in the overlay or the setup window, for the core.
    Act(Action),
    /// The player pressed this hotkey (its accelerator string), for the core.
    Hotkey(String),
    /// The search window's text box now holds this, for the core.
    SearchQuery(String),
    /// The expanded panel's content is this tall (CSS px).
    PanelHeight(f64),
    /// The title reader is done with a capture of `region`: what it read, or `None`
    /// if the capture failed.
    Read { region: ScreenRect, reading: Option<TitleReading> },
}

/// The overlay reports how tall its expanded content is, so the window fits it.
#[tauri::command]
fn panel_height(height: f64, commands: tauri::State<'_, Sender<Command>>) {
    let _ = commands.send(Command::PanelHeight(height));
}

/// Capture the default Title Region of the game's client area (the probe's guess; the
/// app captures the region the player marked at setup).
pub fn capture_title_frame(client: ScreenRect, at_ms: u64) -> Result<Frame, String> {
    let r = default_title_region(client.width, client.height);
    let rgba = capture::capture_rgba(
        client.x + r.x as i32,
        client.y + r.y as i32,
        r.width as i32,
        r.height as i32,
    )?;
    Ok(Frame { width: r.width, height: r.height, rgba, at_ms })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let overlay = app.get_webview_window("overlay").expect("overlay window in tauri.conf.json");
            // Not content-protected: the panel shows in the player's screenshots and
            // streams. The core keeps it off the Title Region instead, so our own capture
            // never reads it.
            // Clicks pass through to the game until the panel is draggable.
            overlay.set_ignore_cursor_events(true)?;

            // MeowDB pages are cached on disk (ADR 0003). WebView2 shares the app cache
            // folder, hence the subfolder.
            let source = MeowDbSource::new(app.path().app_cache_dir()?.join("cache"))?;
            let mut compass = Compass::new(source);
            let settings_path = settings::path(app.handle())?;
            // The core brings older settings up to date (hotkeys included).
            compass.restore(settings::load(&settings_path));
            let (commands, commands_rx) = mpsc::channel();
            let unavailable = hotkeys::install(app, &compass.panel_state().hotkeys, commands.clone())?;
            compass.hotkeys_unavailable(unavailable);
            app.manage(commands.clone());
            let (picks, picks_rx) = mpsc::channel();
            app.manage(Bridge {
                commands: commands.clone(),
                picks,
                game: Mutex::new(None),
                view: Mutex::new(None),
            });
            let setup_windows = SetupWindows::new(app.handle())?;
            let search_window = SearchWindow::new(app.handle(), commands.clone());
            // The core only takes moves of a draggable panel, so the shell's own
            // repositioning is harmless; otherwise they are the player's drag.
            overlay.on_window_event(move |event| {
                if let WindowEvent::Moved(to) = event {
                    let _ = commands.send(Command::Act(Action::PanelMoved { x: to.x, y: to.y }));
                }
            });

            let handle = app.handle().clone();
            // Both off the UI thread. Spawned from setup, so the process is already
            // Per-Monitor-V2 DPI aware.
            let reader = TitleReaderThread::spawn(app.state::<Sender<Command>>().inner().clone())?;
            let channels = Channels { commands: commands_rx, picks: picks_rx };
            let windows = Windows { overlay, setup: setup_windows, search: search_window };
            std::thread::Builder::new().name("compass-loop".into()).spawn(move || {
                compass_loop(handle, windows, compass, reader, channels, settings_path)
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            panel_height,
            setup::setup_view,
            setup::pick_game_window,
            setup::start_marking,
            setup::adjust_title_selection,
            setup::confirm_title_selection,
            setup::mark_title_region,
            setup::cancel_marking,
            setup::finish_setup,
            search::search_view,
            search::search_query,
            search::search_move,
            search::search_enter,
            search::search_escape,
            search::search_toggle_taxis,
            search::search_toggle_teleport_scrolls,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Our windows: the overlay, setup (with its region window) and mob search.
struct Windows {
    overlay: WebviewWindow,
    setup: SetupWindows,
    search: SearchWindow,
}

/// What the compass loop hears from hotkeys, the overlay, the setup window and the
/// title reader.
struct Channels {
    commands: Receiver<Command>,
    /// Titles of game windows the player picked by hand.
    picks: Receiver<String>,
}

/// Captures and reads the Title Region on its own thread. Reading a title takes tens of
/// ms (far more in a debug build), so the compass loop never waits for it: it keeps
/// following the game window at its own pace and takes each reading when it arrives.
struct TitleReaderThread {
    regions: Sender<ScreenRect>,
    /// The clock frames and ticks are timed by.
    start: Instant,
    /// A capture was asked for and its reading hasn't come back yet.
    busy: bool,
}

impl TitleReaderThread {
    fn spawn(commands: Sender<Command>) -> std::io::Result<Self> {
        let start = Instant::now();
        let (regions, asked) = mpsc::channel::<ScreenRect>();
        std::thread::Builder::new().name("title-reader".into()).spawn(move || {
            let reader = TitleReader::bundled();
            for region in asked {
                let r = region;
                let reading = match capture::capture_rgba(r.x, r.y, r.width as i32, r.height as i32) {
                    Ok(rgba) => {
                        let at_ms = start.elapsed().as_millis() as u64;
                        Some(reader.read(&Frame { width: r.width, height: r.height, rgba, at_ms }))
                    }
                    Err(e) => {
                        eprintln!("capture failed: {e}");
                        None
                    }
                };
                if commands.send(Command::Read { region, reading }).is_err() {
                    return;
                }
            }
        })?;
        Ok(Self { regions, start, busy: false })
    }

    /// Capture and read `region`, unless the last capture is still being read.
    fn read(&mut self, region: ScreenRect) {
        if !self.busy {
            self.busy = self.regions.send(region).is_ok();
        }
    }

    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

fn compass_loop(
    app: AppHandle,
    windows: Windows,
    mut compass: Compass,
    mut reader: TitleReaderThread,
    channels: Channels,
    settings_path: PathBuf,
) {
    let Windows { overlay, setup: mut setup_windows, search: mut search_window } = windows;
    let mut window = OverlayWindow::new(overlay);
    let mut saved = compass.settings();
    let mut pending = (saved.clone(), Instant::now());
    let mut emitted: Option<PanelState> = None;
    let mut game = None;
    let mut expanded = EXPANDED;
    let mut next_capture = Instant::now();
    let mut next_window_check = Instant::now();
    loop {
        // Wait for the player until the next check is due, so hotkeys act at once.
        let due = next_capture.min(next_window_check);
        match channels.commands.recv_timeout(due.saturating_duration_since(Instant::now())) {
            Ok(Command::Act(action)) => {
                compass.act(action);
            }
            Ok(Command::Hotkey(pressed)) => {
                if let Some(page) = compass.on_hotkey(&pressed) {
                    open_in_browser(&app, &page.url);
                }
            }
            Ok(Command::SearchQuery(query)) => {
                compass.search_query(&query);
            }
            Ok(Command::PanelHeight(height)) => {
                expanded.1 = height.clamp(*EXPANDED_HEIGHTS.start(), *EXPANDED_HEIGHTS.end());
                set_panel_sizes(&mut compass, &window.overlay, expanded);
            }
            Ok(Command::Read { region, reading }) => {
                reader.busy = false;
                // A reading of a region no longer captured (setup marked a new one, or
                // the game window moved) would judge the wrong spot.
                if let (Some(reading), true) = (reading, compass.panel_state().title_region == Some(region)) {
                    compass.on_reading(reading);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(title) = channels.picks.try_recv() {
            compass.pick_game_window(title);
            next_window_check = Instant::now(); // look for it at once
        }
        gamewin::look_for(&compass.panel_state().game_window_title);
        // Checked after every wake-up, so a stream of drag moves can't starve them.
        if Instant::now() >= next_window_check {
            next_window_check = Instant::now() + WINDOW_TICK;
            game = find_game(&mut compass, &window.overlay, expanded);
        }
        // Fetches in flight (map index, map pages) are checked on, never waited for.
        compass.tick(reader.now_ms());
        if Instant::now() >= next_capture {
            next_capture = Instant::now() + TICK;
            // The region the player marked; none until setup is done, nor after the
            // game's resolution changes.
            if let Some(region) = compass.panel_state().title_region {
                reader.read(region);
            }
        }
        let state = compass.panel_state();
        if state.quit {
            app.exit(0);
            return;
        }
        setup_windows.sync(state, game.map(|g| g.client));
        search_window.sync(state);
        if emitted.as_ref() != Some(state) {
            if let Err(e) = app.emit_to("overlay", "overlay-state", state) {
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
        client: g.client,
        frame: g.frame,
        screens: gamewin::monitor_work_areas(),
    }));
    game
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

/// Open a page (the shown map's on MeowDB) in the player's browser.
fn open_in_browser(app: &AppHandle, url: &str) {
    if let Err(e) = app.opener().open_url(url, None::<&str>) {
        eprintln!("couldn't open {url}: {e}");
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
        // Its own thread, at its own steady pace: the overlay should be back above the
        // game within a tick of the game coming to the front, whatever the compass loop
        // is doing (setting window sizes and positions can take a while).
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
