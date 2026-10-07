//! Mob search: the search window's state (query, suggestions, highlight) and the picked
//! mob, as the overlay renders them, plus how far the world crawl behind it has got.

use crate::MEOWDB_CREDIT;
use crate::crawl::{Crawl, CrawlProgress};
use crate::map_data::Mob;
use crate::map_index::MapEntry;
use crate::world::{Distance, Distances, Here, Step, Travel, World};
use serde::Serialize;

/// How many results the panel shows; the search window shows them all.
pub const PANEL_RESULTS: usize = 5;

/// A mob the player can pick: one per MeowDB mob ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobChoice {
    /// MeowDB's ID for the mob.
    pub id: String,
    pub name: String,
    pub level: u32,
}

/// How a map's Density for a mob compares with the mob's densest map (Event maps
/// aside). Ordered densest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DensityTier {
    /// From two-thirds of the densest map's Density.
    High,
    /// From one-third of it.
    Medium,
    Low,
}

/// A map the picked mob spawns on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub map_id: String,
    pub street_name: String,
    pub map_name: String,
    /// The mob's Density on the map in hundredths: its Spawn Count times the map's Mob
    /// Rate (10 spawns at a 1.5x Mob Rate is 1500, a Density of 15).
    pub density_hundredths: u32,
    /// The mob's Spawn Share on the map, in whole percent.
    pub spawn_share_percent: u32,
    pub tier: DensityTier,
    /// How many map changes it is from the map the panel shows.
    pub distance: Distance,
    /// A shortest route there (Quest Portals aside): the map the panel shows, then each
    /// map in turn with how it is reached. Empty when the Distance isn't a number.
    pub route: Vec<RouteStop>,
    /// The kinds of travel besides portals that route takes, each once, in route order
    /// ("5 · ship"). Empty for a route by portals alone.
    pub travel: Vec<Step>,
}

/// One map on a route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteStop {
    pub map_id: String,
    /// Empty for a map that isn't in the map index.
    pub street_name: String,
    /// The map's ID for a map that isn't in the map index.
    pub map_name: String,
    /// How the player gets here from the stop before; `None` for the first stop, the
    /// map the panel shows.
    pub by: Option<Step>,
}

/// Mob search, as the overlay renders it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchState {
    /// The search window is open, and has the keyboard (ADR 0001).
    pub window_open: bool,
    /// What the player has typed in the search window.
    pub query: String,
    /// The known mobs whose name contains the query (case-insensitive), by name.
    pub suggestions: Vec<MobChoice>,
    /// Which suggestion Enter would pick; `None` when there are none.
    pub highlighted: Option<usize>,
    /// The mob the player picked, until they clear the search.
    pub picked: Option<MobChoice>,
    /// Every known map the picked mob spawns on (Event maps left out), by Density Tier
    /// and then Density: the search window's list.
    pub results: Vec<SearchResult>,
    /// The first [`PANEL_RESULTS`] of the results: the panel's list.
    pub top_results: Vec<SearchResult>,
    /// How many results there are besides the panel's.
    pub more_results: u32,
    /// The picked mob spawns only on Event maps (of the maps known so far), which
    /// search leaves out: why it has no results.
    pub only_on_event_maps: bool,
    /// The result whose route the search window shows in full; `None` when there are
    /// no results.
    pub highlighted_result: Option<usize>,
    /// The headline: which of the results is the nearest with a route, whatever its
    /// tier (of those equally near, the densest). `None` with no map shown or no route
    /// to any.
    pub nearest: Option<usize>,
    /// [`crate::MEOWDB_CREDIT`] whenever there are results: they are MeowDB's data.
    pub credit: Option<&'static str>,
    /// How many of the index's maps the world crawl knows so far ("312 of 422 maps
    /// known"): search can only find mobs on those.
    pub crawl: CrawlProgress,
    /// What Distances count besides portals and ships (the checkboxes).
    pub travel: Travel,
}

/// What mob search works over: the maps the crawl knows (`None` before the map index
/// arrives), the map the panel shows (`None`: no map, so no Distances), and the kinds
/// of travel the player has turned on.
#[derive(Clone, Copy)]
pub(crate) struct Searched<'a> {
    pub crawl: Option<&'a Crawl>,
    pub here: Option<Here<'a>>,
    pub travel: Travel,
}

/// The player's side of mob search: what they typed, highlighted and picked.
#[derive(Default)]
pub(crate) struct MobSearch {
    window_open: bool,
    query: String,
    /// The ID of the mob the player moved the highlight to; `None` for the first
    /// suggestion. Kept by ID, so the highlight stays on it as the crawl adds mobs.
    highlighted: Option<String>,
    picked: Option<MobChoice>,
    /// The ID of the map the player moved the result highlight to; `None` for the
    /// first result. Kept by ID, so the highlight stays on it as the results reorder.
    highlighted_result: Option<String>,
}

