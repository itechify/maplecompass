//! MapleCompass shell: a thin adapter around `compass-core`. It finds the game window,
//! captures the Title Region and the EXP Bar Region (and, for Item Lookup, the client
//! area with the pointer), feeds frames, hotkeys and drags to the core and renders the panel state in an overlay that sits just above the game
//! (or, detached, in an ordinary window), and the Grind Tracker in a window of its own
//! beside it. All logic lives in the core.

pub mod capture;
pub mod gamewin;
mod files;
mod grind_tracker;
mod hotkeys;
pub mod meowdb;
mod search;
mod settings;
mod setup;
mod sizemove;
mod tray;
mod view;
mod world;

use compass_core::{
    Action, Compass, SavedWorld, ExpBarReader, ExpBarReading, Frame, GameWindow, ItemCapture, ItemTooltipReader,
    ItemTooltipReading, Layer, MIN_PANEL_WIDTH, PanelControl, PanelSizes, PlacementMode, PanelState, Request,
    ScreenPoint, ScreenRect, Size, TitleReader, TitleReading, default_title_region,
};
use meowdb::MeowDbSource;
use grind_tracker::GrindTrackerWindow;
use setup::{Bridge, SetupWindows};
use sizemove::{Gesture, SizeMove};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};
use tauri_plugin_opener::OpenerExt;

/// Capture and recognition rate: ~3 Hz keeps a map change within about a second.
const TICK: Duration = Duration::from_millis(333);
/// EXP bar capture rate: EXP tracking needs the figures about once a second.
const EXP_BAR_TICK: Duration = Duration::from_millis(1000);
/// Game window and z-order check rate: a docked panel follows a moving game window
/// and the overlay is back above the game within ~50 ms of the game coming to front.
const WINDOW_TICK: Duration = Duration::from_millis(50);
/// Settings are saved once they have stopped changing for this long (after a drag).
const SAVE_DELAY: Duration = Duration::from_millis(500);
/// What the world crawl learned is saved once it has stopped changing for this long:
/// a few hundred KB to write, so not after every page.
const WORLD_SAVE_DELAY: Duration = Duration::from_secs(10);
/// The overlay's size in CSS px (logical); `src/styles.css` lays out to fit. The
/// expanded panel is as wide as the core says (the player sets it by a bottom corner),
/// and its height follows its content, as the overlay reports it; the core keeps it
/// within the space where the panel is placed. It is never taller than
/// [`MAX_EXPANDED_HEIGHT`]: past that the route or the map list scrolls.
const EXPANDED_HEIGHT: f64 = 72.0;
const MIN_EXPANDED_HEIGHT: f64 = 44.0;
const MAX_EXPANDED_HEIGHT: f64 = 600.0;
const COLLAPSED: (f64, f64) = (44.0, 44.0);
/// A drag or resize the player started whose move or size loop hasn't started within this
/// long never will (the mouse was let go first): the shell places the panel again.
pub(crate) const LOOP_START_GRACE: Duration = Duration::from_secs(1);

