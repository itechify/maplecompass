//! MapleCompass core: the single test seam.
//!
//! Inputs are plain data (captured Title Region and EXP Bar Region frames or their
//! readings, the game window, the clock, the player's actions and hotkeys); the output
//! is a [`PanelState`] holding everything the overlay renders. No OS or UI dependencies
//! live here.

mod crawl;
mod exp_bar;
mod grind_tracker;
mod item_lookup;
mod item_tooltip;
mod map_data;
mod map_index;
mod matcher;
pub mod meowdb;
mod mob_card;
mod pace;
mod panel;
mod personal_best;
mod portal_overrides;
mod portals;
mod prep;
mod recogniser;
mod saved_world;
mod search;
mod session;
mod settings;
mod setup;
mod shown_map;
mod tracker;
mod travel_links;
mod world;

pub use crawl::CrawlProgress;
pub use exp_bar::{ExpBar, ExpBarReader, ExpBarReading};
pub use grind_tracker::{ClientOffset, GrindTrackerWindow};
pub use item_lookup::{ItemChoice, ItemHit, ItemLookup, ItemLookupLine, OurWindow};
pub use item_tooltip::{ItemCapture, ItemTooltip, ItemTooltipReader, ItemTooltipReading};
pub use map_data::{
    HiddenPortal, LISTED_MOBS, MapData, MapDataStatus, MapSpot, MapSummary, Mob, MoreMobs, PerParty,
    Portal, PortalKind,
};
pub use map_index::{Fetched, MapDataSource, MapEntry, SourceError};
pub use mob_card::{DamageRange, Elements, HpPerExp, MobCard, MobDetails, MobPage};
pub use portal_overrides::{PORTAL_OVERRIDES, PortalOverride};
pub use panel::{
    Action, CollapsedSections, DEFAULT_PANEL_WIDTH, Foreground, GameWindow, InsideOffset, Layer, MIN_PANEL_WIDTH, PanelSizes,
    PlacementMode, ScreenPoint,
    ScreenRect, Section, Size,
};
pub use personal_best::{BESTS_VERSION, BestNamed, MapBests, PersonalBestLine, SavedBests, VisitBest, Whose};
pub use saved_world::{SavedMap, SavedMapData, SavedWorld, WORLD_VERSION};
pub use search::{DensityTier, HuntingTarget, MobChoice, RouteStop, SearchResult, SearchState, SearchView, Trip};
pub use session::{Deaths, ExpRate, ExpTracking, TrackingState};
pub use settings::{Hotkeys, Settings};
pub use setup::{
    ExpBarRegion, ExpBarTestRead, MarkTarget, SelectionDirection, SetupReason, SetupState, TestRead,
    TitleRegion,
};
pub use shown_map::{Candidate, Candidates, FIRST_MOBS, LevelRange, Outline};
pub use world::{Distance, Step, Travel};

use crawl::Crawl;
use grind_tracker::GrindTracker;
use item_lookup::ItemLookups;
use search::{MobSearch, Searched};
use shown_map::ShownMap;
use matcher::{Match, Matcher, Snap};
use mob_card::MobCards;
use pace::Pace;
use panel::Panel;
use session::Session;
use setup::Setup;
pub use recogniser::{TitleLines, TitleReader, TitleReading};
use tracker::Tracker;
use world::Here;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::task::Poll;

/// A captured Title Region or EXP Bar Region image, or the game's whole client area for
/// Item Lookup: tightly packed RGBA, top-down rows.
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

