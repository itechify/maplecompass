//! Panel layout: expanded or collapsed, edit mode, and where the panel sits on screen.

use serde::{Deserialize, Serialize};

/// A rectangle in physical screen pixels (virtual-screen coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// The game window as the shell sees it, in physical screen pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameWindow {
    /// The client area: what the game draws in, without caption or borders.
    pub client: ScreenRect,
    /// The visible window frame (caption and borders, without the invisible resize
    /// borders). A docked panel sits flush against it.
    pub frame: ScreenRect,
    /// The work area (screen minus taskbar) of every monitor. A docked panel must fit
    /// wholly inside one of them.
    pub screens: Vec<ScreenRect>,
}

/// The overlay's size when expanded and when collapsed to an icon (physical px).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelSizes {
    pub expanded: Size,
    pub collapsed: Size,
}

/// Something the player did (through a hotkey or the overlay).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Collapse the panel to its icon, or expand it again.
    ToggleExpanded,
    /// Make the click-through overlay draggable, or return it to click-through.
    ToggleEditMode,
    /// The overlay window was moved so its top-left corner is at (x, y), physical
    /// screen px. It moves an inside panel only in edit mode, a detached panel at any
    /// time, and a docked panel never; other moves are ignored.
    PanelMoved { x: i32, y: i32 },
    /// Switch to the next placement: inside, docked right, docked left, detached.
    CyclePlacement,
    /// Show the next of the maps sharing the shown map's title, when the core couldn't
    /// tell which one the player is on (see [`crate::Candidates`]).
    ChooseCandidate,
    /// The player marked the Title Region (relative to the game client area) and wants
    /// a test read of it.
    MarkTitleRegion(crate::Rect),
    /// Keep the marked Title Region, if its test read recognised a map.
    FinishSetup,
    /// The player wants to redo setup (the re-setup hotkey).
    RequestSetup,
    /// The player closed setup without finishing it. On the first run that quits
    /// (`PanelState::quit`): there is nothing to read without a Title Region.
    CancelSetup,
}

/// Where the player wants the panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlacementMode {
    /// Over the game, at a spot relative to the game window.
    #[default]
    Inside,
    /// Outside the game window, against its right edge.
    DockedRight,
    /// Outside the game window, against its left edge.
    DockedLeft,
    /// A free window the player puts anywhere, e.g. on another monitor.
    Detached,
}

/// Where the overlay window sits in the z-order, and whether it shows at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Layer {
    /// Directly above the game window, not topmost: an app in front of the game
    /// covers the overlay too, and bringing the game back brings the overlay with it.
    AboveGame,
    /// An ordinary window (the detached panel), stacked like any other app.
    Free,
    /// Nothing to show: the game is minimised or gone. The overlay draws nothing and
    /// lets every click through.
    #[default]
    Hidden,
}

impl PlacementMode {
    fn next(self) -> Self {
        match self {
            Self::Inside => Self::DockedRight,
            Self::DockedRight => Self::DockedLeft,
            Self::DockedLeft => Self::Detached,
            Self::Detached => Self::Inside,
        }
    }
}

/// Where the inside panel sits: its top-right corner, relative to the game client's
/// top-left corner (physical px). The expanded panel and the collapsed icon share
/// this corner, so collapsing a panel on the right edge leaves the icon on that edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsideOffset {
    pub x: i32,
    pub y: i32,
}

/// A point in physical screen pixels (virtual-screen coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

/// Gap between the default panel and the game client's right edge (physical px).
const DEFAULT_RIGHT_MARGIN: i32 = 16;

/// Where a panel detached before it was ever placed goes: near the top-left of the
/// primary monitor, which always has its origin at (0, 0).
const DEFAULT_DETACHED_AT: ScreenPoint = ScreenPoint { x: 100, y: 100 };

#[derive(Debug, Clone)]
pub(crate) struct Panel {
    pub sizes: PanelSizes,
    pub game: Option<GameWindow>,
    pub expanded: bool,
    pub edit_mode: bool,
    /// `None` until the player drags the panel: then it uses the default position.
    pub inside_offset: Option<InsideOffset>,
    pub mode: PlacementMode,
    /// The detached panel's top-left corner; `None` until it is first detached.
    pub detached_at: Option<ScreenPoint>,
    /// Where the Title Region is captured on screen. A panel inside the game is kept off
    /// it, or the capture would read the panel instead of the minimap title.
    pub avoid: Option<ScreenRect>,
}

impl Default for Panel {
    fn default() -> Self {
        let zero = Size { width: 0, height: 0 };
        Self {
            sizes: PanelSizes { expanded: zero, collapsed: zero },
            game: None,
            expanded: true,
            edit_mode: false,
            inside_offset: None,
            mode: PlacementMode::Inside,
            detached_at: None,
            avoid: None,
        }
    }
}

impl Panel {
    /// The overlay window was moved so its top-left corner is at (x, y): see
    /// [`Action::PanelMoved`].
    pub fn moved_to(&mut self, x: i32, y: i32) {
        match (self.mode, self.edit_mode, &self.game) {
            // A detached panel is an ordinary window: the player moves it any time.
            (PlacementMode::Detached, _, _) => self.detached_at = Some(ScreenPoint { x, y }),
            (PlacementMode::Inside, true, Some(game)) => {
                let right = x + self.size().width as i32;
                let client = game.client;
                self.inside_offset = Some(InsideOffset { x: right - client.x, y: y - client.y });
            }
            // Docked panels stay docked; moves outside edit mode are the shell's own.
            _ => {}
        }
    }

