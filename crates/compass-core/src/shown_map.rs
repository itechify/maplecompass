//! Which map the panel shows for the confirmed Minimap Title, and that map's data.
//!
//! Usually one map has the title. When several share it, the panel shows them without
//! asking if they would all show the same; otherwise it shows the first, outlines each
//! of them so the player can tell them apart, and lets the player choose. A choice
//! holds until the title changes.

use crate::CurrentMap;
use crate::map_data::{MapData, MapDataLoader, MapDataStatus};
use crate::map_index::MapDataSource;
use serde::Serialize;

/// Maps that share one Minimap Title and would show different content, with neither
/// the map the player came from nor the player having settled which it is. The panel
/// shows one of them; the choose-candidate hotkey shows the next.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidates {
    /// In map index order.
    pub maps: Vec<Candidate>,
    /// Which of them the panel shows (an index into `maps`).
    pub shown: usize,
    /// The player chose the shown one with the hotkey. The choice holds until the
    /// map changes.
    pub chosen: bool,
}

/// One of the maps sharing the title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// MeowDB's 9-digit map ID.
    pub id: String,
    /// What sets it apart from the others; `None` until its MeowDB page is in (or if
    /// it can't be had).
    pub outline: Option<Outline>,
}

/// A map at a glance: enough to tell apart maps that share a title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outline {
    /// The lowest and highest mob level; `None` on a map without mobs.
    pub mob_levels: Option<LevelRange>,
    /// The names of its lowest-level mobs, up to [`FIRST_MOBS`], lowest first.
    pub first_mobs: Vec<String>,
    /// How many Hidden Portals it has.
    pub hidden_portals: usize,
}

/// A range of mob levels, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelRange {
    pub lowest: u32,
    pub highest: u32,
}

/// How many mobs an [`Outline`] names.
pub const FIRST_MOBS: usize = 3;

impl Outline {
    fn of(data: &MapData) -> Self {
        // The panel's mobs are sorted by level.
        let mob_levels = match (data.mobs.first(), data.mobs.last()) {
            (Some(lowest), Some(highest)) => Some(LevelRange { lowest: lowest.level, highest: highest.level }),
            _ => None,
        };
        Self {
            mob_levels,
            first_mobs: data.mobs.iter().take(FIRST_MOBS).map(|m| m.name.clone()).collect(),
            hidden_portals: data.hidden_portals.len(),
        }
    }
}

#[derive(Default)]
pub(crate) struct ShownMap {
    /// The maps with the confirmed title, in index order; empty before there is one.
    maps: Vec<CurrentMap>,
    settling: Settling,
    loader: MapDataLoader,
}

/// How far the core got in settling which of the maps sharing the title to show.
#[derive(Clone, Copy)]
enum Settling {
    /// The core knows which map it is (`maps[i]`), or it doesn't matter.
    Settled(usize),
    /// Getting the maps' data, one at a time, to see whether they show the same. The
    /// first is shown meanwhile.
    Comparing,
    /// They show different things and nothing settled which: the player may choose.
    Choosing { shown: usize, chosen: bool },
}

impl Default for Settling {
    fn default() -> Self {
        Self::Settled(0)
    }
}

impl ShownMap {
    /// The title confirmed on the latest frame belongs to `maps` (none: no title yet).
    /// Asks the source for whatever data is needed and not yet had.
    pub fn show(&mut self, maps: Vec<CurrentMap>, source: &mut dyn MapDataSource) {
        if maps != self.maps {
            let came_from = self.connected_maps();
            self.loader.forget();
            // The map the player came from settles it when it leads to just one of
            // them, with no need to compare them.
            let mut connected = maps.iter().enumerate().filter(|(_, m)| came_from.contains(&m.id));
            self.settling = match (maps.len(), connected.next(), connected.next()) {
                (0 | 1, _, _) => Settling::Settled(0),
                (_, Some((i, _)), None) => Settling::Settled(i),
                _ => Settling::Comparing,
            };
            self.maps = maps;
        }
        self.poll(source);
    }

    /// The maps the shown map leads to, if the core is sure it's the player's and has
    /// its data; otherwise none.
    fn connected_maps(&self) -> Vec<String> {
        let sure = match self.settling {
            Settling::Settled(_) | Settling::Choosing { chosen: true, .. } => true,
            Settling::Choosing { chosen: false, .. } | Settling::Comparing => false,
        };
        match (sure, self.data().data()) {
            (true, Some(data)) => data.connected_maps.clone(),
            _ => Vec::new(),
        }
    }

    /// Show the next of the maps sharing the title, if the player may choose.
    pub fn choose_next(&mut self, source: &mut dyn MapDataSource) {
        if let Settling::Choosing { shown, .. } = self.settling {
            self.settling = Settling::Choosing { shown: (shown + 1) % self.maps.len(), chosen: true };
            self.poll(source);
        }
    }

    pub fn map(&self) -> Option<&CurrentMap> {
        self.maps.get(self.shown())
    }

    /// The shown map's data. While the maps sharing its title are being compared, it
    /// is loading: it may yet turn out to be one of several.
    pub fn data(&self) -> MapDataStatus {
        match (self.map(), self.settling) {
            (None, _) => MapDataStatus::NoMap,
            (Some(_), Settling::Comparing) => MapDataStatus::Loading,
            (Some(map), _) => self.loader.status(&map.id).cloned().unwrap_or(MapDataStatus::Loading),
        }
    }

    pub fn candidates(&self) -> Option<Candidates> {
        let Settling::Choosing { shown, chosen } = self.settling else { return None };
        let maps = self
            .maps
            .iter()
            .map(|m| Candidate {
                id: m.id.clone(),
                outline: self.loader.status(&m.id).and_then(MapDataStatus::data).map(Outline::of),
            })
            .collect();
        Some(Candidates { maps, shown, chosen })
    }

    fn shown(&self) -> usize {
        match self.settling {
            Settling::Settled(i) | Settling::Choosing { shown: i, .. } => i,
            Settling::Comparing => 0,
        }
    }

    /// Check on the data asked for, and ask for whatever is needed next.
    pub fn poll(&mut self, source: &mut dyn MapDataSource) {
        match self.settling {
            Settling::Settled(i) => {
                if let Some(map) = self.maps.get(i) {
                    self.loader.load(&map.id, source);
                }
            }
            Settling::Comparing => self.compare(source),
            Settling::Choosing { shown, .. } => {
                // The shown map first, then the rest for their outlines (the shell
                // fetches one page at a time).
                let order = std::iter::once(shown).chain(0..self.maps.len());
                for i in order {
                    if *self.loader.load(&self.maps[i].id, source) == MapDataStatus::Loading {
                        return;
                    }
                }
            }
        }
    }

    /// Ask for the maps' data in turn, one answer before the next question (the shell
    /// fetches one page at a time), and stop at the first map that shows something
    /// different from the first: then the player must choose.
    fn compare(&mut self, source: &mut dyn MapDataSource) {
        for map in &self.maps {
            if *self.loader.load(&map.id, source) == MapDataStatus::Loading {
                return;
            }
            let data = |id: &str| self.loader.status(id).and_then(MapDataStatus::data);
            let same = match (data(&self.maps[0].id), data(&map.id)) {
                (Some(first), Some(this)) => first.shows_same_as(this),
                _ => false,
            };
            if !same {
                self.settling = Settling::Choosing { shown: 0, chosen: false };
                // Go on to the other maps' data, for their outlines.
                self.poll(source);
                return;
            }
        }
        self.settling = Settling::Settled(0);
    }
}
