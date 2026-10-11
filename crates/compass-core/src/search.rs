//! Mob search: the search view (query, suggestions, the picked mob's map list and their
//! highlights) and the Hunting Target it sets, as the overlay renders them, over every map
//! in OSMS's file set.

use crate::OSMS_ONLY_CREDIT;
use crate::map_data::Mob;
use crate::map_index::MapEntry;
use crate::osms::MapSet;
use crate::world::{Distance, Distances, Step, Travel, World};
use serde::Serialize;

/// Which step of the search view is open. It takes over the whole panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchView {
    /// The query box and the mobs whose name contains it.
    Mobs,
    /// The picked mob's ranked map list, to pick the Hunting Target's map from.
    Maps,
}

/// The mob and map the player picked to go and hunt, with the map's figures and the way
/// there from the map the panel shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntingTarget {
    pub mob: MobChoice,
    /// The map, as the mob's map list ranks it now: Density Tier, Density, Spawn Share,
    /// and Distance and route from the map the panel shows.
    pub map: SearchResult,
    /// The player has come to the map from another: they are there, and the target
    /// stays to say so until they leave it.
    pub arrived: bool,
    /// The way there as it stood when the player picked it; `None` when they picked the
    /// map they were on, or no route led there.
    pub trip: Option<Trip>,
    /// Where the overlay loads OSMS's default animation of the mob from, as its Mob Card
    /// does; `None` until it's in, if it can't be had, or if OSMS names none.
    pub image: Option<String>,
}

impl HuntingTarget {
    /// The next map on the route, when the route goes there from the map the panel shows
    /// through a portal; `None` once the player has arrived, or when there is no route.
    pub(crate) fn next_map_by_portal(&self) -> Option<&str> {
        match self.map.route.get(1) {
            Some(RouteStop { map_id, by: Some(Step::Portal), .. }) if !self.arrived => Some(map_id),
            _ => None,
        }
    }
}

/// The way to a Hunting Target as it stood when the player picked it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trip {
    /// The name of the map the player picked it on.
    pub from: String,
    /// How many map changes the route from there took.
    pub map_changes: u32,
    /// The kinds of travel besides portals it took, each once, in route order.
    pub travel: Vec<Step>,
}

/// A mob the player can pick: one per mob ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobChoice {
    /// The mob's ID, the game's (OSMS's and MeowDB's alike).
    pub id: String,
    pub name: String,
    pub level: u32,
}

/// How a map's Density for a mob compares with the mob's densest map (Event and Dev maps
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
    /// The map's Map Mark, by OSMS's name for it (see [`crate::PanelState::map_marks`]);
    /// `None` for a map without one.
    pub mark: Option<String>,
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
    /// The map's Map Mark, by OSMS's name for it (see [`crate::PanelState::map_marks`]);
    /// `None` for a map without one, or not in the map index.
    pub mark: Option<String>,
    /// How the player gets here from the stop before; `None` for the first stop, the
    /// map the panel shows.
    pub by: Option<Step>,
}

/// Mob search, as the overlay renders it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchState {
    /// The search view's open step; `None` for the normal view. While it is open the
    /// overlay has the keyboard (ADR 0001).
    pub view: Option<SearchView>,
    /// What the player has typed in the query box.
    pub query: String,
    /// The known mobs whose name contains the query (case-insensitive), by name.
    pub suggestions: Vec<MobChoice>,
    /// Which suggestion Enter would pick; `None` when there are none.
    pub highlighted: Option<usize>,
    /// The mob whose map list the search view shows; `None` outside that step.
    pub picked: Option<MobChoice>,
    /// Every map the picked mob spawns on (Event and Dev maps left out), by Density Tier
    /// and then Density: the search view's map list.
    pub results: Vec<SearchResult>,
    /// The picked mob spawns only on maps search leaves out (Event maps; the Dev map has
    /// no mobs): why it has no results.
    pub only_on_event_maps: bool,
    /// The map Enter would pick from the map list; `None` when there are no results.
    pub highlighted_result: Option<usize>,
    /// The headline: which of the results is the nearest with a route, whatever its
    /// tier (of those equally near, the densest). `None` with no map shown or no route
    /// to any.
    pub nearest: Option<usize>,
    /// [`crate::OSMS_ONLY_CREDIT`] whenever there are results or a Hunting Target.
    pub credit: Option<&'static str>,
    /// What Distances count besides portals and ships (the checkboxes).
    pub travel: Travel,
    /// The Hunting Target the panel's Navigation section shows, until the player leaves its
    /// map having arrived, or clears it. Kept for this session only.
    pub hunting: Option<HuntingTarget>,
}

