//! First-run setup: the Title Region the player marks, the EXP Bar Region they may mark
//! after it, their test reads, and when to ask for setup again.

use crate::panel::{ScreenRect, Size};
use crate::{CurrentMap, ExpBar, MEOWDB_CREDIT, Rect};
use serde::{Deserialize, Serialize};

/// Why the player needs to (re)do setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupReason {
    /// No Title Region has been marked yet.
    FirstRun,
    /// The game's client area is no longer the size it had when the Title Region (or the
    /// EXP Bar Region) was marked, so what they surround has probably moved.
    ResolutionChanged,
    /// The player asked to redo setup. The saved regions are read until setup is
    /// finished again.
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

/// The test read of a freshly marked EXP Bar Region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ExpBarTestRead {
    /// Waiting for the first frame captured from the marked region.
    Reading,
    /// The latest frame's figures read whole.
    Read { bar: ExpBar },
    /// The latest frame's figures couldn't be read whole.
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
    /// The test read of the EXP Bar Region just marked; `None` when none is being marked.
    pub exp_bar_test_read: Option<ExpBarTestRead>,
    /// An EXP Bar Region is kept and fits the game window: EXP tracking is on.
    pub exp_bar_kept: bool,
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

/// A marked EXP Bar Region and the size of the game client area it was marked on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpBarRegion {
    /// What the player marked, relative to the client area's top-left corner.
    pub marked: Rect,
    /// The client area's size when it was marked.
    pub client: Size,
}

