//! Small hand-built worlds for mob search: a made-up map index and each map's page,
//! served by a [`FakeSource`] for both the shown map and the world crawl.

use super::FakeSource;
use compass_core::{Action, Compass, MapData, MapEntry, Mob, PanelState, Portal, PortalKind, SearchResult};
use std::task::Poll;

/// The search hotkey (the default).
pub const SEARCH: &str = "ctrl+shift+f";

/// A map in a hand-built world.
pub struct WorldMap {
    pub entry: MapEntry,
    pub page: MapData,
}

/// A map on Victoria Island with this Mob Rate (hundredths) and these mobs.
pub fn map(id: &str, name: &str, mob_rate_percent: u32, mobs: Vec<Mob>) -> WorldMap {
    WorldMap {
        entry: MapEntry {
            id: id.into(),
            street_name: "Victoria Road".into(),
            map_name: name.into(),
            region: "Victoria Island".into(),
        },
        page: MapData { mobs, mob_rate_percent, ..MapData::default() },
    }
}

/// A mob with MeowDB ID `id`, `spawn_count` of it on the map.
pub fn mob(id: &str, name: &str, level: u32, spawn_count: u32) -> Mob {
    Mob { id: id.into(), name: name.into(), level, hp: 0, exp: 0, spawn_count, spawn_share_percent: 0 }
}

pub fn orange(spawn_count: u32) -> Mob {
    mob("9", "Orange Mushroom", 8, spawn_count)
}

impl WorldMap {
    /// The same map with a portal of this kind to map `to`.
    pub fn exit(mut self, kind: PortalKind, to: &str) -> Self {
        let name = format!("portal{}", self.page.portals.len());
        self.page.portals.push(Portal { name, to_map_id: to.into(), kind });
        self
    }

    /// The same map in another region of MeowDB's index ("Event", "Hidden Street", …).
    pub fn in_region(mut self, region: &str) -> Self {
        self.entry.region = region.into();
        self
    }

    /// The same map with a portal the player can see to each of the maps `to`.
    pub fn to(self, to: &[&str]) -> Self {
        to.iter().fold(self, |map, to| map.exit(PortalKind::Visible, to))
    }
}

/// A source serving a hand-built world: its index, and each map's page for the crawl.
pub fn world(maps: Vec<WorldMap>) -> FakeSource {
    let index = maps.iter().map(|m| m.entry.clone()).collect();
    let pages: Vec<(String, MapData)> = maps.into_iter().map(|m| (m.entry.id, m.page)).collect();
    FakeSource::with_index(index).with_pages(move |id| {
        Poll::Ready(Ok(pages.iter().find(|(page_id, _)| page_id == id).map(|(_, p)| p.clone()).unwrap_or_default()))
    })
}

/// A core over `maps`, crawled (every page cached, so at once).
pub fn crawled(maps: Vec<WorldMap>) -> Compass {
    let mut compass = Compass::new(world(maps));
    compass.tick(0);
    compass.tick(50);
    compass
}

/// Open the search view afresh (closing it first if it is open), type `query` and press
/// Enter: the mob's map list.
pub fn search_for(compass: &mut Compass, query: &str) -> PanelState {
    compass.act(Action::SearchEscape);
    compass.on_hotkey(SEARCH);
    compass.search_query(query);
    compass.act(Action::SearchEnter).clone()
}

/// The result the headline names as the nearest.
pub fn nearest(state: &PanelState) -> Option<&SearchResult> {
    state.search.nearest.map(|at| &state.search.results[at])
}

/// Tick every 50 ms from `from_ms` up to `to_ms`.
pub fn tick_through(compass: &mut Compass, from_ms: u64, to_ms: u64) -> PanelState {
    for at in (from_ms..to_ms).step_by(50) {
        compass.tick(at);
    }
    compass.tick(to_ms).clone()
}
