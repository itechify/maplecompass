//! MapleCompass core: the single test seam.
//!
//! Inputs are plain data (captured Title Region frames or their readings, the game
//! window, the clock, the player's actions and hotkeys); the output is a [`PanelState`]
//! holding everything the overlay renders. No OS or UI dependencies live here.

mod map_data;
mod map_index;
mod matcher;
pub mod meowdb;
mod panel;
mod portal_overrides;
mod portals;
mod prep;
mod recogniser;
mod settings;
mod setup;
mod shown_map;
mod tracker;

pub use map_data::{
    HiddenPortal, LISTED_MOBS, MapData, MapDataStatus, MapSpot, MapSummary, Mob, MoreMobs, PerParty,
};
pub use map_index::{Fetched, MapDataSource, MapEntry, SourceError};
pub use portal_overrides::{PORTAL_OVERRIDES, PortalOverride};
pub use panel::{
    Action, GameWindow, InsideOffset, Layer, PanelSizes, PlacementMode, ScreenPoint, ScreenRect,
    Size,
};
pub use settings::{Hotkeys, Settings};
pub use setup::{SetupReason, SetupState, TestRead, TitleRegion};
pub use shown_map::{Candidate, Candidates, FIRST_MOBS, LevelRange, Outline};

use shown_map::ShownMap;
use matcher::{Match, Matcher, Snap};
use panel::Panel;
use setup::Setup;
pub use recogniser::{TitleLines, TitleReader, TitleReading};
use tracker::Tracker;
use serde::Serialize;
use std::task::Poll;

/// A captured Title Region image: tightly packed RGBA, top-down rows.
#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Capture time in milliseconds (any monotonic origin).
    pub at_ms: u64,
}

/// A rectangle in physical pixels, relative to the game window's client area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A guess at the Title Region for a game client area of the given size (physical px),
/// for the probe, which has no setup. The app captures the region the player marked
/// (`PanelState::title_region`).
///
/// On the samples (1920x1200 client, default UI size) the title band is 66 px tall at
/// client (0..4, 31). The region keeps ~15 px of slack above and below it, and is wide
/// enough for the longest map name in MeowDB ("Physical Fitness Test <Normal Waiting
/// Room>", ~470 px of panel). The core trims each frame where the panel ends, so a
/// narrow minimap's surroundings are never read. Clamped to the client area.
pub fn default_title_region(client_width: u32, client_height: u32) -> Rect {
    const Y: u32 = 16;
    const WIDTH: u32 = 560;
    const HEIGHT: u32 = 96;
    let y = Y.min(client_height);
    Rect { x: 0, y, width: WIDTH.min(client_width), height: HEIGHT.min(client_height - y) }
}

/// The map the panel shows: a real map from the map index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentMap {
    /// MeowDB's 9-digit map ID, zero padding kept.
    pub id: String,
    pub street_name: String,
    pub map_name: String,
    /// The map's MeowDB page.
    pub url: String,
}

/// The credit for everything MapleCompass shows from MeowDB (ADR 0003).
pub const MEOWDB_CREDIT: &str = "NiaMeowDB (meowdb.com)";

/// Whether minimap titles can be recognised as real maps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Recognition {
    /// The map index hasn't arrived yet (it is asked for at startup).
    #[default]
    Starting,
    Ready,
    /// MeowDB couldn't be reached, so titles are snapped to a copy of the map index
    /// saved at `saved_at_unix_ms` (ms since the Unix epoch).
    SavedCopy {
        #[serde(rename = "savedAtUnixMs")]
        saved_at_unix_ms: u64,
    },
    /// The map index couldn't be had, so readings can't be snapped to real maps.
    Unavailable,
}

