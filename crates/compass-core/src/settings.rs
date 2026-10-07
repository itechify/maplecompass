//! What MapleCompass remembers between sessions. Plain data: the shell stores it as
//! JSON in the app's config directory and hands it back to the core at startup.

use crate::panel::InsideOffset;
use crate::setup::TitleRegion;
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
    pub hotkeys: Hotkeys,
    /// The Title Region marked at setup; `None` until setup is first finished.
    pub title_region: Option<TitleRegion>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            expanded: true,
            inside_offset: None,
            hotkeys: Hotkeys::default(),
            title_region: None,
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
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self { toggle_panel: "ctrl+shift+m".into(), edit_mode: "ctrl+shift+e".into() }
    }
}
