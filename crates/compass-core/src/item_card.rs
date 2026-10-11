//! The Item Card (ADR 0010): the player hovers an item in the game and presses the hotkey
//! (Ctrl+Shift+I by default). The shell captures the game's client area and the pointer,
//! the Item Tooltip reader reads the item's name, and the card shows at the pointer what
//! the game's tooltip doesn't: what a shop pays for the item, what players pay for it, where
//! it comes from and what it's used for. The name is matched against OSMS's items on the
//! player's machine: nothing is asked at the press.
//!
//! One card shows at a time, until the player closes it or presses the hotkey again. What
//! keeps a card from showing goes in the Item Lookup alert.

use crate::item_lookup::{ItemLookup, OurWindow, item_name, overlaps};
use crate::item_tooltip::{ItemTooltip, ItemTooltipReading};
use crate::osms_items::{ItemFacts, ItemSet};
use crate::panel::{GameWindow, ScreenPoint, ScreenRect, Size};
use serde::Serialize;

/// What a shop pays for an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ShopPrice {
    /// A shop pays this for one.
    Sells { mesos: u64 },
    /// A shop won't buy it.
    NotSold,
    /// A Cash Shop item.
    Cash,
    /// OSMS has no price for it (scrolls, starter gear, arrows), or it is a throwing star or
    /// a bullet: OSMS gives what recharging them costs, not what a shop pays for them.
    Unknown,
}

/// Where an item comes from, besides mobs' drops.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ItemSource {
    /// Quest `name` gives it, as `reward` says.
    Quest { name: String, reward: QuestReward },
    /// A recipe of `discipline` (Smithing, Tailoring...) at `level` makes it.
    Recipe { discipline: String, level: u32 },
}

/// How a quest gives an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum QuestReward {
    /// Every time.
    Always,
    /// At random, `hundredths` of a percent of the time (714: 7.14%).
    Chance { hundredths: u32 },
    /// If the player picks it from the quest's rewards.
    Pick,
    /// Not every time, but OSMS gives no chance.
    Sometimes,
}

/// What an item is used for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ItemUse {
    /// Quest `name` needs `count` of it.
    Quest { name: String, count: u32 },
    /// The recipe that makes `makes` needs `count` of it.
    Recipe { makes: String, count: u32 },
}

/// The first few of a list, and how many more there are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listed<T> {
    pub shown: Vec<T>,
    pub more: u32,
}

/// How many of each of the card's lists show: "and N more" stands for the rest.
const LISTED: usize = 3;

impl<T: Clone> Listed<T> {
    fn of(all: &[T]) -> Self {
        Listed {
            shown: all.iter().take(LISTED).cloned().collect(),
            more: u32::try_from(all.len().saturating_sub(LISTED)).unwrap_or(u32::MAX),
        }
    }
}

/// What players pay one another for an item, by BasilMarket's price list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Market {
    /// No price list is held yet: one is being fetched.
    Loading,
    /// Players paid `mesos` for one, by BasilMarket's recommended price from `sales` sales.
    /// `page` is the item's price page, with its range and history.
    Sold {
        mesos: u64,
        sales: u32,
        page: String,
        /// When the price list was saved (ms since the Unix epoch), if over a day ago: it
        /// couldn't be refreshed since. `None` for a fresher list.
        #[serde(rename = "fromUnixMs")]
        from_unix_ms: Option<u64>,
    },
    /// BasilMarket has seen no sales of it: it shows the NPC shop's price or nothing. Its
    /// price page, if BasilMarket lists it.
    NoSales {
        page: Option<String>,
        #[serde(rename = "fromUnixMs")]
        from_unix_ms: Option<u64>,
    },
    /// No price list could be had, and none is saved.
    Unavailable,
}

/// The Item Card's credit (ADR 0010): its data is OSMS Data Explorer's, and its market
/// prices BasilMarket's.
pub const ITEM_CARD_CREDIT: &str = "Data: OSMS Data Explorer · Prices: BasilMarket.net";

