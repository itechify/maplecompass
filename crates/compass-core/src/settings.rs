//! What MapleCompass remembers between sessions. Plain data: the shell stores it as
//! JSON in the app's config directory and hands it back to the core at startup.

use crate::grind_tracker::ClientOffset;
use crate::panel::{CollapsedSections, InsideOffset, PlacementMode, ScreenPoint};
use crate::setup::{ExpBarRegion, TitleRegion};
use crate::world::Travel;
use serde::{Deserialize, Serialize};

/// The settings version this core writes. Version 1 had other Ctrl+Shift letters, and
/// versions 2 and 3 had Win+Alt hotkeys (3 was written by the EXP tracking branch);
/// [`crate::Compass::restore`] replaces all of them with the defaults. Files without an
/// EXP Bar Region (all before version 3) start with EXP tracking off.
pub(crate) const VERSION: u32 = 4;

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
    /// The expanded panel's width (CSS px, logical) as the player set it; `None` for the
    /// default width.
    pub panel_width: Option<u32>,
    pub hotkeys: Hotkeys,
    /// The Title Region marked at setup; `None` until setup is first finished.
    pub title_region: Option<TitleRegion>,
    /// The EXP Bar Region marked at setup; `None` while EXP tracking is off.
    pub exp_bar_region: Option<ExpBarRegion>,
    /// The title of the game window the player picked by hand; `None` to look for the
    /// "MapleStory" window.
    pub game_window_title: Option<String>,
    /// What Distance counts besides portals and ships (the map list's
    /// checkboxes). Saved as top-level `taxis` and `teleportScrolls`.
    #[serde(flatten)]
    pub travel: Travel,
    /// The sections the player collapsed to their headings; files from before this
    /// have both open.
    pub collapsed_sections: CollapsedSections,
    /// The player wants the Grind Tracker's window open; files from before it had a
    /// window have it open.
    pub grind_tracker_open: bool,
    /// Where the player dragged the Grind Tracker's window, relative to the game client;
    /// `None` for just below the panel.
    pub grind_tracker_at: Option<ClientOffset>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            expanded: true,
            inside_offset: None,
            placement_mode: PlacementMode::Inside,
            detached_at: None,
            panel_width: None,
            hotkeys: Hotkeys::default(),
            title_region: None,
            exp_bar_region: None,
            game_window_title: None,
            travel: Travel::default(),
            collapsed_sections: CollapsedSections::default(),
            grind_tracker_open: true,
            grind_tracker_at: None,
        }
    }
}

/// Global hotkeys, as accelerator strings such as `"ctrl+shift+c"`.
///
/// They fire while the game has focus and the game never sees them. The defaults use
/// Ctrl+Shift+letter: Windows and other apps already hold many Win+Alt letters
/// (Win+Alt+F among them), so a player who binds one of these in the game rebinds it
/// here instead. Files saved before edit mode was retired also have `editMode`, and
/// files saved before the Grind Tracker had a window have `resetSession` (Ctrl+Shift+R,
/// too easily pressed while attacking with Ctrl); both are ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Hotkeys {
    /// Collapse the panel to its icon, or expand it again.
    pub toggle_panel: String,
    /// Switch placement: inside, docked right, docked left, detached, and round again.
    pub cycle_placement: String,
    /// Open the shown map's MeowDB page in the browser.
    pub open_meowdb_page: String,
    /// Open setup again, to mark the Title Region anew.
    pub setup: String,
    /// Show the next of the maps sharing the shown map's title, when the core couldn't
    /// tell which one the player is on.
    pub choose_candidate: String,
    /// Open the mob search view.
    pub search: String,
    /// Open the Grind Tracker's window, or close it.
    pub grind_tracker: String,
}

impl Hotkeys {
    /// Every hotkey, for the shell to register: one list, so none is forgotten.
    pub fn all(&self) -> [&str; 7] {
        [
            &self.toggle_panel,
            &self.cycle_placement,
            &self.open_meowdb_page,
            &self.setup,
            &self.choose_candidate,
            &self.grind_tracker,
            &self.search,
        ]
    }
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            toggle_panel: "ctrl+shift+c".into(),
            cycle_placement: "ctrl+shift+p".into(),
            open_meowdb_page: "ctrl+shift+m".into(),
            setup: "ctrl+shift+s".into(),
            choose_candidate: "ctrl+shift+n".into(),
            search: "ctrl+shift+f".into(),
            grind_tracker: "ctrl+shift+g".into(),
        }
    }
}
