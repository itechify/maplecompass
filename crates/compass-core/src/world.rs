//! The world graph: which maps lead to which, from the crawled pages, and how many map
//! changes each map is from another.
//!
//! It is directed, one node per known map. Every exit to another map is an edge
//! (visible, Hidden Portal or Door, one-way or not), except Quest Portals, which are
//! kept apart: a map reachable only through one is "quest only". The hand-kept travel
//! links (ships) are edges too. Every edge is one map change, so Distance is the length
//! of the shortest path, found breadth-first.

use crate::crawl::Crawl;
use crate::map_data::PortalKind;
use crate::travel_links::{TAXI_TOWNS, TELEPORT_SCROLL_TOWNS, TRAVEL_LINKS, TravelLink};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};

/// How far a map is from the map the panel shows. Ordered as results are within a
/// Density Tier: nearest first, then no route, then quest only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Distance {
    /// The fewest map changes to get there (0: it's the map the player is on).
    MapChanges(u32),
    /// No known route leads there, Quest Portals or not.
    NoRoute,
    /// Only a route through a Quest Portal leads there.
    QuestOnly,
    /// No map is shown, so there is nowhere to measure from.
    Unknown,
}

/// How a route goes from one map to the next, in one map change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Step {
    /// Through a portal, Hidden Portal or Door (never a Quest Portal).
    Portal,
    /// A ship ride between continents.
    Ship,
    /// A taxi ride between towns.
    Taxi,
    /// A Teleport Scroll to a town.
    TeleportScroll,
}

/// The kinds of travel the player has turned on, besides portals and ships (which
/// always count).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Travel {
    pub taxis: bool,
    pub teleport_scrolls: bool,
}

/// Where each map leads in one map change, and how.
type Exits<'a> = HashMap<&'a str, Vec<(&'a str, Step)>>;

/// The maps each known map leads to, in one map change.
pub(crate) struct World<'a> {
    /// Through any exit but a Quest Portal, then by the travel links.
    exits: Exits<'a>,
    /// Through Quest Portals.
    quest_exits: Exits<'a>,
    /// Where every map leads, crawled or not, after its own exits: the named-town
    /// Teleport Scrolls, when they're on.
    from_anywhere: Vec<(&'a str, Step)>,
}

impl<'a> World<'a> {
    /// The graph of every map the crawl knows, the ships, and the `travel` the player
    /// has turned on.
    pub fn of(crawl: &'a Crawl, travel: Travel) -> Self {
        let mut world = Self { exits: HashMap::new(), quest_exits: HashMap::new(), from_anywhere: Vec::new() };
        for (entry, data) in crawl.known() {
            for portal in &data.portals {
                let exits = match portal.kind {
                    PortalKind::Quest => &mut world.quest_exits,
                    PortalKind::Visible | PortalKind::Hidden | PortalKind::Door => &mut world.exits,
                };
                exits.entry(entry.id.as_str()).or_default().push((portal.to_map_id.as_str(), Step::Portal));
            }
        }
        // After the portals, so that of equally short routes the one by portals is
        // taken. A link counts whether or not its maps have been crawled yet.
        for &TravelLink { from, to, by } in TRAVEL_LINKS {
            world.exits.entry(from).or_default().push((to, by));
        }
        if travel.taxis {
            for &from in TAXI_TOWNS {
                let to = TAXI_TOWNS.iter().filter(|&&to| to != from).map(|&to| (to, Step::Taxi));
                world.exits.entry(from).or_default().extend(to);
            }
        }
        if travel.teleport_scrolls {
            // A nearest-town scroll goes to the map's own nearest town.
            for (entry, data) in crawl.known() {
                if let Some(town) = data.nearest_town.as_deref().filter(|&town| town != entry.id) {
                    world.exits.entry(entry.id.as_str()).or_default().push((town, Step::TeleportScroll));
                }
            }
            world.from_anywhere = TELEPORT_SCROLL_TOWNS.iter().map(|&to| (to, Step::TeleportScroll)).collect();
        }
        world
    }

    /// How far every map is from map `from`.
    pub fn distances_from(&self, from: &str) -> Distances {
        let no_quests = HashMap::new();
        Distances {
            routable: self.map_changes(from, &no_quests),
            with_quests: self.map_changes(from, &self.quest_exits),
        }
    }

    /// Breadth-first from `from` over the exits, `more` exits and the ways from
    /// anywhere: each map reached, with the fewest map changes to it.
    fn map_changes(&self, from: &str, more: &Exits) -> HashMap<String, Reached> {
        let mut reached = HashMap::from([(from.to_string(), Reached { changes: 0, came_from: None })]);
        let mut queue = VecDeque::from([(from.to_string(), 0)]);
        while let Some((map, changes)) = queue.pop_front() {
            let own = self.exits.get(map.as_str()).into_iter().chain(more.get(map.as_str())).flatten();
            for &(to, step) in own.chain(&self.from_anywhere) {
                if !reached.contains_key(to) {
                    let came_from = Some((map.clone(), step));
                    reached.insert(to.to_string(), Reached { changes: changes + 1, came_from });
                    queue.push_back((to.to_string(), changes + 1));
                }
            }
        }
        reached
    }
}

/// A map a breadth-first search reached.
struct Reached {
    /// The fewest map changes to it.
    changes: u32,
    /// The map before it on a shortest route there, and how it leads on; `None` for
    /// the map the search started from.
    came_from: Option<(String, Step)>,
}

/// How far every map is from one map.
pub(crate) struct Distances {
    routable: HashMap<String, Reached>,
    with_quests: HashMap<String, Reached>,
}

impl Distances {
    pub fn to(&self, map_id: &str) -> Distance {
        match self.routable.get(map_id) {
            Some(reached) => Distance::MapChanges(reached.changes),
            None if self.with_quests.contains_key(map_id) => Distance::QuestOnly,
            None => Distance::NoRoute,
        }
    }

    /// A shortest route to map `map_id` without Quest Portals: each map's ID in order,
    /// from the map measured from, with how it is reached (`None` for the first).
    /// Empty when there is no such route.
    pub fn route_to(&self, map_id: &str) -> Vec<(String, Option<Step>)> {
        let mut route = Vec::new();
        let mut at = map_id;
        while let Some(reached) = self.routable.get(at) {
            route.push((at.to_string(), reached.came_from.as_ref().map(|&(_, step)| step)));
            match &reached.came_from {
                Some((before, _)) => at = before,
                None => break,
            }
        }
        route.reverse();
        route
    }
}
