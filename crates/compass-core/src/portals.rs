//! The Hidden Portal rule and position hints: which of a map's exits the player can't
//! see, and roughly where each one is.

use crate::map_data::{HiddenPortal, MapSpot};
use crate::portal_overrides::PortalOverride;

/// The street name of a Hidden Street.
const HIDDEN_STREET: &str = "Hidden Street";

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
/// are Doors: drawn into the scenery, so the player sees them. An entry in `overrides`
/// for the map and portal settles it either way, except for the destination.
pub(crate) fn hidden_portals(
    map: &MapFacts,
    exits: Vec<Exit>,
    overrides: &[PortalOverride],
) -> Vec<HiddenPortal> {
    exits
        .into_iter()
        .filter(|e| e.to_map_id != map.id && e.to_map_id != DEAD_END)
        .filter(|e| {
            let overridden = overrides.iter().find(|o| o.map_id == map.id && o.portal == e.name);
            match (overridden, e.kind) {
                (Some(o), _) => o.hidden,
                (None, INVISIBLE) => !map.is_town,
                (None, HIDDEN | SCRIPT) => true,
                (None, _) => false,
            }
        })
        .map(|e| {
            let (position, spot) = position_hint(map.minimap, e.x, e.y);
            HiddenPortal {
                position,
                spot,
                to_hidden_street: e.to_street_name == HIDDEN_STREET,
                quest: e.kind == SCRIPT,
                name: e.name,
                to_map_id: e.to_map_id,
                to_map_name: e.to_map_name,
                to_street_name: e.to_street_name,
            }
        })
        .collect()
}

/// Roughly where (x, y) is on the map: in words ("far right, lower") and as a cell of
/// the minimap's 5x3 grid, which is `None` when the map has no minimap bounds.
fn position_hint(minimap: Minimap, x: f64, y: f64) -> (String, Option<MapSpot>) {
    if minimap.width <= 0.0 || minimap.height <= 0.0 {
        return ("position unknown".into(), None);
    }
    let across = (x + minimap.center_x) / minimap.width;
    let down = (y + minimap.center_y) / minimap.height;
    let column: u8 = match across {
        a if a < 0.2 => 0,
        a if a < 0.4 => 1,
        a if a < 0.6 => 2,
        a if a < 0.8 => 3,
        _ => 4,
    };
    let row: u8 = match down {
        d if d < 1.0 / 3.0 => 0,
        d if d < 2.0 / 3.0 => 1,
        _ => 2,
    };
    let horizontal = ["far left", "left", "centre", "right", "far right"][column as usize];
    let vertical = ["upper", "middle", "lower"][row as usize];
    (format!("{horizontal}, {vertical}"), Some(MapSpot { column, row }))
}
