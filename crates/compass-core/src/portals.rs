//! The Hidden Portal rule: what kind of portal each of a map's exits is, and which the
//! player can't see.

use crate::map_data::{HiddenPortal, Portal, PortalKind};
use crate::portal_overrides::PortalOverride;

/// The street name of a Hidden Street.
const HIDDEN_STREET: &str = "Hidden Street";

/// MeowDB's dead-end destination: a portal that goes nowhere.
pub(crate) const DEAD_END: &str = "999999999";

/// Portal types, the same numbers in MeowDB's pages and OSMS's files.
const INVISIBLE: u64 = 1;
const HIDDEN: u64 = 10;
const SCRIPT: u64 = 11;

/// One exit from a map, as its source lists it.
pub(crate) struct Exit {
    pub name: String,
    pub kind: u64,
    pub to_map_id: String,
    pub to_map_name: String,
    pub to_street_name: String,
}

/// What the rule needs to know of the map itself.
pub(crate) struct MapFacts<'a> {
    pub id: &'a str,
    pub is_town: bool,
}

/// The map's portals to other maps (not to itself or the dead end) with their kinds,
/// and its Hidden Portals; both in the source's order.
///
/// A portal is hidden when its type is invisible (1), hidden (10) or script (11, a
/// Quest Portal), except that invisible portals on a Town are Doors: drawn into the
/// scenery, so the player sees them. An entry in `overrides` for the map and portal
/// settles whether it is hidden either way, except for the destination. A script
/// portal is a Quest Portal whether hidden or not.
pub(crate) fn portals(
    map: &MapFacts,
    exits: Vec<Exit>,
    overrides: &[PortalOverride],
) -> (Vec<Portal>, Vec<HiddenPortal>) {
    let mut portals = Vec::new();
    let mut hidden_portals = Vec::new();
    for e in exits.into_iter().filter(|e| e.to_map_id != map.id && e.to_map_id != DEAD_END) {
        let overridden = overrides.iter().find(|o| o.map_id == map.id && o.portal == e.name);
        let hidden = match (overridden, e.kind) {
            (Some(o), _) => o.hidden,
            (None, INVISIBLE) => !map.is_town,
            (None, HIDDEN | SCRIPT) => true,
            (None, _) => false,
        };
        let kind = match e.kind {
            SCRIPT => PortalKind::Quest,
            _ if hidden => PortalKind::Hidden,
            INVISIBLE if map.is_town => PortalKind::Door,
            _ => PortalKind::Visible,
        };
        portals.push(Portal { name: e.name.clone(), to_map_id: e.to_map_id.clone(), kind });
        if hidden {
            hidden_portals.push(HiddenPortal {
                to_hidden_street: e.to_street_name == HIDDEN_STREET,
                quest: e.kind == SCRIPT,
                name: e.name,
                to_map_id: e.to_map_id,
                to_map_name: e.to_map_name,
                to_street_name: e.to_street_name,
            });
        }
    }
    (portals, hidden_portals)
}