/// Everything the overlay renders.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelState {
    pub recognition: Recognition,
    /// The last map the core was confident about.
    pub map: Option<CurrentMap>,
    /// How well the latest frame matched the map shown, 0..=100 (0 when it didn't).
    pub confidence: u8,
    /// The latest frame didn't confirm the map shown (unreadable, low confidence, or
    /// a different map not yet confirmed).
    pub not_sure: bool,
    /// The maps sharing the shown map's title, when the core couldn't tell which of
    /// them the player is on; `None` when it could.
    pub candidates: Option<Candidates>,
    /// The shown map's Map Summary, mobs and Hidden Portals from MeowDB, or why they
    /// aren't there.
    pub data: MapDataStatus,
    /// [`MEOWDB_CREDIT`] whenever a map is shown: its names and data are MeowDB's.
    pub credit: Option<&'static str>,
    /// Expanded to the full panel, or collapsed to a small icon.
    pub expanded: bool,
    /// The player asked to move the panel (the edit-mode hotkey).
    pub edit_mode: bool,
    /// A visible game window was found (not minimised).
    pub game_found: bool,
    /// Where the player wants the panel: inside, docked or detached.
    pub placement_mode: PlacementMode,
    /// Where the overlay goes on screen; `None` while there is no game window, unless
    /// the panel is detached. Inside the game it never covers `title_region`: the
    /// overlay shows in screen captures, ours included.
    pub placement: Option<ScreenRect>,
    /// Where the overlay sits in the z-order, or hidden.
    pub layer: Layer,
    /// The overlay takes the mouse and the player drags it, so the shell leaves its
    /// position alone except when its size or layer changes. Otherwise clicks pass
    /// through to the game and the shell keeps the overlay at `placement`.
    pub draggable: bool,
    /// First-run setup and re-setup.
    pub setup: SetupState,
    /// Where on screen to capture the Title Region (physical px); `None` while there
    /// is no game window or the region isn't known to fit it.
    pub title_region: Option<ScreenRect>,
    /// The title of the game window to look for: "MapleStory", or one the player
    /// picked by hand.
    pub game_window_title: String,
    /// The global hotkeys to register, as the overlay names them.
    pub hotkeys: Hotkeys,
    /// Hotkeys the shell couldn't register, so they do nothing, to tell the player.
    pub unavailable_hotkeys: Vec<String>,
    /// Those of them that don't name a key combination; the rest another app has.
    pub invalid_hotkeys: Vec<String>,
    /// The player closed first-run setup: with no Title Region there is nothing to
    /// read, so the app quits.
    pub quit: bool,
}

/// A page to open in the player's browser (the open-page hotkey asked for it): the one
/// thing the shell does on the core's say beyond rendering the panel state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenInBrowser {
    pub url: String,
}

/// The game window's title, unless the player picked another by hand.
pub const GAME_WINDOW_TITLE: &str = "MapleStory";

/// How long to wait before asking for the map index again after a failure. MeowDB is
/// asked politely (ADR 0003): never on every frame.
const INDEX_RETRY_MS: u64 = 60_000;

/// The compass core: feed it frames, render its [`PanelState`].
pub struct Compass {
    /// Reads frames fed to [`Compass::on_frame`]; loaded on the first one, so a shell
    /// that reads on its own thread never loads it twice.
    reader: Option<TitleReader>,
    source: Box<dyn MapDataSource + Send>,
    matcher: Option<Matcher>,
    /// When asking for the map index last failed (frame time).
    index_failed_at: Option<u64>,
    tracker: Tracker,
    shown_map: ShownMap,
    panel: Panel,
    setup: Setup,
    /// The last settings restored; the panel's own fields live in `panel`.
    settings: Settings,
    state: PanelState,
}

impl Compass {
    /// A core using the given map data source and default settings. The map index is
    /// asked for on the first tick or frame.
    pub fn new(source: impl MapDataSource + Send + 'static) -> Self {
        let mut compass = Self {
            reader: None,
            source: Box::new(source),
            matcher: None,
            index_failed_at: None,
            tracker: Tracker::default(),
            shown_map: ShownMap::default(),
            panel: Panel::default(),
            setup: Setup::default(),
            settings: Settings::default(),
            state: PanelState::default(),
        };
        compass.sync_panel();
        compass
    }

    /// Read a captured Title Region frame and update the panel state: the same as
    /// [`Compass::on_reading`] with what a [`TitleReader`] reads in the frame, here.
    pub fn on_frame(&mut self, frame: Frame) -> &PanelState {
        let reading = self.reader.get_or_insert_with(TitleReader::bundled).read(&frame);
        self.on_reading(reading)
    }

