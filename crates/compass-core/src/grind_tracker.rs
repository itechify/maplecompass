//! The Grind Tracker's window: whether the player wants it, and where it goes on screen.
//! It shows the Session's figures beside the panel, in a window of its own that the
//! player drags anywhere and closes with its ×.

use crate::panel::{GameWindow, ScreenPoint, ScreenRect, Size, clear_of, contains, holds};
use serde::{Deserialize, Serialize};

/// A window's top-left corner relative to the game client's top-left corner (physical
/// px). It may lie outside the client: beside the game, or on another monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientOffset {
    pub x: i32,
    pub y: i32,
}

/// The Grind Tracker's window, as the shell shows it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrindTrackerWindow {
    /// The player wants it open: the panel's Grind Tracker button shows as on.
    pub open: bool,
    /// Where it goes on screen; `None` while it is hidden: closed, with Grind Tracker
    /// off (no EXP Bar Region, or no game window), or while setup is open. Never over the
    /// Title Region or the EXP Bar Region: it shows in screen captures, ours included.
    pub placement: Option<ScreenRect>,
}

/// The gap between the panel and the Grind Tracker's window below (or above) it.
const GAP: i32 = 8;

#[derive(Debug, Clone)]
pub(crate) struct GrindTracker {
    pub open: bool,
    /// Where the player dragged it; `None` until they do: it follows the panel.
    pub at: Option<ClientOffset>,
    /// Its size in physical px for the current display, as the shell measures it.
    pub size: Size,
}

impl Default for GrindTracker {
    fn default() -> Self {
        Self { open: true, at: None, size: Size { width: 0, height: 0 } }
    }
}

impl GrindTracker {
    /// The player dragged the window so its top-left corner is at (x, y), physical
    /// screen px: it keeps that spot relative to the `game` window.
    pub fn moved_to(&mut self, x: i32, y: i32, game: &GameWindow) {
        self.at = Some(ClientOffset { x: x - game.client.x, y: y - game.client.y });
    }

    /// Where the window goes beside the `game`, with the expanded panel at `panel`:
    /// where the player dragged it, or else just below the panel (above it when there is
    /// no room below), its right edge in line with the panel's. Always wholly on a
    /// monitor, and moved the shortest way off any of `avoid`.
    pub fn placement(&self, game: &GameWindow, panel: ScreenRect, avoid: &[ScreenRect]) -> ScreenRect {
        let Size { width, height } = self.size;
        let (w, h) = (width as i32, height as i32);
        let spot = match self.at {
            Some(at) => ScreenRect { x: game.client.x + at.x, y: game.client.y + at.y, width, height },
            None => {
                let x = panel.x + panel.width as i32 - w;
                let below = ScreenRect { x, y: panel.y + panel.height as i32 + GAP, width, height };
                let above = ScreenRect { y: panel.y - GAP - h, ..below };
                let screen = screen_at(game, panel.x, panel.y);
                [below, above].into_iter().find(|spot| contains(screen, *spot)).unwrap_or(below)
            }
        };
        let screen = screen_at(game, spot.x, spot.y);
        let spot = ScreenRect {
            x: spot.x.min(screen.x + screen.width as i32 - w).max(screen.x),
            y: spot.y.min(screen.y + screen.height as i32 - h).max(screen.y),
            ..spot
        };
        let within = if contains(game.client, spot) { game.client } else { screen };
        clear_of(spot, avoid, within)
    }
}

/// The monitor's work area holding point (x, y), or else the one holding the game's
/// client area's top-left corner, or else the client area itself.
fn screen_at(game: &GameWindow, x: i32, y: i32) -> ScreenRect {
    let holding = |x, y| game.screens.iter().copied().find(|screen| holds(*screen, ScreenPoint { x, y }));
    holding(x, y).or_else(|| holding(game.client.x, game.client.y)).unwrap_or(game.client)
}
