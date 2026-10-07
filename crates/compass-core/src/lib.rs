//! MapleCompass core: the single test seam.
//!
//! Inputs are plain data (captured Title Region frames); the output is a [`PanelState`]
//! holding everything the overlay renders. No OS or UI dependencies live here.

mod panel;
mod prep;
mod recogniser;

pub use panel::{Action, PanelSizes, ScreenRect, Size};

use panel::Panel;
use recogniser::Recogniser;
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

/// The street name and map name read from the minimap title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Title {
    pub street_name: String,
    pub map_name: String,
}

/// Everything the overlay renders.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelState {
    pub title: Option<Title>,
    /// Expanded to the full panel, or collapsed to a small icon.
    pub expanded: bool,
    /// The overlay takes the mouse so the player can drag it; otherwise clicks pass
    /// through to the game.
    pub edit_mode: bool,
    /// Where the overlay goes on screen; `None` while there is no game window.
    pub placement: Option<ScreenRect>,
}

/// The compass core: feed it frames, render its [`PanelState`].
pub struct Compass {
    recogniser: Recogniser,
    panel: Panel,
    state: PanelState,
}

impl Compass {
    /// A core using the bundled title recogniser.
    pub fn new() -> Self {
        let mut compass =
            Self { recogniser: Recogniser::bundled(), panel: Panel::default(), state: PanelState::default() };
        compass.sync_panel();
        compass
    }

    /// Read a captured Title Region frame and update the panel state.
    pub fn on_frame(&mut self, frame: Frame) -> &PanelState {
        let read = frame_to_rgb(&frame).and_then(|img| self.recogniser.read(&img));
        self.state.title = read.map(|raw| Title { street_name: raw.street, map_name: raw.map });
        &self.state
    }

    /// The game window's client area moved, resized, appeared or went away.
    pub fn on_game_window(&mut self, client: Option<ScreenRect>) -> &PanelState {
        self.panel.game = client;
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

    pub fn panel_state(&self) -> &PanelState {
        &self.state
    }

    fn sync_panel(&mut self) -> &PanelState {
        self.state.expanded = self.panel.expanded;
        self.state.edit_mode = self.panel.edit_mode;
        self.state.placement = self.panel.placement();
        &self.state
    }
}

impl Default for Compass {
    fn default() -> Self {
        Self::new()
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
