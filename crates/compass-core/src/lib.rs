//! MapleCompass core: the single test seam.
//!
//! Inputs are plain data (captured Title Region frames or their readings, the game
//! window, the clock, the player's actions and hotkeys); the output is a [`PanelState`]
//! holding everything the overlay renders. No OS or UI dependencies live here.

mod crawl;
mod map_data;
mod map_index;
mod matcher;
pub mod meowdb;
mod panel;
mod portal_overrides;
mod portals;
mod prep;
mod recogniser;
mod search;
mod settings;
mod setup;
mod shown_map;
mod tracker;
mod travel_links;
mod world;

pub use crawl::CrawlProgress;
pub use map_data::{
    HiddenPortal, MapData, MapDataStatus, MapSummary, Mob, PerParty, Portal, PortalKind,
};
pub use map_index::{Fetched, MapDataSource, MapEntry, SourceError};
pub use portal_overrides::{PORTAL_OVERRIDES, PortalOverride};
pub use panel::{
    Action, Foreground, GameWindow, InsideOffset, Layer, PanelSizes, PlacementMode, ScreenPoint,
    ScreenRect, Size,
};
pub use search::{DensityTier, MobChoice, PANEL_RESULTS, RouteStop, SearchResult, SearchState};
pub use settings::{Hotkeys, Settings};
pub use setup::{SelectionDirection, SetupReason, SetupState, TestRead, TitleRegion};
pub use shown_map::{Candidate, Candidates, FIRST_MOBS, LevelRange, Outline};
pub use world::{Distance, Step, Travel};

use crawl::Crawl;
use search::{MobSearch, Searched};
use shown_map::ShownMap;
use matcher::{Match, Matcher, Snap};
use panel::Panel;
use setup::Setup;
pub use recogniser::{TitleLines, TitleReader, TitleReading};
use tracker::Tracker;
use world::Here;
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
    /// The overlay takes the mouse: while it is draggable, in search mode, and while the
    /// pointer is over one of its controls (so the × is clickable at any time). Otherwise
    /// clicks pass through it to the game.
    pub takes_mouse: bool,
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
    /// Hotkeys the shell couldn't register (another app has them), to tell the player.
    pub unavailable_hotkeys: Vec<String>,
    /// The shell keeps the hotkeys registered only while this is set: while the game or
    /// one of our windows is in front. Other apps keep their own Ctrl+Shift shortcuts.
    pub hotkeys_live: bool,
    /// The player closed first-run setup: with no Title Region there is nothing to
    /// read, so the app quits.
    pub quit: bool,
    /// Mob search, and how far the world crawl behind it has got.
    pub search: SearchState,
}

