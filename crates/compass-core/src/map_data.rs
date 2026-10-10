//! One map's hunting information from MeowDB, and the panel's progress getting it.

use crate::shown_map::LevelRange;
use serde::{Deserialize, Serialize};

/// One map's data, as the map data source reads it from the map's MeowDB page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapData {
    /// `None` on a map without mobs, such as a town: MeowDB has no summary for it.
    pub summary: Option<MapSummary>,
    /// In the panel, sorted by level (then name); from a source, in any order.
    pub mobs: Vec<Mob>,
    /// The exits the player can't see, in MeowDB's order. Empty when there are none.
    pub hidden_portals: Vec<HiddenPortal>,
    /// The IDs of the maps its portals lead to. They tell apart maps sharing a title
    /// (the one the player came from is connected to the right one); not rendered.
    #[serde(skip_serializing)]
    pub connected_maps: Vec<String>,
    /// Every portal to another map (not to the map itself or MeowDB's dead end), with
    /// its kind, in MeowDB's order: the map's edges in the world's portal graph. Not
    /// rendered.
    #[serde(skip_serializing)]
    pub portals: Vec<Portal>,
    /// The map's Mob Rate in hundredths: 150 is a 1.5x respawn rate.
    #[serde(skip_serializing)]
    pub mob_rate_percent: u32,
    /// The map's nearest town (MeowDB's `returnMap`), where a nearest-town Teleport
    /// Scroll goes; a town's is itself. `None` when MeowDB gives the dead end. Not
    /// rendered.
    #[serde(skip_serializing)]
    pub nearest_town: Option<String>,
    /// In the panel, the mobs it lists: the [`LISTED_MOBS`] with the largest spawn
    /// share, sorted as `mobs` is. From a source, empty.
    pub listed_mobs: Vec<Mob>,
    /// In the panel, the mobs it doesn't list, summed up; `None` when it lists them all.
    pub more_mobs: Option<MoreMobs>,
}

/// How many mobs the panel lists; the rest are summed up in [`MoreMobs`].
pub const LISTED_MOBS: usize = 5;

/// The mobs the panel doesn't list: how many, and their levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoreMobs {
    pub count: usize,
    pub levels: LevelRange,
}

/// A portal from a map to another map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Portal {
    /// MeowDB's name for the portal on its map (e.g. `"in00"`).
    pub name: String,
    pub to_map_id: String,
    pub kind: PortalKind,
}

/// What kind of portal it is, as the Hidden Portal rule and the portal overrides tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PortalKind {
    /// The player can see it: on the minimap, or a walk-into exit.
    Visible,
    /// A Hidden Portal that works for anyone.
    Hidden,
    /// A town's portal into a building, drawn into the scenery.
    Door,
    /// A Quest Portal: it only works while the player meets a quest condition.
    Quest,
}

/// A portal to another map that is drawn neither on the minimap nor in the world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenPortal {
    /// MeowDB's name for the portal on its map (e.g. `"in00"`); with the map ID, what
    /// the portal override list is keyed by.
    pub name: String,
    pub to_map_id: String,
    pub to_map_name: String,
    pub to_street_name: String,
    /// The destination is a Hidden Street.
    pub to_hidden_street: bool,
    /// A Quest Portal: it only works while the player meets a quest condition.
    pub quest: bool,
}

/// The Map Summary: MeowDB's ceiling figures (every mob killed as soon as it spawns),
/// with the map's mob rate not applied, as MeowDB gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapSummary {
    pub mobs_per_hour: PerParty,
    /// The EXP-per-hour ceiling.
    pub exp_per_hour: PerParty,
}

/// A figure for a player alone on the map, and for a party of 6 or more. MeowDB gives
/// only these two; sizes in between are not derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerParty {
    pub solo: u64,
    pub full_party: u64,
}

/// A mob that spawns on the map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mob {
    /// MeowDB's ID for the mob (e.g. `"3"`, Blue Snail): the same mob on every map.
    pub id: String,
    pub name: String,
    pub level: u32,
    pub hp: u64,
    pub exp: u64,
    /// How many of this mob the map holds at once.
    pub spawn_count: u32,
    /// Its spawn count as a share of all the map's spawns, in whole percent.
    pub spawn_share_percent: u32,
}

/// The shown map's MeowDB data, or why there is none (yet).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum MapDataStatus {
    /// No map is shown.
    #[default]
    NoMap,
    /// The map's page is being fetched.
    Loading,
    Ready(MapData),
    /// MeowDB couldn't be reached, but a copy of the map's page saved at
    /// `saved_at_unix_ms` (ms since the Unix epoch) could: its figures, to be shown as
    /// such.
    SavedCopy {
        #[serde(rename = "savedAtUnixMs")]
        saved_at_unix_ms: u64,
        #[serde(flatten)]
        data: MapData,
    },
    /// MeowDB couldn't be reached, and there is no saved copy.
    Unavailable,
    /// MeowDB's page no longer looks the way the reader expects. Nothing from it is
    /// shown, rather than partial or wrong numbers.
    FormatChanged,
}

impl MapDataStatus {
    /// The map's data, fresh or from a saved copy.
    pub fn data(&self) -> Option<&MapData> {
        match self {
            Self::Ready(data) | Self::SavedCopy { data, .. } => Some(data),
            _ => None,
        }
    }

    /// The map's solo EXP-per-hour ceiling, from its Map Summary; `None` without data
    /// or a Map Summary (a town).
    pub fn solo_exp_ceiling(&self) -> Option<u64> {
        self.data()?.summary.map(|s| s.exp_per_hour.solo)
    }
}

impl MapData {
    /// Whether the panel would show the same for both maps: the same Map Summary, mobs
    /// and Hidden Portals. (Mobs sorted as the panel sorts them.)
    pub(crate) fn shows_same_as(&self, other: &MapData) -> bool {
        self.summary == other.summary
            && self.mobs == other.mobs
            && self.hidden_portals == other.hidden_portals
    }
}

/// A source's data as the panel shows it: mobs sorted by level (then name), and the
/// ones it lists picked out.
pub(crate) fn for_panel(mut data: MapData) -> MapData {
    data.mobs.sort_by(|a, b| a.level.cmp(&b.level).then_with(|| a.name.cmp(&b.name)));
    // The most common mobs, ties going to the lower level, as they come in `mobs`.
    let mut by_share: Vec<usize> = (0..data.mobs.len()).collect();
    by_share.sort_by_key(|&i| std::cmp::Reverse(data.mobs[i].spawn_share_percent));
    let listed = &by_share[..by_share.len().min(LISTED_MOBS)];
    let (shown, rest): (Vec<_>, Vec<_>) =
        data.mobs.iter().enumerate().partition(|(i, _)| listed.contains(i));
    // Both in level order, so the rest's first and last are its lowest and highest.
    data.more_mobs = match (rest.first(), rest.last()) {
        (Some((_, lowest)), Some((_, highest))) => Some(MoreMobs {
            count: rest.len(),
            levels: LevelRange { lowest: lowest.level, highest: highest.level },
        }),
        _ => None,
    };
    data.listed_mobs = shown.into_iter().map(|(_, mob)| mob.clone()).collect();
    data
}
