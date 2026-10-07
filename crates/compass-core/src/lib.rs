//! MapleCompass core: the single test seam.
//!
//! Inputs are plain data (captured Title Region frames); the output is a [`PanelState`]
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
mod tracker;

pub use map_data::{HiddenPortal, MapData, MapDataStatus, MapSummary, Mob, PerParty};
pub use map_index::{MapDataSource, MapEntry, SourceError};
pub use portal_overrides::{PORTAL_OVERRIDES, PortalOverride};
pub use panel::{
    Action, GameWindow, InsideOffset, Layer, PanelSizes, PlacementMode, ScreenPoint, ScreenRect,
    Size,
};
pub use settings::{Hotkeys, Settings};

use map_data::MapDataLoader;
use matcher::Matcher;
use panel::Panel;
use recogniser::Recogniser;
use tracker::Tracker;
use serde::Serialize;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The Title Region to capture before the player has marked one, for a game client
/// area of the given size (physical px).
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
#[serde(rename_all = "camelCase")]
pub enum Recognition {
    /// No frame seen yet, so the map index hasn't been asked for.
    #[default]
    Starting,
    Ready,
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
    /// The shown map's Map Summary and mobs from MeowDB, or why they aren't there.
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
    /// the panel is detached.
    pub placement: Option<ScreenRect>,
    /// Where the overlay sits in the z-order, or hidden.
    pub layer: Layer,
    /// The overlay takes the mouse and the player drags it, so the shell leaves its
    /// position alone except when its size or layer changes. Otherwise clicks pass
    /// through to the game and the shell keeps the overlay at `placement`.
    pub draggable: bool,
}

/// How long to wait before asking for the map index again after a failure. MeowDB is
/// asked politely (ADR 0003): never on every frame.
const INDEX_RETRY_MS: u64 = 60_000;

/// The compass core: feed it frames, render its [`PanelState`].
pub struct Compass {
    recogniser: Recogniser,
    source: Box<dyn MapDataSource + Send>,
    matcher: Option<Matcher>,
    /// When asking for the map index last failed (frame time).
    index_failed_at: Option<u64>,
    tracker: Tracker,
    map_data: MapDataLoader,
    panel: Panel,
    /// The last settings restored; the panel's own fields live in `panel`.
    settings: Settings,
    state: PanelState,
}

impl Compass {
    /// A core using the bundled title recogniser, the given map data source and
    /// default settings. The map index is asked for on the first frame.
    pub fn new(source: impl MapDataSource + Send + 'static) -> Self {
        let mut compass = Self {
            recogniser: Recogniser::bundled(),
            source: Box::new(source),
            matcher: None,
            index_failed_at: None,
            tracker: Tracker::default(),
            map_data: MapDataLoader::default(),
            panel: Panel::default(),
            settings: Settings::default(),
            state: PanelState::default(),
        };
        compass.sync_panel();
        compass
    }

    /// Read a captured Title Region frame and update the panel state.
    pub fn on_frame(&mut self, frame: Frame) -> &PanelState {
        self.load_map_index(frame.at_ms);
        let Some(matcher) = &self.matcher else { return &self.state };
        let read = frame_to_rgb(&frame).and_then(|img| self.recogniser.read(&img));
        let snap = read.and_then(|raw| matcher.snap(&raw.street, &raw.map));
        let tracked = self.tracker.update(snap);
        self.state.map = tracked.current.map(|i| {
            let entry = matcher.map(i);
            CurrentMap {
                id: entry.id.clone(),
                street_name: entry.street_name.clone(),
                map_name: entry.map_name.clone(),
                url: meowdb::map_url(&entry.id),
            }
        });
        let shown = self.state.map.as_ref().map(|m| m.id.as_str());
        self.state.data = self.map_data.show(shown, &mut *self.source).clone();
        self.state.credit = shown.map(|_| MEOWDB_CREDIT);
        self.state.confidence = (tracked.confidence * 100.0).round() as u8;
        self.state.not_sure = tracked.not_sure;
        &self.state
    }

    /// The game window moved, resized, appeared or went away (`None`: not found, or
    /// minimised).
    pub fn on_game_window(&mut self, game: Option<GameWindow>) -> &PanelState {
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
        self.panel.act(action);
        self.sync_panel()
    }

    /// Pick up settings saved by an earlier session.
    pub fn restore(&mut self, settings: Settings) -> &PanelState {
        self.panel.expanded = settings.expanded;
        self.panel.inside_offset = settings.inside_offset;
        self.panel.mode = settings.placement;
        self.panel.detached_at = settings.detached_at;
        self.settings = settings;
        self.sync_panel()
    }

    /// The settings to save: whatever the player has changed since [`Compass::restore`].
    pub fn settings(&self) -> Settings {
        Settings {
            expanded: self.panel.expanded,
            inside_offset: self.panel.inside_offset,
            placement: self.panel.mode,
            detached_at: self.panel.detached_at,
            ..self.settings.clone()
        }
    }

    /// Ask the source for the map index on the first frame, and after a failure, again
    /// no sooner than [`INDEX_RETRY_MS`] later.
    fn load_map_index(&mut self, at_ms: u64) {
        if self.matcher.is_some()
            || self.index_failed_at.is_some_and(|t| at_ms.saturating_sub(t) < INDEX_RETRY_MS)
        {
            return;
        }
        match self.source.map_index() {
            Ok(index) => {
                self.matcher = Some(Matcher::new(index));
                self.state.recognition = Recognition::Ready;
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
        self.state.expanded = self.panel.expanded;
        self.state.edit_mode = self.panel.edit_mode;
        self.state.game_found = self.panel.game.is_some();
        self.state.placement_mode = self.panel.mode;
        self.state.placement = self.panel.placement();
        self.state.layer = self.panel.layer();
        self.state.draggable = self.panel.draggable();
        &self.state
    }
}

fn frame_to_rgb(frame: &Frame) -> Option<image::RgbImage> {
    let (w, h) = (frame.width as usize, frame.height as usize);
    if frame.rgba.len() != w * h * 4 {
        return None;
    }
    let rgb = frame.rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    image::RgbImage::from_raw(frame.width, frame.height, rgb)
}