/// Width allowed for the EXP figures, per px of marked height. The figures grow to the
/// right as the EXP does. On the samples the digits are 11 px tall and 8 px apart, and
/// "9999999 [99.99%]" would span ~135 px: 12 heights of a mark around the digits alone,
/// more than enough for a mark that takes in the taller "EXP." label too.
const EXP_WIDTH_PER_MARKED_HEIGHT: u32 = 12;

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
    /// The EXP Bar Region kept when setup was last finished; `None`: tracking is off.
    pub saved_exp_bar: Option<ExpBarRegion>,
    /// The region being test-read, and how that is going.
    marking: Option<(TitleRegion, TestRead)>,
    /// The EXP Bar Region being test-read, and how that is going.
    marking_exp_bar: Option<(ExpBarRegion, ExpBarTestRead)>,
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
        if let Some(client) = self.client() {
            self.marking = Some((TitleRegion { marked, client }, TestRead::Reading));
        }
    }

    /// The player marked the EXP Bar Region: test-read it.
    pub fn mark_exp_bar(&mut self, marked: Rect) {
        if let Some(client) = self.client() {
            self.marking_exp_bar = Some((ExpBarRegion { marked, client }, ExpBarTestRead::Reading));
        }
    }

    /// Drop the EXP Bar Region being marked; the one kept (if any) stays.
    pub fn discard_exp_bar_mark(&mut self) {
        self.marking_exp_bar = None;
    }

    /// Turn EXP tracking off: forget the kept EXP Bar Region and any being marked.
    pub fn clear_exp_bar(&mut self) {
        self.saved_exp_bar = None;
        self.marking_exp_bar = None;
    }

    fn client(&self) -> Option<Size> {
        self.game.map(|g| Size { width: g.width, height: g.height })
    }

    /// Finish setup, keeping:
    /// - the marked Title Region if its test read recognised a map, or with none being
    ///   marked, the kept one if it still fits;
    /// - the marked EXP Bar Region if its figures read whole, or with none being marked,
    ///   the kept one if it still fits (one marked at another resolution is dropped).
    ///
    /// Nothing changes while either mark's test read hasn't succeeded.
    pub fn finish(&mut self) {
        let title = match &self.marking {
            Some((region, TestRead::Recognised { .. })) => *region,
            Some(_) => return,
            None => match self.saved {
                Some(saved) if self.fits(saved.client) => saved,
                _ => return,
            },
        };
        let exp_bar = match &self.marking_exp_bar {
            Some((region, ExpBarTestRead::Read { .. })) => Some(*region),
            Some(_) => return,
            None => self.saved_exp_bar.filter(|r| self.fits(r.client)),
        };
        self.saved = Some(title);
        self.saved_exp_bar = exp_bar;
        self.marking = None;
        self.marking_exp_bar = None;
        self.requested = false;
    }

    /// The re-setup hotkey: open setup, even if it was needed before and closed.
    pub fn request(&mut self) {
        self.requested = true;
        self.window_open = true;
    }

    /// The player closed setup without finishing it.
    pub fn cancel(&mut self) {
        self.marking = None;
        self.marking_exp_bar = None;
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

    /// What an EXP Bar Region frame read (`None`: not whole). While an EXP Bar Region is
    /// being marked, that frame is its test read.
    pub fn on_exp_bar_read(&mut self, bar: Option<ExpBar>) {
        if let Some((_, read)) = &mut self.marking_exp_bar {
            *read = bar.map_or(ExpBarTestRead::Failed, |bar| ExpBarTestRead::Read { bar });
        }
    }

    pub fn state(&self) -> SetupState {
        let test_read = self.marking.as_ref().map(|(_, read)| read.clone());
        SetupState {
            needed: self.needed(),
            credit: matches!(test_read, Some(TestRead::Recognised { .. })).then_some(MEOWDB_CREDIT),
            test_read,
            window_open: self.window_open,
            exp_bar_test_read: self.marking_exp_bar.as_ref().map(|(_, read)| read.clone()),
            exp_bar_kept: self.saved_exp_bar.is_some_and(|r| self.fits(r.client)),
        }
    }

    fn needed(&self) -> Option<SetupReason> {
        let exp_bar_fits = self.saved_exp_bar.is_none_or(|r| self.fits(r.client));
        match self.saved {
            None => Some(SetupReason::FirstRun),
            Some(saved) if !self.fits(saved.client) || !exp_bar_fits => Some(SetupReason::ResolutionChanged),
            Some(_) if self.requested => Some(SetupReason::Requested),
            Some(_) => None,
        }
    }

    /// Whether a region was marked on a client area the size of the game's now. Without
    /// a game window there is nothing to compare, so it is taken to fit.
    fn fits(&self, client: Size) -> bool {
        self.game.is_none_or(|g| (g.width, g.height) == (client.width, client.height))
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
        if !self.fits(region.client) {
            return None;
        }
        let fit = (longest_name.unwrap_or(FALLBACK_LONGEST_NAME) as f32
            * region.marked.height as f32
            * CHAR_WIDTH_PER_MARKED_HEIGHT)
            .ceil() as u32;
        Some(on_screen(extend(region.marked, game, fit), game))
    }

    /// Where to capture the EXP Bar Region on screen: the region being marked, else the
    /// kept one, if it fits the game window as for [`Setup::title_region`].
    pub fn exp_bar_region(&self) -> Option<ScreenRect> {
        let game = self.game?;
        let region = match &self.marking_exp_bar {
            Some((region, _)) => region,
            None => self.saved_exp_bar.as_ref()?,
        };
        if !self.fits(region.client) {
            return None;
        }
        let fit = region.marked.height * EXP_WIDTH_PER_MARKED_HEIGHT;
        Some(on_screen(extend(region.marked, game, fit), game))
    }
}

/// The region to capture for a mark: the marked top-left corner and height, widened to
/// at least `fit` px (the minimap panel widens for long map names, the EXP figures for
/// more digits), all within the client area.
fn extend(marked: Rect, client: ScreenRect, fit: u32) -> Rect {
    Rect {
        x: marked.x,
        y: marked.y,
        width: marked.width.max(fit).min(client.width.saturating_sub(marked.x)),
        height: marked.height.min(client.height.saturating_sub(marked.y)),
    }
}

/// A client-relative rectangle on screen.
fn on_screen(r: Rect, game: ScreenRect) -> ScreenRect {
    ScreenRect { x: game.x + r.x as i32, y: game.y + r.y as i32, width: r.width, height: r.height }
}
