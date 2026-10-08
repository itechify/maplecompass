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
mod view;

use compass_core::{
    Action, Compass, Frame, GameWindow, Layer, PanelSizes, PanelState, Request, ScreenRect, Size, TitleReader,
    TitleReading,
    default_title_region,
};
use meowdb::MeowDbSource;
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
/// expanded panel's height follows its content, as the overlay reports it; the core
/// keeps it within the space where the panel is placed.
const EXPANDED: (f64, f64) = (380.0, 72.0);
const MIN_EXPANDED_HEIGHT: f64 = 44.0;
const COLLAPSED: (f64, f64) = (44.0, 44.0);
/// A drag has ended once the overlay hasn't moved for this long: then the shell puts it
/// where the core says again (clear of the Title Region, inside the game window).
const DRAG_SETTLE: Duration = Duration::from_millis(300);
/// How long a move of the shell's own can take to come back as a window event.
const OWN_MOVE_ECHO: Duration = Duration::from_secs(1);

/// What the compass loop is asked to do.
#[derive(Debug, Clone)]
pub(crate) enum Command {
    /// Something the player did in the overlay or the setup window, for the core.
    Act(Action),
    /// The player pressed this hotkey (its accelerator string), for the core.
    Hotkey(String),
    /// The overlay's query box now holds this, for the core.
    SearchQuery(String),
    /// The player clicked the search view's mob or map with this MeowDB ID.
    SearchPick(String),
    /// The expanded panel's content is this tall (CSS px).
    PanelHeight(f64),
    /// The overlay window moved here (its outer top-left corner): the player dragged
    /// it, or the shell moved or resized it.
    Moved(PhysicalPosition<i32>),
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
            // Clicks pass through until there is a panel to show; then it is always
            // clickable, but never activated by a click (`focusable: false` in
            // tauri.conf.json is WS_EX_NOACTIVATE), so the game keeps the keyboard.
            overlay.set_ignore_cursor_events(true)?;

            // MeowDB pages are cached on disk (ADR 0003). WebView2 shares the app cache
            // folder, hence the subfolder.
            let source = MeowDbSource::new(app.path().app_cache_dir()?.join("cache"))?;
            let mut compass = Compass::new(source);
            let settings_path = settings::path(app.handle())?;
            // The core brings older settings up to date (hotkeys included).
            compass.restore(settings::load(&settings_path));
            let (commands, commands_rx) = mpsc::channel();
            // Registered only while the game or one of our windows is in front.
            let hotkeys = hotkeys::install(app, &compass.panel_state().hotkeys, commands.clone())?;
            app.manage(commands.clone());
            let (picks, picks_rx) = mpsc::channel();
            app.manage(Bridge {
                commands: commands.clone(),
                picks,
                game: Mutex::new(None),
                view: Default::default(),
            });
            let setup_windows = SetupWindows::new(app.handle())?;
            overlay.on_window_event(move |event| match event {
                // The compass loop tells the player's drags from its own moves.
                WindowEvent::Moved(to) => {
                    let _ = commands.send(Command::Moved(*to));
                }
                // Leaving the overlay for another window (the game) with the search view
                // open is Esc: it has the keyboard only while it's in front. Not when
                // focus only moved within our windows, as into the overlay's own webview.
                WindowEvent::Focused(false) if !gamewin::foreground_is_ours() => {
                    let _ = commands.send(Command::Act(Action::SearchEscape));
                }
                _ => {}
            });