/// What the compass loop is asked to do.
#[derive(Debug, Clone)]
pub(crate) enum Command {
    /// Something the player did in the overlay or the setup window, for the core.
    Act(Action),
    /// The player pressed this hotkey (its accelerator string), for the core.
    Hotkey(String),
    /// The player clicked one of the icons at the top of the panel, for the core.
    Press(PanelControl),
    /// The overlay's query box now holds this, for the core.
    SearchQuery(String),
    /// The player clicked the search view's mob or map with this MeowDB ID.
    SearchPick(String),
    /// The player clicked the name of the listed mob with this MeowDB ID in the panel.
    OpenMobPage(String),
    /// The player picked the item with this MeowDB ID from the ones an Item Lookup found
    /// sharing the name it read.
    PickItem(String),
    /// The expanded panel's content is this tall (CSS px).
    PanelHeight(f64),
    /// The overlay window moved here (its outer top-left corner). The player's move only
    /// during a drag; otherwise the shell's own, or the system's (a DPI or monitor
    /// change).
    Moved(PhysicalPosition<i32>),
    /// The player pressed the mouse to drag the overlay (by its header or collapsed icon)
    /// or to resize it (by a corner grip or an edge), so the moves or sizes that follow
    /// are theirs.
    GestureStarted(Gesture),
    /// The overlay window is now this size. The player's resize only during a resize;
    /// otherwise the shell's own.
    Resized(PhysicalSize<u32>),
    /// Windows' move or size loop on the overlay started: a drag or resize is running.
    SizeMoveStarted,
    /// Windows' move or size loop on the overlay ended: the player let go of the mouse.
    SizeMoveEnded,
    /// The Grind Tracker's content is this tall (CSS px).
    GrindTrackerHeight(f64),
    /// Windows told of a drag of the Grind Tracker's window.
    GrindTrackerSizeMove(SizeMove),
    /// The player chose Quit in the tray icon's menu: save the settings and exit.
    Quit,
    /// The player started MapleCompass again; that one exited, and this one shows itself.
    LaunchedAgain,
    /// The reader is done with a capture of the Title Region `region`: what it read, or
    /// `None` if the capture failed.
    Read { region: ScreenRect, reading: Option<TitleReading> },
    /// The reader is done with a capture of the EXP Bar Region `region`: what it read,
    /// or `None` if the capture failed.
    ReadExpBar { region: ScreenRect, reading: Option<ExpBarReading> },
    /// The reader is done with an Item Lookup's capture: what it read.
    ReadItemTooltip(ItemTooltipReading),
    /// The world an earlier launch's crawl learned has been read (or built from the
    /// cached pages, and so is still to be saved: `from_file` false).
    WorldLoaded { world: Box<SavedWorld>, from_file: bool },
}

/// The overlay's drags and resizes, as the compass loop hears them.
fn overlay_size_move(what: SizeMove) -> Command {
    match what {
        SizeMove::Pressed(gesture) => Command::GestureStarted(gesture),
        SizeMove::LoopStarted => Command::SizeMoveStarted,
        SizeMove::LoopEnded => Command::SizeMoveEnded,
    }
}

/// The overlay reports how tall its expanded content is, so the window fits it.
#[tauri::command]
fn panel_height(height: f64, commands: tauri::State<'_, Sender<Command>>) {
    let _ = commands.send(Command::PanelHeight(height));
}

