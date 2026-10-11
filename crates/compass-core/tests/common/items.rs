//! Made-up item data in OSMS Data Explorer's exact JSON shape (ADR 0010): invented items,
//! scrolls, quests, recipes and Cash Shop items, written out as the four item files the
//! shell fetches. The names are the game's, as players see them; every figure, quest and
//! recipe is made up. Nothing from OSMS is committed.

use compass_core::osms_items::ItemFiles;
use serde_json::{Map as Object, Value, json};

/// An item in `items.json`'s `items`.
#[derive(Debug, Clone)]
pub struct Item {
    pub id: u64,
    pub name: String,
    /// `category`: "Equipment", "Consumable", "Etc" or "Setup".
    pub category: String,
    /// `sub_category`: "Hat", "Weapon", "Consume", "Etc", "Install"...
    pub sub_category: String,
    /// `weapon_type`, on weapons ("1H Sword", "Claw").
    pub weapon_type: Option<String>,
    /// `stats`, as written.
    pub stats: Object<String, Value>,
    /// Equipment's top-level `price`, as written (OSMS once wrote one as a string).
    pub equip_price: Option<Value>,
}

fn item(id: u64, name: &str, category: &str, sub_category: &str) -> Item {
    Item {
        id,
        name: name.into(),
        category: category.into(),
        sub_category: sub_category.into(),
        weapon_type: None,
        stats: Object::new(),
        equip_price: None,
    }
}

/// A potion, a throwing star or another item used from the Use tab.
pub fn use_item(id: u64, name: &str) -> Item {
    item(id, name, "Consumable", "Consume")
}

/// An ore, a mob's drop or another Etc item.
pub fn etc(id: u64, name: &str) -> Item {
    item(id, name, "Etc", "Etc")
}

/// A chair or another Setup item.
pub fn setup(id: u64, name: &str) -> Item {
    item(id, name, "Setup", "Install")
}

/// Equipment that isn't a weapon: `sub_category` "Hat", "Overall", "Earring"...
pub fn equip(id: u64, name: &str, sub_category: &str) -> Item {
    item(id, name, "Equipment", sub_category)
}

/// A weapon of `weapon_type` ("1H Sword", "Claw").
pub fn weapon(id: u64, name: &str, weapon_type: &str) -> Item {
    Item { weapon_type: Some(weapon_type.into()), ..item(id, name, "Equipment", "Weapon") }
}

impl Item {
    /// What a shop pays for it: top-level `price` on equipment, `stats.price` on the rest.
    pub fn sells_for(self, mesos: u64) -> Self {
        self.priced(json!(mesos))
    }

    /// Equipment's price written as a string, as OSMS once wrote Pink Jester's.
    pub fn sells_for_text(self, mesos: &str) -> Self {
        self.priced(json!(mesos))
    }

    fn priced(mut self, price: Value) -> Self {
        if self.category == "Equipment" {
            self.equip_price = Some(price);
        } else {
            self.stats.insert("price".into(), price);
        }
        self
    }

    /// A shop won't buy it (`notSale`), and OSMS gives it the placeholder price of 1.
    pub fn not_for_sale(mut self) -> Self {
        self.stats.insert("notSale".into(), json!(1));
        self.priced(json!(1))
    }

    /// Recharged at a shop for `unit` mesos per star or bullet (`unitPrice`).
    pub fn recharged_at(mut self, unit: f64) -> Self {
        self.stats.insert("unitPrice".into(), json!(unit));
        self
    }

    /// Another stat, as OSMS writes it.
    pub fn stat(mut self, name: &str, value: Value) -> Self {
        self.stats.insert(name.into(), value);
        self
    }

    fn json(&self) -> Value {
        let mut item = Object::new();
        item.insert("id".into(), json!(self.id));
        item.insert("name".into(), json!(self.name));
        item.insert("category".into(), json!(self.category));
        item.insert("sub_category".into(), json!(self.sub_category));
        item.insert("stats".into(), Value::Object(self.stats.clone()));
        if self.category == "Equipment" {
            item.insert("req_job_label".into(), json!("All"));
        } else {
            item.insert("description".into(), json!(format!("A made-up {}.", self.name)));
        }
        if let Some(price) = &self.equip_price {
            item.insert("price".into(), price.clone());
        }
        if let Some(weapon_type) = &self.weapon_type {
            item.insert("weapon_type".into(), json!(weapon_type));
            item.insert("attack_speed_label".into(), json!("Fast"));
        }
        Value::Object(item)
    }
}

/// A scroll in `items.json`'s `scrolls`: no price, no category.
#[derive(Debug, Clone)]
pub struct Scroll {
    pub id: u64,
    pub name: String,
}

pub fn scroll(id: u64, name: &str) -> Scroll {
    Scroll { id, name: name.into() }
}