/// One item's Item Card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemCard {
    /// The game's item ID, 8 digits as the game writes them (`"02000000"`).
    pub id: String,
    pub name: String,
    pub shop: ShopPrice,
    pub market: Market,
    /// The quests that give it, then the recipes that make it.
    pub comes_from: Listed<ItemSource>,
    /// The quests that need it, then the recipes that use it.
    pub used_for: Listed<ItemUse>,
    /// When OSMS's item files were saved (ms since the Unix epoch), if that was over 7 days
    /// ago: the card shows a Saved Copy, as OSMS couldn't be reached since. `None` for
    /// files fresher than that.
    pub saved_copy_from_unix_ms: Option<u64>,
    /// [`ITEM_CARD_CREDIT`].
    pub credit: &'static str,
}

/// One of several items sharing the name read, for the player to pick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemCardChoice {
    /// The game's item ID, 8 digits.
    pub id: String,
    /// What kind of item it is: a weapon's kind ("One-Handed Sword"), other equipment's
    /// slot ("Overall"), or its inventory tab ("Use", "Etc", "Setup"); "Scroll" or "Cash".
    pub kind: String,
    /// Where the overlay loads its picture from (OSMS's, as the game draws it), once it is
    /// in; `None` until then.
    pub icon: Option<String>,
}

/// What the Item Card shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ItemCardView {
    /// One item's card.
    Card(ItemCard),
    /// Several items have the name read, and don't look alike (their kinds or shop prices
    /// differ): the player picks theirs, by the picture the game's tooltip shows.
    Choose { name: String, items: Vec<ItemCardChoice> },
}

/// The Item Card showing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShownItemCard {
    /// Counts the cards the player opened, so the window can tell a new card from the last
    /// one when both show the same item.
    pub card: u32,
    #[serde(flatten)]
    pub view: ItemCardView,
}

/// What a press of the Item Card's hotkey came to.
pub(crate) enum Pressed {
    /// The card of the item read shows.
    Shown,
    /// No Item Tooltip by the pointer: the card showing closed, and nothing is said.
    Closed,
    /// The alert says why no card shows.
    Alert(ItemLookup),
}

/// The Item Card showing, if any.
#[derive(Default)]
pub(crate) struct ItemCards {
    shown: Option<Shown>,
    cards: u32,
}

/// The card showing: which items, and where the pointer was.
struct Shown {
    card: u32,
    pointer: ScreenPoint,
    showing: Showing,
}

/// What the card showing shows.
enum Showing {
    /// The card of these items, which share their name, kind and shop price: they look
    /// alike, so their quests and recipes are told together.
    Card(Vec<u32>),
    /// These items share their name, but not their kind or shop price: the player picks
    /// one, by its picture and kind. Each is listed once for the items that look alike.
    Choose(Vec<Vec<u32>>),
}

impl ItemCards {
    /// What the capture read as (`None`: it failed), with where our windows are on screen,
    /// matched against `items`.
    pub fn read(
        &mut self,
        reading: Option<ItemTooltipReading>,
        ours: &[(OurWindow, ScreenRect)],
        items: Result<&ItemSet, ItemLookup>,
    ) -> Pressed {
        // One card at a time: the press replaces the card showing, or closes it.
        let had_card = self.shown.take().is_some();
        let Some(reading) = reading else { return Pressed::Alert(ItemLookup::Unreadable) };
        let (area, name) = match &reading.tooltip {
            ItemTooltip::None if had_card => return Pressed::Closed,
            ItemTooltip::None => return Pressed::Alert(ItemLookup::NoTooltip),
            ItemTooltip::Found { area, name } => (*area, name),
        };
        if let Some((by, _)) = ours.iter().find(|(_, window)| overlaps(*window, area)) {
            return Pressed::Alert(ItemLookup::Covered { by: *by });
        }
        let Some(read) = name.as_deref().and_then(item_name) else { return Pressed::Alert(ItemLookup::Unreadable) };
        let items = match items {
            Ok(items) => items,
            Err(missing) => return Pressed::Alert(missing),
        };
        let mut alike: Vec<(&ItemFacts, Vec<u32>)> = Vec::new();
        for item in items.named(&read) {
            match alike.iter_mut().find(|(first, _)| first.kind == item.kind && first.shop == item.shop) {
                Some((_, ids)) => ids.push(item.id),
                None => alike.push((item, vec![item.id])),
            }
        }
        let mut groups: Vec<Vec<u32>> = alike.into_iter().map(|(_, ids)| ids).collect();
        let showing = match groups.len() {
            0 => return Pressed::Alert(ItemLookup::NoSuchItem { read }),
            1 => Showing::Card(groups.remove(0)),
            _ => Showing::Choose(groups),
        };
        self.cards += 1;
        self.shown = Some(Shown { card: self.cards, pointer: reading.pointer, showing });
        Pressed::Shown
    }