impl MobSearch {
    /// The search hotkey: open the window, with nothing typed yet.
    pub fn open(&mut self) {
        if !self.window_open {
            self.window_open = true;
            self.type_query("");
        }
    }

    /// The text box now holds `query`.
    pub fn type_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.highlighted = None;
    }

    /// Move the highlight `by` suggestions (up when negative), stopping at either end.
    /// With no suggestions listed, move it through the results instead.
    pub fn move_highlight(&mut self, by: isize, searched: &Searched) {
        if !self.window_open {
            return;
        }
        let suggestions = self.suggestions(searched.crawl);
        if !move_through(&mut self.highlighted, &suggestions, by, MobChoice::key) {
            let results = self.results(searched);
            move_through(&mut self.highlighted_result, &results, by, SearchResult::key);
        }
    }

    /// Enter: pick the highlighted mob and close the window; with nothing typed, clear
    /// the search and close it. A query matching no mob leaves the window open.
    pub fn enter(&mut self, crawl: Option<&Crawl>) {
        if !self.window_open {
            return;
        }
        if self.query.trim().is_empty() {
            self.picked = None;
            self.highlighted_result = None;
            self.window_open = false;
            return;
        }
        let suggestions = self.suggestions(crawl);
        if let Some(at) = highlight_in(&suggestions, &self.highlighted, MobChoice::key) {
            self.picked = Some(suggestions[at].clone());
            self.highlighted_result = None;
            self.window_open = false;
        }
    }

    /// Esc: close the window, keeping the current search.
    pub fn escape(&mut self) {
        self.window_open = false;
    }

    /// The search state for the panel.
    pub fn state(&self, searched: &Searched) -> SearchState {
        let Searched { crawl, travel, .. } = *searched;
        let suggestions = self.suggestions(crawl);
        let results = self.results(searched);
        let only_on_event_maps = match (&self.picked, crawl) {
            (Some(mob), Some(crawl)) if results.is_empty() => {
                crawl.known().any(|(entry, data)| entry.is_event() && data.mobs.iter().any(|m| m.id == mob.id))
            }
            _ => false,
        };
        SearchState {
            window_open: self.window_open,
            query: self.query.clone(),
            highlighted: highlight_in(&suggestions, &self.highlighted, MobChoice::key),
            picked: self.picked.clone(),
            suggestions,
            credit: (!results.is_empty()).then_some(MEOWDB_CREDIT),
            nearest: nearest(&results),
            highlighted_result: highlight_in(&results, &self.highlighted_result, SearchResult::key),
            top_results: results.iter().take(PANEL_RESULTS).cloned().collect(),
            more_results: results.len().saturating_sub(PANEL_RESULTS) as u32,
            only_on_event_maps,
            results,
            crawl: crawl.map(Crawl::progress).unwrap_or_default(),
            travel,
        }
    }

    /// The picked mob's results; none with no mob picked.
    fn results(&self, searched: &Searched) -> Vec<SearchResult> {
        match (&self.picked, searched.crawl) {
            (Some(mob), Some(crawl)) => results(&mob.id, crawl, searched.here, searched.travel),
            _ => Vec::new(),
        }
    }

    /// The known mobs whose name contains the query, whatever the case: each once, by
    /// name (then level and ID).
    fn suggestions(&self, crawl: Option<&Crawl>) -> Vec<MobChoice> {
        let query = self.query.trim().to_lowercase();
        let Some(crawl) = crawl.filter(|_| !query.is_empty()) else { return Vec::new() };
        let mut found: Vec<MobChoice> = Vec::new();
        for (_, data) in crawl.known() {
            for mob in &data.mobs {
                if mob.name.to_lowercase().contains(&query) && !found.iter().any(|m| m.id == mob.id) {
                    found.push(MobChoice { id: mob.id.clone(), name: mob.name.clone(), level: mob.level });
                }
            }
        }
        found.sort_by(|a, b| (&a.name, a.level, &a.id).cmp(&(&b.name, b.level, &b.id)));
        found
    }
}

impl MobChoice {
    /// What a highlight on it is kept by: the mob's ID.
    fn key(&self) -> &str {
        &self.id
    }
}

impl SearchResult {
    /// What a highlight on it is kept by: the map's ID.
    fn key(&self) -> &str {
        &self.map_id
    }
}