    /// Take what a Title Region frame read as (read by a [`TitleReader`], possibly on
    /// another thread) and update the panel state.
    pub fn on_reading(&mut self, reading: TitleReading) -> &PanelState {
        self.load_map_index(reading.at_ms);
        let Some(matcher) = &self.matcher else {
            // Without the map index nothing can be recognised.
            self.setup.on_read(None);
            return self.sync_panel();
        };
        let snap = reading.lines.and_then(|lines| match matcher.snap(&lines.street, &lines.map)? {
            Match::Sure(snap) => Some(snap),
            // Names differing only in a numeral the reading doesn't make out: the
            // one within the player's reach, if just one is; otherwise not sure.
            Match::OneOf { maps, score } => {
                let reach = self.shown_map.within_reach();
                let mut reachable = maps.into_iter().filter(|&i| {
                    matcher.same_title_as(i).into_iter().any(|j| reach.contains(&matcher.map(j).id))
                });
                match (reachable.next(), reachable.next()) {
                    (Some(index), None) => Some(Snap { index, score }),
                    _ => None,
                }
            }
        });
        let current_map = |i: usize| {
            let entry = matcher.map(i);
            CurrentMap {
                id: entry.id.clone(),
                street_name: entry.street_name.clone(),
                map_name: entry.map_name.clone(),
                url: meowdb::map_url(&entry.id),
            }
        };
        // A test read judges this one frame: the tracker would keep showing the last
        // confident map through a badly marked region.
        self.setup.on_read(snap.map(|s| current_map(s.index)));
        let tracked = self.tracker.update(snap);
        let titled = tracked.current.map_or_else(Vec::new, |i| matcher.same_title_as(i));
        let maps = titled.into_iter().map(current_map).collect();
        self.shown_map.show(maps, &mut *self.source);
        self.sync_shown_map();
        self.state.confidence = (tracked.confidence * 100.0).round() as u8;
        self.state.not_sure = tracked.not_sure;
        self.sync_panel()
    }

    fn sync_shown_map(&mut self) {
        self.state.map = self.shown_map.map().cloned();
        self.state.data = self.shown_map.data();
        self.state.candidates = self.shown_map.candidates();
        self.state.credit = self.state.map.as_ref().map(|_| MEOWDB_CREDIT);
    }

    /// The game window moved, resized, appeared or went away (`None`: not found, or
    /// minimised).
    pub fn on_game_window(&mut self, game: Option<GameWindow>) -> &PanelState {
        self.setup.game = game.as_ref().map(|g| g.client);
        self.panel.game = game;
        self.sync_panel()
    }

    /// The overlay's expanded and collapsed sizes, in physical px for the current display.
    pub fn set_panel_sizes(&mut self, sizes: PanelSizes) -> &PanelState {
        self.panel.sizes = sizes;
        self.sync_panel()
    }

    /// Apply something the player did.
    pub fn act(&mut self, action: Action) -> &PanelState {
        match action {
            Action::ToggleExpanded => self.panel.expanded = !self.panel.expanded,
            Action::ToggleEditMode => self.panel.edit_mode = !self.panel.edit_mode,
            Action::PanelMoved { x, y } => self.panel.moved_to(x, y),
            Action::CyclePlacement => self.panel.cycle_placement(),
            Action::ChooseCandidate => {
                self.shown_map.choose_next(&mut *self.source);
                self.sync_shown_map();
            }
            Action::MarkTitleRegion(marked) => self.setup.mark(marked),
            Action::FinishSetup => self.setup.finish(),
            Action::RequestSetup => self.setup.request(),
            Action::CancelSetup => {
                self.state.quit |= self.setup.state().needed == Some(SetupReason::FirstRun);
                self.setup.cancel();
            }
        }
        self.sync_panel()
    }

    /// The player pressed one of the global hotkeys (an accelerator string, as listed
    /// in the panel state's `hotkeys`). Returns the page to open, if it asks for one.
    pub fn on_hotkey(&mut self, pressed: &str) -> Option<OpenInBrowser> {
        let hotkeys = &self.settings.hotkeys;
        let is = |hotkey: &str| same_hotkey(hotkey, pressed);
        let action = if is(&hotkeys.toggle_panel) {
            Action::ToggleExpanded
        } else if is(&hotkeys.edit_mode) {
            Action::ToggleEditMode
        } else if is(&hotkeys.cycle_placement) {
            Action::CyclePlacement
        } else if is(&hotkeys.setup) {
            Action::RequestSetup
        } else if is(&hotkeys.choose_candidate) {
            Action::ChooseCandidate
        } else if is(&hotkeys.open_meowdb_page) {
            return self.state.map.as_ref().map(|map| OpenInBrowser { url: map.url.clone() });
        } else {
            return None;
        };
        self.act(action);
        None
    }

