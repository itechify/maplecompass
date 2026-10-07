//! Which map the panel shows for the confirmed Minimap Title, and that map's data.
//!
//! Usually one map has the title. When several share it, the panel shows them without
//! asking if they would all show the same; otherwise it shows the first and lets the
//! player choose. A choice holds until the title changes.

use crate::map_data::{MapDataLoader, MapDataStatus};
use crate::map_index::MapDataSource;
use crate::{Candidates, CurrentMap};

#[derive(Default)]
pub(crate) struct ShownMap {
    /// The maps with the confirmed title, in index order; empty before there is one.
    maps: Vec<CurrentMap>,
    which: Which,
    loader: MapDataLoader,
}

#[derive(Clone, Copy)]
enum Which {
    /// The core knows which map it is (`maps[i]`), or it doesn't matter.
    Settled(usize),
    /// Getting the maps' data, one at a time, to see whether they show the same. The
    /// first is shown meanwhile.
    Comparing,
    /// They show different things and nothing settled which: the player may choose.
    Choosing { shown: usize, chosen: bool },
}

impl Default for Which {
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
            self.which = match (maps.len(), connected.next(), connected.next()) {
                (0 | 1, _, _) => Which::Settled(0),
                (_, Some((i, _)), None) => Which::Settled(i),
                _ => Which::Comparing,
            };
            self.maps = maps;
        }
        self.load(source);
    }

    /// The maps the shown map leads to, if the core is sure it's the player's and has
    /// its data; otherwise none.
    fn connected_maps(&self) -> Vec<String> {
        let sure = match self.which {
            Which::Settled(_) | Which::Choosing { chosen: true, .. } => true,
            Which::Choosing { chosen: false, .. } | Which::Comparing => false,
        };
        match (sure, self.data()) {
            (true, MapDataStatus::Ready(data)) => data.connected_maps,
            _ => Vec::new(),
        }
    }

    /// Show the next of the maps sharing the title, if the player may choose.
    pub fn choose_next(&mut self, source: &mut dyn MapDataSource) {
        if let Which::Choosing { shown, .. } = self.which {
            self.which = Which::Choosing { shown: (shown + 1) % self.maps.len(), chosen: true };
            self.load(source);
        }
    }

    pub fn map(&self) -> Option<&CurrentMap> {
        self.maps.get(self.shown())
    }

    /// The shown map's data. While the maps sharing its title are being compared, it
    /// is loading: it may yet turn out to be one of several.
    pub fn data(&self) -> MapDataStatus {
        match (self.map(), self.which) {
            (None, _) => MapDataStatus::NoMap,
            (Some(_), Which::Comparing) => MapDataStatus::Loading,
            (Some(map), _) => self.loader.status(&map.id).cloned().unwrap_or(MapDataStatus::Loading),
        }
    }

    pub fn candidates(&self) -> Option<Candidates> {
        match self.which {
            Which::Choosing { shown, chosen } => Some(Candidates {
                ids: self.maps.iter().map(|m| m.id.clone()).collect(),
                shown,
                chosen,
            }),
            _ => None,
        }
    }

    fn shown(&self) -> usize {
        match self.which {
            Which::Settled(i) | Which::Choosing { shown: i, .. } => i,
            Which::Comparing => 0,
        }
    }

    fn load(&mut self, source: &mut dyn MapDataSource) {
        if let Which::Comparing = self.which {
            // Asks for the shown map's data too, once per frame like any other.
            self.compare(source);
        } else if let Some(map) = self.maps.get(self.shown()) {
            self.loader.load(&map.id, source);
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
            let first = self.loader.status(&self.maps[0].id);
            let same = match (first, self.loader.status(&map.id)) {
                (Some(MapDataStatus::Ready(first)), Some(MapDataStatus::Ready(this))) => {
                    first.shows_same_as(this)
                }
                _ => false,
            };
            if !same {
                self.which = Which::Choosing { shown: 0, chosen: false };
                return;
            }
        }
        self.which = Which::Settled(0);
    }
}
