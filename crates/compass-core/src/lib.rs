//! MapleCompass core: the single test seam.
//!
//! Inputs are plain data (captured Title Region and EXP Bar Region frames or their
//! readings, the game window, the clock, the player's actions and hotkeys); the output
//! is a [`PanelState`] holding everything the overlay renders. No OS or UI dependencies
//! live here.

mod bests_page;
mod exp_bar;
mod grind_tracker;
mod hotkey_settings;
mod item_lookup;
mod item_tooltip;
mod map_data;
mod map_index;
mod map_preview;
mod matcher;
pub mod meowdb;
mod mob_card;
pub mod osms;
mod pace;
mod panel;
mod personal_best;
mod pictures;
mod portal_overrides;
mod portals;
mod prep;
mod recogniser;
mod search;
mod session;
mod settings;
mod setup;
mod shown_map;
mod tracker;
mod travel_links;
mod world;

pub use bests_page::{BestsPage, BestsPageMap};
pub use exp_bar::{ExpBar, ExpBarReader, ExpBarReading};
pub use grind_tracker::{ClientOffset, GrindTrackerWindow};
pub use hotkey_settings::{KeyPress, RebindProblem, SettingsNotice, SettingsView};
pub use item_lookup::{ItemChoice, ItemHit, ItemLookup, ItemLookupLine, OurWindow};
pub use item_tooltip::{ItemCapture, ItemTooltip, ItemTooltipReader, ItemTooltipReading};
pub use map_data::{
    HiddenPortal, MapData, MapDataStatus, MapSummary, Mob, PerParty, Portal, PortalKind,
};
pub use map_index::{Fetched, MapDataSource, MapEntry, SourceError};
pub use map_preview::{Landing, Leads, MapPreview, PicturePoint, PortalMark, PreviewImage};
pub use mob_card::{DamageRange, Elements, HpPerExp, MobCard, MobDetails};
pub use portal_overrides::{PORTAL_OVERRIDES, PortalOverride};
pub use panel::{
    Action, CollapsedSections, DEFAULT_PANEL_WIDTH, Foreground, GameWindow, InsideOffset, Layer, MIN_PANEL_WIDTH, PanelSizes,
    PlacementMode, ScreenPoint,
    ScreenRect, Section, Size,
};
pub use personal_best::{BESTS_VERSION, MapBests, SavedBests, VisitBest};
pub use search::{DensityTier, HuntingTarget, MobChoice, RouteStop, SearchResult, SearchState, SearchView, Trip};
pub use session::{Deaths, ExpRate, ExpTracking, TrackingState};
pub use settings::{Hotkey, Hotkeys, Settings};
pub use setup::{
    ExpBarRegion, ExpBarTestRead, MarkTarget, SelectionDirection, SetupReason, SetupState, TestRead,
    TitleRegion,
};
pub use shown_map::{Candidate, Candidates, FIRST_MOBS, LevelRange, Outline};
pub use world::{Distance, Step, Travel};

use grind_tracker::GrindTracker;
use hotkey_settings::HotkeySettings;
use item_lookup::ItemLookups;
use search::{MobSearch, Searched};
use shown_map::ShownMap;
use matcher::{Match, Matcher, Snap};
use pace::Pace;
use pictures::Pictures;
use panel::Panel;
use session::Session;
use setup::Setup;
pub use recogniser::{TitleLines, TitleReader, TitleReading};
use tracker::Tracker;
use serde::{Deserialize, Serialize};
use osms::MapSet;
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
/// enough for the longest name in the map list ("Physical Fitness Test <Normal Waiting
/// Room>", ~470 px of panel). The core trims each frame where the panel ends, so a
/// narrow minimap's surroundings are never read. Clamped to the client area.
pub fn default_title_region(client_width: u32, client_height: u32) -> Rect {
    const Y: u32 = 16;
    const WIDTH: u32 = 560;
    const HEIGHT: u32 = 96;
    let y = Y.min(client_height);
    Rect { x: 0, y, width: WIDTH.min(client_width), height: HEIGHT.min(client_height - y) }
}

