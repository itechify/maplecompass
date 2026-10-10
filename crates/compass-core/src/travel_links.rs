//! The hand-kept travel links: ways between maps that no portal shows, such as the
//! ships between regions. Each is one map change. Like the Portal Overrides, the
//! list is small and kept by hand, so fix an entry when the game turns out to differ.
//! Maps are given by their 9-digit IDs.

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
///
/// - Southperry → Lith Harbor, one way. Southperry (000000060) has Shanks, who sails
///   to Lith Harbor (010000000) once, with no ship back to Maple Island.
///
/// The ship between Ellinia Station (010002090) and Orbis Ticketing Booth (020000010) is
/// left out while Ossyria isn't in the game, as OSMS's set has it: add it back, both
/// ways, when the set has Ossyria again.
pub(crate) const TRAVEL_LINKS: &[TravelLink] =
    &[TravelLink { from: "000000060", to: "010000000", by: Step::Ship }];

/// The towns the Victoria Island taxi runs between, every one to every other,
/// when the player has turned taxis on: Lith Harbor, Henesys, Ellinia, Kerning City
/// and Perion. Sleepywood (010005000) is left out: it isn't known to have a taxi.
pub(crate) const TAXI_TOWNS: &[&str] = &["010000000", "010001000", "010002000", "010003000", "010004000"];

/// The towns a named Teleport Scroll takes the player to from any map, when the player
/// has turned Teleport Scrolls on: Henesys, Ellinia, Perion, Kerning City, Lith Harbor
/// and Sleepywood. The scrolls to Orbis (020000000) and El Nath (020001000) are left out
/// while Ossyria isn't in the game.
pub(crate) const TELEPORT_SCROLL_TOWNS: &[&str] =
    &["010001000", "010002000", "010004000", "010003000", "010000000", "010005000"];
