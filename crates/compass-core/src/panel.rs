//! Panel layout: expanded or collapsed, dragging, and where the panel sits on screen.

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

/// Which window is in front: the game, one of MapleCompass's own, or another app's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foreground {
    Game,
    MapleCompass,
    OtherApp,
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
    /// The player dragged the overlay window so its top-left corner is at (x, y),
    /// physical screen px. It moves an inside or detached panel, and a docked panel
    /// never.
    PanelMoved { x: i32, y: i32 },
    /// The player dragged a bottom corner of the expanded panel so it is `width` CSS px
    /// (logical) wide. Its height follows its content all the same.
    PanelResized { width: u32 },
    /// Switch to the next placement: inside, docked right, docked left, detached.
    CyclePlacement,
    /// Show the next of the maps sharing the shown map's title, when the core couldn't
    /// tell which one the player is on (see [`crate::Candidates`]).
    ChooseCandidate,
    /// The player marked the Title Region (relative to the game client area) and wants
    /// a test read of it.
    MarkTitleRegion(crate::Rect),
    /// The player marked the EXP Bar Region (relative to the game client area) and wants
    /// a test read of it.
    MarkExpBarRegion(crate::Rect),
    /// Open the marking window on the Title Region or the EXP Bar Region, with a
    /// keyboard-adjustable selection.
    BeginSelection(crate::MarkTarget),
    /// Arrow keys move the selection; Alt resizes it, Shift uses ten-pixel steps.
    AdjustSelection { direction: crate::SelectionDirection, resize: bool, coarse: bool },
    /// Test-read the keyboard selection, as a mark around what it was opened on.
    ConfirmSelection,
    /// Close marking without changing the previous test read or saved region.
    CancelSelection,
    /// Drop the EXP Bar Region being marked (its test read failed, or the player skips
    /// it); the one kept, if any, stays.
    DiscardExpBarMark,
    /// Turn EXP tracking off: forget the EXP Bar Region, kept or being marked.
    ClearExpBarRegion,
    /// Finish setup: keep the marked Title Region if its test read recognised a map (or,
    /// with none marked, the saved one if it still fits), and likewise the EXP Bar
    /// Region if its figures read whole. Refused while either test read hasn't succeeded.
    FinishSetup,
    /// The player wants to redo setup (the re-setup hotkey).
    RequestSetup,
    /// The player started MapleCompass again while it runs; only one runs at a time, so
    /// this one shows itself: the panel, expanded, wherever it can show; setup
    /// otherwise. An open setup window stays as it is (the shell brings it in front).
    LaunchedAgain,
    /// The player closed setup without finishing it. On the first run that quits
    /// (`PanelState::quit`): there is nothing to read without a Title Region.
    CancelSetup,
    /// Start measuring afresh: the Session goes back to waiting for an EXP change. Until
    /// the EXP next changes, the reset can be undone.
    ResetSession,
    /// Bring back the Session the last reset ended, if the EXP hasn't changed since.
    UndoReset,
    /// Forget the higher of the shown map's Personal Best and this Visit's best there: one
    /// that came from a bug. The map's next-best Visit takes over. Until the player leaves
    /// the map, the Forget can be undone.
    ForgetBest,
    /// Bring back the best the last Forget took, if the player hasn't left the map since.
    UndoForget,
    /// Open the Grind Tracker's window, or close it (the panel's button, or its hotkey).
    ToggleGrindTracker,
    /// Close the Grind Tracker's window (its ×). The Session carries on.
    CloseGrindTracker,
    /// The player dragged the Grind Tracker's window so its top-left corner is at
    /// (x, y), physical screen px.
    GrindTrackerMoved { x: i32, y: i32 },
    /// Open the search view on its mobs, with nothing typed yet (the search hotkey, or
    /// the 🔍 at the top of the panel). A collapsed panel expands to show it.
    OpenSearch,
    /// Move the search view's highlight up the suggestions (or the map list).
    SearchPrevious,
    /// Move the search view's highlight down the suggestions (or the map list).
    SearchNext,
    /// Enter in the search view: pick the highlighted mob and list its maps, or pick
    /// the highlighted map and make it the Hunting Target.
    SearchEnter,
    /// Esc or the back arrow in the search view, or focus moving to another window: back
    /// to the normal view, with nothing changed.
    SearchEscape,
    /// The Hunting section's ×: there is no Hunting Target any more.
    ClearHuntingTarget,
    /// The Hunting section's 🔍: open the search view on the Hunting Target mob's map
    /// list, to pick another of its maps.
    ReopenMapList,
    /// Turn taxis on or off for Distance (the Taxis checkbox in the map list).
    ToggleTaxis,
    /// Turn Teleport Scrolls on or off for Distance (the Teleport Scrolls checkbox in
    /// the map list).
    ToggleTeleportScrolls,
    /// A click on a section's heading: collapse the section to it, or open it again.
    ToggleSection(Section),
}