/// The panel's credit when it lists mobs: their Mob Cards show images from maplestory.io
/// (ADR 0006).
pub const MEOWDB_AND_IMAGES_CREDIT: &str = "NiaMeowDB (meowdb.com) · images: maplestory.io";

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
    /// [`MEOWDB_CREDIT`] whenever a map is shown: its names and data are MeowDB's; with
    /// Mob Cards, [`MEOWDB_AND_IMAGES_CREDIT`].
    pub credit: Option<&'static str>,
    /// The Mob Card of each mob the panel lists, in its order; empty while it lists none.
    pub mob_cards: Vec<MobCard>,
    /// Expanded to the full panel, or collapsed to a small icon.
    pub expanded: bool,
    /// The sections the player collapsed to their headings.
    pub collapsed_sections: CollapsedSections,
    /// A visible game window was found (not minimised).
    pub game_found: bool,
    /// Where the player wants the panel: inside, docked or detached.
    pub placement_mode: PlacementMode,
    /// The expanded panel's width (CSS px, logical), as the player set it by dragging
    /// a bottom corner: never under [`MIN_PANEL_WIDTH`]. The shell sizes the window to it.
    pub panel_width: u32,
    /// Where the overlay goes on screen; `None` while there is no game window, unless
    /// the panel is detached. Inside the game it never covers `title_region` or
    /// `exp_bar_region`: the overlay shows in screen captures, ours included.
    pub placement: Option<ScreenRect>,
    /// Where the overlay sits in the z-order, or hidden.
    pub layer: Layer,
    /// The player can drag the panel by its header (inside or detached). The overlay is
    /// clickable whenever it shows, and never takes the keyboard on a click (ADR 0001).
    pub draggable: bool,
    /// First-run setup and re-setup.
    pub setup: SetupState,
    /// Where on screen to capture the Title Region (physical px); `None` while there
    /// is no game window or the region isn't known to fit it.
    pub title_region: Option<ScreenRect>,
    /// Where on screen to capture the EXP Bar Region (physical px); `None` while EXP
    /// tracking is off, there is no game window, or the region isn't known to fit it.
    pub exp_bar_region: Option<ScreenRect>,
    /// What the EXP bar showed when its figures last read whole; `None` while there is
    /// no `exp_bar_region` or nothing has read yet.
    pub exp_bar: Option<ExpBar>,
    /// EXP tracking: the Session's state, Time to Level, rates and deaths.
    pub exp_tracking: ExpTracking,
    /// The Grind Tracker's window, which shows `exp_tracking`.
    pub grind_tracker: GrindTrackerWindow,
    /// The title of the game window to look for: "MapleStory", or one the player
    /// picked by hand.
    pub game_window_title: String,
    /// The global hotkeys to register, as the overlay names them.
    pub hotkeys: Hotkeys,
    /// Hotkeys the shell couldn't register, so they do nothing, to tell the player.
    pub unavailable_hotkeys: Vec<String>,
    /// Those of them that don't name a key combination; the rest another app has.
    pub invalid_hotkeys: Vec<String>,
    /// The shell keeps the hotkeys registered only while this is set: while the game or
    /// one of our windows is in front. Other apps keep their own Ctrl+Shift shortcuts.
    pub hotkeys_live: bool,
    /// The player closed first-run setup: with no Title Region there is nothing to
    /// read, so the app quits.
    pub quit: bool,
    /// Mob search, and how far the world crawl behind it has got.
    pub search: SearchState,
    /// What the latest Item Lookup came to, for a few seconds; `None` once it has gone.
    pub item_lookup: Option<ItemLookupLine>,
    /// Where its alert goes on screen: at the top centre of the game window, clear of the
    /// captured regions; `None` while there is none or no game window.
    pub item_lookup_placement: Option<ScreenRect>,
}

/// What a hotkey asks the shell to do at once, beyond rendering the panel state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Open this page in the player's browser (the open-page hotkey, or a click on a
    /// listed mob's name).
    OpenInBrowser { url: String },
    /// Bring the overlay, with the search view already open, back in front (the search
    /// hotkey): the game may have come in front of it. Our own window only (ADR 0001).
    RaiseOverlay,
    /// Item Lookup (ADR 0007): read the pointer's position and capture the game's whole
    /// client area now, while the Item Tooltip shows, for [`Compass::on_item_capture`]
    /// (or [`Compass::on_item_tooltip_reading`] with what an [`ItemTooltipReader`] reads
    /// in it, or `None` if the capture failed).
    CaptureItemTooltip,
}

/// Something the player can do with a hotkey or with the matching icon at the top of
/// the panel, or with an icon at the top of the Grind Tracker's window (which have no
/// hotkeys). Either way it goes through [`Compass::press`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PanelControl {
    /// Collapse the panel to its icon, or expand it again.
    TogglePanel,
    /// Switch placement: inside, docked right, docked left, detached.
    CyclePlacement,
    /// Open the shown map's MeowDB page in the browser.
    OpenMeowdbPage,
    /// Help & setup: open setup again.
    Setup,
    /// Show the next of the maps sharing the shown map's title.
    ChooseCandidate,
    /// Open the search view.
    Search,
    /// Open the Grind Tracker's window, or close it.
    ToggleGrindTracker,
    /// The Grind Tracker window's reset button: start the Session afresh, back to waiting
    /// for an EXP change.
    ResetSession,
    /// The Grind Tracker window's Undo, after a reset: bring the old Session back.
    UndoReset,
    /// The Forget in the Grind Tracker's Personal Best line, opened under the pointer:
    /// forget the higher of the map's Personal Best and this Visit's best there.
    ForgetBest,
    /// The Grind Tracker window's Undo, after a Forget: bring the best back.
    UndoForget,
    /// The Grind Tracker window's ×: close it.
    CloseGrindTracker,
}

