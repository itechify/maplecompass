//! The hand-kept travel links: ways between maps that no portal shows, such as the
//! ships between regions. Each is one map change. Like the Portal Overrides, the
//! list is small and kept by hand, so fix an entry when the game turns out to differ.
//! Maps are given by their 9-digit MeowDB IDs.

use crate::world::Step;

/// A way from one map to another that isn't a portal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TravelLink {
    /// The map the player leaves from.
    pub from: &'static str,
    /// The map the player arrives on.
    pub to: &'static str,
    /// How: by ship, say.
    pub by: Step,
}

/// The links the app ships with.
///
/// Ships run station to station: from the map where the player walks in to board
/// (portal-reachable from the town) to the map where the ride lets them off. The NPCs,
/// tickets, waiting rooms and ride maps in between are all one ride, so one map change.
/// Checked against MeowDB's pages (October 2026):
///
/// - Ellinia ↔ Orbis. Ellinia Station (010002090) exits only to Ellinia, and has the
///   ticket seller Joel and the boarding NPC Cherry. Orbis Ticketing Booth (020000010)
///   exits only to Orbis, and has the ticket seller Agatha and the Platform Usher. The
///   Orbis platform maps aren't reachable by portal: Station Tunnel <To Ellinia>
///   (020000011) is entered through the Platform Usher, and its west exit only leads
///   back to the booth; Station<To Ellinia> (020000012), past the tunnel, has the
///   boarding NPC Rini. Before Takeoff <To Ellinia> (020000013) has no exits, and sends
///   the player back to the booth. The rides drop the player at Ellinia Station and at
///   the booth: the ride maps' nearest towns are those two maps. So the booth is both
///   where the Orbis ship is boarded from and where the Ellinia ship arrives.
/// - Southperry → Lith Harbor, one way. Southperry (000000060) has Shanks, who sails
///   to Lith Harbor (010000000) once, with no ship back to Maple Island.
pub(crate) const TRAVEL_LINKS: &[TravelLink] = &[
    TravelLink { from: "010002090", to: "020000010", by: Step::Ship },
    TravelLink { from: "020000010", to: "010002090", by: Step::Ship },
    TravelLink { from: "000000060", to: "010000000", by: Step::Ship },
];

/// The towns the Victoria Island taxi runs between, every one to every other,
/// when the player has turned taxis on: Lith Harbor, Henesys, Ellinia, Kerning City
/// and Perion. Sleepywood (010005000) is left out: it isn't known to have a taxi.
pub(crate) const TAXI_TOWNS: &[&str] = &["010000000", "010001000", "010002000", "010003000", "010004000"];

/// The towns a named Teleport Scroll takes the player to from any map, when the player
/// has turned Teleport Scrolls on: Henesys, Ellinia, Perion, Kerning City, Lith Harbor,
/// Sleepywood, Orbis and El Nath.
pub(crate) const TELEPORT_SCROLL_TOWNS: &[&str] =
    &["010001000", "010002000", "010004000", "010003000", "010000000", "010005000", "020000000", "020001000"];