            let handle = app.handle().clone();
            // Both off the UI thread. Spawned from setup, so the process is already
            // Per-Monitor-V2 DPI aware.
            let reader = TitleReaderThread::spawn(app.state::<Sender<Command>>().inner().clone())?;
            let channels = Channels { commands: commands_rx, picks: picks_rx };
            let windows = Windows { overlay, setup: setup_windows, hotkeys };
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
            search::search_open,
            search::search_query,
            search::search_move,
            search::search_enter,
            search::search_pick,
            search::search_escape,
            search::hunting_reopen,
            search::hunting_clear,
            search::search_toggle_taxis,
            search::search_toggle_teleport_scrolls,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Our windows: the overlay (with mob search in it) and setup (with its region window).
struct Windows {
    overlay: WebviewWindow,
    setup: SetupWindows,
    /// Not a window, but registered and released as the foreground changes.
    hotkeys: hotkeys::GlobalHotkeys,
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
    let Windows { overlay, setup: mut setup_windows, mut hotkeys } = windows;
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
                match compass.on_hotkey(&pressed) {
                    Some(Request::OpenInBrowser { url }) => open_in_browser(&app, &url),
                    // Right after our hotkey, so Windows lets us take the foreground.
                    Some(Request::RaiseOverlay) => search::take_focus(&window.overlay),
                    None => {}
                }
            }
            Ok(Command::SearchQuery(query)) => {
                compass.search_query(&query);
            }
            Ok(Command::SearchPick(id)) => {
                compass.search_pick(&id);
            }
            Ok(Command::PanelHeight(height)) => {
                expanded.1 = height.max(MIN_EXPANDED_HEIGHT);
                set_panel_sizes(&mut compass, &window.overlay, expanded);
            }
            Ok(Command::Moved(to)) => {
                if window.player_moved(to) {
                    compass.act(Action::PanelMoved { x: to.x, y: to.y });
                }
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
            // The hotkeys are live only while the game or one of our windows is in front.
            compass.on_foreground(gamewin::foreground(game.map(|g| g.handle)));
            if let Some(unavailable) = hotkeys.sync(&app, compass.panel_state().hotkeys_live) {
                compass.hotkeys_unavailable(unavailable);
            }
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
    focusable: Option<bool>,
    search_open: bool,
    /// Tells the z-order keeper to hold the overlay just above the game.
    above_game: Arc<AtomicBool>,
    /// Where the overlay was last seen (moved to, by anyone).
    seen_at: Option<PhysicalPosition<i32>>,
    /// Where the shell moved the overlay lately, and when: their move events are ours.
    placed: Vec<(PhysicalPosition<i32>, Instant)>,
    /// When the player last dragged the overlay.
    dragged_at: Option<Instant>,
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
        let seen_at = overlay.outer_position().ok();
        Self {
            overlay,
            layer: None,
            click_through: true,
            focusable: None,
            search_open: false,
            above_game,
            seen_at,
            placed: Vec::new(),
            dragged_at: None,
        }
    }

    /// The overlay moved to `to`: whether the player dragged it there. Not when the shell
    /// moved it there, nor when it didn't move (a resize).
    fn player_moved(&mut self, to: PhysicalPosition<i32>) -> bool {
        let now = Instant::now();
        self.placed.retain(|(_, at)| now.duration_since(*at) < OWN_MOVE_ECHO);
        let ours = self.seen_at.replace(to) == Some(to) || self.placed.iter().any(|(placed, _)| *placed == to);
        if !ours {
            self.dragged_at = Some(now);
        }
        !ours
    }

    fn show(&mut self, state: &PanelState) {
        self.above_game.store(state.layer == Layer::AboveGame, Ordering::Relaxed);
        let layer_changed = self.layer != Some(state.layer);
        let free = state.layer == Layer::Free;
        if layer_changed {
            self.layer = Some(state.layer);
            // Detached, it is an ordinary window: it can be found in the taskbar. Never
            // topmost: above the game it is kept just above it in the z-order instead.
            let _ = self.overlay.set_always_on_top(false);
            let _ = self.overlay.set_skip_taskbar(!free);
        }
        // Only the open search view takes the typing. Otherwise the overlay can't be
        // activated (WS_EX_NOACTIVATE), so a click on it leaves the keyboard with the game.
        let search_open = state.search.view.is_some();
        if self.focusable != Some(search_open) {
            self.focusable = Some(search_open);
            let _ = self.overlay.set_focusable(search_open);
        }
        // Always clickable while it shows. Hidden (the game is gone) the webview draws
        // nothing, so clicks pass through and nothing is in the way.
        let click_through = state.layer == Layer::Hidden;
        if self.click_through != click_through {
            self.click_through = click_through;
            if let Err(e) = self.overlay.set_ignore_cursor_events(click_through) {
                eprintln!("click-through toggle failed: {e}");
            }
        }
        // The search view opens right after our hotkey or a click on the overlay, so
        // Windows lets us take the foreground; when it closes, Windows gives focus back
        // to the window below.
        match (self.search_open, search_open) {
            (false, true) => search::take_focus(&self.overlay),
            (true, false) => search::give_focus_back(&self.overlay),
            _ => {}
        }
        self.search_open = search_open;
        if let Some(placement) = state.placement {
            // While the player drags it the position is theirs; once the drag settles it
            // goes where the core says (clear of the Title Region, inside the game).
            let dragging = self.dragged_at.is_some_and(|at| at.elapsed() < DRAG_SETTLE);
            if let Some(to) = place(&self.overlay, placement, dragging && !layer_changed) {
                self.placed.push((to, Instant::now()));
            }
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

/// Size the overlay to the core's placement, and move it there unless the player is
/// dragging it. Returns where it was moved to, if it was.
fn place(overlay: &WebviewWindow, placement: ScreenRect, player_drags_it: bool) -> Option<PhysicalPosition<i32>> {
    let size = PhysicalSize::new(placement.width, placement.height);
    if overlay.inner_size().ok() != Some(size) {
        let _ = overlay.set_size(size);
    }
    let to = PhysicalPosition::new(placement.x, placement.y);
    if player_drags_it || overlay.outer_position().ok() == Some(to) {
        return None;
    }
    overlay.set_position(to).ok().map(|()| to)
}