/// The game window's title, unless the player picked another by hand.
pub const GAME_WINDOW_TITLE: &str = "MapleStory";

/// How long to wait before asking for the map index again after a failure. MeowDB is
/// asked politely (ADR 0003): never on every frame.
const INDEX_RETRY_MS: u64 = 60_000;

/// The world crawl's progress reaches the panel no more often than this (capture time),
/// unless the crawl is done: each change re-renders the panel.
const PROGRESS_EVERY_MS: u64 = 1_000;

/// The compass core: feed it frames, render its [`PanelState`].
pub struct Compass {
    /// Reads frames fed to [`Compass::on_frame`]; loaded on the first one, so a shell
    /// that reads on its own thread never loads it twice.
    reader: Option<TitleReader>,
    /// Reads frames fed to [`Compass::on_exp_bar_frame`]; loaded on the first one.
    exp_bar_reader: Option<ExpBarReader>,
    /// What the EXP bar showed when its figures last read whole.
    exp_bar: Option<ExpBar>,
    /// The Session measured from the EXP bar's readings.
    session: Session,
    /// The Grind Tracker's window.
    grind_tracker: GrindTracker,
    source: Box<dyn MapDataSource + Send>,
    matcher: Option<Matcher>,
    /// When asking for the map index last failed (frame time).
    index_failed_at: Option<u64>,
    tracker: Tracker,
    shown_map: ShownMap,
    /// The world crawl, from when the map index arrives.
    crawl: Option<Crawl>,
    /// What an earlier launch's crawl learned, for the crawl to start from.
    saved_world: SavedWorld,
    /// The shell is loading the saved world: the crawl holds until it arrives.
    world_awaited: bool,
    /// When the crawl's progress last reached the panel (capture time).
    progress_shown_at: Option<u64>,
    /// The wall clock (ms since the Unix epoch), as the shell last told it.
    wall_clock: Option<u64>,
    /// The listed mobs' pages and images, for their Mob Cards.
    mob_cards: MobCards,
    /// MeowDB's pace, kept by the crawl and the Mob Cards' pages together.
    meowdb_pace: Pace,
    /// What the player typed, highlighted and picked in mob search.
    search: MobSearch,
    /// Reads captures fed to [`Compass::on_item_capture`]; loaded on the first one.
    item_reader: Option<ItemTooltipReader>,
    /// The latest Item Lookup.
    item_lookups: ItemLookups,
    /// The Item Lookup alert's size, in physical px for the current display.
    item_lookup_size: Size,
    /// What the shell must do that came of time passing (a page MeowDB's answer settled),
    /// for [`Compass::take_request`].
    requests: VecDeque<Request>,
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
            exp_bar_reader: None,
            exp_bar: None,
            session: Session::default(),
            grind_tracker: GrindTracker::default(),
            source: Box::new(source),
            matcher: None,
            index_failed_at: None,
            tracker: Tracker::default(),
            shown_map: ShownMap::default(),
            crawl: None,
            saved_world: SavedWorld::default(),
            world_awaited: false,
            progress_shown_at: None,
            wall_clock: None,
            mob_cards: MobCards::default(),
            meowdb_pace: Pace::default(),
            search: MobSearch::default(),
            item_reader: None,
            item_lookups: ItemLookups::default(),
            item_lookup_size: Size { width: 0, height: 0 },
            requests: VecDeque::new(),
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
        // Another title confirmed is another map: a new Visit. Losing the title, or choosing
        // among the maps sharing it, is not.
        if self.shown_map.show(maps, &mut *self.source) {
            self.session.arrive();
        }
        self.sync_shown_map();
        self.state.confidence = (tracked.confidence * 100.0).round() as u8;
        self.state.not_sure = tracked.not_sure;
        self.sync_panel()
    }

    /// Read a captured EXP Bar Region frame and update the panel state: the same as
    /// [`Compass::on_exp_bar_reading`] with what an [`ExpBarReader`] reads in it, here.
    pub fn on_exp_bar_frame(&mut self, frame: Frame) -> &PanelState {
        let reading = self.exp_bar_reader.get_or_insert_with(ExpBarReader::bundled).read(&frame);
        self.on_exp_bar_reading(reading)
    }

    /// Take what an EXP Bar Region frame read as (read by an [`ExpBarReader`], possibly
    /// on another thread) and update the panel state. A frame whose figures don't read
    /// whole keeps the last reading; time still passes for the Session, which says so
    /// once the bar has gone unread for a while.
    pub fn on_exp_bar_reading(&mut self, reading: ExpBarReading) -> &PanelState {
        self.setup.on_exp_bar_read(reading.bar);
        match reading.bar {
            Some(bar) if self.tracking_on() => self.session.on_bar(bar, reading.at_ms),
            None if self.tracking_on() => self.session.on_unread(reading.at_ms),
            _ => self.session.tick(reading.at_ms),
        }
        if reading.bar.is_some() {
            self.exp_bar = reading.bar;
        }
        self.sync_panel()
    }

    fn sync_shown_map(&mut self) {
        let map = self.shown_map.map().cloned();
        let data = self.shown_map.data();
        // Distances are measured from the map shown (with Candidates, the one shown),
        // and its page's exits count before the crawl gets to it.
        let moved = map.as_ref().map(|m| &m.id) != self.state.map.as_ref().map(|m| &m.id);
        let paged = data.data() != self.state.data.data();
        // Arriving on the Hunting Target's map (with Candidates, showing it), or leaving it
        // after.
        if let (true, Some(map)) = (moved, &map) {
            self.search.arrived(&map.id);
        }
        self.state.map = map;
        self.state.data = data;
        if moved || paged {
            self.sync_search();
        }
        self.state.candidates = self.shown_map.candidates();
        let listed = self.state.data.data().map_or(&[][..], |data| &data.listed_mobs);
        self.mob_cards.list(listed);
        self.state.mob_cards = self.mob_cards.cards();
        let credit = if self.state.mob_cards.is_empty() { MEOWDB_CREDIT } else { MEOWDB_AND_IMAGES_CREDIT };
        self.state.credit = self.state.map.as_ref().map(|_| credit);
    }

    /// The game window moved, resized, appeared or went away (`None`: not found, or
    /// minimised).
    pub fn on_game_window(&mut self, game: Option<GameWindow>) -> &PanelState {
        self.setup.game = game.as_ref().map(|g| g.client);
        if let Some(game) = &game {
            self.panel.screens = game.screens.clone();
        }
        self.panel.game = game;
        self.sync_panel()
    }

    /// The overlay's expanded and collapsed sizes, in physical px for the current display.
    pub fn set_panel_sizes(&mut self, sizes: PanelSizes) -> &PanelState {
        self.panel.sizes = sizes;
        self.sync_panel()
    }

    /// The Item Lookup alert's size, in physical px for the current display.
    pub fn set_item_lookup_size(&mut self, size: Size) -> &PanelState {
        self.item_lookup_size = size;
        self.sync_panel()
    }

    /// The Grind Tracker window's size, in physical px for the current display.
    pub fn set_grind_tracker_size(&mut self, size: Size) -> &PanelState {
        self.grind_tracker.size = size;
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
            Action::PanelMoved { x, y } => self.panel.moved_to(x, y),
            Action::PanelResized { width } => self.panel.resized_to(width),
            Action::CyclePlacement => self.panel.cycle_placement(),
            Action::ChooseCandidate => {
                self.shown_map.choose_next(&mut *self.source);
                self.sync_shown_map();
            }
            Action::MarkTitleRegion(marked) => self.setup.mark(marked),
            Action::MarkExpBarRegion(marked) => {
                // The reading of the region marked before doesn't belong to this one.
                self.exp_bar = None;
                self.setup.mark_exp_bar(marked);
            }
            Action::DiscardExpBarMark => self.setup.discard_exp_bar_mark(),
            Action::ClearExpBarRegion => self.setup.clear_exp_bar(),
            Action::BeginSelection(target) => self.setup.begin_selection(target),
            Action::AdjustSelection { direction, resize, coarse } => {
                self.setup.adjust_selection(direction, resize, coarse);
            }
            Action::ConfirmSelection => {
                if self.setup.confirm_selection() == Some(MarkTarget::ExpBar) {
                    self.exp_bar = None;
                }
            }
            Action::CancelSelection => self.setup.cancel_selection(),
            Action::FinishSetup => {
                // A newly marked EXP bar: the player wants to see what it measures.
                if self.setup.finish() {
                    self.grind_tracker.open = true;
                }
            }
            Action::RequestSetup => self.setup.request(),
            Action::LaunchedAgain => {
                if self.setup.state().window_open {
                    // Setup shows already.
                } else if self.panel.layer() != Layer::Hidden {
                    self.panel.expanded = true;
                } else {
                    self.setup.request();
                }
            }
            Action::CancelSetup => {
                self.state.quit |= self.setup.state().needed == Some(SetupReason::FirstRun);
                self.setup.cancel();
            }
            Action::ResetSession => self.session.reset_by_player(),
            Action::UndoReset => self.session.undo_reset(),
            Action::ForgetBest => {
                if let Some(map) = bests_map(&self.state) {
                    self.session.forget_best(map);
                }
            }
            Action::UndoForget => self.session.undo_forget_best(),
            Action::ToggleGrindTracker => self.grind_tracker.open = !self.grind_tracker.open,
            Action::CloseGrindTracker => self.grind_tracker.open = false,
            Action::GrindTrackerMoved { x, y } => {
                if let (Some(game), Some(panel)) = (&self.panel.game, self.panel.placement()) {
                    self.grind_tracker.moved_to(x, y, game, panel, self.panel.mode, &self.panel.avoid);
                }
            }
            Action::OpenSearch => {
                // The search view is on the expanded panel.
                self.panel.expanded = true;
                self.search.open();
                self.sync_search();
            }
            Action::ReopenMapList => {
                self.panel.expanded = true;
                self.search.reopen();
                self.sync_search();
            }
            Action::SearchPrevious | Action::SearchNext => {
                let by = if action == Action::SearchNext { 1 } else { -1 };
                self.search.move_highlight(by, &searched(&self.crawl, &self.state, &self.settings));
                self.sync_search();
            }
            Action::SearchEnter => {
                self.search.enter(&searched(&self.crawl, &self.state, &self.settings));
                self.sync_search();
            }
            Action::SearchEscape => {
                self.search.escape();
                self.sync_search();
            }
            Action::ClearHuntingTarget => {
                self.search.clear();
                self.sync_search();
            }
            // Settings, so they count whether or not the search view is open.
            Action::ToggleTaxis => {
                self.settings.travel.taxis = !self.settings.travel.taxis;
                self.sync_search();
            }
            Action::ToggleTeleportScrolls => {
                self.settings.travel.teleport_scrolls = !self.settings.travel.teleport_scrolls;
                self.sync_search();
            }
            Action::ToggleSection(section) => {
                self.settings.collapsed_sections = self.settings.collapsed_sections.toggled(section);
            }
        }
        self.sync_panel()
    }

    /// The player typed in the search view: `query` is everything in the query box.
    pub fn search_query(&mut self, query: &str) -> &PanelState {
        self.search.type_query(query);
        self.sync_search();
        self.sync_panel()
    }

    /// The player clicked one of the search view's items: the mob (in the suggestions)
    /// or the map (in the map list) with this MeowDB ID. Like Enter on it.
    pub fn search_pick(&mut self, id: &str) -> &PanelState {
        self.search.pick(id, &searched(&self.crawl, &self.state, &self.settings));
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
    /// anything: open a page, or bring the overlay with the search view open back in
    /// front.
    pub fn on_hotkey(&mut self, pressed: &str) -> Option<Request> {
        let hotkeys = &self.settings.hotkeys;
        let is = |hotkey: &str| same_hotkey(hotkey, pressed);
        if is(&hotkeys.item_lookup) {
            // Only over a game window, and not while the last capture is being read.
            let capture = self.panel.game.is_some() && self.item_lookups.start();
            return capture.then_some(Request::CaptureItemTooltip);
        }
        let control = if is(&hotkeys.toggle_panel) {
            PanelControl::TogglePanel
        } else if is(&hotkeys.cycle_placement) {
            PanelControl::CyclePlacement
        } else if is(&hotkeys.setup) {
            PanelControl::Setup
        } else if is(&hotkeys.choose_candidate) {
            PanelControl::ChooseCandidate
        } else if is(&hotkeys.search) {
            PanelControl::Search
        } else if is(&hotkeys.grind_tracker) {
            PanelControl::ToggleGrindTracker
        } else if is(&hotkeys.open_meowdb_page) {
            PanelControl::OpenMeowdbPage
        } else {
            return None;
        };
        self.press(control)
    }

    /// The player used a control: its hotkey, or its icon at the top of the panel. Returns
    /// what the shell must do at once, if anything.
    pub fn press(&mut self, control: PanelControl) -> Option<Request> {
        let action = match control {
            PanelControl::TogglePanel => Action::ToggleExpanded,
            PanelControl::CyclePlacement => Action::CyclePlacement,
            PanelControl::Setup => Action::RequestSetup,
            PanelControl::ChooseCandidate => Action::ChooseCandidate,
            PanelControl::ToggleGrindTracker => Action::ToggleGrindTracker,
            PanelControl::ResetSession => Action::ResetSession,
            PanelControl::UndoReset => Action::UndoReset,
            PanelControl::ForgetBest => Action::ForgetBest,
            PanelControl::UndoForget => Action::UndoForget,
            PanelControl::CloseGrindTracker => Action::CloseGrindTracker,
            // An open search view stays as it is; the shell brings the overlay in front on
            // opening it.
            PanelControl::Search if self.search.is_open() => return Some(Request::RaiseOverlay),
            PanelControl::Search => Action::OpenSearch,
            PanelControl::OpenMeowdbPage => {
                return self.state.map.as_ref().map(|map| Request::OpenInBrowser { url: map.url.clone() });
            }
        };
        self.act(action);
        None
    }

    /// The player clicked a listed mob's name in the panel: open its MeowDB page. `None`
    /// for a mob the panel doesn't list.
    pub fn open_mob_page(&self, id: &str) -> Option<Request> {
        let card = self.state.mob_cards.iter().find(|card| card.id == id)?;
        Some(Request::OpenInBrowser { url: card.url.clone() })
    }

    /// Read an Item Lookup capture ([`Request::CaptureItemTooltip`]) and update the panel
    /// state: the same as [`Compass::on_item_tooltip_reading`] with what an
    /// [`ItemTooltipReader`] reads in it, here.
    pub fn on_item_capture(&mut self, capture: ItemCapture) -> &PanelState {
        let reading = self.item_reader.get_or_insert_with(ItemTooltipReader::bundled).read(&capture);
        self.on_item_tooltip_reading(Some(reading))
    }

    /// Take what an Item Lookup capture read as (read by an [`ItemTooltipReader`], possibly
    /// on another thread; `None` if the capture failed) and look its name up on MeowDB. A
    /// page to open comes from [`Compass::take_request`], at once if MeowDB's answer was
    /// at hand, else once it arrives.
    pub fn on_item_tooltip_reading(&mut self, reading: Option<ItemTooltipReading>) -> &PanelState {
        // Our windows show in the capture: one over the name would be read instead.
        let panel = self.state.placement.filter(|_| self.state.layer != Layer::Hidden);
        let ours: Vec<(OurWindow, ScreenRect)> = [
            panel.map(|at| (OurWindow::Panel, at)),
            self.state.grind_tracker.placement.map(|at| (OurWindow::GrindTracker, at)),
            self.state.item_lookup_placement.map(|at| (OurWindow::Alert, at)),
        ]
        .into_iter()
        .flatten()
        .collect();
        let open = self.item_lookups.read(reading, &ours, &mut *self.source, &mut self.meowdb_pace);
        self.requests.extend(open.map(|url| Request::OpenInBrowser { url }));
        self.sync_panel()
    }

    /// The player picked one of the items an Item Lookup found sharing the name read:
    /// open its MeowDB page. `None` for an item the panel doesn't list.
    pub fn pick_item(&mut self, id: &str) -> Option<Request> {
        let url = self.item_lookups.pick(id)?;
        self.sync_panel();
        Some(Request::OpenInBrowser { url })
    }

    /// What the shell must do that came of time passing or of a reading, rather than of
    /// the player's press: an Item Lookup's page, once MeowDB has answered. The shell
    /// takes them after each call, until there are none.
    pub fn take_request(&mut self) -> Option<Request> {
        self.requests.pop_front()
    }

    /// Another window came to the front. The hotkeys are live while the game or one of
    /// our windows is in front, and nowhere else.
    pub fn on_foreground(&mut self, foreground: Foreground) -> &PanelState {
        self.state.hotkeys_live = foreground != Foreground::OtherApp;
        self.sync_panel()
    }

    /// These hotkeys couldn't be registered: `taken` because another app has them,
    /// `invalid` because they don't name a key combination.
    pub fn hotkeys_unavailable(&mut self, taken: Vec<String>, invalid: Vec<String>) -> &PanelState {
        self.state.unavailable_hotkeys = taken.into_iter().chain(invalid.iter().cloned()).collect();
        self.state.invalid_hotkeys = invalid;
        self.sync_panel()
    }

    /// Pick up settings saved by an earlier session. Hotkeys from before version 4
    /// (version 1's Ctrl+Shift letters, versions 2 and 3's Win+Alt) are replaced by the
    /// defaults; files without an EXP Bar Region (all before version 3) start with EXP
    /// tracking off.
    pub fn restore(&mut self, mut settings: Settings) -> &PanelState {
        // Version 4 brought the Ctrl+Shift defaults; a later version keeps the hotkeys.
        if settings.version < 4 {
            settings.hotkeys = Hotkeys::default();
        }
        settings.version = settings::VERSION;
        self.panel.expanded = settings.expanded;
        self.panel.inside_offset = settings.inside_offset;
        self.panel.mode = settings.placement_mode;
        self.panel.detached_at = settings.detached_at;
        self.panel.resized_to(settings.panel_width.unwrap_or(DEFAULT_PANEL_WIDTH));
        self.setup.saved = settings.title_region;
        self.setup.saved_exp_bar = settings.exp_bar_region;
        self.grind_tracker.open = settings.grind_tracker_open;
        self.grind_tracker.at = settings.grind_tracker_at;
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
            panel_width: (self.panel.width != DEFAULT_PANEL_WIDTH).then_some(self.panel.width),
            title_region: self.setup.saved,
            exp_bar_region: self.setup.saved_exp_bar,
            grind_tracker_open: self.grind_tracker.open,
            grind_tracker_at: self.grind_tracker.at,
            ..self.settings.clone()
        }
    }

    /// The wall clock now, in ms since the Unix epoch: what dates the world crawl's maps,
    /// and tells which are stale. The shell tells it as time passes.
    pub fn on_wall_clock(&mut self, now_unix_ms: u64) {
        self.wall_clock = Some(now_unix_ms);
    }

    /// The shell is loading the world an earlier launch saved: the crawl holds until
    /// [`Compass::restore_world`], so it doesn't ask MeowDB for maps already known.
    pub fn await_world(&mut self) {
        self.world_awaited = true;
    }

    /// Pick up the world an earlier launch's crawl learned ([`SavedWorld::default`] when
    /// none was saved), and let the crawl go on. One saved in another shape is left. A
    /// map the crawl knows already is kept unless the saved one is newer.
    pub fn restore_world(&mut self, world: SavedWorld) -> &PanelState {
        self.world_awaited = false;
        if world.version == WORLD_VERSION {
            if let Some(crawl) = &mut self.crawl {
                crawl.restore(&world);
            }
            self.saved_world = world;
        }
        self.sync_search();
        self.sync_panel()
    }

    /// Every map the world crawl knows, for the shell to save.
    pub fn world(&self) -> SavedWorld {
        self.crawl.as_ref().map_or_else(|| self.saved_world.clone(), Crawl::saved)
    }

    /// Changes whenever what [`Compass::world`] holds does, so the shell saves it only then.
    pub fn world_revision(&self) -> u64 {
        self.crawl.as_ref().map_or(0, Crawl::revision)
    }

    /// Pick up the Personal Bests an earlier launch saved ([`SavedBests::default`] when
    /// none were). The shell restores them at startup, before any reading.
    pub fn restore_bests(&mut self, bests: SavedBests) -> &PanelState {
        self.session.restore_bests(bests);
        self.sync_panel()
    }

    /// Every map's Personal Bests, for the shell to save: the Visit under way's included,
    /// since closing MapleCompass ends it.
    pub fn bests(&self) -> SavedBests {
        self.session.bests().saved()
    }

    /// Changes whenever what [`Compass::bests`] holds does, so the shell saves them only then.
    pub fn bests_revision(&self) -> u64 {
        self.session.bests().revision()
    }

    /// Time passed: check on fetches in flight, move the world crawl on, and let the
    /// Session pause. The shell
    /// calls this many times a second (`at_ms` on the same clock as frames), so nothing
    /// waits for the network.
    pub fn tick(&mut self, at_ms: u64) -> &PanelState {
        self.session.tick(at_ms);
        self.load_map_index(at_ms);
        if self.matcher.is_some() {
            self.shown_map.poll(&mut *self.source);
            self.sync_shown_map();
            // Once the shown map's page lists its mobs.
            self.mob_cards.poll(&mut *self.source, &mut self.meowdb_pace, at_ms);
            self.state.mob_cards = self.mob_cards.cards();
        }
        // An Item Lookup's search, which the player waits on, before the crawl.
        let open = self.item_lookups.poll(&mut *self.source, &mut self.meowdb_pace, at_ms);
        self.requests.extend(open.map(|url| Request::OpenInBrowser { url }));
        // After the shown map, which never waits for the crawl, and its listed mobs' pages.
        let crawling = !self.world_awaited && !self.mob_cards.wants_meowdb(at_ms);
        if let Some(crawl) = self.crawl.as_mut().filter(|_| crawling) {
            crawl.poll(&mut *self.source, &mut self.meowdb_pace, at_ms, self.wall_clock);
            // Search covers the maps known so far: redone as the crawl learns more, but
            // no more than once a second while it goes on.
            let progress = crawl.progress();
            let due = self.progress_shown_at.is_none_or(|t| at_ms.saturating_sub(t) >= PROGRESS_EVERY_MS);
            if progress != self.state.search.crawl && (due || progress.done) {
                self.progress_shown_at = Some(at_ms);
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
        self.crawl = Some(Crawl::new(index.clone(), &self.saved_world));
        self.matcher = Some(Matcher::new(index));
        self.sync_search();
    }

    pub fn panel_state(&self) -> &PanelState {
        &self.state
    }

    fn sync_panel(&mut self) -> &PanelState {
        self.sync_setup();
        // Never over a captured region: the capture would read the panel.
        self.panel.avoid = [self.state.title_region, self.state.exp_bar_region].into_iter().flatten().collect();
        self.state.expanded = self.panel.expanded;
        self.state.collapsed_sections = self.settings.collapsed_sections;
        self.state.game_found = self.panel.game.is_some();
        self.state.placement_mode = self.panel.mode;
        self.state.panel_width = self.panel.width;
        self.state.placement = self.panel.placement();
        self.state.layer = self.panel.layer();
        self.state.draggable = self.panel.draggable();
        self.sync_grind_tracker();
        self.sync_item_lookup();
        self.state.hotkeys = self.settings.hotkeys.clone();
        self.state.game_window_title =
            self.settings.game_window_title.clone().unwrap_or_else(|| GAME_WINDOW_TITLE.into());
        &self.state
    }

    /// The Item Lookup's alert, at the top centre of the game window while it has one,
    /// clear of the captured regions.
    fn sync_item_lookup(&mut self) {
        let line = self.item_lookups.line();
        self.state.item_lookup_placement = match (&line, &self.panel.game) {
            (Some(_), Some(game)) => Some(item_lookup::alert_placement(game, self.item_lookup_size, &self.panel.avoid)),
            _ => None,
        };
        self.state.item_lookup = line;
    }

    fn sync_setup(&mut self) {
        self.setup.follow_needed();
        self.state.setup = self.setup.state();
        self.state.title_region = self.setup.title_region(self.matcher.as_ref().map(Matcher::longest_name));
        self.state.exp_bar_region = self.setup.exp_bar_region();
        if !self.tracking_on() {
            self.exp_bar = None;
        }
        self.state.exp_bar = self.exp_bar;
        // Off while there is no region to read; the Session itself carries on.
        self.state.exp_tracking = if self.tracking_on() {
            let ceiling = self.state.data.solo_exp_ceiling();
            self.session.tracking(ceiling, bests_map(&self.state), self.wall_clock)
        } else {
            ExpTracking::default()
        };
    }

    /// The Grind Tracker's window shows while the player wants it and there is something
    /// to track, but not while setup is open (it may be marking the EXP bar).
    fn sync_grind_tracker(&mut self) {
        let shows = self.grind_tracker.open && self.tracking_on() && !self.state.setup.window_open;
        let placement = match (&self.panel.game, self.panel.placement()) {
            (Some(game), Some(panel)) if shows => {
                Some(self.grind_tracker.placement(game, panel, self.panel.mode, &self.panel.avoid))
            }
            _ => None,
        };
        self.state.grind_tracker = GrindTrackerWindow { open: self.grind_tracker.open, placement };
    }

    /// Whether EXP tracking is on: there is an EXP Bar Region to read, kept and fitting
    /// the game window.
    fn tracking_on(&self) -> bool {
        self.setup.exp_bar_region().is_some()
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

/// The shown map, by MeowDB ID, if Personal Bests are kept there: only where the map is
/// certain, and has a Map Summary (in a town, quest turn-ins would set one). Certain is
/// no Candidates waiting for a choice: one map has the title, the map the player came
/// from settled it, or the player chose it. Maps that share a title but would show the
/// same are certain too, as they show the same: the panel always shows the first of
/// them, and their bests are kept under its ID.
fn bests_map(state: &PanelState) -> Option<&str> {
    let certain = state.candidates.as_ref().is_none_or(|c| c.chosen);
    let summary = state.data.solo_exp_ceiling().is_some();
    state.map.as_ref().filter(|_| certain && summary).map(|m| m.id.as_str())
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