/// Where MeowDB's pages and maplestory.io's mob images are cached. WebView2 shares the app
/// cache folder, hence the subfolder.
fn cache_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_cache_dir()?.join("cache"))
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
        // First, so a second launch exits before it sets anything up: only one
        // MapleCompass runs, and the running one shows itself instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(commands) = app.try_state::<Sender<Command>>() {
                let _ = commands.send(Command::LaunchedAgain);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        // The Mob Cards' images, from our saved copies of maplestory.io's.
        .register_uri_scheme_protocol(meowdb::MOB_IMAGE_SCHEME, |ctx, request| match cache_dir(ctx.app_handle()) {
            Ok(dir) => meowdb::serve_mob_image(&dir, &request),
            Err(_) => tauri::http::Response::builder().status(500).body(Vec::new()).unwrap_or_default(),
        })
        .setup(|app| {
            let overlay = app.get_webview_window("overlay").expect("overlay window in tauri.conf.json");
            // Not content-protected: the panel shows in the player's screenshots and
            // streams. The core keeps it off the Title Region instead, so our own capture
            // never reads it.
            // Clicks pass through until there is a panel to show; then it is always
            // clickable, but never activated by a click (`focusable: false` in
            // tauri.conf.json is WS_EX_NOACTIVATE), so the game keeps the keyboard.
            overlay.set_ignore_cursor_events(true)?;

            // MeowDB pages are cached on disk (ADR 0003), and maplestory.io's mob images
            // (ADR 0006).
            let cache = cache_dir(app.handle())?;
            let source = MeowDbSource::new(cache.clone())?;
            let mut compass = Compass::new(source);
            let settings_path = settings::path(app.handle())?;
            // The core brings older settings up to date (hotkeys included).
            compass.restore(settings::load(&settings_path));
            let (commands, commands_rx) = mpsc::channel();
            // What the crawl learned at earlier launches: read (or built from the cached
            // pages) on a thread of its own, while the crawl holds for it.
            compass.await_world();
            let (loaded_to, world_dir) = (commands.clone(), cache.clone());
            std::thread::Builder::new().name("world-loader".into()).spawn(move || {
                let world::Loaded { world, from_file } = world::load(&world_dir);
                let _ = loaded_to.send(Command::WorldLoaded { world: Box::new(world), from_file });
            })?;
            // Registered only while the game or one of our windows is in front.
            let hotkeys = hotkeys::install(app, &compass.panel_state().hotkeys, commands.clone())?;
            // The one way to quit at any time, the game open or not.
            tray::install(app, commands.clone())?;
            app.manage(commands.clone());
            let (picks, picks_rx) = mpsc::channel();
            app.manage(Bridge {
                commands: commands.clone(),
                picks,
                game: Mutex::new(None),
                view: Default::default(),
            });
            let setup_windows = SetupWindows::new(app.handle())?;
            // Without it the player's drags and resizes don't stick, but the panel works.
            if let Err(e) =
                sizemove::watch(&overlay, commands.clone(), overlay_size_move, sizemove::overlay_resizable())
            {
                eprintln!("{e}");
            }
            let grind_tracker = GrindTrackerWindow::new(app.handle(), commands.clone());
            overlay.on_window_event(move |event| match event {
                // Only moves during a drag the overlay started are the player's.
                WindowEvent::Moved(to) => {
                    let _ = commands.send(Command::Moved(*to));
                }
                // Only sizes during a resize the overlay started are the player's.
                WindowEvent::Resized(size) => {
                    let _ = commands.send(Command::Resized(*size));
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
            let reader = ReaderThread::spawn(app.state::<Sender<Command>>().inner().clone())?;
            let channels = Channels { commands: commands_rx, picks: picks_rx };
            let windows = Windows { overlay, setup: setup_windows, grind_tracker, hotkeys };
            std::thread::Builder::new().name("compass-loop".into()).spawn(move || {
                compass_loop(handle, windows, compass, reader, channels, Files { settings: settings_path, cache })
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            panel_height,
            setup::setup_view,
            setup::pick_game_window,
            setup::start_marking,
            setup::adjust_selection,
            setup::confirm_selection,
            setup::mark_region,
            setup::discard_exp_bar_mark,
            setup::clear_exp_bar_region,
            setup::cancel_marking,
            setup::finish_setup,
            search::panel_press,
            search::open_mob_page,
            search::pick_item,
            search::search_query,
            search::search_move,
            search::search_enter,
            search::search_pick,
            search::search_escape,
            search::hunting_reopen,
            search::hunting_clear,
            search::search_toggle_taxis,
            search::search_toggle_teleport_scrolls,
            search::panel_toggle_section,
            grind_tracker::grind_tracker_height,
            grind_tracker::grind_tracker_view,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Our windows: the overlay (with mob search in it), setup (with its region window) and
/// the Grind Tracker's.
struct Windows {
    overlay: WebviewWindow,
    setup: SetupWindows,
    grind_tracker: GrindTrackerWindow,
    /// Not a window, but registered and released as the foreground changes.
    hotkeys: hotkeys::GlobalHotkeys,
}

/// Where the compass loop saves the settings, and what the world crawl learned.
struct Files {
    settings: PathBuf,
    cache: PathBuf,
}

/// The world crawl's saves: what was saved last, so it is saved only when it changed.
struct WorldSaves {
    /// The saved world has been read: until then there is nothing to save, and the file
    /// must not be overwritten with less.
    loaded: bool,
    /// The core's world revision last saved (or read).
    saved: Option<u64>,
    /// The revision waiting to be saved, and since when.
    pending: Option<(u64, Instant)>,
}

impl WorldSaves {
    /// Save the world if it changed and has settled (`now` on Quit), off the loop
    /// unless it is the last save before quitting.
    fn save(&mut self, compass: &Compass, cache: &std::path::Path, now: bool) {
        let revision = compass.world_revision();
        if !self.loaded || self.saved == Some(revision) {
            self.pending = None;
            return;
        }
        if self.pending.is_none_or(|(pending, _)| pending != revision) {
            self.pending = Some((revision, Instant::now()));
        }
        if !now && self.pending.is_some_and(|(_, since)| since.elapsed() < WORLD_SAVE_DELAY) {
            return;
        }
        self.saved = Some(revision);
        self.pending = None;
        let (world, cache) = (compass.world(), cache.to_path_buf());
        let write = move || {
            if let Err(e) = world::save(&cache, &world) {
                eprintln!("saving the world failed: {e}");
            }
        };
        if now {
            write();
        } else if let Err(e) = std::thread::Builder::new().name("world-saver".into()).spawn(write) {
            eprintln!("saving the world failed: {e}");
        }
    }
}

/// The wall clock, in ms since the Unix epoch.
fn unix_now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// What the compass loop hears from hotkeys, the overlay, the setup window and the
/// reader thread.
struct Channels {
    commands: Receiver<Command>,
    /// Titles of game windows the player picked by hand.
    picks: Receiver<String>,
}

/// A region for the reader thread to capture and read, or an Item Lookup's capture,
/// taken already, to read.
#[derive(Debug, Clone)]
enum Capture {
    Title(ScreenRect),
    ExpBar(ScreenRect),
    Item(Box<ItemCapture>),
}

/// Captures and reads the Title Region and the EXP Bar Region on its own thread, and reads
/// Item Lookup's captures (taken at the press, while the Item Tooltip shows). Reading
/// takes tens of ms (far more in a debug build), so the compass loop never waits for it:
/// it keeps following the game window at its own pace and takes each reading when it
/// arrives.
struct ReaderThread {
    captures: Sender<Capture>,
    /// The clock frames and ticks are timed by.
    start: Instant,
    /// A Title Region capture was asked for and its reading hasn't come back yet.
    title_busy: bool,
    /// Likewise for the EXP Bar Region.
    exp_bar_busy: bool,
}

impl ReaderThread {
    fn spawn(commands: Sender<Command>) -> std::io::Result<Self> {
        let start = Instant::now();
        let (captures, asked) = mpsc::channel::<Capture>();
        std::thread::Builder::new().name("reader".into()).spawn(move || {
            // The readers share one recogniser.
            let title_reader = TitleReader::bundled();
            let exp_bar_reader = ExpBarReader::bundled();
            let item_reader = ItemTooltipReader::bundled();
            let grab = |r: ScreenRect| match capture::capture_rgba(r.x, r.y, r.width as i32, r.height as i32) {
                Ok(rgba) => {
                    let at_ms = start.elapsed().as_millis() as u64;
                    Some(Frame { width: r.width, height: r.height, rgba, at_ms })
                }
                Err(e) => {
                    eprintln!("capture failed: {e}");
                    None
                }
            };
            for capture in asked {
                let command = match capture {
                    Capture::Title(region) => {
                        Command::Read { region, reading: grab(region).map(|f| title_reader.read(&f)) }
                    }
                    Capture::ExpBar(region) => {
                        Command::ReadExpBar { region, reading: grab(region).map(|f| exp_bar_reader.read(&f)) }
                    }
                    Capture::Item(capture) => Command::ReadItemTooltip(item_reader.read(&capture)),
                };
                if commands.send(command).is_err() {
                    return;
                }
            }
        })?;
        Ok(Self { captures, start, title_busy: false, exp_bar_busy: false })
    }

    /// Capture and read the Title Region `region`, unless the last capture of it is
    /// still being read.
    fn read_title(&mut self, region: ScreenRect) {
        if !self.title_busy {
            self.title_busy = self.captures.send(Capture::Title(region)).is_ok();
        }
    }

    /// Likewise for the EXP Bar Region.
    fn read_exp_bar(&mut self, region: ScreenRect) {
        if !self.exp_bar_busy {
            self.exp_bar_busy = self.captures.send(Capture::ExpBar(region)).is_ok();
        }
    }

    /// Item Lookup: take the pointer's position and capture the game's client area now,
    /// while the Item Tooltip shows, and have it read. Only reads the pointer position
    /// and copies screen pixels (ADR 0001).
    fn read_item_tooltip(&self) -> Result<(), String> {
        let pointer = gamewin::cursor_position().ok_or("no pointer position")?;
        let client = gamewin::find_game_client().ok_or("no game window")?;
        let rgba = capture::capture_rgba(client.x, client.y, client.width as i32, client.height as i32)?;
        let frame = Frame { width: client.width, height: client.height, rgba, at_ms: self.now_ms() };
        let capture = ItemCapture { frame, origin: ScreenPoint { x: client.x, y: client.y }, pointer };
        self.captures.send(Capture::Item(Box::new(capture))).map_err(|_| "the reader has stopped".to_string())
    }

    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

fn compass_loop(
    app: AppHandle,
    windows: Windows,
    mut compass: Compass,
    mut reader: ReaderThread,
    channels: Channels,
    files: Files,
) {
    let settings_path = files.settings;
    let mut world_saves = WorldSaves { loaded: false, saved: None, pending: None };
    let Windows { overlay, setup: mut setup_windows, mut grind_tracker, mut hotkeys } = windows;
    let mut window = OverlayWindow::new(overlay);
    let mut saved = compass.settings();
    let mut pending = (saved.clone(), Instant::now());
    let mut emitted: Option<PanelState> = None;
    let mut game = None;
    let mut expanded_height = EXPANDED_HEIGHT;
    let mut next_capture = Instant::now();
    let mut next_exp_bar_capture = Instant::now();
    let mut next_window_check = Instant::now();
    loop {
        // Wait for the player until the next check is due, so hotkeys act at once.
        let due = next_capture.min(next_exp_bar_capture).min(next_window_check);
        match channels.commands.recv_timeout(due.saturating_duration_since(Instant::now())) {
            Ok(Command::Act(action)) => {
                compass.act(action);
            }
            Ok(Command::Hotkey(pressed)) => {
                let request = compass.on_hotkey(&pressed);
                carry_out(&app, &window.overlay, &reader, &mut compass, request);
            }
            Ok(Command::Press(control)) => {
                let request = compass.press(control);
                carry_out(&app, &window.overlay, &reader, &mut compass, request);
            }
            Ok(Command::SearchQuery(query)) => {
                compass.search_query(&query);
            }
            Ok(Command::SearchPick(id)) => {
                compass.search_pick(&id);
            }
            Ok(Command::OpenMobPage(id)) => {
                let request = compass.open_mob_page(&id);
                carry_out(&app, &window.overlay, &reader, &mut compass, request);
            }
            Ok(Command::PickItem(id)) => {
                let request = compass.pick_item(&id);
                carry_out(&app, &window.overlay, &reader, &mut compass, request);
            }
            Ok(Command::PanelHeight(height)) => {
                expanded_height = height.clamp(MIN_EXPANDED_HEIGHT, MAX_EXPANDED_HEIGHT);
                set_panel_sizes(&mut compass, &window.overlay, expanded_height);
            }
            Ok(Command::LaunchedAgain) => {
                compass.act(Action::LaunchedAgain);
                setup_windows.bring_to_front();
            }
            Ok(Command::Quit) => {
                // Whatever the player changed last (a drag, say) is saved first.
                world_saves.save(&compass, &files.cache, true);
                let current = compass.settings();
                if current != saved
                    && let Err(e) = settings::save(&settings_path, &current)
                {
                    eprintln!("saving settings failed: {e}");
                }
                app.exit(0);
                return;
            }
            Ok(Command::GestureStarted(gesture)) => window.gesture_started(gesture),
            Ok(Command::Resized(size)) => window.player_resized(size),
            Ok(Command::SizeMoveStarted) => window.size_move_started(),
            Ok(Command::SizeMoveEnded) => {
                // Once the player lets go of a resize, its width is the panel's (the core
                // keeps it over the minimum), and where the window ended up is where the
                // panel is: a bottom-left corner moves its left edge.
                if let Some(size) = window.size_move_ended()
                    && let Ok(scale) = window.overlay.scale_factor()
                {
                    compass.act(Action::PanelResized { width: (f64::from(size.width) / scale).round() as u32 });
                    set_panel_sizes(&mut compass, &window.overlay, expanded_height);
                    if let Ok(at) = window.overlay.outer_position() {
                        compass.act(Action::PanelMoved { x: at.x, y: at.y });
                    }
                }
            }
            Ok(Command::Moved(to)) => {
                if window.player_moved() {
                    compass.act(Action::PanelMoved { x: to.x, y: to.y });
                }
            }
            Ok(Command::GrindTrackerHeight(height)) => {
                grind_tracker.set_height(height);
                if let Some(size) = grind_tracker.size() {
                    compass.set_grind_tracker_size(size);
                }
            }
            Ok(Command::GrindTrackerSizeMove(what)) => {
                if let Some(at) = grind_tracker.size_move(what) {
                    compass.act(Action::GrindTrackerMoved { x: at.x, y: at.y });
                }
            }
            Ok(Command::Read { region, reading }) => {
                reader.title_busy = false;
                // A reading of a region no longer captured (setup marked a new one, or
                // the game window moved) would judge the wrong spot.
                if let (Some(reading), true) = (reading, compass.panel_state().title_region == Some(region)) {
                    compass.on_reading(reading);
                }
            }
            Ok(Command::ReadExpBar { region, reading }) => {
                reader.exp_bar_busy = false;
                if let (Some(reading), true) = (reading, compass.panel_state().exp_bar_region == Some(region)) {
                    compass.on_exp_bar_reading(reading);
                }
            }
            Ok(Command::ReadItemTooltip(reading)) => {
                compass.on_item_tooltip_reading(Some(reading));
            }
            Ok(Command::WorldLoaded { world, from_file }) => {
                compass.restore_world(*world);
                world_saves.loaded = true;
                // Read from its file, it is saved already; built from the pages, not yet.
                world_saves.saved = from_file.then(|| compass.world_revision());
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
            game = find_game(&mut compass, &window.overlay, expanded_height);
            // For the display it is on now.
            if let Some(size) = grind_tracker.size() {
                compass.set_grind_tracker_size(size);
            }
            // The hotkeys are live only while the game or one of our windows is in front.
            compass.on_foreground(gamewin::foreground(game.map(|g| g.handle)));
            if let Some((taken, invalid)) = hotkeys.sync(&app, compass.panel_state().hotkeys_live) {
                compass.hotkeys_unavailable(taken, invalid);
            }
        }
        // Fetches in flight (map index, map pages) are checked on, never waited for.
        compass.on_wall_clock(unix_now_ms());
        compass.tick(reader.now_ms());
        // An Item Lookup's page, once MeowDB has answered.
        while let Some(request) = compass.take_request() {
            carry_out(&app, &window.overlay, &reader, &mut compass, Some(request));
        }
        if Instant::now() >= next_capture {
            next_capture = Instant::now() + TICK;
            // The region the player marked; none until setup is done, nor after the
            // game's resolution changes.
            if let Some(region) = compass.panel_state().title_region {
                reader.read_title(region);
            }
        }
        if Instant::now() >= next_exp_bar_capture {
            next_exp_bar_capture = Instant::now() + EXP_BAR_TICK;
            // None while EXP tracking is off.
            if let Some(region) = compass.panel_state().exp_bar_region {
                reader.read_exp_bar(region);
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
        grind_tracker.sync(state);

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
        world_saves.save(&compass, &files.cache, false);
    }
}

/// Find the game window and the monitors, and feed them to the core.
fn find_game(compass: &mut Compass, overlay: &WebviewWindow, expanded_height: f64) -> Option<gamewin::Game> {
    set_panel_sizes(compass, overlay, expanded_height);
    let game = gamewin::find_game();
    compass.on_game_window(game.map(|g| GameWindow {
        client: g.client,
        frame: g.frame,
        screens: gamewin::monitor_work_areas(),
    }));
    game
}

/// Tell the core the panel's sizes in physical px, for the overlay's current display:
/// expanded, as wide as the core says and as tall as its content.
fn set_panel_sizes(compass: &mut Compass, overlay: &WebviewWindow, expanded_height: f64) {
    if let Ok(scale) = overlay.scale_factor() {
        let width = f64::from(compass.panel_state().panel_width);
        compass.set_panel_sizes(PanelSizes {
            expanded: physical((width, expanded_height), scale),
            collapsed: physical(COLLAPSED, scale),
        });
    }
}

/// Do what a hotkey, a panel icon or an Item Lookup asked of the shell. Right after the
/// player's own key press or click, so Windows lets us take the foreground.
fn carry_out(
    app: &AppHandle,
    overlay: &WebviewWindow,
    reader: &ReaderThread,
    compass: &mut Compass,
    request: Option<Request>,
) {
    match request {
        Some(Request::OpenInBrowser { url }) => open_in_browser(app, &url),
        Some(Request::RaiseOverlay) => search::take_focus(overlay),
        Some(Request::CaptureItemTooltip) => {
            if let Err(e) = reader.read_item_tooltip() {
                eprintln!("Item Lookup: {e}");
                compass.on_item_tooltip_reading(None);
            }
        }
        None => {}
    }
}

/// Open a page (the shown map's on MeowDB) in the player's browser.
fn open_in_browser(app: &AppHandle, url: &str) {
    if let Err(e) = app.opener().open_url(url, None::<&str>) {
        eprintln!("couldn't open {url}: {e}");
    }
}

/// CSS px to physical px, rounded up so the panel's content is never clipped.
pub(crate) fn physical((width, height): (f64, f64), scale: f64) -> Size {
    Size { width: (width * scale).ceil() as u32, height: (height * scale).ceil() as u32 }
}

/// The player's drag or resize: it runs in Windows' move or size loop until the mouse is
/// let go.
#[derive(Debug, Clone, Copy)]
struct PlayerGesture {
    gesture: Gesture,
    /// When the player pressed the mouse to start it, until Windows' loop has started too.
    starting_since: Option<Instant>,
}

/// The overlay window, kept in line with the panel state. Window properties are only
/// set when they change.
struct OverlayWindow {
    overlay: WebviewWindow,
    layer: Option<Layer>,
    click_through: bool,
    focusable: Option<bool>,
    /// Whether the overlay is listed in the taskbar (only while detached).
    in_taskbar: Option<bool>,
    search_open: bool,
    /// Tells the z-order keeper to hold the overlay just above the game.
    above_game: Arc<AtomicBool>,
    /// The player's drag or resize, from the mouse press that starts it until Windows'
    /// move or size loop ends.
    gesture: Option<PlayerGesture>,
    /// The size the player's resize last gave the overlay.
    resized_to: Option<PhysicalSize<u32>>,
    /// Whether the window's minimum size is the expanded panel's (or none, collapsed).
    min_size_expanded: Option<bool>,
}

impl OverlayWindow {
    fn new(overlay: WebviewWindow) -> Self {
        let above_game = keep_just_above_game(&overlay, "z-order");
        Self {
            overlay,
            layer: None,
            click_through: true,
            focusable: None,
            in_taskbar: None,
            search_open: false,
            above_game,
            gesture: None,
            resized_to: None,
            min_size_expanded: None,
        }
    }

    /// The player pressed the mouse to drag or resize the overlay.
    fn gesture_started(&mut self, gesture: Gesture) {
        self.gesture = Some(PlayerGesture { gesture, starting_since: Some(Instant::now()) });
        self.resized_to = None;
    }

    /// Windows' move or size loop started: the drag or resize the player started is
    /// running. Any other loop isn't the player's.
    fn size_move_started(&mut self) {
        self.under_way();
        if let Some(player) = &mut self.gesture {
            player.starting_since = None;
        }
    }

    /// Windows' move or size loop ended: the player let go of the mouse. The size their
    /// resize left the overlay, once (when it changed anything).
    fn size_move_ended(&mut self) -> Option<PhysicalSize<u32>> {
        let ended = self.gesture.take_if(|player| player.starting_since.is_none())?;
        if ended.gesture == Gesture::Resize { self.resized_to.take() } else { None }
    }

    /// The player's drag or resize, from the mouse press until they let go. One
    /// whose loop hasn't started within `LOOP_START_GRACE` never will.
    fn under_way(&mut self) -> Option<Gesture> {
        let starting_since = self.gesture.and_then(|player| player.starting_since);
        if starting_since.is_some_and(|at| at.elapsed() >= LOOP_START_GRACE) {
            self.gesture = None;
        }
        self.gesture.map(|player| player.gesture)
    }

    /// The player's drag or resize while Windows' loop runs it.
    fn running(&self) -> Option<Gesture> {
        self.gesture.filter(|player| player.starting_since.is_none()).map(|player| player.gesture)
    }

    /// The overlay window is now `size`: the player's, while they resize it. Any other
    /// size is the shell's own.
    fn player_resized(&mut self, size: PhysicalSize<u32>) {
        if self.running() == Some(Gesture::Resize) {
            self.resized_to = Some(size);
        }
    }

    /// The overlay window moved: whether that was the player's drag. Any other move is
    /// the shell's own or the system's, and the core never hears of it.
    fn player_moved(&self) -> bool {
        self.running() == Some(Gesture::Drag)
    }

    fn show(&mut self, state: &PanelState) {
        self.above_game.store(state.layer == Layer::AboveGame, Ordering::Relaxed);
        let layer_changed = self.layer != Some(state.layer);
        if layer_changed {
            self.layer = Some(state.layer);
            // Never topmost by itself: above the game it is kept just above it in the
            // z-order instead, wherever it is placed.
            let _ = self.overlay.set_always_on_top(false);
        }
        // Detached, it can be found in the taskbar.
        let detached = state.placement_mode == PlacementMode::Detached;
        if self.in_taskbar != Some(detached) {
            self.in_taskbar = Some(detached);
            let _ = self.overlay.set_skip_taskbar(!detached);
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
        // Expanded, the player can't resize the panel narrower than the core allows;
        // collapsed, the window is the icon's size, and they can't resize it.
        sizemove::set_resizable(state.expanded);
        if self.min_size_expanded != Some(state.expanded) {
            self.min_size_expanded = Some(state.expanded);
            let min = LogicalSize::new(f64::from(MIN_PANEL_WIDTH), MIN_EXPANDED_HEIGHT);
            let _ = self.overlay.set_min_size(state.expanded.then_some(min));
        }
        // While the player resizes it the size (and position) is theirs too.
        let gesture = self.under_way();
        if let Some(placement) = state.placement
            && gesture != Some(Gesture::Resize)
        {
            // While the player drags it the position is theirs; once they let go it goes
            // where the core says (clear of the Title Region, inside the game).
            place(&self.overlay, placement, gesture == Some(Gesture::Drag) && !layer_changed);
        }
    }
}

/// Keep our `window` just above the game in the z-order while the flag returned is set,
/// on a thread of its own (named `name`), at its own steady pace: the window should be
/// back above the game within a tick of the game coming to the front, whatever the
/// compass loop is doing (setting window sizes and positions can take a while). It gets
/// the window's handle once, here: asking Tauri for it from another thread waits on the
/// main thread's event loop, which can leave the keeper idle for seconds while another
/// app covers the game, and the window stays topmost over that app.
pub(crate) fn keep_just_above_game(window: &WebviewWindow, name: &str) -> Arc<AtomicBool> {
    let above_game = Arc::new(AtomicBool::new(false));
    #[cfg(windows)]
    match window.hwnd() {
        Ok(hwnd) => {
            let (handle, keeper_flag) = (hwnd.0 as isize, above_game.clone());
            let spawned =
                std::thread::Builder::new().name(name.into()).spawn(move || keep_above_game(handle, keeper_flag));
            if let Err(e) = spawned {
                eprintln!("{name} keeper not started: {e}");
            }
        }
        Err(e) => eprintln!("{name} keeper not started: no {} window handle: {e}", window.label()),
    }
    #[cfg(not(windows))]
    let _ = (window, name);
    above_game
}

/// While `above_game` is set, check every `WINDOW_TICK` that our window (its handle) is
/// still just above the game in the z-order, and restack it if not: the game came to the
/// front, or another app did. Nothing here waits on the main thread.
#[cfg(windows)]
fn keep_above_game(window: isize, above_game: Arc<AtomicBool>) {
    let window = windows::Win32::Foundation::HWND(window as *mut _);
    let mut last_error = None;
    loop {
        std::thread::sleep(WINDOW_TICK);
        if !above_game.load(Ordering::Relaxed) {
            continue;
        }
        if let Some(game) = gamewin::find_game() {
            // Polled 20 times a second: report a failure once, not every time.
            let error = gamewin::keep_above(window, game.handle).err().map(|e| e.to_string());
            if error.is_some() && error != last_error {
                eprintln!("keeping our window above the game failed: {}", error.as_deref().unwrap_or_default());
            }
            last_error = error;
        }
    }
}

/// Size our window to the core's placement, and move it there unless the player is
/// dragging it.
pub(crate) fn place(overlay: &WebviewWindow, placement: ScreenRect, player_drags_it: bool) {
    let size = PhysicalSize::new(placement.width, placement.height);
    if overlay.inner_size().ok() != Some(size) {
        let _ = overlay.set_size(size);
    }
    let to = PhysicalPosition::new(placement.x, placement.y);
    if !player_drags_it && overlay.outer_position().ok() != Some(to) {
        let _ = overlay.set_position(to);
    }
}