/// Where a highlight kept by key (`moved_to`) is in `items`: on the item with that key,
/// or on the first when it was never moved or its item is gone; `None` with no items.
fn highlight_in<T>(items: &[T], moved_to: &Option<String>, key: fn(&T) -> &str) -> Option<usize> {
    if items.is_empty() {
        return None;
    }
    let moved_to = moved_to.as_deref().and_then(|moved_to| items.iter().position(|item| key(item) == moved_to));
    Some(moved_to.unwrap_or(0))
}

/// Move a highlight kept by key `by` items through `items` (up when negative), stopping
/// at either end. False, leaving it, when there are no items.
fn move_through<T>(highlight: &mut Option<String>, items: &[T], by: isize, key: fn(&T) -> &str) -> bool {
    let Some(at) = highlight_in(items, highlight, key) else { return false };
    let to = at.saturating_add_signed(by).min(items.len() - 1);
    *highlight = Some(key(&items[to]).to_string());
    true
}

/// Which of the ranked `results` is the nearest with a route, whatever its tier; of
/// those equally near, the densest (then the first ranked).
fn nearest(results: &[SearchResult]) -> Option<usize> {
    results
        .iter()
        .enumerate()
        .filter_map(|(at, r)| match r.distance {
            Distance::MapChanges(changes) => Some((changes, std::cmp::Reverse(r.density_hundredths), at)),
            Distance::NoRoute | Distance::QuestOnly | Distance::Unknown => None,
        })
        // Of equals, the first ranked.
        .min()
        .map(|(_, _, at)| at)
}

/// The shortest route to map `map_id` with its maps' names, or none.
fn route(distances: &Distances, map_id: &str, crawl: &Crawl) -> Vec<RouteStop> {
    distances
        .route_to(map_id)
        .into_iter()
        .map(|(map_id, by)| {
            let (street_name, map_name) = match crawl.entry(&map_id) {
                Some(entry) => (entry.street_name.clone(), entry.map_name.clone()),
                None => (String::new(), map_id.clone()),
            };
            RouteStop { map_id, street_name, map_name, by }
        })
        .collect()
}

/// Every known, non-Event map mob `mob_id` spawns on, ranked: by Density Tier; within a
/// tier, maps with a route by Distance from the map the player is on (by portals, ships
/// and the `travel` turned on), then those with no known route, then the quest-only
/// ones; then by Density (densest first), then name.
fn results(mob_id: &str, crawl: &Crawl, here: Option<Here>, travel: Travel) -> Vec<SearchResult> {
    let distances = here.map(|here| World::of(crawl, Some(here), travel).distances_from(here.id));
    let distance = |map_id: &str| distances.as_ref().map_or(Distance::Unknown, |d| d.to(map_id));
    let found: Vec<(&MapEntry, &Mob, u32)> = crawl
        .known()
        .filter(|(entry, _)| !entry.is_event())
        .flat_map(|(entry, data)| {
            data.mobs
                .iter()
                .filter(|mob| mob.id == mob_id)
                .map(move |mob| (entry, mob, mob.spawn_count * data.mob_rate_percent))
        })
        .collect();
    let densest = found.iter().map(|&(_, _, density)| density).max().unwrap_or(0);
    let tier = |density: u32| {
        // Whole numbers: exactly two-thirds is High, exactly one-third Medium.
        let (density, densest) = (u64::from(density) * 3, u64::from(densest));
        if density >= densest * 2 {
            DensityTier::High
        } else if density >= densest {
            DensityTier::Medium
        } else {
            DensityTier::Low
        }
    };
    let mut results: Vec<SearchResult> = found
        .into_iter()
        .map(|(entry, mob, density)| SearchResult {
            map_id: entry.id.clone(),
            street_name: entry.street_name.clone(),
            map_name: entry.map_name.clone(),
            density_hundredths: density,
            spawn_share_percent: mob.spawn_share_percent,
            tier: tier(density),
            distance: distance(&entry.id),
            route: distances.as_ref().map(|d| route(d, &entry.id, crawl)).unwrap_or_default(),
            travel: Vec::new(),
        })
        .map(|mut result| {
            for step in result.route.iter().filter_map(|stop| stop.by) {
                if step != Step::Portal && !result.travel.contains(&step) {
                    result.travel.push(step);
                }
            }
            result
        })
        .collect();
    results.sort_by(|a, b| {
        (a.tier, a.distance, std::cmp::Reverse(a.density_hundredths), &a.street_name, &a.map_name, &a.map_id).cmp(&(
            b.tier,
            b.distance,
            std::cmp::Reverse(b.density_hundredths),
            &b.street_name,
            &b.map_name,
            &b.map_id,
        ))
    });
    results
}
