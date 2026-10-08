//! Mob Cards: what the panel shows when the player hovers a listed mob's name. Each
//! holds the mob's image (from maplestory.io) and the figures from its MeowDB page that
//! the panel's row doesn't show.
//!
//! Once the shown map's page is in, its listed mobs' pages and images are asked for one
//! at a time (ADR 0006). Mob pages go ahead of the world crawl and keep to MeowDB's pace
//! with it; images keep to maplestory.io's own. An answer is kept for the run, so a mob
//! met again on another map isn't asked for again.

use crate::map_data::Mob;
use crate::map_index::{Fetched, MapDataSource, SourceError};
use crate::meowdb;
use crate::pace::Pace;
use serde::Serialize;
use std::task::Poll;

/// The figures from a mob's MeowDB page that the panel's row (level, HP, EXP, share)
/// doesn't show. A figure MeowDB shows as "—" is `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobDetails {
    pub elements: Elements,
    /// The damage it deals when the player walks into it.
    pub touch_damage: Option<DamageRange>,
    /// `None` when MeowDB lists no MP for it.
    pub mp: Option<u64>,
    pub accuracy: Option<u32>,
    pub avoid: Option<u32>,
    pub physical_defense: Option<u32>,
    pub magic_defense: Option<u32>,
    /// MeowDB's word for how fast it moves: "Slow", "Normal", "Fast".
    pub speed: String,
    /// MeowDB's words for how it gets about: "Stays on its floor", "Flies".
    pub movement: String,
    pub hp_per_exp: Option<HpPerExp>,
}

/// The elements a mob is weak to, strong against and immune to, as MeowDB names them
/// ("Fire", "Poison"). All empty: no elemental affinity.
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
    /// Its place among the mobs MeowDB ranks, 1 being the lowest HP per EXP.
    pub rank: u32,
    /// How many mobs MeowDB ranks.
    pub ranked: u32,
}

/// One listed mob's Mob Card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobCard {
    /// MeowDB's ID for the mob, as in the panel's [`Mob`].
    pub id: String,
    /// The mob's MeowDB page, which a click on its name opens.
    pub url: String,
    /// Where the overlay loads the mob's image from (the shell's copy of maplestory.io's);
    /// `None` until it's in, or if it can't be had.
    pub image: Option<String>,
    #[serde(flatten)]
    pub page: MobPage,
}

/// The figures from a mob's MeowDB page, or why there are none (yet).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum MobPage {
    /// The page is being fetched.
    Loading,
    Ready(MobDetails),
    /// MeowDB couldn't be reached, but a copy of the page saved at `saved_at_unix_ms`
    /// (ms since the Unix epoch) could: its figures, to be shown as such.
    SavedCopy {
        #[serde(rename = "savedAtUnixMs")]
        saved_at_unix_ms: u64,
        #[serde(flatten)]
        details: MobDetails,
    },
    /// MeowDB couldn't be reached, and there is no saved copy.
    Unavailable,
    /// MeowDB's page no longer looks the way the reader expects: nothing from it is
    /// shown, rather than a card with figures missing.
    FormatChanged,
}

/// How long an unreachable page or image waits before it is asked for again, while its
/// mob is still listed.
const RETRY_MS: u64 = 5 * 60_000;

/// How many answers given at once (from the source's cache) are taken in one tick.
const AT_ONCE_PER_TICK: usize = 8;

/// The listed mobs' pages and images, asked for and kept.
#[derive(Default)]
pub(crate) struct MobCards {
    /// The mobs the panel lists now, in its order.
    listed: Vec<Mob>,
    pages: Lane<Fetched<MobDetails>>,
    images: Lane<String>,
    /// maplestory.io's pace; MeowDB's is shared with the crawl.
    image_pace: Pace,
}

impl MobCards {
    /// The panel lists these mobs now: their pages and images are wanted.
    pub fn list(&mut self, mobs: &[Mob]) {
        self.listed = mobs.to_vec();
    }

