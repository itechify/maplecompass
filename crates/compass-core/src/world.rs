//! The world graph: which maps lead to which, from the crawled pages, and how many map
//! changes each map is from another.
//!
//! It is directed, one node per known map. Every exit to another map is an edge
//! (visible, Hidden Portal or Door, one-way or not), except Quest Portals, which are
//! kept apart: a map reachable only through one is "quest only". Every edge is one map
//! change, so Distance is the length of the shortest path, found breadth-first.

use crate::crawl::Crawl;
use crate::map_data::PortalKind;
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

/// The maps each known map leads to, in one map change.
pub(crate) struct World<'a> {
    /// Through any exit but a Quest Portal.
    exits: HashMap<&'a str, Vec<&'a str>>,
    /// Through Quest Portals.
    quest_exits: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> World<'a> {
    /// The graph of every map the crawl knows.
    pub fn of(crawl: &'a Crawl) -> Self {
        let mut world = Self { exits: HashMap::new(), quest_exits: HashMap::new() };
        for (entry, data) in crawl.known() {
            for portal in &data.portals {
                let exits = match portal.kind {
                    PortalKind::Quest => &mut world.quest_exits,
                    PortalKind::Visible | PortalKind::Hidden | PortalKind::Door => &mut world.exits,
                };
                exits.entry(entry.id.as_str()).or_default().push(portal.to_map_id.as_str());
            }
        }
        world
    }

    /// How far every map is from map `from`.
    pub fn distances_from(&self, from: &str) -> Distances {
        let no_quests = HashMap::new();
        Distances {
            routable: map_changes(from, &self.exits, &no_quests),
            with_quests: map_changes(from, &self.exits, &self.quest_exits),
        }
    }
}

/// Breadth-first from `from` over both sets of exits: the fewest map changes to each
/// map reached.
fn map_changes(from: &str, exits: &HashMap<&str, Vec<&str>>, more: &HashMap<&str, Vec<&str>>) -> HashMap<String, u32> {
    let mut reached = HashMap::from([(from.to_string(), 0)]);
    let mut queue = VecDeque::from([(from.to_string(), 0)]);
    while let Some((map, changes)) = queue.pop_front() {
        let next = exits.get(map.as_str()).into_iter().chain(more.get(map.as_str())).flatten();
        for &to in next {
            if !reached.contains_key(to) {
                reached.insert(to.to_string(), changes + 1);
                queue.push_back((to.to_string(), changes + 1));
            }
        }
    }
    reached
}

/// How far every map is from one map.
pub(crate) struct Distances {
    routable: HashMap<String, u32>,
    with_quests: HashMap<String, u32>,
}

impl Distances {
    pub fn to(&self, map_id: &str) -> Distance {
        match self.routable.get(map_id) {
            Some(&changes) => Distance::MapChanges(changes),
            None if self.with_quests.contains_key(map_id) => Distance::QuestOnly,
            None => Distance::NoRoute,
        }
    }
}
