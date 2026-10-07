//! MapleCompass core: the single test seam.
//!
//! Inputs are plain data (captured Title Region frames); the output is a [`PanelState`]
//! holding everything the overlay renders. No OS or UI dependencies live here.

mod prep;
mod recogniser;

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
}

/// The compass core: feed it frames, render its [`PanelState`].
pub struct Compass {
    recogniser: Recogniser,
    state: PanelState,
}

impl Compass {
    /// A core using the bundled title recogniser.
    pub fn new() -> Self {
        Self { recogniser: Recogniser::bundled(), state: PanelState::default() }
    }

    /// Read a captured Title Region frame and update the panel state.
    pub fn on_frame(&mut self, frame: Frame) -> &PanelState {
        let read = frame_to_rgb(&frame).and_then(|img| self.recogniser.read(&img));
        self.state.title = read.map(|raw| Title { street_name: raw.street, map_name: raw.map });
        &self.state
    }

    pub fn panel_state(&self) -> &PanelState {
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