    pub fn cycle_placement(&mut self) {
        let was = self.placement();
        self.mode = self.mode.next();
        // Detaching for the first time leaves the panel where it was.
        if self.mode == PlacementMode::Detached && self.detached_at.is_none() {
            self.detached_at = was.map(|r| ScreenPoint { x: r.x, y: r.y });
        }
    }

    pub fn layer(&self) -> Layer {
        match (self.mode, &self.game) {
            (PlacementMode::Detached, _) => Layer::Free,
            (_, Some(_)) => Layer::AboveGame,
            (_, None) => Layer::Hidden,
        }
    }

    /// The overlay takes the mouse so the player can drag it: a detached panel always,
    /// an inside panel in edit mode, a docked or hidden one never.
    pub fn draggable(&self) -> bool {
        match self.layer() {
            Layer::Free => true,
            Layer::AboveGame => self.edit_mode && self.mode == PlacementMode::Inside,
            Layer::Hidden => false,
        }
    }

    fn size(&self) -> Size {
        if self.expanded { self.sizes.expanded } else { self.sizes.collapsed }
    }

    /// Where the overlay goes on screen. `None` while there is no game window, unless
    /// the panel is detached: a detached panel doesn't depend on the game.
    pub fn placement(&self) -> Option<ScreenRect> {
        if self.mode == PlacementMode::Detached {
            let at = self.detached_at.unwrap_or(DEFAULT_DETACHED_AT);
            let size = self.size();
            return Some(ScreenRect { x: at.x, y: at.y, width: size.width, height: size.height });
        }
        let game = self.game.as_ref()?;
        match self.mode {
            PlacementMode::DockedRight | PlacementMode::DockedLeft => {
                let docked = self.docked(game);
                // Never off-screen: without room beside the game, it goes inside.
                let fits = game.screens.iter().any(|screen| contains(*screen, docked));
                Some(if fits { docked } else { self.inside(game.client) })
            }
            _ => Some(self.inside(game.client)),
        }
    }

    /// Against the outside of the game window's frame, level with the client's top.
    fn docked(&self, game: &GameWindow) -> ScreenRect {
        let size = self.size();
        let x = if self.mode == PlacementMode::DockedLeft {
            game.frame.x - size.width as i32
        } else {
            game.frame.x + game.frame.width as i32
        };
        ScreenRect { x, y: game.client.y, width: size.width, height: size.height }
    }

    fn inside(&self, game: ScreenRect) -> ScreenRect {
        let size = self.size();
        let (w, h) = (size.width as i32, size.height as i32);
        let (x, y) = match self.inside_offset {
            Some(offset) => (game.x + offset.x - w, game.y + offset.y),
            // Vertically centred on the right edge: clear of the minimap (top-left)
            // and the in-game Quest Helper (top-right).
            None => (
                game.x + game.width as i32 - DEFAULT_RIGHT_MARGIN - w,
                game.y + (game.height as i32 - h) / 2,
            ),
        };
        // Keep the whole panel inside the client (its top-left wins if it can't fit).
        let x = x.min(game.x + game.width as i32 - w).max(game.x);
        let y = y.min(game.y + game.height as i32 - h).max(game.y);
        let panel = ScreenRect { x, y, width: size.width, height: size.height };
        match self.avoid {
            Some(title) => clear_of(panel, title, game),
            None => panel,
        }
    }
}

/// `panel`, moved the shortest way (below, above, right or left of `title`) so it no
/// longer overlaps `title` while staying inside `within`. Left where it is when it
/// doesn't overlap, or when no such spot fits.
fn clear_of(panel: ScreenRect, title: ScreenRect, within: ScreenRect) -> ScreenRect {
    if !overlaps(panel, title) {
        return panel;
    }
    let (w, h) = (panel.width as i32, panel.height as i32);
    let spots = [
        ScreenRect { y: title.y + title.height as i32, ..panel },
        ScreenRect { y: title.y - h, ..panel },
        ScreenRect { x: title.x + title.width as i32, ..panel },
        ScreenRect { x: title.x - w, ..panel },
    ];
    spots
        .into_iter()
        .filter(|spot| contains(within, *spot))
        .min_by_key(|spot| (spot.x - panel.x).abs() + (spot.y - panel.y).abs())
        .unwrap_or(panel)
}

fn overlaps(a: ScreenRect, b: ScreenRect) -> bool {
    let right = |r: ScreenRect| r.x as i64 + r.width as i64;
    let bottom = |r: ScreenRect| r.y as i64 + r.height as i64;
    (a.x as i64) < right(b) && (b.x as i64) < right(a) && (a.y as i64) < bottom(b) && (b.y as i64) < bottom(a)
}

/// Whether `inner` lies wholly inside `outer`.
fn contains(outer: ScreenRect, inner: ScreenRect) -> bool {
    let right = |r: ScreenRect| r.x as i64 + r.width as i64;
    let bottom = |r: ScreenRect| r.y as i64 + r.height as i64;
    inner.x >= outer.x && inner.y >= outer.y && right(inner) <= right(outer) && bottom(inner) <= bottom(outer)
}
