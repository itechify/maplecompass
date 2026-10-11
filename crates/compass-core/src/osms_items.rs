//! OSMS Data Explorer's item files (ADR 0010): the raw texts the shell fetches from
//! [`crate::osms::DATA_URL`], and the reader that turns them into every item's Item Card
//! figures. A set of its own, apart from the map set ([`crate::osms`]): fetched on the
//! same daily schedule, but read, saved and swapped on its own, so a change in these files
//! takes down only the Item Card. Credit "OSMS Data Explorer (osmsdataexplorer.com)".

use crate::item_card::{ItemSource, ItemUse, QuestReward, ShopPrice};
use crate::item_lookup::{closest, item_key};
use crate::map_index::SourceError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// OSMS's item files as fetched: each file's text, unread. Saved and swapped whole.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFiles {
    /// `items.json`: every item but the Cash Shop's, with its category, figures and price,
    /// and every scroll.
    pub items: String,
    /// `quests.json`: every quest, with the items it gives and needs.
    pub quests: String,
    /// `crafting.json`: every recipe, by discipline and level.
    pub crafting: String,
    /// `cash_shop.json`: every Cash Shop item.
    pub cash_shop: String,
}

impl ItemFiles {
    /// The files' names under [`crate::osms::DATA_URL`], in the order of the fields.
    pub const NAMES: [&str; 4] = ["items.json", "quests.json", "crafting.json", "cash_shop.json"];
}

/// OSMS's item files as the source saved them, at an earlier launch or earlier in this
/// one, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedItemFiles {
    pub files: ItemFiles,
    /// When they were saved (ms since the Unix epoch).
    pub saved_at_unix_ms: u64,
}

/// One item, as its Item Card shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemFacts {
    /// The game's item ID.
    pub id: u32,
    pub name: String,
    /// What kind of item it is, to tell it from others sharing its name: a weapon's kind
    /// ("One-Handed Sword"), other equipment's slot ("Overall"), or its inventory tab
    /// ("Use", "Etc", "Setup"); "Scroll" or "Cash".
    pub kind: String,
    pub shop: ShopPrice,
    /// The quests that give it, in OSMS's order, then the recipes that make it.
    pub comes_from: Vec<ItemSource>,
    /// The quests that need it, in OSMS's order, then the recipes that use it.
    pub used_for: Vec<ItemUse>,
}

/// Every item in OSMS's item files: the game's items, its scrolls and the Cash Shop's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemSet {
    items: Vec<ItemFacts>,
    /// Where each item is in `items`, by its ID: the first, should two share one.
    by_id: HashMap<u32, usize>,
    /// Every name, as [`item_key`] makes it for matching, with the items that have it.
    names: Vec<(Vec<char>, Vec<usize>)>,
}

/// What an item set holds, for the probe's report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemCounts {
    pub items: usize,
    /// Those a shop pays a known price for.
    pub shop_priced: usize,
    /// Those a shop won't buy.
    pub not_sold: usize,
    pub cash: usize,
    /// Those a quest gives or a recipe makes.
    pub with_sources: usize,
    /// Those a quest or a recipe needs.
    pub with_uses: usize,
}

impl ItemSet {
    /// How many items it holds, and of which kinds.
    pub fn counts(&self) -> ItemCounts {
        let count = |keep: fn(&ItemFacts) -> bool| self.items.iter().filter(|item| keep(item)).count();
        ItemCounts {
            items: self.items.len(),
            shop_priced: count(|item| matches!(item.shop, ShopPrice::Sells { .. })),
            not_sold: count(|item| item.shop == ShopPrice::NotSold),
            cash: count(|item| item.shop == ShopPrice::Cash),
            with_sources: count(|item| !item.comes_from.is_empty()),
            with_uses: count(|item| !item.used_for.is_empty()),
        }
    }

    /// Item `id`, if the set has it.
    pub(crate) fn item(&self, id: u32) -> Option<&ItemFacts> {
        self.by_id.get(&id).map(|&at| &self.items[at])
    }