/// A section of the current map's facts that the player can collapse to its heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Section {
    HiddenPortals,
    Mobs,
}

/// Which sections are collapsed to their headings: both open until the player collapses
/// one, and the same on every map. Saved in the settings as `collapsedSections`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CollapsedSections {
    pub hidden_portals: bool,
    pub mobs: bool,
}

impl CollapsedSections {
    /// The same, with `section` collapsed if it was open and open if it was collapsed.
    pub(crate) fn toggled(self, section: Section) -> Self {
        match section {
            Section::HiddenPortals => Self { hidden_portals: !self.hidden_portals, ..self },
            Section::Mobs => Self { mobs: !self.mobs, ..self },
        }
    }
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

/// Where the overlay window sits in the z-order, and whether it shows at all. Wherever
/// the panel is placed (inside, docked or detached), it belongs to the game window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Layer {
    /// Directly above the game window, never above an app in front of it: such an app
    /// covers the overlay too, and bringing the game back brings the overlay with it.
    AboveGame,
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

/// The default panel's top sits a third of the client's height down.
const DEFAULT_TOP_FRACTION: i32 = 3;

/// Where a panel detached before it was ever placed goes: near the top-left of the
/// primary monitor, which always has its origin at (0, 0).
const DEFAULT_DETACHED_AT: ScreenPoint = ScreenPoint { x: 100, y: 100 };

/// The expanded panel's width until the player resizes it (CSS px, logical).
pub const DEFAULT_PANEL_WIDTH: u32 = 380;
/// The narrowest the player can make the panel (CSS px, logical): its header's buttons
/// and the Map Summary's figures still fit side by side.
pub const MIN_PANEL_WIDTH: u32 = 320;

#[derive(Debug, Clone)]
pub(crate) struct Panel {
    pub sizes: PanelSizes,
    /// The expanded panel's width as the player set it (CSS px, logical); the shell
    /// turns it into [`PanelSizes::expanded`] for the display.
    pub width: u32,
    pub game: Option<GameWindow>,
    pub expanded: bool,
    /// `None` until the player drags the panel: then it uses the default position.
    pub inside_offset: Option<InsideOffset>,
    pub mode: PlacementMode,
    /// The detached panel's top-left corner; `None` until it is first detached.
    pub detached_at: Option<ScreenPoint>,
    /// Where the Title Region and the EXP Bar Region are captured on screen. A panel
    /// inside the game is kept off them, or the capture would read the panel instead.
    pub avoid: Vec<ScreenRect>,
    /// The monitors' work areas, as last known with the game window: a detached panel
    /// stays within them after the game is gone.
    pub screens: Vec<ScreenRect>,
}

impl Default for Panel {
    fn default() -> Self {
        let zero = Size { width: 0, height: 0 };
        Self {
            sizes: PanelSizes { expanded: zero, collapsed: zero },
            width: DEFAULT_PANEL_WIDTH,
            game: None,
            expanded: true,
            inside_offset: None,
            mode: PlacementMode::Inside,
            detached_at: None,
            avoid: Vec::new(),
            screens: Vec::new(),
        }
    }
}

impl Panel {
    /// The overlay window was moved so its top-left corner is at (x, y): see
    /// [`Action::PanelMoved`].
    pub fn moved_to(&mut self, x: i32, y: i32) {
        match (self.mode, &self.game) {
            (PlacementMode::Detached, _) => self.detached_at = Some(ScreenPoint { x, y }),
            (PlacementMode::Inside, Some(game)) => {
                let right = x + self.size().width as i32;
                let client = game.client;
                self.inside_offset = Some(InsideOffset { x: right - client.x, y: y - client.y });
            }
            // Docked panels stay docked.
            _ => {}
        }
    }