/// What mob search works over: every map in OSMS's file set (`None` before it arrives),
/// the ID of the map the panel shows (`None`: no map, so no Distances), and the kinds of
/// travel the player has turned on.
#[derive(Clone, Copy)]
pub(crate) struct Searched<'a> {
    pub set: Option<&'a MapSet>,
    pub here: Option<&'a str>,
    pub travel: Travel,
}

/// The player's side of mob search: what they typed, highlighted and picked, and the
/// Hunting Target.
#[derive(Default)]
pub(crate) struct MobSearch {
    view: Option<SearchView>,
    query: String,
    /// The ID of the mob the player moved the highlight to; `None` for the first
    /// suggestion. Kept by ID, so the highlight stays on it as the suggestions change.
    highlighted: Option<String>,
    /// The mob whose map list the search view shows.
    picked: Option<MobChoice>,
    /// The ID of the map the player moved the map list's highlight to; `None` for the
    /// first map. Kept by ID, so the highlight stays on it as the list reorders.
    highlighted_result: Option<String>,
    target: Option<Target>,
}

/// The Hunting Target: its mob, the ID of its map, the trip there as it stood when the
/// player picked it, and whether they have arrived.
#[derive(Clone)]
struct Target {
    mob: MobChoice,
    map_id: String,
    trip: Option<Trip>,
    arrived: bool,
}

impl MobSearch {
    /// Open the search view on its first step, with nothing typed yet. Already open, it
    /// stays as it is.
    pub fn open(&mut self) {
        if self.view.is_none() {
            self.view = Some(SearchView::Mobs);
            self.type_query("");
        }
    }

    /// Open the search view on the Hunting Target mob's map list, highlighting its map.
    /// Without a target, nothing happens.
    pub fn reopen(&mut self) {
        if let Some(Target { mob, map_id, .. }) = self.target.clone() {
            self.type_query("");
            self.view = Some(SearchView::Maps);
            self.picked = Some(mob);
            self.highlighted_result = Some(map_id);
        }
    }

    pub fn is_open(&self) -> bool {
        self.view.is_some()
    }

