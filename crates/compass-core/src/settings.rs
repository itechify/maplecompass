//! What MapleCompass remembers between sessions. Plain data: the shell stores it as
//! JSON in the app's config directory and hands it back to the core at startup.

use crate::panel::{InsideOffset, PlacementMode, ScreenPoint};
use serde::{Deserialize, Serialize};

/// The player's settings. Missing fields (an older or empty file) take their defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Bumped when the shape changes incompatibly.
    pub version: u32,
    /// Expanded to the full panel, or collapsed to its icon.
    pub expanded: bool,
    /// Where the player dragged the inside panel; `None` for the default position.
    pub inside_offset: Option<InsideOffset>,
    /// Inside, docked right or left, or detached.
    pub placement: PlacementMode,
    /// Where the player put the detached panel (its top-left corner, physical screen
    /// px); `None` if it was never detached.
    pub detached_at: Option<ScreenPoint>,
    pub hotkeys: Hotkeys,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            expanded: true,
            inside_offset: None,
            placement: PlacementMode::Inside,
            detached_at: None,
            hotkeys: Hotkeys::default(),
        }
    }
}

/// Global hotkeys, as accelerator strings such as `"ctrl+shift+m"`.
///
/// They fire while the game has focus and the game never sees them, so the defaults
/// use Ctrl+Shift+letter, which MapleStory doesn't bind (it uses plain letters,
/// F-keys and the number row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Hotkeys {
    /// Collapse the panel to its icon, or expand it again.
    pub toggle_panel: String,
    /// Make the overlay draggable, or return it to click-through.
    pub edit_mode: String,
    /// Switch placement: inside, docked right, docked left, detached, and round again.
    pub cycle_placement: String,
    /// Open the shown map's MeowDB page in the browser.
    pub open_meowdb_page: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            toggle_panel: "ctrl+shift+m".into(),
            edit_mode: "ctrl+shift+e".into(),
            cycle_placement: "ctrl+shift+d".into(),
            open_meowdb_page: "ctrl+shift+o".into(),
        }
    }
}