    /// The player resized the panel to `width` (see [`Action::PanelResized`]): never
    /// narrower than [`MIN_PANEL_WIDTH`].
    pub fn resized_to(&mut self, width: u32) {
        self.width = width.max(MIN_PANEL_WIDTH);
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
        if self.game.is_some() { Layer::AboveGame } else { Layer::Hidden }
    }

    /// The player can drag the panel by its header: inside or detached, at any time; a
    /// docked or hidden one never.
    pub fn draggable(&self) -> bool {
        self.layer() == Layer::AboveGame && matches!(self.mode, PlacementMode::Inside | PlacementMode::Detached)
    }

    fn size(&self) -> Size {
        if self.expanded { self.sizes.expanded } else { self.sizes.collapsed }
    }

    /// Where the overlay goes on screen. `None` while there is no game window, unless
    /// the panel is detached: a detached panel doesn't depend on the game. Never taller
    /// than the space where it goes: the game's client area inside, the monitor's work
    /// area below its top docked or detached.
    pub fn placement(&self) -> Option<ScreenRect> {
        if self.mode == PlacementMode::Detached {
            let at = self.detached_at.unwrap_or(DEFAULT_DETACHED_AT);
            let size = self.size();
            let panel = ScreenRect { x: at.x, y: at.y, width: size.width, height: size.height };
            // On the monitor its top-left corner is on (the last ones known, as it
            // doesn't depend on the game).
            let on = self.screens.iter().copied().find(|screen| holds(*screen, at));
            return Some(on.map_or(panel, |screen| below_top(panel, screen)));
        }
        let game = self.game.as_ref()?;
        match self.mode {
            PlacementMode::DockedRight | PlacementMode::DockedLeft => {
                let docked = self.docked(game);
                // Never off-screen: without room beside the game, it goes inside.
                let fits = game.screens.iter().find_map(|&screen| {
                    let docked = below_top(docked, screen);
                    (docked.height > 0 && contains(screen, docked)).then_some(docked)
                });
                Some(fits.unwrap_or_else(|| self.inside(game.client)))
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

    /// Inside the game's client area, no taller than it, and clear of the captured
    /// regions: beside them at full height if there is room, or else in the tallest of
    /// the spaces above, between and below them.
    fn inside(&self, game: ScreenRect) -> ScreenRect {
        let height = self.size().height.min(game.height);
        let panel = clear_of(self.inside_at(game, height), &self.avoid, game);
        if clear(panel, &self.avoid) {
            return panel;
        }
        let height = height.min(tallest_gap(&self.avoid, game));
        clear_of(self.inside_at(game, height), &self.avoid, game)
    }

    /// The panel `height` px tall at its spot inside the game's client area: where the
    /// player dragged it, or the default one.
    fn inside_at(&self, game: ScreenRect, height: u32) -> ScreenRect {
        let size = Size { height, ..self.size() };
        let (w, h) = (size.width as i32, size.height as i32);
        let (x, y) = match self.inside_offset {
            Some(offset) => (game.x + offset.x - w, game.y + offset.y),
            // On the right edge, its top a third of the way down: clear of the minimap
            // (top-left) and the in-game Quest Helper (top-right). The top stays put as
            // the panel grows and shrinks with its content, so the map name doesn't move.
            None => (
                game.x + game.width as i32 - DEFAULT_RIGHT_MARGIN - w,
                game.y + game.height as i32 / DEFAULT_TOP_FRACTION,
            ),
        };
        // Keep the whole panel inside the client (its top-left wins if it can't fit).
        let x = x.min(game.x + game.width as i32 - w).max(game.x);
        let y = y.min(game.y + game.height as i32 - h).max(game.y);
        ScreenRect { x, y, width: size.width, height: size.height }
    }
}

/// `rect`, cut short at the bottom of `screen` if it reaches past it.
fn below_top(rect: ScreenRect, screen: ScreenRect) -> ScreenRect {
    let room = (bottom(screen) - rect.y as i64).max(0);
    ScreenRect { height: rect.height.min(room as u32), ..rect }
}

fn bottom(r: ScreenRect) -> i64 {
    r.y as i64 + r.height as i64
}

/// Whether pixel `at` lies inside `rect`.
pub(crate) fn holds(rect: ScreenRect, at: ScreenPoint) -> bool {
    let (x, y) = (at.x as i64, at.y as i64);
    (rect.x as i64) <= x && x < rect.x as i64 + rect.width as i64 && (rect.y as i64) <= y && y < bottom(rect)
}

/// The tallest stretch of `within`, top to bottom, that none of `avoid` reaches into:
/// above the first, between two, or below the last.
fn tallest_gap(avoid: &[ScreenRect], within: ScreenRect) -> u32 {
    let mut spans: Vec<(i64, i64)> = avoid.iter().map(|r| (r.y as i64, bottom(*r))).collect();
    spans.sort();
    let (mut from, mut tallest) = (within.y as i64, 0);
    for (top, end) in spans {
        tallest = tallest.max(top - from);
        from = from.max(end);
    }
    tallest.max(bottom(within) - from).max(0) as u32
}

/// `panel`, moved the shortest way (below, above, right or left of one of the `avoid`
/// rectangles) so it overlaps none of them while staying inside `within`. Left where it
/// is when it overlaps none, or when no such spot fits.
pub(crate) fn clear_of(panel: ScreenRect, avoid: &[ScreenRect], within: ScreenRect) -> ScreenRect {
    if clear(panel, avoid) {
        return panel;
    }
    let (w, h) = (panel.width as i32, panel.height as i32);
    avoid
        .iter()
        .flat_map(|r| {
            [
                ScreenRect { y: r.y + r.height as i32, ..panel },
                ScreenRect { y: r.y - h, ..panel },
                ScreenRect { x: r.x + r.width as i32, ..panel },
                ScreenRect { x: r.x - w, ..panel },
            ]
        })
        .filter(|spot| contains(within, *spot) && clear(*spot, avoid))
        .min_by_key(|spot| (spot.x - panel.x).abs() + (spot.y - panel.y).abs())
        .unwrap_or(panel)
}

/// Whether `panel` overlaps none of `avoid`.
fn clear(panel: ScreenRect, avoid: &[ScreenRect]) -> bool {
    avoid.iter().all(|r| !overlaps(panel, *r))
}

fn overlaps(a: ScreenRect, b: ScreenRect) -> bool {
    let right = |r: ScreenRect| r.x as i64 + r.width as i64;
    let bottom = |r: ScreenRect| r.y as i64 + r.height as i64;
    (a.x as i64) < right(b) && (b.x as i64) < right(a) && (a.y as i64) < bottom(b) && (b.y as i64) < bottom(a)
}

/// Whether `inner` lies wholly inside `outer`.
pub(crate) fn contains(outer: ScreenRect, inner: ScreenRect) -> bool {
    let right = |r: ScreenRect| r.x as i64 + r.width as i64;
    let bottom = |r: ScreenRect| r.y as i64 + r.height as i64;
    inner.x >= outer.x && inner.y >= outer.y && right(inner) <= right(outer) && bottom(inner) <= bottom(outer)
}
