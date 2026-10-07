//! First-run setup: the Title Region the player marks, its test read, and when to ask
//! for setup again.

use crate::panel::{ScreenRect, Size};
use crate::{CurrentMap, MEOWDB_CREDIT, Rect};
use serde::{Deserialize, Serialize};

/// Why the player needs to (re)do setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupReason {
    /// No Title Region has been marked yet.
    FirstRun,
    /// The game's client area is no longer the size it had when the Title Region was
    /// marked, so the title has probably moved or changed size.
    ResolutionChanged,
    /// The player asked to redo setup. The saved Title Region is read until a new one
    /// is finished.
    Requested,
}

/// The test read of a freshly marked Title Region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum TestRead {
    /// Waiting for the first frame captured from the marked region.
    Reading,
    /// The latest frame snapped to this real map.
    Recognised { map: CurrentMap },
    /// The latest frame didn't match a real map confidently (or the map index couldn't
    /// be had).
    Failed,
}

/// Setup, as the overlay and the setup window show it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    /// Why setup is needed; `None` once the Title Region is marked and still fits.
    pub needed: Option<SetupReason>,
    /// The test read of the region just marked; `None` when none is being marked.
    pub test_read: Option<TestRead>,
    /// [`MEOWDB_CREDIT`] when the test read shows a map: its names are MeowDB's.
    pub credit: Option<&'static str>,
    /// The setup window shows: when setup becomes needed, and when the player asks
    /// for it, until it is finished or closed.
    pub window_open: bool,
}

/// A marked Title Region and the size of the game client area it was marked on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleRegion {
    /// What the player marked, relative to the client area's top-left corner.
    pub marked: Rect,
    /// The client area's size when it was marked.
    pub client: Size,
}

/// Before the map index is loaded, the longest name in it is taken to be as long as
/// MeowDB's longest: "Physical Fitness Test <Normal Waiting Room>".
const FALLBACK_LONGEST_NAME: usize = 43;

/// Title width allowed per character, per px of marked height. On the samples the two
/// title lines span 36 px and average 8.9 px per character (0.25); the extra fifth
/// covers wide letters and a mark cut tighter than the text.
const CHAR_WIDTH_PER_MARKED_HEIGHT: f32 = 0.3;

#[derive(Debug, Default)]
pub(crate) struct Setup {
    pub game: Option<ScreenRect>,
    /// The Title Region kept when setup was last finished.
    pub saved: Option<TitleRegion>,
    /// The region being test-read, and how that is going.
    marking: Option<(TitleRegion, TestRead)>,
    /// The player asked to redo setup and hasn't finished or cancelled it.
    requested: bool,
    /// The setup window is showing.
    window_open: bool,
    /// Why setup was needed when the window was last opened for it (or closed).
    opened_for: Option<SetupReason>,
}

impl Setup {
    /// The player marked the Title Region: test-read it.
    pub fn mark(&mut self, marked: Rect) {
        if let Some(game) = self.game {
            let client = Size { width: game.width, height: game.height };
            self.marking = Some((TitleRegion { marked, client }, TestRead::Reading));
        }
    }

    /// Keep the marked region, if its test read recognised a map.
    pub fn finish(&mut self) {
        if let Some((region, TestRead::Recognised { .. })) = &self.marking {
            self.saved = Some(*region);
            self.marking = None;
            self.requested = false;
        }
    }

    /// The re-setup hotkey: open setup, even if it was needed before and closed.
    pub fn request(&mut self) {
        self.requested = true;
        self.window_open = true;
    }

    /// The player closed setup without finishing it.
    pub fn cancel(&mut self) {
        self.marking = None;
        self.requested = false;
        self.window_open = false;
    }

    /// Open the setup window when setup becomes needed (or needed for a new reason),
    /// and close it once it isn't.
    pub fn follow_needed(&mut self) {
        let needed = self.needed();
        if needed.is_none() {
            self.window_open = false;
        } else if needed != self.opened_for {
            self.window_open = true;
        }
        self.opened_for = needed;
    }

    /// The map a frame snapped to (`None`: unreadable or not confident). While a region
    /// is being marked, that frame is its test read.
    pub fn on_read(&mut self, map: Option<CurrentMap>) {
        if let Some((_, read)) = &mut self.marking {
            *read = map.map_or(TestRead::Failed, |map| TestRead::Recognised { map });
        }
    }

    pub fn state(&self) -> SetupState {
        let test_read = self.marking.as_ref().map(|(_, read)| read.clone());
        SetupState {
            needed: self.needed(),
            credit: matches!(test_read, Some(TestRead::Recognised { .. })).then_some(MEOWDB_CREDIT),
            test_read,
            window_open: self.window_open,
        }
    }

    fn needed(&self) -> Option<SetupReason> {
        match self.saved {
            None => Some(SetupReason::FirstRun),
            Some(saved) if !self.fits(&saved) => Some(SetupReason::ResolutionChanged),
            Some(_) if self.requested => Some(SetupReason::Requested),
            Some(_) => None,
        }
    }

    /// Whether a region was marked on a client area the size of the game's now. Without
    /// a game window there is nothing to compare, so it is taken to fit.
    fn fits(&self, region: &TitleRegion) -> bool {
        self.game.is_none_or(|g| (g.width, g.height) == (region.client.width, region.client.height))
    }

    /// Where to capture the Title Region on screen: the region being marked, else the
    /// saved one. `None` while there is no game window, or no region marked on a client
    /// area of the game's current size: reading the wrong spot would go unnoticed.
    pub fn title_region(&self, longest_name: Option<usize>) -> Option<ScreenRect> {
        let game = self.game?;
        let region = match &self.marking {
            Some((region, _)) => region,
            None => self.saved.as_ref()?,
        };
        if !self.fits(region) {
            return None;
        }
        let r = extend(region.marked, game, longest_name.unwrap_or(FALLBACK_LONGEST_NAME));
        Some(ScreenRect {
            x: game.x + r.x as i32,
            y: game.y + r.y as i32,
            width: r.width,
            height: r.height,
        })
    }
}

/// The region to capture for a mark: the marked top-left corner and height, widened to
/// fit the longest name (the minimap panel widens for long map names), all within the
/// client area.
fn extend(marked: Rect, client: ScreenRect, longest_name: usize) -> Rect {
    let fit = (longest_name as f32 * marked.height as f32 * CHAR_WIDTH_PER_MARKED_HEIGHT).ceil() as u32;
    Rect {
        x: marked.x,
        y: marked.y,
        width: marked.width.max(fit).min(client.width.saturating_sub(marked.x)),
        height: marked.height.min(client.height.saturating_sub(marked.y)),
    }
}