/// The map the panel shows: a real map from the map list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentMap {
    /// The 9-digit map ID, zero padding kept.
    pub id: String,
    pub street_name: String,
    pub map_name: String,
    /// The map's MeowDB page, which the title and the open-page hotkey open.
    pub url: String,
}

/// Search's credit (ADR 0008): its results and routes are worked out from OSMS Data
/// Explorer's data, and open no pages, so MeowDB isn't credited.
pub const OSMS_ONLY_CREDIT: &str = "Data: OSMS Data Explorer (osmsdataexplorer.com)";

/// The panel's credit (ADR 0008): its data is OSMS Data Explorer's, and its title and
/// mobs' names open NiaMeowDB's pages.
pub const OSMS_AND_MEOWDB_CREDIT: &str =
    "Data: OSMS Data Explorer (osmsdataexplorer.com) · Pages: NiaMeowDB (meowdb.com)";

/// Whether minimap titles can be recognised as real maps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Recognition {
    /// OSMS's file set hasn't arrived yet (it is asked for at startup).
    #[default]
    Starting,
    Ready,
    /// OSMS hasn't been reached for over 7 days, so titles are snapped to a copy of its
    /// file set saved at `saved_at_unix_ms` (ms since the Unix epoch).
    SavedCopy {
        #[serde(rename = "savedAtUnixMs")]
        saved_at_unix_ms: u64,
    },
    /// OSMS couldn't be reached and no file set is saved, so readings can't be snapped to
    /// real maps. The map list is asked for again every minute.
    Unavailable,
    /// OSMS's file set doesn't read, and there is no earlier one to fall back on:
    /// MapleCompass needs an update.
    FormatChanged,
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
    /// The shown map's Map Summary, mobs and Hidden Portals from OSMS's file set, or why they
    /// aren't there.
    pub data: MapDataStatus,
    /// The shown map's Map Preview: its picture and every portal on it, marked. `None`
    /// with no map shown, on a map without a picture, and while it isn't settled which
    /// of the maps sharing a title the player is on.
    pub map_preview: Option<MapPreview>,
    /// [`OSMS_AND_MEOWDB_CREDIT`] whenever a map is shown.
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
    /// Whether the overlay shows, on top of everything, or is hidden: it shows only with
    /// the game, while the game or one of our windows is in front.
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
    /// Not while the player rebinds one: the combination they press must reach the overlay.
    pub hotkeys_live: bool,
    /// The settings view, which takes over the panel while it is open; `None` while it
    /// is closed. Like the search view, it has the keyboard while it is open.
    pub settings_view: Option<SettingsView>,
    /// The player closed first-run setup: with no Title Region there is nothing to
    /// read, so the app quits.
    pub quit: bool,
    /// Mob search, over every map in OSMS's file set.
    pub search: SearchState,
    /// What the latest Item Lookup came to, for a few seconds; `None` once it has gone.
    pub item_lookup: Option<ItemLookupLine>,
    /// Where its alert goes on screen: at the top centre of the game window, clear of the
    /// captured regions; `None` while there is none, no game window, or another app in
    /// front (see [`Layer`]).
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
    /// Open the Personal Bests page in the player's browser (the Grind Tracker's button):
    /// the shell renders `page` into a page of its own.
    OpenBestsPage { page: BestsPage },
}

