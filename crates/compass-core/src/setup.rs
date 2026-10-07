//! First-run setup: the Title Region the player marks, its test read, and when to ask
//! for setup again.

use crate::{CurrentMap, Rect};
use crate::panel::{Action, ScreenRect, Size};
use serde::{Deserialize, Serialize};

/// Why the player needs to (re)do setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupReason {
    /// No Title Region has been marked yet.
    FirstRun,
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
}

impl Setup {
    pub fn act(&mut self, action: Action) {
        match action {
            Action::MarkTitleRegion(marked) => {
                if let Some(game) = self.game {
                    let client = Size { width: game.width, height: game.height };
                    self.marking = Some((TitleRegion { marked, client }, TestRead::Reading));
                }
            }
            Action::FinishSetup => {
                if let Some((region, TestRead::Recognised { .. })) = &self.marking {
                    self.saved = Some(*region);
                    self.marking = None;
                }
            }
            _ => {}
        }
    }

    /// The map a frame snapped to (`None`: unreadable or not confident). While a region
    /// is being marked, that frame is its test read.
    pub fn on_read(&mut self, map: Option<CurrentMap>) {
        if let Some((_, read)) = &mut self.marking {
            *read = map.map_or(TestRead::Failed, |map| TestRead::Recognised { map });
        }
    }

    pub fn state(&self) -> SetupState {
        SetupState {
            needed: self.needed(),
            test_read: self.marking.as_ref().map(|(_, read)| read.clone()),
        }
    }

    fn needed(&self) -> Option<SetupReason> {
        match self.saved {
            None => Some(SetupReason::FirstRun),
            Some(_) => None,
        }
    }

    /// Where to capture the Title Region on screen: the region being marked, else the
    /// saved one. `None` while there is no game window or no region.
    pub fn title_region(&self, longest_name: Option<usize>) -> Option<ScreenRect> {
        let game = self.game?;
        let region = match &self.marking {
            Some((region, _)) => region,
            None => self.saved.as_ref()?,
        };
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
