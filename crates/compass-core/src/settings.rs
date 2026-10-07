//! What MapleCompass remembers between sessions. Plain data: the shell stores it as
//! JSON in the app's config directory and hands it back to the core at startup.

use crate::panel::{InsideOffset, PlacementMode, ScreenPoint};
use crate::setup::TitleRegion;
use serde::{Deserialize, Serialize};

/// The settings version this core writes. Version 1 had Ctrl+Shift hotkeys, which
/// [`crate::Compass::restore`] replaces with the defaults.
pub(crate) const VERSION: u32 = 2;

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
    /// Inside, docked right or left, or detached. (Version 1 called it `placement`.)
    #[serde(alias = "placement")]
    pub placement_mode: PlacementMode,
    /// Where the player put the detached panel (its top-left corner, physical screen
    /// px); `None` if it was never detached.
    pub detached_at: Option<ScreenPoint>,
    pub hotkeys: Hotkeys,
    /// The Title Region marked at setup; `None` until setup is first finished.
    pub title_region: Option<TitleRegion>,
    /// The title of the game window the player picked by hand; `None` to look for the
    /// "MapleStory" window.
    pub game_window_title: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            expanded: true,
            inside_offset: None,
            placement_mode: PlacementMode::Inside,
            detached_at: None,
            hotkeys: Hotkeys::default(),
            title_region: None,
            game_window_title: None,
        }
    }
}

/// Global hotkeys, as accelerator strings such as `"super+alt+c"`.
///
/// They fire while the game has focus and the game never sees them, so the defaults
/// use Win+Alt+letter: MapleStory players commonly bind Ctrl and Shift, and the
/// letters avoid Windows' own Win+Alt combinations (G, R, T, K, B, D, M, H).
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
    /// Open setup again, to mark the Title Region anew.
    pub setup: String,
    /// Show the next of the maps sharing the shown map's title, when the core couldn't
    /// tell which one the player is on.
    pub choose_candidate: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            toggle_panel: "super+alt+c".into(),
            edit_mode: "super+alt+e".into(),
            cycle_placement: "super+alt+p".into(),
            open_meowdb_page: "super+alt+o".into(),
            setup: "super+alt+s".into(),
            choose_candidate: "super+alt+n".into(),
        }
    }
}