/// Something the player can do with a hotkey or with the matching icon at the top of
/// the panel, or with the panel's settings icon or an icon at the top of the Grind
/// Tracker's window (which have no hotkeys). Either way it goes through [`Compass::press`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PanelControl {
    /// Collapse the panel to its icon, or expand it again.
    TogglePanel,
    /// Switch placement: inside, docked right, docked left, detached.
    CyclePlacement,
    /// Open the shown map's MeowDB page in the browser.
    OpenMeowdbPage,
    /// Help & Setup: open setup again.
    Setup,
    /// Show the next of the maps sharing the shown map's title.
    ChooseCandidate,
    /// Open the search view.
    Search,
    /// Open the settings view, to rebind the hotkeys.
    Settings,
    /// Open the Grind Tracker's window, or close it.
    ToggleGrindTracker,
    /// The Grind Tracker window's reset button: start the Session afresh, back to waiting
    /// for an EXP change.
    ResetSession,
    /// The Grind Tracker window's Undo, after a reset: bring the old Session back.
    UndoReset,
    /// The × on the Grind Tracker's Best row: remove the shown map's Personal Best from
    /// its history, the best of the earlier Visits.
    RemoveBest,
    /// The Grind Tracker window's ×: close it.
    CloseGrindTracker,
    /// The Grind Tracker window's Personal Bests button: open the Personal Bests page in
    /// the browser.
    OpenBestsPage,
}

/// The game window's title, unless the player picked another by hand.
pub const GAME_WINDOW_TITLE: &str = "MapleStory";

/// How long a file set is used before it is fetched afresh: a day. Game data changes only
/// on patch days, and a day is soon enough to catch one. At startup, a set saved under a
/// day ago by the wall clock is used as it is; while running, the set is fetched afresh
/// every day by capture time, however old the one held, so a set used at one daily ask is
/// always fetched afresh at the next.
const SET_RECHECK_MS: u64 = 24 * 60 * 60 * 1000;

/// With no file set at all, how long to wait before asking again after OSMS couldn't be
/// reached: never on every frame.
const SET_RETRY_MS: u64 = 60_000;