/// A Cash Shop item in `cash_shop.json`.
#[derive(Debug, Clone)]
pub struct CashItem {
    pub id: u64,
    pub name: String,
    /// Its Cash Shop category ("Equipment", "Weather Effect").
    pub category: String,
}

pub fn cash(id: u64, name: &str, category: &str) -> CashItem {
    CashItem { id, name: name.into(), category: category.into() }
}

/// A quest in `quests.json`, with the items it gives and needs.
#[derive(Debug, Clone)]
pub struct Quest {
    pub id: u64,
    pub name: String,
    rewards: Vec<Value>,
    weighted: Vec<Value>,
    choices: Vec<Value>,
    needs: Vec<Value>,
}

pub fn quest(id: u64, name: &str) -> Quest {
    Quest { id, name: name.into(), rewards: Vec::new(), weighted: Vec::new(), choices: Vec::new(), needs: Vec::new() }
}

impl Quest {
    /// Always gives `count` of the item.
    pub fn gives(mut self, id: u64, name: &str, count: u32) -> Self {
        self.rewards.push(json!({"type": "item", "guaranteed": true, "id": id, "name": name, "count": count}));
        self
    }

    /// Gives the item at random, `chance_pct` percent of the time, among its weighted
    /// rewards for any class.
    pub fn gives_at_random(mut self, id: u64, name: &str, chance_pct: f64) -> Self {
        self.rewards.push(json!({"type": "item", "guaranteed": false, "id": id, "name": name, "count": 1, "prop": 1}));
        self.weighted.push(json!({"id": id, "name": name, "count": 1, "label": name, "weight": 1, "chance_pct": chance_pct}));
        self
    }

    /// Gives the item if the player picks it from its rewards.
    pub fn gives_as_a_pick(mut self, id: u64, name: &str) -> Self {
        self.rewards.push(json!({"type": "item", "guaranteed": false, "id": id, "name": name, "count": 1, "prop": -1}));
        self.choices.push(json!({"id": id, "name": name, "count": 1, "label": name}));
        self
    }

    /// Needs `count` of the item to complete.
    pub fn needs(mut self, id: u64, name: &str, count: u32) -> Self {
        self.needs.push(json!({"type": "item", "id": id, "name": name, "count": count, "label": format!("{name} x {count}")}));
        self
    }

    fn json(&self) -> Value {
        let mut rewards = vec![
            json!({"type": "exp", "value": 100}),
            json!({"type": "money", "value": 500}),
            json!({"type": "next_quest", "value": "A Made-up Sequel"}),
        ];
        rewards.extend(self.rewards.iter().cloned());
        // OSMS writes a quest's ID as a string where a quest needs another, unlike every
        // other ID.
        let mut needs = vec![
            json!({"type": "quest", "id": "1000", "name": "A Made-up Start", "state": 2,
                   "label": "Complete: A Made-up Start (#001000)"}),
            json!({"type": "citizenship", "grade": 1, "town": 1, "town_name": "Henesys", "label": "Henesys citizen"}),
            json!({"type": "mob", "id": 100100, "name": "Snail", "count": 10, "label": "Defeat Snail x 10"}),
        ];
        needs.extend(self.needs.iter().cloned());
        let mut quest = json!({
            "id": self.id.to_string(),
            "name": self.name,
            "area": 11,
            "region": "Maple Island",
            "description": "A made-up quest.",
            "npc_name": "Made-up NPC",
            "rewards": rewards,
            "requirements": "Made up",
            "requirements_list": needs,
        });
        if !self.weighted.is_empty() {
            let total = self.weighted.len();
            quest["reward_weighted"] = json!([{"type": "weighted_random", "class_specific": false, "groups": [
                {"job_mask": 0, "job_name": "Any Class", "total_weight": total, "items": self.weighted}
            ]}]);
        }
        if !self.choices.is_empty() {
            quest["reward_choices"] = json!([{"type": "choose_one", "class_specific": false, "groups": [
                {"job_mask": 0, "job_name": "Any Class", "items": self.choices}
            ]}]);
        }
        quest
    }
}

/// A recipe in `crafting.json`: what it makes, at what level of which discipline, from what.
#[derive(Debug, Clone)]
pub struct Recipe {
    pub discipline: String,
    pub level: u32,
    pub makes: (u64, String),
    /// Each ingredient by name, as OSMS gives them, and how many.
    pub ingredients: Vec<(String, u32)>,
}

pub fn recipe(discipline: &str, level: u32, makes_id: u64, makes: &str) -> Recipe {
    Recipe { discipline: discipline.into(), level, makes: (makes_id, makes.into()), ingredients: Vec::new() }
}

impl Recipe {
    pub fn from(mut self, name: &str, count: u32) -> Self {
        self.ingredients.push((name.into(), count));
        self
    }
}

