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
            self.loader.forget();
            self.which = if maps.len() > 1 { Which::Comparing } else { Which::Settled(0) };
            self.maps = maps;
        }
        self.load(source);
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

    pub fn data(&self) -> MapDataStatus {
        match self.map() {
            None => MapDataStatus::NoMap,
            Some(map) => self.loader.status(&map.id).cloned().unwrap_or(MapDataStatus::Loading),
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
            self.compare(source);
        }
        if let Some(map) = self.maps.get(self.shown()) {
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
