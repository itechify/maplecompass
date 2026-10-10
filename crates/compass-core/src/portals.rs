//! The Hidden Portal rule: what kind of portal each of a map's exits is, and which the
//! player can't see; and the marks on the Map Preview that follow from it.

use crate::map_data::{HiddenPortal, Portal, PortalKind};
use crate::map_preview::{Landing, Leads, PicturePoint, PortalMark};
use crate::portal_overrides::PortalOverride;

/// The street name of a Hidden Street.
const HIDDEN_STREET: &str = "Hidden Street";

/// The dead end: where a portal that goes nowhere leads (spawn points), and the nearest
/// town of a map that has none.
pub(crate) const DEAD_END: &str = "999999999";

/// Portal types, the game's numbers as OSMS's files (and MeowDB's pages) give them.
const INVISIBLE: u64 = 1;
const HIDDEN: u64 = 10;
const SCRIPT: u64 = 11;

/// One portal on a map, as its source lists it.
pub(crate) struct Exit {
    pub name: String,
    pub kind: u64,
    pub to_map_id: String,
    pub to: Destination,
    /// The source marks it as a teleport within the map.
    pub within_map: bool,
    /// The name of the portal it lands at on its destination; for a teleport within the
    /// map, another of the map's own.
    pub landing: String,
    /// Where it is in the map's picture.
    pub at: PicturePoint,
}

/// A portal's destination as the player knows it.
pub(crate) struct Destination {
    pub map_name: String,
    pub street_name: String,
    /// It is in the map list.
    pub known: bool,
}

/// A portal a teleport within the map may land at.
struct Spot {
    name: String,
    at: PicturePoint,
    /// Its mark; a portal to the dead end (a spawn point) has none.
    mark: Option<usize>,
}

/// What the rule needs to know of the map itself.
pub(crate) struct MapFacts<'a> {
    pub id: &'a str,
    pub is_town: bool,
}

/// A map's portals, sorted by the rule.
pub(crate) struct MapPortals {
    /// Its portals to other maps, with their kinds: its edges in the world's portal graph.
    pub portals: Vec<Portal>,
    pub hidden_portals: Vec<HiddenPortal>,
    /// Every portal but those to the dead end, for the Map Preview.
    pub marks: Vec<PortalMark>,
}

/// The map's portals to other maps (not to itself or the dead end) with their kinds, its
/// Hidden Portals, and its marks; all in the source's order.
///
/// A portal is hidden when its type is invisible (1), hidden (10) or script (11, a
/// Quest Portal), except that invisible portals on a Town are Doors: drawn into the
/// scenery, so the player sees them. An entry in `overrides` for the map and portal
/// settles whether it is hidden either way, except for the destination. A script
/// portal is a Quest Portal whether hidden or not.
///
/// A teleport within the map (or a portal into the map itself) is marked, but is
/// neither an edge nor a Hidden Portal. It is never a Door, which leads into a building,
/// so an invisible one is hidden on a Town too. It lands where the portal it names is,
/// at that portal's mark if it has one (a spawn point, to the dead end, has none). Two
/// that land at each other's marks are a pair, both ways.
pub(crate) fn portals(map: &MapFacts, exits: Vec<Exit>, overrides: &[PortalOverride]) -> MapPortals {
    let mut sorted = MapPortals { portals: Vec::new(), hidden_portals: Vec::new(), marks: Vec::new() };
    // Every portal, for the teleports to land at.
    let mut spots: Vec<Spot> = Vec::new();
    // The teleports' marks, and the portals they land at.
    let mut teleports: Vec<(usize, String)> = Vec::new();
    for e in exits {
        if e.to_map_id == DEAD_END {
            spots.push(Spot { name: e.name, at: e.at, mark: None });
            continue;
        }
        spots.push(Spot { name: e.name.clone(), at: e.at, mark: Some(sorted.marks.len()) });
        let within = e.within_map || e.to_map_id == map.id;
        let overridden = overrides.iter().find(|o| o.map_id == map.id && o.portal == e.name);
        let hidden = match (overridden, e.kind) {
            (Some(o), _) => o.hidden,
            (None, INVISIBLE) => within || !map.is_town,
            (None, HIDDEN | SCRIPT) => true,
            (None, _) => false,
        };
        let kind = match e.kind {
            SCRIPT => PortalKind::Quest,
            _ if hidden => PortalKind::Hidden,
            INVISIBLE if map.is_town && !within => PortalKind::Door,
            _ => PortalKind::Visible,
        };
        let leads = match (within, e.to.known) {
            (true, _) => Leads::WithinTheMap,
            (false, true) => Leads::AnotherMap,
            (false, false) => Leads::MapNotInTheData,
        };
        let row = (hidden && !within).then_some(sorted.hidden_portals.len());
        sorted.marks.push(PortalMark {
            at: e.at,
            leads,
            kind,
            to_map_name: e.to.map_name.clone(),
            to_street_name: e.to.street_name.clone(),
            lands_at: None,
            hidden_portal: row,
        });
        if within {
            teleports.push((sorted.marks.len() - 1, e.landing));
            continue;
        }
        sorted.portals.push(Portal { name: e.name.clone(), to_map_id: e.to_map_id.clone(), kind });
        if row.is_some() {
            sorted.hidden_portals.push(HiddenPortal {
                to_hidden_street: e.to.street_name == HIDDEN_STREET,
                quest: e.kind == SCRIPT,
                name: e.name,
                to_map_id: e.to_map_id,
                to_map_name: e.to.map_name,
                to_street_name: e.to.street_name,
            });
        }
    }
    for (mark, landing) in &teleports {
        sorted.marks[*mark].lands_at = spots
            .iter()
            .find(|spot| spot.name == *landing)
            .map(|spot| Landing { at: spot.at, mark: spot.mark, both_ways: false });
    }
    // A pair: each lands at the other's mark.
    let lands_at_mark = |from: usize| sorted.marks[from].lands_at.and_then(|landing| landing.mark);
    let pairs: Vec<usize> =
        teleports.iter().map(|&(mark, _)| mark).filter(|&mark| lands_at_mark(mark).and_then(lands_at_mark) == Some(mark)).collect();
    for mark in pairs {
        if let Some(landing) = &mut sorted.marks[mark].lands_at {
            landing.both_ways = true;
        }
    }
    sorted
}
