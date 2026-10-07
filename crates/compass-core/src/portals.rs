//! The Hidden Portal rule and position hints: which of a map's exits the player can't
//! see, and roughly where each one is.

use crate::map_data::HiddenPortal;

/// MeowDB's dead-end destination: a portal that goes nowhere.
const DEAD_END: &str = "999999999";

/// MeowDB portal types.
const INVISIBLE: u64 = 1;
const HIDDEN: u64 = 10;
const SCRIPT: u64 = 11;

/// One exit from a map to another, as MeowDB lists it.
pub(crate) struct Exit {
    pub name: String,
    pub kind: u64,
    pub x: f64,
    pub y: f64,
    pub to_map_id: String,
    pub to_map_name: String,
    pub to_street_name: String,
}

/// What a map's MeowDB page says about the map itself, for the rule and the hints.
pub(crate) struct MapFacts<'a> {
    pub id: &'a str,
    pub is_town: bool,
    pub minimap: Minimap,
}

/// MeowDB's minimap bounds: a point (x, y) on the map sits at
/// `((x + center_x) / width, (y + center_y) / height)` of the minimap, from its top-left.
/// All zero on maps without a minimap.
#[derive(Clone, Copy)]
pub(crate) struct Minimap {
    pub width: f64,
    pub height: f64,
    pub center_x: f64,
    pub center_y: f64,
}

/// The map's Hidden Portals, in MeowDB's order.
///
/// An exit counts when it leads to another map (not the dead end) and its type is
/// invisible (1), hidden (10) or script (11, a Quest Portal). Invisible exits on a Town
/// are Doors: drawn into the scenery, so the player sees them.
pub(crate) fn hidden_portals(map: &MapFacts, exits: Vec<Exit>) -> Vec<HiddenPortal> {
    exits
        .into_iter()
        .filter(|e| e.to_map_id != map.id && e.to_map_id != DEAD_END)
        .filter(|e| match e.kind {
            INVISIBLE => !map.is_town,
            HIDDEN | SCRIPT => true,
            _ => false,
        })
        .map(|e| HiddenPortal {
            position: position_hint(map.minimap, e.x, e.y),
            to_hidden_street: e.to_street_name == "Hidden Street",
            quest: false,
            name: e.name,
            to_map_id: e.to_map_id,
            to_map_name: e.to_map_name,
            to_street_name: e.to_street_name,
        })
        .collect()
}

/// Roughly where (x, y) is on the map, in words: "far right, lower".
fn position_hint(minimap: Minimap, x: f64, y: f64) -> String {
    if minimap.width <= 0.0 || minimap.height <= 0.0 {
        return "position unknown".into();
    }
    let across = (x + minimap.center_x) / minimap.width;
    let down = (y + minimap.center_y) / minimap.height;
    let horizontal = match across {
        a if a < 0.2 => "far left",
        a if a < 0.4 => "left",
        a if a < 0.6 => "centre",
        a if a < 0.8 => "right",
        _ => "far right",
    };
    let vertical = match down {
        d if d < 1.0 / 3.0 => "upper",
        d if d < 2.0 / 3.0 => "middle",
        _ => "lower",
    };
    format!("{horizontal}, {vertical}")
}