/// What a hotkey asks the shell to do at once, beyond rendering the panel state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Open this page in the player's browser (the open-page hotkey).
    OpenInBrowser { url: String },
    /// Bring the overlay, already in search mode, back in front (the search hotkey): the
    /// game may have come in front of it. Our own window only (ADR 0001).
    RaiseOverlay,
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
    /// The world crawl, from when the map index arrives.
    crawl: Option<Crawl>,
    /// What the player typed, highlighted and picked in mob search.
    search: MobSearch,
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
            crawl: None,
            search: MobSearch::default(),
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
        let map = self.shown_map.map().cloned();
        let data = self.shown_map.data();
        // Distances are measured from the map shown (with Candidates, the one shown),
        // and its page's exits count before the crawl gets to it.
        let moved = map.as_ref().map(|m| &m.id) != self.state.map.as_ref().map(|m| &m.id);
        let paged = data.data() != self.state.data.data();
        self.state.map = map;
        self.state.data = data;
        if moved || paged {
            self.sync_search();
        }
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

    /// The mouse pointer is at this spot on screen, physical px (`None`: the shell
    /// couldn't tell). The overlay takes the mouse while it is over one of its controls.
    pub fn on_pointer(&mut self, pointer: Option<ScreenPoint>) -> &PanelState {
        self.panel.pointer = pointer;
        self.sync_panel()
    }

    /// The overlay's clickable controls (the mob search section's ×) are at these spots
    /// on screen, physical px: none while it shows none.
    pub fn set_overlay_controls(&mut self, controls: Vec<ScreenRect>) -> &PanelState {
        self.panel.controls = controls;
        self.sync_panel()
    }

    /// Apply something the player did.
    pub fn act(&mut self, action: Action) -> &PanelState {
        match action {
            Action::ToggleExpanded => {
                self.panel.expanded = !self.panel.expanded;
                // The query box goes with the panel: like Esc.
                if !self.panel.expanded {
                    self.search.escape();
                    self.sync_search();
                }
            }
            Action::ToggleEditMode => self.panel.edit_mode = !self.panel.edit_mode,
            Action::PanelMoved { x, y } => self.panel.moved_to(x, y),
            Action::CyclePlacement => self.panel.cycle_placement(),
            Action::ChooseCandidate => {
                self.shown_map.choose_next(&mut *self.source);
                self.sync_shown_map();
            }
            Action::MarkTitleRegion(marked) => self.setup.mark(marked),
            Action::BeginTitleSelection => self.setup.begin_selection(),
            Action::AdjustTitleSelection { direction, resize, coarse } => {
                self.setup.adjust_selection(direction, resize, coarse);
            }
            Action::ConfirmTitleSelection => self.setup.confirm_selection(),
            Action::CancelTitleSelection => self.setup.cancel_selection(),
            Action::FinishSetup => self.setup.finish(),
            Action::RequestSetup => self.setup.request(),
            Action::CancelSetup => {
                self.state.quit |= self.setup.state().needed == Some(SetupReason::FirstRun);
                self.setup.cancel();
            }
            Action::OpenSearch => {
                // The query box is on the expanded panel.
                self.panel.expanded = true;
                self.search.open();
                self.sync_search();
            }
            Action::SearchPrevious | Action::SearchNext => {
                let by = if action == Action::SearchNext { 1 } else { -1 };
                self.search.move_highlight(by, &searched(&self.crawl, &self.state, &self.settings));
                self.sync_search();
            }
            Action::SearchEnter => {
                self.search.enter(self.crawl.as_ref());
                self.sync_search();
            }
            Action::SearchEscape => {
                self.search.escape();
                self.sync_search();
            }
            Action::ClearSearch => {
                self.search.clear();
                self.sync_search();
            }
            // Settings, so they count whether or not the window is open.
            Action::ToggleTaxis => {
                self.settings.travel.taxis = !self.settings.travel.taxis;
                self.sync_search();
            }
            Action::ToggleTeleportScrolls => {
                self.settings.travel.teleport_scrolls = !self.settings.travel.teleport_scrolls;
                self.sync_search();
            }
        }
        self.sync_panel()
    }

    /// The player typed in search mode: `query` is everything in the query box.
    pub fn search_query(&mut self, query: &str) -> &PanelState {
        self.search.type_query(query);
        self.sync_search();
        self.sync_panel()
    }

    /// Bring the panel's mob search up to date with the player's inputs, the maps the
    /// crawl knows, the map shown and the travel toggles.
    fn sync_search(&mut self) {
        let search = self.search.state(&searched(&self.crawl, &self.state, &self.settings));
        self.state.search = search;
    }

    /// The player pressed one of the global hotkeys (an accelerator string, as listed
    /// in the panel state's `hotkeys`). Returns what the shell must do at once, if
    /// anything: open a page, or bring the overlay in search mode back in front.
    pub fn on_hotkey(&mut self, pressed: &str) -> Option<Request> {
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
        } else if is(&hotkeys.search) {
            // Search mode stays as it is; the shell brings the overlay in front on entering it.
            if self.state.search.search_mode {
                return Some(Request::RaiseOverlay);
            }
            Action::OpenSearch
        } else if is(&hotkeys.open_meowdb_page) {
            return self.state.map.as_ref().map(|map| Request::OpenInBrowser { url: map.url.clone() });
        } else {
            return None;
        };
        self.act(action);
        None
    }

    /// Another window came to the front. The hotkeys are live while the game or one of
    /// our windows is in front, and nowhere else.
    pub fn on_foreground(&mut self, foreground: Foreground) -> &PanelState {
        self.state.hotkeys_live = foreground != Foreground::OtherApp;
        self.sync_panel()
    }

    /// These hotkeys couldn't be registered (another app has them).
    pub fn hotkeys_unavailable(&mut self, hotkeys: Vec<String>) -> &PanelState {
        self.state.unavailable_hotkeys = hotkeys;
        self.sync_panel()
    }

    /// Pick up settings saved by an earlier session. Hotkeys from before version 4
    /// (version 1's Ctrl+Shift letters, versions 2 and 3's Win+Alt) are replaced by the
    /// defaults.
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
        // The travel toggles change Distances.
        self.sync_search();
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

    /// Time passed: check on fetches in flight, and move the world crawl on. The shell
    /// calls this many times a second (`at_ms` on the same clock as frames), so nothing
    /// waits for the network.
    pub fn tick(&mut self, at_ms: u64) -> &PanelState {
        self.load_map_index(at_ms);
        if self.matcher.is_some() {
            self.shown_map.poll(&mut *self.source);
            self.sync_shown_map();
        }
        // After the shown map, which never waits for the crawl.
        if let Some(crawl) = &mut self.crawl {
            crawl.poll(&mut *self.source, at_ms);
            // Search covers the maps known so far: redone as the crawl learns more.
            if crawl.progress() != self.state.search.crawl {
                self.sync_search();
            }
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
        let index = match index {
            Ok(Fetched::Fresh(index)) => {
                self.state.recognition = Recognition::Ready;
                index
            }
            Ok(Fetched::SavedCopy { data, saved_at_unix_ms }) => {
                self.state.recognition = Recognition::SavedCopy { saved_at_unix_ms };
                data
            }
            Err(_) => {
                self.index_failed_at = Some(at_ms);
                self.state.recognition = Recognition::Unavailable;
                return;
            }
        };
        self.crawl = Some(Crawl::new(index.clone()));
        self.matcher = Some(Matcher::new(index));
        self.sync_search();
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
        self.state.takes_mouse = self.panel.takes_mouse(self.state.search.search_mode);
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

/// What mob search works over: the maps the crawl knows, the map the panel shows with
/// its page (once it has it), and the travel the player has turned on.
fn searched<'a>(crawl: &'a Option<Crawl>, state: &'a PanelState, settings: &Settings) -> Searched<'a> {
    Searched {
        crawl: crawl.as_ref(),
        here: state.map.as_ref().map(|map| Here { id: &map.id, page: state.data.data() }),
        travel: settings.travel,
    }
}

/// Whether two accelerator strings name the same keys ("Ctrl+Shift+C", "shift + ctrl + c").
fn same_hotkey(a: &str, b: &str) -> bool {
    let keys = |s: &str| {
        let mut keys: Vec<String> = s.split('+').map(|k| k.trim().to_lowercase()).collect();
        keys.sort();
        keys
    };
    keys(a) == keys(b)
}