    /// The player picked item `id` from the items listed: its card shows in the list's
    /// place. Nothing changes for an item not listed.
    pub fn pick(&mut self, id: &str) {
        let Some(Shown { showing, .. }) = &mut self.shown else { return };
        let Showing::Choose(groups) = showing else { return };
        if let Some(picked) = groups.iter().find(|ids| ids.first().is_some_and(|&first| item_id(first) == id)) {
            *showing = Showing::Card(picked.clone());
        }
    }

    /// The pictures of the items listed to pick from, if the card lists any, below
    /// [`crate::osms::PICTURES_URL`].
    pub fn pictures(&self) -> Vec<String> {
        match &self.shown {
            Some(Shown { showing: Showing::Choose(groups), .. }) => {
                groups.iter().filter_map(|ids| ids.first()).map(|&id| icon_path(id)).collect()
            }
            _ => Vec::new(),
        }
    }

    /// The player closed the card.
    pub fn close(&mut self) {
        self.shown = None;
    }

    /// The name and kind of the item whose card shows, as `items` has it; `None` while no
    /// card shows, or a choice does.
    pub fn item<'a>(&self, items: &'a ItemSet) -> Option<&'a ItemFacts> {
        match &self.shown {
            Some(Shown { showing: Showing::Card(ids), .. }) => items.item(*ids.first()?),
            _ => None,
        }
    }

    /// Where the pointer was when the card showing opened.
    pub fn pointer(&self) -> Option<ScreenPoint> {
        self.shown.as_ref().map(|shown| shown.pointer)
    }

    /// The card showing, as `items` has its items (a Saved Copy from `saved_copy_from`, if
    /// they are one), with the pictures in so far (by `picture`, from a path below
    /// [`crate::osms::PICTURES_URL`]).
    pub fn view(
        &self,
        items: Option<&ItemSet>,
        saved_copy_from: Option<u64>,
        market: impl Fn(&[u32]) -> Market,
        picture: impl Fn(&str) -> Option<String>,
    ) -> Option<ShownItemCard> {
        let shown = self.shown.as_ref()?;
        let items = items?;
        let view = match &shown.showing {
            Showing::Card(ids) => ItemCardView::Card(ItemCard {
                market: market(ids),
                saved_copy_from_unix_ms: saved_copy_from,
                ..card(ids, items)?
            }),
            Showing::Choose(groups) => {
                let listed: Vec<&ItemFacts> = groups.iter().filter_map(|ids| items.item(*ids.first()?)).collect();
                ItemCardView::Choose {
                    name: listed.first()?.name.clone(),
                    items: listed
                        .iter()
                        .map(|item| ItemCardChoice {
                            id: item_id(item.id),
                            kind: item.kind.clone(),
                            icon: picture(&icon_path(item.id)),
                        })
                        .collect(),
                }
            }
        };
        Some(ShownItemCard { card: shown.card, view })
    }
}