    /// These hotkeys couldn't be registered: `taken` because another app has them,
    /// `invalid` because they don't name a key combination.
    pub fn hotkeys_unavailable(&mut self, taken: Vec<String>, invalid: Vec<String>) -> &PanelState {
        self.state.unavailable_hotkeys = taken.into_iter().chain(invalid.iter().cloned()).collect();
        self.state.invalid_hotkeys = invalid;
        self.sync_panel()
    }

    /// Pick up settings saved by an earlier session. Hotkeys from before version 2
    /// (Ctrl+Shift, which players bind in the game) are replaced by the defaults.
    pub fn restore(&mut self, mut settings: Settings) -> &PanelState {
        if settings.version < settings::VERSION {
            settings.hotkeys = Hotkeys::default();
            settings.version = settings::VERSION;
        }
        self.panel.expanded = settings.expanded;
        self.panel.inside_offset = settings.inside_offset;
        self.panel.mode = settings.placement_mode;
        self.panel.detached_at = settings.detached_at;
        self.setup.saved = settings.title_region;
        self.settings = settings;
        self.sync_panel()
    }

    /// The player picked the game window by hand: look for windows with this title
    /// from now on.
    pub fn pick_game_window(&mut self, title: String) -> &PanelState {
        self.settings.game_window_title = Some(title);
        self.sync_panel()
    }

    /// The settings to save: whatever the player has changed since [`Compass::restore`].
    pub fn settings(&self) -> Settings {
        Settings {
            expanded: self.panel.expanded,
            inside_offset: self.panel.inside_offset,
            placement_mode: self.panel.mode,
            detached_at: self.panel.detached_at,
            title_region: self.setup.saved,
            ..self.settings.clone()
        }
    }

    /// Time passed: check on fetches in flight. The shell calls this many times a
    /// second (`at_ms` on the same clock as frames), so nothing waits for the network.
    pub fn tick(&mut self, at_ms: u64) -> &PanelState {
        self.load_map_index(at_ms);
        if self.matcher.is_some() {
            self.shown_map.poll(&mut *self.source);
            self.sync_shown_map();
        }
        self.sync_panel()
    }

    /// Ask the source for the map index at startup (the first tick or frame) until it
    /// answers, and after a failure, again no sooner than [`INDEX_RETRY_MS`] later.
    fn load_map_index(&mut self, at_ms: u64) {
        if self.matcher.is_some()
            || self.index_failed_at.is_some_and(|t| at_ms.saturating_sub(t) < INDEX_RETRY_MS)
        {
            return;
        }
        let index = match self.source.map_index() {
            Poll::Pending => return,
            Poll::Ready(index) => index,
        };
        match index {
            Ok(Fetched::Fresh(index)) => {
                self.matcher = Some(Matcher::new(index));
                self.state.recognition = Recognition::Ready;
            }
            Ok(Fetched::SavedCopy { data, saved_at_unix_ms }) => {
                self.matcher = Some(Matcher::new(data));
                self.state.recognition = Recognition::SavedCopy { saved_at_unix_ms };
            }
            Err(_) => {
                self.index_failed_at = Some(at_ms);
                self.state.recognition = Recognition::Unavailable;
            }
        }
    }

    pub fn panel_state(&self) -> &PanelState {
        &self.state
    }

    fn sync_panel(&mut self) -> &PanelState {
        self.sync_setup();
        // Never over the Title Region: the capture would read the panel.
        self.panel.avoid = self.state.title_region;
        self.state.expanded = self.panel.expanded;
        self.state.edit_mode = self.panel.edit_mode;
        self.state.game_found = self.panel.game.is_some();
        self.state.placement_mode = self.panel.mode;
        self.state.placement = self.panel.placement();
        self.state.layer = self.panel.layer();
        self.state.draggable = self.panel.draggable();
        self.state.hotkeys = self.settings.hotkeys.clone();
        self.state.game_window_title =
            self.settings.game_window_title.clone().unwrap_or_else(|| GAME_WINDOW_TITLE.into());
        &self.state
    }

    fn sync_setup(&mut self) {
        self.setup.follow_needed();
        self.state.setup = self.setup.state();
        self.state.title_region = self.setup.title_region(self.matcher.as_ref().map(Matcher::longest_name));
    }
}

/// Whether two accelerator strings name the same keys ("Super+Alt+C", "super + alt + c").
fn same_hotkey(a: &str, b: &str) -> bool {
    let keys = |s: &str| {
        let mut keys: Vec<String> = s.split('+').map(|k| k.trim().to_lowercase()).collect();
        keys.sort();
        keys
    };
    keys(a) == keys(b)
}
