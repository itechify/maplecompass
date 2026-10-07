//! The hand-kept portal override list: exceptions to the Hidden Portal rule found in
//! play. Add an entry when a portal the panel lists turns out to be plainly visible in
//! the game, or when one it leaves out turns out to be hidden. Each entry is keyed by the
//! map's 9-digit MeowDB ID and the portal's MeowDB name (`HiddenPortal::name`, or the
//! `name` of an entry in the map page's `portals`).

/// Force one portal in or out of a map's Hidden Portals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalOverride {
    /// The map the portal is on: MeowDB's 9-digit ID, zero padding kept.
    pub map_id: &'static str,
    /// The portal's MeowDB name, e.g. `"in00"`.
    pub portal: &'static str,
    /// `true` lists the portal as a Hidden Portal whatever its type or map; `false`
    /// leaves it out. An exit into the same map or to the dead end is never listed.
    pub hidden: bool,
}

/// The overrides the app ships with. None has been found in play yet.
pub const PORTAL_OVERRIDES: &[PortalOverride] = &[];