    /// The items with the name read from an Item Tooltip as `read`: the name, but for the
    /// letters the recogniser confuses (I, l and 1; O and 0), case, spacing and
    /// punctuation; or one clearly closer to it than any other. Empty when no name is.
    pub(crate) fn named(&self, read: &str) -> Vec<&ItemFacts> {
        let keys: Vec<&[char]> = self.names.iter().map(|(name, _)| &name[..]).collect();
        closest(&item_key(read), &keys)
            .map(|name| self.names[name].1.iter().map(|&item| &self.items[item]).collect())
            .unwrap_or_default()
    }
}

/// Read OSMS's item files. All or nothing: a file that isn't JSON, or isn't shaped as the
/// reader expects, is [`SourceError::FormatChanged`], never partial figures.
pub fn read(files: &ItemFiles) -> Result<ItemSet, SourceError> {
    read_set(files).map_err(|why| SourceError::FormatChanged(format!("OSMS items: {why}")))
}

fn read_set(files: &ItemFiles) -> Result<ItemSet, String> {
    let file: ItemsFile = parse(&files.items, "items.json")?;
    let quests: QuestsFile = parse(&files.quests, "quests.json")?;
    let crafting: CraftingFile = parse(&files.crafting, "crafting.json")?;
    let cash_shop: CashShopFile = parse(&files.cash_shop, "cash_shop.json")?;
    let mut items = Vec::new();
    let facts = |id, name: &str, kind: String, shop| ItemFacts {
        id,
        name: name.trim().to_string(),
        kind,
        shop,
        comes_from: Vec::new(),
        used_for: Vec::new(),
    };
    for item in &file.items {
        items.push(facts(item.id, &item.name, kind(item), shop_price(item)?));
    }
    for scroll in &file.scrolls {
        items.push(facts(scroll.id, &scroll.name, "Scroll".into(), ShopPrice::Unknown));
    }
    for item in cash_shop.categories.iter().flat_map(|category| &category.items) {
        items.push(facts(item.id, &item.name, "Cash".into(), ShopPrice::Cash));
    }
    sources_and_uses(&mut items, &quests, &crafting);
    let mut by_id = HashMap::new();
    let mut names: Vec<(Vec<char>, Vec<usize>)> = Vec::new();
    let mut by_name: HashMap<Vec<char>, usize> = HashMap::new();
    for (at, item) in items.iter().enumerate() {
        by_id.entry(item.id).or_insert(at);
        let name = item_key(&item.name);
        match by_name.get(&name) {
            Some(&known) => names[known].1.push(at),
            None => {
                by_name.insert(name.clone(), names.len());
                names.push((name, vec![at]));
            }
        }
    }
    Ok(ItemSet { items, by_id, names })
}

/// Each item's quests and recipes: the quests that give it and the recipes that make it,
/// the quests that need it and the recipes that use it. Quests name items by ID; recipes
/// name what they make by ID, and their ingredients by name.
fn sources_and_uses(items: &mut [ItemFacts], quests: &QuestsFile, crafting: &CraftingFile) {
    let mut at_id: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut at_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (at, item) in items.iter().enumerate() {
        at_id.entry(item.id).or_default().push(at);
        at_name.entry(item.name.clone()).or_default().push(at);
    }
    let by_id = |id: u32| at_id.get(&id).cloned().unwrap_or_default();
    for quest in &quests.quests {
        let name = quest.name.trim();
        for reward in quest.rewards.iter().filter(|reward| reward.kind == "item") {
            let Some(id) = reward.id.as_ref().and_then(item_id) else { continue };
            let source = ItemSource::Quest { name: name.into(), reward: quest_reward(quest, reward, id) };
            for at in by_id(id) {
                add(&mut items[at].comes_from, source.clone());
            }
        }
        for need in quest.requirements_list.iter().filter(|need| need.kind == "item") {
            let (Some(id), Some(count)) = (need.id.as_ref().and_then(item_id), need.count) else { continue };
            for at in by_id(id) {
                add(&mut items[at].used_for, ItemUse::Quest { name: name.into(), count });
            }
        }
    }
    for discipline in &crafting.disciplines {
        let levels = discipline.output_types.iter().flat_map(|output| &output.levels);
        for (level, recipe) in levels.flat_map(|level| level.recipes.iter().map(move |recipe| (level.level, recipe))) {
            let source = ItemSource::Recipe { discipline: discipline.discipline.trim().into(), level };
            for at in by_id(recipe.output_id) {
                add(&mut items[at].comes_from, source.clone());
            }
            for ingredient in &recipe.ingredients {
                let used = ItemUse::Recipe { makes: recipe.result_item_name.trim().into(), count: ingredient.count };
                for &at in at_name.get(ingredient.item_name.trim()).into_iter().flatten() {
                    add(&mut items[at].used_for, used.clone());
                }
            }
        }
    }
}