/// The card of the items `ids`, which look alike: the first's, with the quests and
/// recipes of all of them. Its market line is left to the price list.
fn card(ids: &[u32], items: &ItemSet) -> Option<ItemCard> {
    let alike: Vec<&ItemFacts> = ids.iter().filter_map(|&id| items.item(id)).collect();
    let first = alike.first()?;
    let comes_from = unique(alike.iter().flat_map(|item| &item.comes_from));
    let used_for = unique(alike.iter().flat_map(|item| &item.used_for));
    Some(ItemCard {
        id: item_id(first.id),
        name: first.name.clone(),
        shop: first.shop,
        market: Market::Loading,
        comes_from: Listed::of(&comes_from),
        used_for: Listed::of(&used_for),
        saved_copy_from_unix_ms: None,
        credit: ITEM_CARD_CREDIT,
    })
}

/// `list` with each entry once, where it first is.
fn unique<'a, T: Clone + PartialEq + 'a>(list: impl IntoIterator<Item = &'a T>) -> Vec<T> {
    let mut once: Vec<T> = Vec::new();
    for entry in list {
        if !once.contains(entry) {
            once.push(entry.clone());
        }
    }
    once
}

/// The game's item ID as the game writes it: 8 digits.
fn item_id(id: u32) -> String {
    format!("{id:08}")
}

/// Where OSMS keeps item `id`'s picture, below [`crate::osms::PICTURES_URL`].
fn icon_path(id: u32) -> String {
    format!("current/images/items/{}.png", item_id(id))
}

/// The card's corner sits this far right of and below the pointer's tip (physical px), like
/// the game's own tooltip: a click without moving the mouse still lands on the item.
const POINTER_GAP: i32 = 8;
/// Gap kept between the card and a captured region it moves clear of (physical px).
const REGION_GAP: i32 = 8;

/// Where the card of `size` goes for the pointer at `pointer`: its corner just below and
/// right of the pointer, or left of it, or above it, whichever first stays inside the
/// game's client area and off `avoid` (the captured regions, since our windows show in our
/// captures). With none of them clear, below and right of the pointer, moved below (or
/// above) the regions it would cover.
pub(crate) fn card_placement(game: &GameWindow, pointer: ScreenPoint, size: Size, avoid: &[ScreenRect]) -> ScreenRect {
    let client = game.client;
    let (width, height) = (size.width.min(client.width), size.height.min(client.height));
    let (right, left) = (pointer.x + POINTER_GAP, pointer.x - POINTER_GAP - width as i32);
    let (below, above) = (pointer.y + POINTER_GAP, pointer.y - POINTER_GAP - height as i32);
    let at = |x, y| ScreenRect { x, y, width, height };
    let spots = [at(right, below), at(left, below), at(right, above), at(left, above)];
    let clear = |card: &ScreenRect| !avoid.iter().any(|region| overlaps(*card, *region));
    if let Some(card) = spots.iter().find(|card| inside(**card, client) && clear(card)) {
        return *card;
    }
    let mut card = clamp(at(right, below), client);
    // Each region is gone over twice, as a move off one may cover another.
    for _ in 0..2 {
        for region in avoid {
            if overlaps(card, *region) {
                let under = region.y + region.height as i32 + REGION_GAP;
                card.y = if inside(at(card.x, under), client) { under } else { region.y - REGION_GAP - height as i32 };
            }
        }
    }
    card
}

fn inside(card: ScreenRect, client: ScreenRect) -> bool {
    card.x >= client.x
        && card.y >= client.y
        && card.x + card.width as i32 <= client.x + client.width as i32
        && card.y + card.height as i32 <= client.y + client.height as i32
}

/// `card` moved the least to lie inside `client`, which is at least as big.
fn clamp(card: ScreenRect, client: ScreenRect) -> ScreenRect {
    let x = card.x.clamp(client.x, client.x + (client.width - card.width) as i32);
    let y = card.y.clamp(client.y, client.y + (client.height - card.height) as i32);
    ScreenRect { x, y, ..card }
}