/// Made-up item data: what `items.json`, `quests.json`, `crafting.json` and
/// `cash_shop.json` hold.
#[derive(Debug, Clone, Default)]
pub struct ItemWorld {
    pub items: Vec<Item>,
    pub scrolls: Vec<Scroll>,
    pub quests: Vec<Quest>,
    pub recipes: Vec<Recipe>,
    pub cash: Vec<CashItem>,
}

pub fn item_world(items: Vec<Item>) -> ItemWorld {
    ItemWorld { items, ..ItemWorld::default() }
}

impl ItemWorld {
    pub fn with(mut self, item: Item) -> Self {
        self.items.push(item);
        self
    }

    pub fn with_scroll(mut self, scroll: Scroll) -> Self {
        self.scrolls.push(scroll);
        self
    }

    pub fn with_quest(mut self, quest: Quest) -> Self {
        self.quests.push(quest);
        self
    }

    pub fn with_recipe(mut self, recipe: Recipe) -> Self {
        self.recipes.push(recipe);
        self
    }

    pub fn with_cash(mut self, item: CashItem) -> Self {
        self.cash.push(item);
        self
    }

    /// The data as OSMS's four item files.
    pub fn files(&self) -> ItemFiles {
        ItemFiles {
            items: self.items_json().to_string(),
            quests: self.quests_json().to_string(),
            crafting: self.crafting_json().to_string(),
            cash_shop: self.cash_shop_json().to_string(),
        }
    }

    fn items_json(&self) -> Value {
        let count = |category: &str| self.items.iter().filter(|item| item.category == category).count();
        let scrolls: Vec<Value> = self
            .scrolls
            .iter()
            .map(|scroll| {
                json!({"id": scroll.id, "name": scroll.name, "equip_slot": "Other", "stat_type": "Attack",
                       "tier": "Lesser", "description": "Improves something.\nSuccess rate: 100%"})
            })
            .collect();
        json!({
            "items": self.items.iter().map(Item::json).collect::<Vec<_>>(),
            "scrolls": scrolls,
            "counts": {"equipment": count("Equipment"), "consumables": count("Consumable"), "etc": count("Etc"),
                       "setup": count("Setup"), "scrolls": self.scrolls.len()},
            "item_total": self.items.len(),
            "scroll_total": self.scrolls.len(),
            "equipment_meta": {"armor_order": [], "job_filters": [], "weapon_type_order": [], "weapon_types_by_class": {}},
        })
    }

    fn quests_json(&self) -> Value {
        json!({
            "quests": self.quests.iter().map(Quest::json).collect::<Vec<_>>(),
            "regions": [],
            "chains": [],
            "total": self.quests.len(),
        })
    }

    fn crafting_json(&self) -> Value {
        let mut disciplines: Vec<(String, Vec<Value>)> = Vec::new();
        for (n, recipe) in self.recipes.iter().enumerate() {
            let ingredients: Vec<Value> =
                recipe.ingredients.iter().map(|(name, count)| json!({"item_name": name, "count": count})).collect();
            let row = json!({"level": recipe.level, "recipes": [{
                "output_id": recipe.makes.0, "result_item_name": recipe.makes.1, "result_count": 1,
                "req_level": recipe.level, "craft_exp": 20, "meso_cost": 1000, "ingredients": ingredients, "id": n + 1,
            }]});
            match disciplines.iter_mut().find(|(name, _)| *name == recipe.discipline) {
                Some((_, levels)) => levels.push(row),
                None => disciplines.push((recipe.discipline.clone(), vec![row])),
            }
        }
        let disciplines: Vec<Value> = disciplines
            .into_iter()
            .map(|(discipline, levels)| {
                json!({"discipline": discipline, "skill_id": 92000000, "count": levels.len(),
                       "output_types": [{"output_type": "Equipment", "count": levels.len(), "levels": levels}]})
            })
            .collect();
        json!({"disciplines": disciplines, "total": self.recipes.len()})
    }

    fn cash_shop_json(&self) -> Value {
        let mut categories: Vec<(String, Vec<Value>)> = Vec::new();
        for item in &self.cash {
            let row = json!({"id": item.id, "name": item.name, "category": item.category, "sub_category": null,
                             "price": 1000, "count": 1, "period": 90, "on_sale": true,
                             "description": "A made-up Cash item.", "thumbnail": format!("images/items/{:08}.png", item.id)});
            match categories.iter_mut().find(|(name, _)| *name == item.category) {
                Some((_, items)) => items.push(row),
                None => categories.push((item.category.clone(), vec![row])),
            }
        }
        let categories: Vec<Value> =
            categories.into_iter().map(|(category, items)| json!({"category": category, "items": items})).collect();
        json!({"categories": categories, "total": self.cash.len()})
    }
}