/// How `quest` gives item `id`, its `reward`: always, at the chance of its weighted
/// rewards that gives it (the best, if it is among several classes'), or as a pick.
fn quest_reward(quest: &OsmsQuest, reward: &Reward, id: u32) -> QuestReward {
    if reward.guaranteed.unwrap_or(true) {
        return QuestReward::Always;
    }
    let weighted = quest.reward_weighted.iter().flat_map(|reward| &reward.groups).flat_map(|group| &group.items);
    let chance = weighted.filter(|item| item.id == id).map(|item| item.chance_pct).fold(None, |best: Option<f64>, pct| {
        Some(best.map_or(pct, |best| best.max(pct)))
    });
    if let Some(pct) = chance.filter(|pct| pct.is_finite() && *pct >= 0.0) {
        return QuestReward::Chance { hundredths: (pct * 100.0).round() as u32 };
    }
    let mut choices = quest.reward_choices.iter().flat_map(|reward| &reward.groups).flat_map(|group| &group.items);
    if choices.any(|item| item.id == id) { QuestReward::Pick } else { QuestReward::Sometimes }
}

/// An item's ID as `quests.json` writes it: a number, though it writes other IDs (the
/// quests a quest needs) as strings.
fn item_id(id: &Value) -> Option<u32> {
    match id {
        Value::Number(n) => n.as_u64().and_then(|id| u32::try_from(id).ok()),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// `list` with `entry` at its end, unless it has it already.
fn add<T: PartialEq>(list: &mut Vec<T>, entry: T) {
    if !list.contains(&entry) {
        list.push(entry);
    }
}

/// What kind of item `item` is: a weapon's kind, spelt out as the game's tooltips do ("1H
/// Sword" is a "One-Handed Sword"), other equipment's slot, or the inventory tab the rest
/// go in.
fn kind(item: &OsmsItem) -> String {
    if let Some(weapon) = &item.weapon_type {
        let weapon = weapon.trim();
        return match (weapon.strip_prefix("1H "), weapon.strip_prefix("2H ")) {
            (Some(kind), _) => format!("One-Handed {kind}"),
            (_, Some(kind)) => format!("Two-Handed {kind}"),
            _ => weapon.to_string(),
        };
    }
    match item.category.as_str() {
        "Equipment" => item.sub_category.trim().to_string(),
        "Consumable" => "Use".into(),
        category => category.to_string(),
    }
}

/// What a shop pays for `item`: equipment's price is top-level, the rest's in its stats.
/// One a shop won't buy carries a placeholder price of 1; throwing stars' and bullets'
/// price isn't known to be what a shop pays for them (they carry a recharge price).
fn shop_price(item: &OsmsItem) -> Result<ShopPrice, String> {
    // `notSale: 1`; a 0 or a false would say a shop buys it.
    if item.stats.get("notSale").is_some_and(|flag| !matches!(flag, Value::Bool(false)) && flag.as_u64() != Some(0)) {
        return Ok(ShopPrice::NotSold);
    }
    if item.stats.contains_key("unitPrice") {
        return Ok(ShopPrice::Unknown);
    }
    let price = if item.category == "Equipment" { item.price.as_ref() } else { item.stats.get("price") };
    match price.map(mesos).transpose().map_err(|why| format!("item {}: {why}", item.id))? {
        Some(mesos) => Ok(ShopPrice::Sells { mesos }),
        None => Ok(ShopPrice::Unknown),
    }
}

/// A price as OSMS writes it: a number, or now and then a number in a string.
fn mesos(price: &Value) -> Result<u64, String> {
    let mesos = match price {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    };
    mesos.ok_or_else(|| format!("price {price} isn't mesos"))
}

fn parse<T: serde::de::DeserializeOwned>(text: &str, file: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|e| format!("{file}: {e}"))
}

/// `items.json`.
#[derive(Deserialize)]
struct ItemsFile {
    items: Vec<OsmsItem>,
    scrolls: Vec<OsmsScroll>,
}

/// A scroll in `items.json`'s `scrolls`.
#[derive(Deserialize)]
struct OsmsScroll {
    id: u32,
    name: String,
}

/// `quests.json`.
#[derive(Deserialize)]
struct QuestsFile {
    quests: Vec<OsmsQuest>,
}

/// A quest in `quests.json`.
#[derive(Deserialize)]
struct OsmsQuest {
    name: String,
    /// What it gives: EXP, mesos, items...
    #[serde(default)]
    rewards: Vec<Reward>,
    /// Its rewards given at random, with each item's chance.
    #[serde(default)]
    reward_weighted: Vec<RewardGroups<WeightedItem>>,
    /// Its rewards the player picks one of.
    #[serde(default)]
    reward_choices: Vec<RewardGroups<ChoiceItem>>,
    /// What it needs: mobs defeated, items, other quests...
    #[serde(default)]
    requirements_list: Vec<Requirement>,
}

/// One of a quest's rewards.
#[derive(Deserialize)]
struct Reward {
    /// "item", "exp", "money"...
    #[serde(rename = "type")]
    kind: String,
    /// An item's ID.
    #[serde(default)]
    id: Option<Value>,
    /// Whether an item is given every time; it is unless `false`.
    #[serde(default)]
    guaranteed: Option<bool>,
}

/// A quest's random or picked rewards, by class.
#[derive(Deserialize)]
struct RewardGroups<T> {
    groups: Vec<RewardGroup<T>>,
}

#[derive(Deserialize)]
struct RewardGroup<T> {
    items: Vec<T>,
}

/// An item a quest gives at random, and how often (percent).
#[derive(Deserialize)]
struct WeightedItem {
    id: u32,
    chance_pct: f64,
}

/// An item a quest gives if the player picks it.
#[derive(Deserialize)]
struct ChoiceItem {
    id: u32,
}

/// One of the things a quest needs.
#[derive(Deserialize)]
struct Requirement {
    /// "item", "mob", "quest"...
    #[serde(rename = "type")]
    kind: String,
    /// An item's or a mob's ID, or a quest's as a string.
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    count: Option<u32>,
}

/// `crafting.json`.
#[derive(Deserialize)]
struct CraftingFile {
    disciplines: Vec<Discipline>,
}

/// A crafting discipline ("Smithing"), with its recipes by what they make and their level.
#[derive(Deserialize)]
struct Discipline {
    discipline: String,
    output_types: Vec<OutputType>,
}

#[derive(Deserialize)]
struct OutputType {
    levels: Vec<RecipeLevel>,
}

/// The recipes of one level of a discipline.
#[derive(Deserialize)]
struct RecipeLevel {
    level: u32,
    recipes: Vec<OsmsRecipe>,
}

/// A recipe: what it makes, from what.
#[derive(Deserialize)]
struct OsmsRecipe {
    output_id: u32,
    result_item_name: String,
    ingredients: Vec<Ingredient>,
}

/// One of a recipe's ingredients, by name, and how many.
#[derive(Deserialize)]
struct Ingredient {
    item_name: String,
    count: u32,
}

/// `cash_shop.json`.
#[derive(Deserialize)]
struct CashShopFile {
    categories: Vec<CashCategory>,
}

/// One of the Cash Shop's categories, with its items.
#[derive(Deserialize)]
struct CashCategory {
    items: Vec<CashItem>,
}

/// A Cash Shop item.
#[derive(Deserialize)]
struct CashItem {
    id: u32,
    name: String,
}

/// An item in `items.json`'s `items`.
#[derive(Deserialize)]
struct OsmsItem {
    id: u32,
    name: String,
    category: String,
    /// Equipment's slot ("Hat", "Weapon"), or "Consume", "Etc" or "Install".
    sub_category: String,
    /// A weapon's kind ("1H Sword", "Claw").
    #[serde(default)]
    weapon_type: Option<String>,
    /// Its figures, among them `price` on all but equipment.
    #[serde(default)]
    stats: serde_json::Map<String, Value>,
    /// Equipment's price.
    #[serde(default)]
    price: Option<Value>,
}