    /// Time passed: check on the requests in flight, and ask for the next page and image
    /// when their site's pace allows.
    pub fn poll(&mut self, source: &mut dyn MapDataSource, meowdb: &mut Pace, at_ms: u64) {
        let wanted: Vec<&str> = self.listed.iter().map(|m| m.id.as_str()).collect();
        self.pages.poll(&wanted, meowdb, at_ms, |id| source.mob_page(id));
        self.images.poll(&wanted, &mut self.image_pace, at_ms, |id| source.mob_image(id));
    }

    /// Whether a listed mob's page is still to be asked for or on its way: the world
    /// crawl waits until it isn't.
    pub fn wants_meowdb(&self, at_ms: u64) -> bool {
        let wanted: Vec<&str> = self.listed.iter().map(|m| m.id.as_str()).collect();
        self.pages.in_flight.is_some() || self.pages.next(&wanted, at_ms).is_some()
    }

    /// The listed mobs' cards, in the panel's order.
    pub fn cards(&self) -> Vec<MobCard> {
        self.listed
            .iter()
            .map(|mob| MobCard {
                id: mob.id.clone(),
                url: meowdb::mob_url(&mob.id),
                image: match self.images.answer(&mob.id) {
                    Some(Ok(image)) => Some(image.clone()),
                    _ => None,
                },
                page: match self.pages.answer(&mob.id) {
                    None => MobPage::Loading,
                    Some(Ok(Fetched::Fresh(details))) => MobPage::Ready(details.clone()),
                    Some(Ok(Fetched::SavedCopy { data, saved_at_unix_ms })) => {
                        MobPage::SavedCopy { saved_at_unix_ms: *saved_at_unix_ms, details: data.clone() }
                    }
                    Some(Err(SourceError::Unreachable(_))) => MobPage::Unavailable,
                    Some(Err(SourceError::FormatChanged(_))) => MobPage::FormatChanged,
                },
            })
            .collect()
    }
}

/// Requests to one site, one at a time: each mob's answer, and the mob whose answer is
/// awaited.
struct Lane<T> {
    /// Each mob asked about, its latest answer, and when it came.
    answers: Vec<(String, Result<T, SourceError>, u64)>,
    in_flight: Option<String>,
}

impl<T> Default for Lane<T> {
    fn default() -> Self {
        Self { answers: Vec::new(), in_flight: None }
    }
}

impl<T> Lane<T> {
    fn answer(&self, id: &str) -> Option<&Result<T, SourceError>> {
        self.answers.iter().find(|(asked, ..)| asked == id).map(|(_, answer, _)| answer)
    }

    /// The next wanted mob to ask about: the first never asked, else the first whose
    /// site couldn't be reached and whose retry is due.
    fn next<'a>(&self, wanted: &[&'a str], at_ms: u64) -> Option<&'a str> {
        let unasked = wanted.iter().find(|id| self.answer(id).is_none());
        let retry = || {
            wanted.iter().find(|id| {
                self.answers.iter().any(|(asked, answer, came_ms)| {
                    asked == *id
                        && matches!(answer, Err(SourceError::Unreachable(_)))
                        && at_ms.saturating_sub(*came_ms) >= RETRY_MS
                })
            })
        };
        unasked.or_else(retry).copied()
    }

    /// Check on the request in flight (even for a mob no longer listed), and ask about
    /// the next wanted mob when `pace` allows.
    fn poll(
        &mut self,
        wanted: &[&str],
        pace: &mut Pace,
        at_ms: u64,
        mut ask: impl FnMut(&str) -> Poll<Result<T, SourceError>>,
    ) {
        for _ in 0..AT_ONCE_PER_TICK {
            let (id, first_ask) = match self.in_flight.take() {
                Some(id) => (id, false),
                None if !pace.ready(at_ms) => return,
                None => match self.next(wanted, at_ms) {
                    Some(id) => (id.to_string(), true),
                    None => return,
                },
            };
            let answer = match ask(&id) {
                Poll::Pending => {
                    if first_ask {
                        pace.sent(at_ms);
                    }
                    self.in_flight = Some(id);
                    return;
                }
                Poll::Ready(answer) => answer,
            };
            self.answers.retain(|(asked, ..)| *asked != id);
            self.answers.push((id, answer, at_ms));
            if !first_ask {
                // It came over the network: the next one waits out the pace.
                return;
            }
        }
    }
}
