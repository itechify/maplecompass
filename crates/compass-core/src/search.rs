//! Mob search: the search window's state (query, suggestions, highlight) and the picked
//! mob, as the overlay renders them, plus how far the world crawl behind it has got.

use crate::MEOWDB_CREDIT;
use crate::crawl::{Crawl, CrawlProgress};
use crate::map_data::Mob;
use crate::map_index::MapEntry;
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
    /// and then Density. The panel shows the first [`PANEL_RESULTS`].
    pub results: Vec<SearchResult>,
    /// [`crate::MEOWDB_CREDIT`] whenever there are results: they are MeowDB's data.
    pub credit: Option<&'static str>,
    /// How many of the index's maps the world crawl knows so far ("312 of 422 maps
    /// known"): search can only find mobs on those.
    pub crawl: CrawlProgress,
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
    pub fn move_highlight(&mut self, by: isize, crawl: Option<&Crawl>) {
        let suggestions = self.suggestions(crawl);
        if let Some(at) = self.highlight_in(&suggestions) {
            let to = at.saturating_add_signed(by).min(suggestions.len() - 1);
            self.highlighted = Some(suggestions[to].id.clone());
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
            self.window_open = false;
            return;
        }
        let suggestions = self.suggestions(crawl);
        if let Some(at) = self.highlight_in(&suggestions) {
            self.picked = Some(suggestions[at].clone());
            self.window_open = false;
        }
    }

    /// Esc: close the window, keeping the current search.
    pub fn escape(&mut self) {
        self.window_open = false;
    }

    /// The search state for the panel, over the maps the crawl knows.
    pub fn state(&self, crawl: Option<&Crawl>) -> SearchState {
        let suggestions = self.suggestions(crawl);
        let results = match (&self.picked, crawl) {
            (Some(mob), Some(crawl)) => results(&mob.id, crawl),
            _ => Vec::new(),
        };
        SearchState {
            window_open: self.window_open,
            query: self.query.clone(),
            highlighted: self.highlight_in(&suggestions),
            picked: self.picked.clone(),
            suggestions,
            credit: (!results.is_empty()).then_some(MEOWDB_CREDIT),
            results,
            crawl: crawl.map(Crawl::progress).unwrap_or_default(),
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

    fn highlight_in(&self, suggestions: &[MobChoice]) -> Option<usize> {
        if suggestions.is_empty() {
            return None;
        }
        let moved_to = self.highlighted.as_ref().and_then(|id| suggestions.iter().position(|m| &m.id == id));
        Some(moved_to.unwrap_or(0))
    }
}

/// MeowDB's display region for event maps, which the player can't normally enter.
const EVENT_REGION: &str = "Event";

/// Every known, non-Event map mob `mob_id` spawns on, ranked: by Density Tier, then
/// Density (densest first), then name.
fn results(mob_id: &str, crawl: &Crawl) -> Vec<SearchResult> {
    let found: Vec<(&MapEntry, &Mob, u32)> = crawl
        .known()
        .filter(|(entry, _)| entry.region != EVENT_REGION)
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
        })
        .collect();
    results.sort_by(|a, b| {
        (a.tier, std::cmp::Reverse(a.density_hundredths), &a.street_name, &a.map_name, &a.map_id).cmp(&(
            b.tier,
            std::cmp::Reverse(b.density_hundredths),
            &b.street_name,
            &b.map_name,
            &b.map_id,
        ))
    });
    results
}