/// A file set saved longer ago than this (wall clock) is shown as a Saved Copy, with its
/// date: game data changes only on patch days, so a younger one says nothing.
const SAVED_COPY_AFTER_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// OSMS's file set as read, and when it was saved (ms since the Unix epoch): by an earlier
/// launch, or as it was fetched in this one (the wall clock then); `None` if the wall
/// clock wasn't known.
struct HeldSet {
    maps: MapSet,
    saved_at_unix_ms: Option<u64>,
}

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
    /// Every map's data, once OSMS's file set has been read: the map list, and the
    /// shown map's figures.
    set: Option<HeldSet>,
    /// When to ask for OSMS's file set next (capture time); 0 at startup.
    ask_osms_at: u64,
    /// The source is fetching OSMS's file set.
    osms_in_flight: bool,
    /// The set an earlier launch saved has been looked at (on the first tick or frame).
    saved_set_seen: bool,
    /// The set an earlier launch saved, read, while a fresh one is fetched at startup
    /// because it is too old to use as it is: what the panel falls back on if none can be
    /// had.
    fallback: Option<HeldSet>,
    /// What the file set is read with: [`PORTAL_OVERRIDES`], but for tests of the list.
    portal_overrides: Vec<PortalOverride>,
    /// Built from the map list, once there is one.
    matcher: Option<Matcher>,
    tracker: Tracker,
    shown_map: ShownMap,
    /// The wall clock (ms since the Unix epoch), as the shell last told it.
    wall_clock: Option<u64>,
    /// OSMS's pictures the panel asked for: the Map Preview's picture, and the map's mobs'
    /// animations for their Mob Cards.
    pictures: Pictures,
    /// MeowDB's pace, kept by Item Lookup's searches.
    meowdb_pace: Pace,
    /// What the player typed, highlighted and picked in mob search.
    search: MobSearch,
    /// The settings view, and the hotkey being rebound in it.
    hotkey_settings: HotkeySettings,
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
    /// A core using the given data source and default settings. OSMS's file set is
    /// asked for on the first tick or frame.
    pub fn new(source: impl MapDataSource + Send + 'static) -> Self {
        let mut compass = Self {
            reader: None,
            exp_bar_reader: None,
            exp_bar: None,
            session: Session::default(),
            grind_tracker: GrindTracker::default(),
            source: Box::new(source),
            set: None,
            ask_osms_at: 0,
            osms_in_flight: false,
            saved_set_seen: false,
            fallback: None,
            portal_overrides: PORTAL_OVERRIDES.to_vec(),
            matcher: None,
            tracker: Tracker::default(),
            shown_map: ShownMap::default(),
            wall_clock: None,
            pictures: Pictures::default(),
            meowdb_pace: Pace::default(),
            search: MobSearch::default(),
            hotkey_settings: HotkeySettings::default(),
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

    /// The same core, reading OSMS's file set with these portal overrides in place of
    /// [`PORTAL_OVERRIDES`]. Only for tests of the override list (integration tests can't
    /// see test-only items); the app keeps the list it ships.
    #[doc(hidden)]
    pub fn with_portal_overrides(mut self, overrides: Vec<PortalOverride>) -> Self {
        self.portal_overrides = overrides;
        self
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
        self.load_osms(reading.at_ms);
        let (Some(matcher), Some(set)) = (&self.matcher, &self.set) else {
            // Without the map list nothing can be recognised.
            self.setup.on_read(None);
            return self.sync_panel();
        };
        let snap = reading.lines.and_then(|lines| match matcher.snap(&lines.street, &lines.map)? {
            Match::Sure(snap) => Some(snap),
            // Names differing only in a numeral the reading doesn't make out: the
            // one within the player's reach, if just one is; otherwise not sure.
            Match::OneOf { maps, score } => {
                let reach = self.shown_map.within_reach(&set.maps);
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
        if self.shown_map.show(maps, &set.maps) {
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
        let saved_copy_from = self.saved_copy_from();
        let data = self.set.as_ref().map_or(MapDataStatus::NoMap, |set| self.shown_map.data(&set.maps, saved_copy_from));
        // Distances are measured from the map shown (with Candidates, the one shown).
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
        self.state.candidates = self.set.as_ref().and_then(|set| self.shown_map.candidates(&set.maps));
        self.sync_pictured();
        self.state.credit = self.state.map.as_ref().map(|_| OSMS_AND_MEOWDB_CREDIT);
    }

    /// What shows OSMS's pictures, with those in so far: the Map Preview (its portals on the
    /// Hunting Target's route marked), the map's mobs' Mob Cards and the Hunting Target's mob.
    fn sync_pictured(&mut self) {
        let next_map = self.state.search.hunting.as_ref().and_then(HuntingTarget::next_map_by_portal);
        self.state.map_preview = self
            .set
            .as_ref()
            .and_then(|set| self.shown_map.preview(&set.maps, &self.pictures))
            .map(|preview| preview.on_route_to(next_map));
        let mobs = self.state.data.data().map_or(&[][..], |data| &data.mobs);
        self.state.mob_cards = match &self.set {
            Some(set) => mob_card::cards(mobs, &set.maps, &self.pictures),
            None => Vec::new(),
        };
        if let (Some(hunting), Some(set)) = (&mut self.state.search.hunting, &self.set) {
            hunting.image = set.maps.animation(&hunting.mob.id).and_then(|path| self.pictures.url(path)).map(String::from);
        }
    }

    /// Ask for the pictures the panel wants, one at a time: the Map Preview's first, then
    /// the map's mobs' animations, then the Hunting Target mob's.
    fn poll_pictures(&mut self, at_ms: u64) {
        let Some(set) = &self.set else { return };
        let preview = self.state.map_preview.as_ref().map(|preview| map_preview::picture_path(&preview.picture));
        let mobs = self.state.data.data().map_or(&[][..], |data| &data.mobs);
        let hunted = self.state.search.hunting.as_ref().and_then(|hunting| set.maps.animation(&hunting.mob.id));
        let wanted: Vec<String> = preview
            .into_iter()
            .chain(mob_card::animations(mobs, &set.maps))
            .chain(hunted.map(String::from))
            .collect();
        self.pictures.poll(&wanted, &mut *self.source, at_ms);
        self.sync_pictured();
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
                // The query box goes with the panel: like Esc. So does the settings view.
                if !self.panel.expanded {
                    self.search.escape();
                    self.sync_search();
                    self.hotkey_settings.close();
                }
            }
            Action::PanelMoved { x, y } => self.panel.moved_to(x, y),
            Action::PanelResized { width } => self.panel.resized_to(width),
            Action::CyclePlacement => self.panel.cycle_placement(),
            Action::ChooseCandidate => {
                self.shown_map.choose_next();
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
                } else if self.panel.game.is_some() {
                    // It shows with the game, even if the app the player started it from
                    // is in front for now.
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
            Action::RemoveBest => {
                if let Some(map) = bests_map(&self.state) {
                    self.session.remove_best(map);
                }
            }
            Action::ToggleGrindTracker => self.grind_tracker.open = !self.grind_tracker.open,
            Action::CloseGrindTracker => self.grind_tracker.open = false,
            Action::GrindTrackerMoved { x, y } => {
                if let (Some(game), Some(panel)) = (&self.panel.game, self.panel.placement()) {
                    self.grind_tracker.moved_to(x, y, game, panel, self.panel.mode, &self.panel.avoid);
                }
            }
            Action::OpenSearch => {
                // The search view is on the expanded panel, in the settings view's place.
                self.panel.expanded = true;
                self.hotkey_settings.close();
                self.search.open();
                self.sync_search();
            }
            Action::ReopenMapList => {
                self.panel.expanded = true;
                self.hotkey_settings.close();
                self.search.reopen();
                self.sync_search();
            }
            Action::SearchPrevious | Action::SearchNext => {
                let by = if action == Action::SearchNext { 1 } else { -1 };
                self.search.move_highlight(by, &searched(&self.set, &self.state, &self.settings));
                self.sync_search();
            }
            Action::SearchEnter => {
                self.search.enter(&searched(&self.set, &self.state, &self.settings));
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
            Action::OpenSettings => {
                // On the expanded panel, in the search view's place.
                self.panel.expanded = true;
                self.search.escape();
                self.sync_search();
                self.hotkey_settings.open();
            }
            Action::CloseSettings => self.hotkey_settings.close(),
            Action::BeginRebind(hotkey) => self.hotkey_settings.begin(hotkey),
            Action::CancelRebind => self.hotkey_settings.cancel(),
            Action::TurnOffHotkey => {
                if self.hotkey_settings.turn_off(&mut self.settings.hotkeys) {
                    self.forget_unbound_hotkeys();
                }
            }
            Action::SwapHotkeys => {
                if self.hotkey_settings.swap(&mut self.settings.hotkeys) {
                    self.forget_unbound_hotkeys();
                }
            }
            Action::ResetHotkeys => {
                if self.hotkey_settings.restore(&mut self.settings.hotkeys) {
                    self.forget_unbound_hotkeys();
                }
            }
            Action::UndoResetHotkeys => {
                if self.hotkey_settings.undo_restore(&mut self.settings.hotkeys) {
                    self.forget_unbound_hotkeys();
                }
            }
        }
        self.sync_panel()
    }

    /// The player pressed a key on the settings view: while a hotkey waits for its new
    /// combination, that may be it, or Backspace or Delete to turn it off (see
    /// [`SettingsView`]). Saved with the settings, and registered in place of the old one
    /// once the hotkeys are live again.
    pub fn rebind_key(&mut self, press: KeyPress) -> &PanelState {
        if self.hotkey_settings.press(&press, &mut self.settings.hotkeys) {
            self.forget_unbound_hotkeys();
        }
        self.sync_panel()
    }

    /// The combinations no hotkey has any more aren't unavailable: the shell reports on
    /// the new ones when it registers them.
    fn forget_unbound_hotkeys(&mut self) {
        let hotkeys = &self.settings.hotkeys;
        let bound = |combination: &String| hotkeys.all().iter().any(|hotkey| same_hotkey(hotkey, combination));
        self.state.unavailable_hotkeys.retain(bound);
        self.state.invalid_hotkeys.retain(bound);
    }

    /// The player typed in the search view: `query` is everything in the query box.
    pub fn search_query(&mut self, query: &str) -> &PanelState {
        self.search.type_query(query);
        self.sync_search();
        self.sync_panel()
    }

    /// The player clicked one of the search view's items: the mob (in the suggestions)
    /// or the map (in the map list) with this ID. Like Enter on it.
    pub fn search_pick(&mut self, id: &str) -> &PanelState {
        self.search.pick(id, &searched(&self.set, &self.state, &self.settings));
        self.sync_search();
        self.sync_panel()
    }

    /// Bring the panel's mob search up to date with the player's inputs, OSMS's file set,
    /// the map shown and the travel toggles; and with it what follows from the Hunting
    /// Target: its mob's animation, and the Map Preview's portals on its route.
    fn sync_search(&mut self) {
        let search = self.search.state(&searched(&self.set, &self.state, &self.settings));
        self.state.search = search;
        self.sync_pictured();
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
            PanelControl::RemoveBest => Action::RemoveBest,
            PanelControl::CloseGrindTracker => Action::CloseGrindTracker,
            // An open search view stays as it is; the shell brings the overlay in front on
            // opening it.
            PanelControl::Search if self.search.is_open() => return Some(Request::RaiseOverlay),
            PanelControl::Search => Action::OpenSearch,
            PanelControl::Settings => Action::OpenSettings,
            PanelControl::OpenMeowdbPage => {
                return self.state.map.as_ref().map(|map| Request::OpenInBrowser { url: map.url.clone() });
            }
            // None while no map has a best: the button is hidden then.
            PanelControl::OpenBestsPage => {
                let page = self.bests_page();
                return (!page.maps.is_empty()).then_some(Request::OpenBestsPage { page });
            }
        };
        self.act(action);
        None
    }

    /// The Personal Bests page now: every map's best as saved, with what OSMS's file set
    /// held says of the map.
    fn bests_page(&self) -> BestsPage {
        let map = |id: &str| {
            let maps = &self.set.as_ref()?.maps;
            Some((maps.entry(id)?, maps.data(id)?))
        };
        bests_page::page(&self.session.bests().saved(), map, certain_map(&self.state), self.wall_clock)
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

    /// Another window came to the front. The hotkeys are live, and the overlay, the Grind
    /// Tracker's window and the Item Lookup alert show (with the game), while the game or
    /// one of our windows is in front, and nowhere else (see [`Layer`]).
    pub fn on_foreground(&mut self, foreground: Foreground) -> &PanelState {
        self.panel.foreground = Some(foreground);
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

    /// The wall clock now, in ms since the Unix epoch: what tells how old a saved copy of
    /// OSMS's file set is, and dates Personal Bests. The shell tells it as time passes.
    pub fn on_wall_clock(&mut self, now_unix_ms: u64) {
        self.wall_clock = Some(now_unix_ms);
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

    /// Time passed: check on fetches in flight, and let the Session pause. The shell calls
    /// this many times a second (`at_ms` on the same clock as frames), so nothing waits for
    /// the network.
    pub fn tick(&mut self, at_ms: u64) -> &PanelState {
        self.session.tick(at_ms);
        self.load_osms(at_ms);
        if self.set.is_some() {
            self.sync_shown_map();
            self.poll_pictures(at_ms);
        }
        // An Item Lookup's search, which the player waits on.
        let open = self.item_lookups.poll(&mut *self.source, &mut self.meowdb_pace, at_ms);
        self.requests.extend(open.map(|url| Request::OpenInBrowser { url }));
        self.sync_panel()
    }

    /// OSMS's file set. At startup (the first tick or frame), the set an earlier launch
    /// saved, if it was saved under [`SET_RECHECK_MS`] ago; otherwise, and every
    /// [`SET_RECHECK_MS`] after, one the source fetches afresh, and with no set at all, a
    /// minute after OSMS couldn't be reached. A fetched set that reads replaces the one
    /// held whole. One that can't be had or doesn't read leaves the one held as it is, and
    /// at startup falls back on the saved set, however old.
    fn load_osms(&mut self, at_ms: u64) {
        if !self.osms_in_flight {
            if at_ms < self.ask_osms_at {
                return;
            }
            if !self.saved_set_seen {
                self.saved_set_seen = true;
                if self.take_saved_set() {
                    self.ask_osms_at = at_ms + SET_RECHECK_MS;
                    return;
                }
            }
        }
        let fetched = match self.source.fetch_osms_files() {
            Poll::Pending => {
                self.osms_in_flight = true;
                return;
            }
            Poll::Ready(fetched) => fetched,
        };
        self.osms_in_flight = false;
        self.ask_osms_at = at_ms + SET_RECHECK_MS;
        let read = fetched.and_then(|files| osms::read_with(&files, &self.portal_overrides));
        match (read, self.fallback.take()) {
            // The source saved it as it fetched it: now.
            (Ok(maps), _) => self.take_set(HeldSet { maps, saved_at_unix_ms: self.wall_clock }),
            (Err(_), _) if self.set.is_some() => {}
            (Err(_), Some(saved)) => self.take_set(saved),
            (Err(SourceError::Unreachable(_)), None) => {
                self.state.recognition = Recognition::Unavailable;
                self.ask_osms_at = at_ms + SET_RETRY_MS;
            }
            (Err(SourceError::FormatChanged(_)), None) => self.state.recognition = Recognition::FormatChanged,
        }
    }

    /// Take the set an earlier launch saved if it was saved under [`SET_RECHECK_MS`] ago by
    /// the wall clock, or else keep it to fall back on; whether it was taken. Without the
    /// wall clock its age is unknown, so it isn't taken as it is, and a set that doesn't
    /// read is as good as none.
    fn take_saved_set(&mut self) -> bool {
        let Some(saved) = self.source.saved_osms_files() else { return false };
        let Ok(maps) = osms::read_with(&saved.files, &self.portal_overrides) else { return false };
        let held = HeldSet { maps, saved_at_unix_ms: Some(saved.saved_at_unix_ms) };
        let age = self.wall_clock.and_then(|now| now.checked_sub(saved.saved_at_unix_ms));
        if age.is_some_and(|age| age < SET_RECHECK_MS) {
            self.take_set(held);
            return true;
        }
        self.fallback = Some(held);
        false
    }

    /// Hold `set`, a file set just read, in place of the one held: search and Distance
    /// work over it at once. A new map list is matched against from then on, and the map
    /// the player is on stays theirs.
    fn take_set(&mut self, set: HeldSet) {
        let entries = set.maps.entries();
        if self.set.as_ref().is_none_or(|held| held.maps.entries() != entries) {
            let matcher = Matcher::new(entries);
            if let Some(old) = &self.matcher {
                self.tracker.remap(|at| matcher.index_of(&old.map(at).id));
            }
            self.matcher = Some(matcher);
        }
        self.set = Some(set);
        self.sync_search();
    }

    /// When the file set held was saved, if that was over [`SAVED_COPY_AFTER_MS`] ago, so it
    /// is a Saved Copy: no fresh one could be had since. The panel says so, with its date.
    fn saved_copy_from(&self) -> Option<u64> {
        let saved_at = self.set.as_ref()?.saved_at_unix_ms?;
        let now = self.wall_clock?;
        (now.saturating_sub(saved_at) > SAVED_COPY_AFTER_MS).then_some(saved_at)
    }

    pub fn panel_state(&self) -> &PanelState {
        &self.state
    }

    fn sync_panel(&mut self) -> &PanelState {
        if self.set.is_some() {
            self.state.recognition = match self.saved_copy_from() {
                Some(saved_at_unix_ms) => Recognition::SavedCopy { saved_at_unix_ms },
                None => Recognition::Ready,
            };
        }
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
        let in_front = self.panel.foreground.is_some_and(|foreground| foreground != Foreground::OtherApp);
        self.state.hotkeys_live = in_front && !self.hotkey_settings.is_rebinding();
        self.state.settings_view = self.hotkey_settings.state(&self.settings.hotkeys);
        self.state.game_window_title =
            self.settings.game_window_title.clone().unwrap_or_else(|| GAME_WINDOW_TITLE.into());
        &self.state
    }

    /// The Item Lookup's alert, at the top centre of the game window while it has one,
    /// clear of the captured regions; only while the overlay can show (see [`Layer`]).
    fn sync_item_lookup(&mut self) {
        let line = self.item_lookups.line();
        let shows = self.panel.layer() != Layer::Hidden;
        self.state.item_lookup_placement = match (&line, &self.panel.game) {
            (Some(_), Some(game)) if shows => {
                Some(item_lookup::alert_placement(game, self.item_lookup_size, &self.panel.avoid))
            }
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
    /// to track, but not while setup is open (it may be marking the EXP bar), and only
    /// while the overlay can show (see [`Layer`]).
    fn sync_grind_tracker(&mut self) {
        let shows = self.grind_tracker.open
            && self.tracking_on()
            && !self.state.setup.window_open
            && self.panel.layer() != Layer::Hidden;
        let placement = match (&self.panel.game, self.panel.placement()) {
            (Some(game), Some(panel)) if shows => {
                Some(self.grind_tracker.placement(game, panel, self.panel.mode, &self.panel.avoid))
            }
            _ => None,
        };
        self.state.grind_tracker = GrindTrackerWindow {
            open: self.grind_tracker.open,
            placement,
            personal_bests: self.session.bests().any(),
        };
    }

    /// Whether EXP tracking is on: there is an EXP Bar Region to read, kept and fitting
    /// the game window.
    fn tracking_on(&self) -> bool {
        self.setup.exp_bar_region().is_some()
    }
}

/// What mob search works over: OSMS's file set, the map the panel shows, and the travel
/// the player has turned on.
fn searched<'a>(set: &'a Option<HeldSet>, state: &'a PanelState, settings: &Settings) -> Searched<'a> {
    Searched {
        set: set.as_ref().map(|held| &held.maps),
        here: state.map.as_ref().map(|map| map.id.as_str()),
        travel: settings.travel,
    }
}

/// The shown map, by ID, if Personal Bests are kept there: only where the map is
/// certain, and has a Map Summary (in a town, quest turn-ins would set one).
fn bests_map(state: &PanelState) -> Option<&str> {
    certain_map(state).filter(|_| state.data.solo_exp_ceiling().is_some())
}

/// The shown map, by ID, if the panel is certain the player is on it: no
/// Candidates waiting for a choice. One map has the title, the map the player came from
/// settled it, or the player chose it. Maps that share a title but would show the same
/// are certain too, as they show the same: the panel always shows the first of them, and
/// their bests are kept under its ID.
fn certain_map(state: &PanelState) -> Option<&str> {
    let certain = state.candidates.as_ref().is_none_or(|c| c.chosen);
    state.map.as_ref().filter(|_| certain).map(|m| m.id.as_str())
}

/// Whether two accelerator strings name the same keys ("Ctrl+Shift+C", "shift + ctrl + c").
/// An empty one, a hotkey turned off, names none.
pub(crate) fn same_hotkey(a: &str, b: &str) -> bool {
    let keys = |s: &str| {
        let mut keys: Vec<String> = s.split('+').map(|k| k.trim().to_lowercase()).collect();
        keys.sort();
        keys
    };
    !a.trim().is_empty() && keys(a) == keys(b)
}
