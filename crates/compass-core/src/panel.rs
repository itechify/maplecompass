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
    /// screen px. Only a drag in edit mode moves the panel; other moves are ignored.
    PanelMoved { x: i32, y: i32 },
}

/// Where the inside panel sits: its top-right corner, relative to the game client's
/// top-left corner (physical px). The expanded panel and the collapsed icon share
/// this corner, so collapsing a panel on the right edge leaves the icon on that edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsideOffset {
    pub x: i32,
    pub y: i32,
}

/// Gap between the default panel and the game client's right edge (physical px).
const DEFAULT_RIGHT_MARGIN: i32 = 16;

#[derive(Debug, Clone)]
pub(crate) struct Panel {
    pub sizes: PanelSizes,
    pub game: Option<ScreenRect>,
    pub expanded: bool,
    pub edit_mode: bool,
    /// `None` until the player drags the panel: then it uses the default position.
    pub inside_offset: Option<InsideOffset>,
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
        }
    }
}

impl Panel {
    pub fn act(&mut self, action: Action) {
        match action {
            Action::ToggleExpanded => self.expanded = !self.expanded,
            Action::ToggleEditMode => self.edit_mode = !self.edit_mode,
            Action::PanelMoved { x, y } => {
                if let (true, Some(game)) = (self.edit_mode, self.game) {
                    let right = x + self.size().width as i32;
                    self.inside_offset = Some(InsideOffset { x: right - game.x, y: y - game.y });
                }
            }
        }
    }

    fn size(&self) -> Size {
        if self.expanded { self.sizes.expanded } else { self.sizes.collapsed }
    }

    /// Where the overlay goes on screen, or `None` while there is no game window.
    pub fn placement(&self) -> Option<ScreenRect> {
        let game = self.game?;
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
        Some(ScreenRect { x, y, width: size.width, height: size.height })
    }
}