    /// The text box now holds `query`.
    pub fn type_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.highlighted = None;
    }

    /// Move the open step's highlight `by` items (up when negative), stopping at either
    /// end: through the suggestions, or through the map list.
    pub fn move_highlight(&mut self, by: isize, searched: &Searched) {
        match self.view {
            Some(SearchView::Mobs) => {
                let suggestions = self.suggestions(searched.set);
                move_through(&mut self.highlighted, &suggestions, by, MobChoice::key);
            }
            Some(SearchView::Maps) => {
                let results = self.results(searched);
                move_through(&mut self.highlighted_result, &results, by, SearchResult::key);
            }
            None => {}
        }
    }

    /// Enter: pick the highlighted mob and list its maps, or pick the highlighted map and
    /// make it the Hunting Target. Nothing to pick (nothing typed, or a query matching no
    /// mob) leaves the view as it is.
    pub fn enter(&mut self, searched: &Searched) {
        let id = match self.view {
            Some(SearchView::Mobs) => {
                let suggestions = self.suggestions(searched.set);
                highlight_in(&suggestions, &self.highlighted, MobChoice::key).map(|at| suggestions[at].id.clone())
            }
            Some(SearchView::Maps) => {
                let results = self.results(searched);
                highlight_in(&results, &self.highlighted_result, SearchResult::key).map(|at| results[at].map_id.clone())
            }
            None => None,
        };
        if let Some(id) = id {
            self.pick(&id, searched);
        }
    }

    /// The player picked the mob or map with this ID from the open step's list.
    pub fn pick(&mut self, id: &str, searched: &Searched) {
        match self.view {
            Some(SearchView::Mobs) => {
                if let Some(mob) = self.suggestions(searched.set).into_iter().find(|mob| mob.id == id) {
                    self.picked = Some(mob);
                    self.highlighted_result = None;
                    self.view = Some(SearchView::Maps);
                }
            }
            Some(SearchView::Maps) => {
                let listed = self.results(searched).into_iter().find(|result| result.map_id == id);
                if let (Some(mob), Some(result)) = (&self.picked, listed) {
                    let trip = match (result.distance, result.route.first()) {
                        (Distance::MapChanges(n @ 1..), Some(from)) => {
                            Some(Trip { from: from.map_name.clone(), map_changes: n, travel: result.travel })
                        }
                        _ => None,
                    };
                    self.target = Some(Target { mob: mob.clone(), map_id: id.to_string(), trip, arrived: false });
                    self.close();
                }
            }
            None => {}
        }
    }

    /// Esc or the back arrow: back to the normal view, with nothing changed.
    pub fn escape(&mut self) {
        self.close();
    }

    fn close(&mut self) {
        self.view = None;
        self.type_query("");
        self.picked = None;
        self.highlighted_result = None;
    }

    /// The Navigation section's ×: no Hunting Target any more.
    pub fn clear(&mut self) {
        self.target = None;
    }

    /// The panel now shows map `map_id`: if that is the Hunting Target's map, the player
    /// has arrived there; if they had arrived, they have left it, and the target is done
    /// with.
    pub fn arrived(&mut self, map_id: &str) {
        let Some(target) = &mut self.target else { return };
        if target.map_id == map_id {
            target.arrived = true;
        } else if target.arrived {
            self.clear();
        }
    }

    /// The search state for the panel.
    pub fn state(&self, searched: &Searched) -> SearchState {
        let Searched { set, travel, .. } = *searched;
        let hunting = self.hunting(searched);
        let suggestions = self.suggestions(set);
        let results = self.results(searched);
        let only_on_event_maps = match (&self.picked, set) {
            (Some(mob), Some(set)) if results.is_empty() => {
                set.maps().any(|(entry, data)| entry.is_left_out() && data.mobs.iter().any(|m| m.id == mob.id))
            }
            _ => false,
        };
        SearchState {
            view: self.view,
            query: self.query.clone(),
            highlighted: highlight_in(&suggestions, &self.highlighted, MobChoice::key),
            picked: self.picked.clone(),
            suggestions,
            credit: (!results.is_empty() || hunting.is_some()).then_some(OSMS_ONLY_CREDIT),
            nearest: nearest(&results),
            highlighted_result: highlight_in(&results, &self.highlighted_result, SearchResult::key),
            only_on_event_maps,
            results,
            travel,
            hunting,
        }
    }

    /// The Hunting Target with its map's figures, ranked among the mob's maps, and its
    /// route from the map the panel shows. Without its mob's animation, which is a picture
    /// ([`HuntingTarget::image`]).
    fn hunting(&self, searched: &Searched) -> Option<HuntingTarget> {
        let (target, set) = (self.target.as_ref()?, searched.set?);
        let map = results(&target.mob.id, set, searched.here, searched.travel)
            .into_iter()
            .find(|r| r.map_id == target.map_id)?;
        Some(HuntingTarget {
            mob: target.mob.clone(),
            map,
            arrived: target.arrived,
            trip: target.trip.clone(),
            image: None,
        })
    }

    /// The picked mob's map list; none outside that step.
    fn results(&self, searched: &Searched) -> Vec<SearchResult> {
        match (&self.picked, searched.set) {
            (Some(mob), Some(set)) => results(&mob.id, set, searched.here, searched.travel),
            _ => Vec::new(),
        }
    }

    /// The mobs whose name contains the query, whatever the case: each once, by name
    /// (then level and ID). None outside the view's first step.
    fn suggestions(&self, set: Option<&MapSet>) -> Vec<MobChoice> {
        let query = self.query.trim().to_lowercase();
        let open = self.view == Some(SearchView::Mobs);
        let Some(set) = set.filter(|_| open && !query.is_empty()) else { return Vec::new() };
        let mut found: Vec<MobChoice> = Vec::new();
        for (_, data) in set.maps() {
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
fn route(distances: &Distances, map_id: &str, set: &MapSet) -> Vec<RouteStop> {
    distances
        .route_to(map_id)
        .into_iter()
        .map(|(map_id, by)| {
            let (street_name, map_name, mark) = match set.entry(&map_id) {
                Some(entry) => (entry.street_name.clone(), entry.map_name.clone(), entry.mark.clone()),
                None => (String::new(), map_id.clone(), None),
            };
            RouteStop { map_id, street_name, map_name, mark, by }
        })
        .collect()
}

/// Every map mob `mob_id` spawns on, Event and Dev maps aside, ranked: by Density Tier;
/// within a tier, maps with a route by Distance from the map the player is on (`here`; by
/// portals, ships and the `travel` turned on), then those with no known route, then the
/// quest-only ones; then by Density (densest first), then name.
fn results(mob_id: &str, set: &MapSet, here: Option<&str>, travel: Travel) -> Vec<SearchResult> {
    let distances = here.map(|here| World::of(set, travel).distances_from(here));
    let distance = |map_id: &str| distances.as_ref().map_or(Distance::Unknown, |d| d.to(map_id));
    let found: Vec<(&MapEntry, &Mob, u32)> = set
        .maps()
        .filter(|(entry, _)| !entry.is_left_out())
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
            mark: entry.mark.clone(),
            density_hundredths: density,
            spawn_share_percent: mob.spawn_share_percent,
            tier: tier(density),
            distance: distance(&entry.id),
            route: distances.as_ref().map(|d| route(d, &entry.id, set)).unwrap_or_default(),
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
