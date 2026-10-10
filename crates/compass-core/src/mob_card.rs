//! Mob Cards: what the panel shows when the player hovers a listed mob's name. Each
//! holds OSMS's animation of the mob and the figures from OSMS's set that the panel's row
//! doesn't show (ADR 0008). The figures come with the set, so a card is ready as soon as
//! its map is shown; the animations are pictures, asked for once the map lists its mobs.

use crate::map_data::Mob;
use crate::meowdb;
use crate::osms::MapSet;
use crate::pictures::Pictures;
use serde::Serialize;

/// The figures from OSMS's set that the panel's row (level, HP, EXP, share) doesn't show.
/// A figure the mob doesn't have, or has as 0, is `None` (the panel shows "—").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobDetails {
    pub elements: Elements,
    /// The damage it deals when the player walks into it.
    pub touch_damage: Option<DamageRange>,
    pub mp: Option<u64>,
    pub accuracy: Option<u32>,
    pub avoid: Option<u32>,
    pub physical_defense: Option<u32>,
    pub magic_defense: Option<u32>,
    /// How fast it moves: "Slow", "Normal" or "Fast".
    pub speed: String,
    pub hp_per_exp: Option<HpPerExp>,
}

/// The elements a mob is weak to, strong against and immune to ("Fire", "Poison"). All
/// empty: no elemental affinity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Elements {
    pub weak: Vec<String>,
    pub strong: Vec<String>,
    pub immune: Vec<String>,
}

/// A range of damage, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageRange {
    pub min: u64,
    pub max: u64,
}

/// HP per EXP, lower is better: how much HP the player chews through per point of EXP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HpPerExp {
    /// In hundredths: 1275 is 12.75.
    pub hundredths: u64,
    /// Its place among the set's mobs, 1 being the lowest HP per EXP.
    pub rank: u32,
    /// How many mobs the set has.
    pub ranked: u32,
}

/// One listed mob's Mob Card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobCard {
    /// The mob's ID, as in the panel's [`Mob`].
    pub id: String,
    /// The mob's MeowDB page, which a click on its name opens.
    pub url: String,
    /// Where the overlay loads OSMS's default animation of the mob from, through
    /// MapleCompass's own URL scheme; `None` until it's in, if it can't be had, or if OSMS
    /// names none.
    pub image: Option<String>,
    #[serde(flatten)]
    pub details: MobDetails,
}

/// The pictures the `listed` mobs' cards want: OSMS's animations of them, in the panel's
/// order.
pub(crate) fn animations(listed: &[Mob], set: &MapSet) -> Vec<String> {
    listed.iter().filter_map(|mob| set.animation(&mob.id)).map(String::from).collect()
}

/// The `listed` mobs' cards, in the panel's order: their figures from `set`, and their
/// animations once `pictures` has them.
pub(crate) fn cards(listed: &[Mob], set: &MapSet, pictures: &Pictures) -> Vec<MobCard> {
    listed
        .iter()
        .filter_map(|mob| {
            Some(MobCard {
                id: mob.id.clone(),
                url: meowdb::mob_url(&mob.id),
                image: set.animation(&mob.id).and_then(|path| pictures.url(path)).map(String::from),
                details: set.mob(&mob.id)?.clone(),
            })
        })
        .collect()
}
