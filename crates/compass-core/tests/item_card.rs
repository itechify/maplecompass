//! The Item Card (ADR 0010): the player hovers an item in the game and presses
//! Ctrl+Shift+I. The shell captures the game's client area with the pointer, the Item
//! Tooltip's name is read, and the card shows at the pointer what the game's tooltip
//! doesn't: what a shop pays for the item, what players pay for it on the market, where it
//! comes from and what it's used for. Failures go to the Item Lookup alert.
//!
//! The readings here stand for what the Item Tooltip reader made of a capture. OSMS's item
//! files and BasilMarket's price list are made up in their shapes ([`common::items`]).
//!
//! Worked example, as in `item_lookup.rs`: the game's client area at (812,110), 1920x1200
//! physical px, on one 3440x1440 monitor; an expanded panel of 340x72, so at (2376,510); an
//! Item Card of 300x160. The pointer is at (1000,580), with the Item Tooltip's name line
//! read at (1000,600).

mod common;

use common::{FakeSource, ItemSetAnswer};
use common::items::{
    ItemWorld, Price, cash, equip, etc, item_world, npc_priced, price_list, quest, recipe, scroll, setup, sold,
    sold_in_shops, unpriced, use_item, weapon,
};
use compass_core::{
    Compass, ExpBarRegion, Foreground, GameWindow, Hotkey, ItemCard, ItemHit, ItemCardChoice, ItemCardView, ItemLookup, ItemSource, ItemTooltip,
    ItemTooltipReading, ItemUse, Listed, Market, OurWindow, PanelSizes, QuestReward, PanelState, Rect, Request, ScreenPoint, ScreenRect, Settings, ShopPrice,
    Recognition, Size, SourceError, TitleRegion, meowdb,
};
use compass_core::osms_items::ItemFiles;
use std::sync::atomic::Ordering;
use std::task::Poll;

const GAME: ScreenRect = ScreenRect { x: 812, y: 110, width: 1920, height: 1200 };
const SCREEN: ScreenRect = ScreenRect { x: 0, y: 0, width: 3440, height: 1440 };
const CLIENT: Size = Size { width: 1920, height: 1200 };
/// Where the pointer is when the player presses the hotkey.
const POINTER: ScreenPoint = ScreenPoint { x: 1000, y: 580 };
/// Where the name line of the Item Tooltip read is on screen.
const NAME: ScreenRect = ScreenRect { x: 1000, y: 600, width: 200, height: 19 };
/// The Item Card's size.
const CARD: Size = Size { width: 300, height: 160 };

fn settings() -> Settings {
    Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client: CLIENT }),
        ..Settings::default()
    }
}

/// Made-up item data: a few items of each kind.
fn shop() -> ItemWorld {
    item_world(vec![
        use_item(2_000_000, "Red Potion").sells_for(25),
        etc(4_010_000, "Bronze Ore").sells_for(450),
        equip(1_002_067, "Blue Moon", "Overall").sells_for(11_000),
    ])
}

/// A core over the game window, its windows' sizes known, with `source`'s item data in.
fn started_with(source: FakeSource) -> Compass {
    let mut compass = Compass::new(source);
    compass.restore(settings());
    compass.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    compass.set_item_card_size(CARD);
    let frame = ScreenRect { x: GAME.x - 1, y: GAME.y - 31, width: GAME.width + 2, height: GAME.height + 32 };
    compass.on_game_window(Some(GameWindow { client: GAME, frame, screens: vec![SCREEN] }));
    compass.tick(0);
    compass
}

/// The made-up item data in, and BasilMarket can't be reached.
fn started() -> Compass {
    started_with(FakeSource::osms_offline().with_items(shop()))
}

fn found(name: &str, at_ms: u64) -> Option<ItemTooltipReading> {
    Some(ItemTooltipReading {
        tooltip: ItemTooltip::Found { area: NAME, name: Some(name.into()) },
        pointer: POINTER,
        at_ms,
    })
}

/// The Item Tooltip by the pointer read as `name`, after the player pressed Ctrl+Shift+I.
fn press_over(compass: &mut Compass, name: &str, at_ms: u64) -> PanelState {
    assert_eq!(compass.on_hotkey("ctrl+shift+i"), Some(Request::CaptureItemTooltip));
    compass.on_item_tooltip_reading(found(name, at_ms)).clone()
}

/// The card shown, if it is one card rather than a choice.
fn card(state: &PanelState) -> Option<ItemCard> {
    match state.item_card.as_ref().map(|shown| &shown.view) {
        Some(ItemCardView::Card(card)) => Some(card.clone()),
        _ => None,
    }
}

#[test]
fn ctrl_shift_i_over_an_item_shows_its_card_with_what_a_shop_pays_for_it() {
    let mut compass = started();

    let state = press_over(&mut compass, "Bronze Ore", 1_000);

    let card = card(&state).expect("a card");
    assert_eq!(card.name, "Bronze Ore");
    assert_eq!(card.shop, ShopPrice::Sells { mesos: 450 });
}

/// The game window's Title Region and EXP Bar Region, on screen: the core captures more
/// than was marked around each, wide enough for the longest name and figures.
const TITLE_REGION: ScreenRect = ScreenRect { x: 882, y: 150, width: 620, height: 48 };
const EXP_BAR_REGION: ScreenRect = ScreenRect { x: 1817, y: 1254, width: 384, height: 32 };

/// The Item Tooltip by `pointer` read as `name`, after the player pressed Ctrl+Shift+I.
fn press_at(compass: &mut Compass, pointer: ScreenPoint, name: &str) -> PanelState {
    assert_eq!(compass.on_hotkey("ctrl+shift+i"), Some(Request::CaptureItemTooltip));
    let tooltip = ItemTooltip::Found { area: ScreenRect { x: pointer.x, y: pointer.y + 20, ..NAME }, name: Some(name.into()) };
    compass.on_item_tooltip_reading(Some(ItemTooltipReading { tooltip, pointer, at_ms: 1_000 })).clone()
}

#[test]
fn the_card_shows_with_its_corner_just_below_and_right_of_the_pointer() {
    let mut compass = started();

    let state = press_over(&mut compass, "Bronze Ore", 1_000);

    assert_eq!(state.item_card_placement, Some(ScreenRect { x: 1008, y: 588, width: 300, height: 160 }));
}

#[test]
fn by_the_game_windows_right_edge_the_card_goes_left_of_the_pointer() {
    let mut compass = started();

    let state = press_at(&mut compass, ScreenPoint { x: 2600, y: 580 }, "Bronze Ore");

    assert_eq!(state.item_card_placement, Some(ScreenRect { x: 2292, y: 588, width: 300, height: 160 }));
}

#[test]
fn by_the_game_windows_bottom_edge_the_card_goes_above_the_pointer() {
    let mut compass = started();

    let state = press_at(&mut compass, ScreenPoint { x: 1000, y: 1200 }, "Bronze Ore");

    assert_eq!(state.item_card_placement, Some(ScreenRect { x: 1008, y: 1032, width: 300, height: 160 }));
}

#[test]
fn the_card_goes_to_the_pointers_other_side_rather_than_cover_the_exp_bar_region() {
    let mut compass = started();
    compass.restore(Settings {
        exp_bar_region: Some(ExpBarRegion { marked: Rect { x: 1005, y: 1144, width: 240, height: 32 }, client: CLIENT }),
        ..settings()
    });
    assert_eq!(compass.panel_state().exp_bar_region, Some(EXP_BAR_REGION));

    let state = press_at(&mut compass, ScreenPoint { x: 1800, y: 1100 }, "Bronze Ore");

    assert_eq!(state.item_card_placement, Some(ScreenRect { x: 1492, y: 1108, width: 300, height: 160 }));
}

#[test]
fn with_no_side_of_the_pointer_clear_of_the_minimap_title_the_card_moves_below_it() {
    let mut compass = started();
    assert_eq!(compass.panel_state().title_region, Some(TITLE_REGION));

    // Right of the pointer covers the title; left and above leave the game window.
    let state = press_at(&mut compass, ScreenPoint { x: 1000, y: 130 }, "Bronze Ore");

    assert_eq!(state.item_card_placement, Some(ScreenRect { x: 1008, y: 206, width: 300, height: 160 }));
}

#[test]
fn the_card_hides_while_another_app_is_in_front_and_shows_again_with_the_game() {
    let mut compass = started();
    compass.on_foreground(Foreground::Game);
    press_over(&mut compass, "Bronze Ore", 1_000);

    let state = compass.on_foreground(Foreground::OtherApp).clone();
    assert_eq!(state.item_card_placement, None);
    assert!(state.item_card.is_some(), "it is still the card the player opened");

    let state = compass.on_foreground(Foreground::Game).clone();
    assert_eq!(state.item_card_placement, Some(ScreenRect { x: 1008, y: 588, width: 300, height: 160 }));
}

#[test]
fn without_a_card_there_is_nowhere_to_place_one() {
    let compass = started();

    assert_eq!(compass.panel_state().item_card, None);
    assert_eq!(compass.panel_state().item_card_placement, None);
}

/// The card `name`'s Item Tooltip shows, with `items` in.
fn card_of(items: ItemWorld, name: &str) -> ItemCard {
    let mut compass = started_with(FakeSource::osms_offline().with_items(items));
    card(&press_over(&mut compass, name, 1_000)).unwrap_or_else(|| panic!("no card for {name}"))
}

#[test]
fn equipment_says_what_a_shop_pays_for_it_too() {
    assert_eq!(card_of(shop(), "Blue Moon").shop, ShopPrice::Sells { mesos: 11_000 });
}

#[test]
fn a_price_osms_writes_as_text_is_still_a_price() {
    let items = shop().with(equip(1_002_107, "Pink Jester", "Hat").sells_for_text("1800"));

    assert_eq!(card_of(items, "Pink Jester").shop, ShopPrice::Sells { mesos: 1_800 });
}

#[test]
fn an_item_a_shop_wont_buy_says_so_not_that_it_pays_1_meso() {
    let items = shop().with(etc(4_031_013, "Dark Marble").not_for_sale());

    assert_eq!(card_of(items, "Dark Marble").shop, ShopPrice::NotSold);
}

#[test]
fn a_cash_item_says_it_is_one() {
    let items = shop().with_cash(cash(5_120_000, "Snowy Snow", "Weather Effect"));

    assert_eq!(card_of(items, "Snowy Snow").shop, ShopPrice::Cash);
}

#[test]
fn a_scroll_or_an_item_osms_gives_no_price_says_no_shop_price_is_known() {
    let items = shop()
        .with_scroll(scroll(2_040_000, "Hat Accuracy Scroll: Lesser"))
        .with(equip(1_040_002, "White Undershirt", "Top"));

    assert_eq!(card_of(items.clone(), "Hat Accuracy Scroll: Lesser").shop, ShopPrice::Unknown);
    assert_eq!(card_of(items, "White Undershirt").shop, ShopPrice::Unknown);
}

#[test]
fn throwing_stars_say_no_shop_price_is_known_until_a_sale_in_the_game_confirms_what_they_pay() {
    let items = shop().with(use_item(2_070_000, "Subi Throwing Stars").sells_for(250).recharged_at(0.3));

    assert_eq!(card_of(items, "Subi Throwing Stars").shop, ShopPrice::Unknown);
}

/// What the alert says, and for which hotkey.
fn alert(state: &PanelState) -> Option<(ItemLookup, Hotkey)> {
    state.item_lookup.as_ref().map(|line| (line.outcome.clone(), line.hotkey))
}

fn card_alert(outcome: ItemLookup) -> Option<(ItemLookup, Hotkey)> {
    Some((outcome, Hotkey::ItemCard))
}

/// Ores close in name to one another.
fn ores() -> ItemWorld {
    shop()
        .with(etc(4_010_001, "Steel Ore").sells_for(500))
        .with(etc(4_010_002, "Steel Orb").sells_for(600))
        .with(etc(4_010_007, "Bronze Plate").sells_for(1_000))
}

#[test]
fn letters_the_recogniser_confuses_case_and_spacing_still_find_the_item() {
    for read in ["Br0nze 0re", "Bronze 0re", "bronze ore", "Bronze  Ore", " Bronze Ore "] {
        let mut compass = started();
        let state = press_over(&mut compass, read, 1_000);
        assert_eq!(card(&state).map(|card| card.name), Some("Bronze Ore".into()), "{read:?}");
    }
    let mut compass = started_with(FakeSource::osms_offline().with_items(shop().with(use_item(2_000_002, "White Potion"))));
    let state = press_over(&mut compass, "WhIte Potion", 1_000);
    assert_eq!(card(&state).map(|card| card.name), Some("White Potion".into()), "I for l");
}

#[test]
fn a_scrolled_items_upgrades_are_left_out_of_its_name() {
    let mut compass = started();

    let state = press_over(&mut compass, "Blue Moon (+3)", 1_000);

    assert_eq!(card(&state).map(|card| card.name), Some("Blue Moon".into()));
}

#[test]
fn a_name_one_letter_off_finds_the_item_far_closer_than_any_other() {
    let mut compass = started_with(FakeSource::osms_offline().with_items(ores()));

    let state = press_over(&mut compass, "Bronze Ora", 1_000);

    assert_eq!(card(&state).map(|card| card.name), Some("Bronze Ore".into()));
}

#[test]
fn a_name_as_close_to_two_items_finds_neither_and_the_alert_says_no_item_has_it() {
    let mut compass = started_with(FakeSource::osms_offline().with_items(ores()));

    let state = press_over(&mut compass, "Steel Orx", 1_000);

    assert_eq!(state.item_card, None);
    assert_eq!(alert(&state), card_alert(ItemLookup::NoSuchItem { read: "Steel Orx".into() }));
}

#[test]
fn a_name_no_item_has_shows_no_card_and_the_alert_says_so() {
    let mut compass = started();

    let state = press_over(&mut compass, "Zqxv Nothing", 1_000);

    assert_eq!(state.item_card, None);
    assert_eq!(alert(&state), card_alert(ItemLookup::NoSuchItem { read: "Zqxv Nothing".into() }));
}

#[test]
fn no_item_tooltip_by_the_pointer_says_so_in_the_alert() {
    let mut compass = started();
    compass.on_hotkey("ctrl+shift+i");

    let reading = ItemTooltipReading { tooltip: ItemTooltip::None, pointer: POINTER, at_ms: 1_000 };
    let state = compass.on_item_tooltip_reading(Some(reading)).clone();

    assert_eq!(state.item_card, None);
    assert_eq!(alert(&state), card_alert(ItemLookup::NoTooltip));
}

#[test]
fn a_name_that_cant_be_read_or_a_capture_that_failed_says_so_in_the_alert() {
    let mut compass = started();
    compass.on_hotkey("ctrl+shift+i");
    let tooltip = ItemTooltip::Found { area: NAME, name: None };
    let state = compass.on_item_tooltip_reading(Some(ItemTooltipReading { tooltip, pointer: POINTER, at_ms: 1_000 })).clone();
    assert_eq!(alert(&state), card_alert(ItemLookup::Unreadable));

    compass.on_hotkey("ctrl+shift+i");
    let state = compass.on_item_tooltip_reading(None).clone();
    assert_eq!(alert(&state), card_alert(ItemLookup::Unreadable));
    assert_eq!(state.item_card, None);
}

#[test]
fn the_panel_over_the_items_name_says_so_in_the_alert_as_it_would_be_read_instead() {
    let mut compass = started();
    let panel = compass.panel_state().placement.expect("the panel shows");
    compass.on_hotkey("ctrl+shift+i");

    let tooltip = ItemTooltip::Found { area: ScreenRect { x: panel.x - 50, y: panel.y + 10, ..NAME }, name: Some("Bronze Or".into()) };
    let state = compass.on_item_tooltip_reading(Some(ItemTooltipReading { tooltip, pointer: POINTER, at_ms: 1_000 })).clone();

    assert_eq!(state.item_card, None);
    assert_eq!(alert(&state), card_alert(ItemLookup::Covered { by: OurWindow::Panel }));
}

#[test]
fn a_press_while_the_last_capture_is_still_being_read_is_ignored_whichever_hotkey_asked() {
    let mut compass = started();
    compass.on_hotkey("ctrl+shift+i");

    assert_eq!(compass.on_hotkey("ctrl+shift+i"), None, "the card's capture is still being read");
    assert_eq!(compass.on_hotkey("ctrl+shift+o"), None, "Item Lookup waits for it too");

    compass.on_item_tooltip_reading(found("Bronze Ore", 1_000));
    assert_eq!(compass.on_hotkey("ctrl+shift+i"), Some(Request::CaptureItemTooltip));
}

#[test]
fn without_a_game_window_ctrl_shift_i_does_nothing() {
    let mut compass = Compass::new(FakeSource::osms_offline().with_items(shop()));

    assert_eq!(compass.on_hotkey("ctrl+shift+i"), None);
}

#[test]
fn the_card_stays_until_the_player_closes_it() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);

    let state = compass.tick(60 * 60_000).clone();
    assert_eq!(card(&state).map(|card| card.name), Some("Bronze Ore".into()), "still there an hour on");

    let state = compass.close_item_card().clone();
    assert_eq!(state.item_card, None);
    assert_eq!(state.item_card_placement, None);
}

#[test]
fn pressing_over_another_item_shows_its_card_in_place_of_the_last() {
    let mut compass = started();
    let first = press_over(&mut compass, "Bronze Ore", 1_000).item_card.expect("a card");

    let state = press_at(&mut compass, ScreenPoint { x: 1200, y: 700 }, "Red Potion");

    let second = state.item_card.as_ref().expect("a card");
    assert_eq!(card(&state).map(|card| card.name), Some("Red Potion".into()));
    assert_ne!(second.card, first.card, "a new card");
    assert_eq!(state.item_card_placement, Some(ScreenRect { x: 1208, y: 708, width: 300, height: 160 }));
}

#[test]
fn pressing_the_same_item_again_shows_its_card_afresh() {
    let mut compass = started();
    let first = press_over(&mut compass, "Bronze Ore", 1_000).item_card.expect("a card");

    let second = press_over(&mut compass, "Bronze Ore", 2_000).item_card.expect("a card");

    assert_eq!(second.view, first.view);
    assert_ne!(second.card, first.card, "a new card, which the window shows afresh");
}

#[test]
fn pressing_away_from_any_item_closes_the_card_and_says_nothing() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);
    compass.on_hotkey("ctrl+shift+i");

    let reading = ItemTooltipReading { tooltip: ItemTooltip::None, pointer: POINTER, at_ms: 2_000 };
    let state = compass.on_item_tooltip_reading(Some(reading)).clone();

    assert_eq!(state.item_card, None);
    assert_eq!(state.item_lookup, None, "no alert: the press closed the card");
}

#[test]
fn a_press_over_an_item_without_a_card_closes_the_last_card_and_says_why_in_the_alert() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);

    let state = press_over(&mut compass, "Zqxv Nothing", 2_000);

    assert_eq!(state.item_card, None);
    assert_eq!(alert(&state), card_alert(ItemLookup::NoSuchItem { read: "Zqxv Nothing".into() }));
}

#[test]
fn a_card_showing_takes_the_place_of_the_alert() {
    let mut compass = started();
    press_over(&mut compass, "Zqxv Nothing", 1_000);

    let state = press_over(&mut compass, "Bronze Ore", 2_000);

    assert!(state.item_card.is_some());
    assert_eq!(state.item_lookup, None);
}

#[test]
fn the_card_hides_while_the_next_press_is_read_so_it_is_never_read_in_place_of_the_items_name() {
    let mut compass = started();
    let placed = press_over(&mut compass, "Bronze Ore", 1_000).item_card_placement.expect("the card shows");

    assert_eq!(compass.on_hotkey("ctrl+shift+i"), Some(Request::CaptureItemTooltip));
    assert_eq!(compass.panel_state().item_card_placement, None, "hidden for the capture");

    // The next tooltip's name lies where the card was.
    let tooltip = ItemTooltip::Found { area: ScreenRect { x: placed.x + 10, y: placed.y + 10, ..NAME }, name: Some("Red Potion".into()) };
    let state = compass.on_item_tooltip_reading(Some(ItemTooltipReading { tooltip, pointer: POINTER, at_ms: 2_000 })).clone();

    assert_eq!(card(&state).map(|card| card.name), Some("Red Potion".into()));
}

/// Items that share a name: two Blue Moons that differ, and two Red Potions that don't.
fn namesakes() -> ItemWorld {
    shop()
        .with(equip(1_032_001, "Blue Moon", "Earring").sells_for(10_500))
        .with(use_item(2_000_007, "Red Potion").sells_for(25))
        .with(weapon(1_302_010, "Hero's Gladius", "1H Sword").sells_for(11_500))
        .with(etc(4_031_002, "Hero's Gladius"))
}

fn choice(state: &PanelState) -> Option<(String, Vec<ItemCardChoice>)> {
    match state.item_card.as_ref().map(|shown| &shown.view) {
        Some(ItemCardView::Choose { name, items }) => Some((name.clone(), items.clone())),
        _ => None,
    }
}

fn kinds(items: &[ItemCardChoice]) -> Vec<(&str, &str)> {
    items.iter().map(|item| (item.id.as_str(), item.kind.as_str())).collect()
}

#[test]
fn items_sharing_a_name_whose_cards_differ_are_listed_by_kind_for_the_player_to_pick() {
    let mut compass = started_with(FakeSource::osms_offline().with_items(namesakes()));

    let state = press_over(&mut compass, "Blue Moon", 1_000);

    let (name, items) = choice(&state).expect("a choice");
    assert_eq!(name, "Blue Moon");
    assert_eq!(kinds(&items), [("01002067", "Overall"), ("01032001", "Earring")]);
    assert!(state.item_card_placement.is_some(), "the choice shows at the pointer, as the card would");
}

#[test]
fn picking_one_shows_its_card() {
    let mut compass = started_with(FakeSource::osms_offline().with_items(namesakes()));
    let listed = press_over(&mut compass, "Blue Moon", 1_000).item_card.expect("a choice");

    let state = compass.pick_card_item("01032001").clone();

    let card = card(&state).expect("a card");
    assert_eq!((card.id.as_str(), card.shop), ("01032001", ShopPrice::Sells { mesos: 10_500 }));
    assert_eq!(state.item_card.as_ref().map(|shown| shown.card), Some(listed.card), "the same card, settled");
}

#[test]
fn picking_an_item_the_card_doesnt_list_changes_nothing() {
    let mut compass = started_with(FakeSource::osms_offline().with_items(namesakes()));
    let listed = press_over(&mut compass, "Blue Moon", 1_000).item_card;

    let state = compass.pick_card_item("02000000").clone();

    assert_eq!(state.item_card, listed);
}

#[test]
fn items_sharing_a_name_whose_cards_would_be_the_same_show_the_card_without_asking() {
    let mut compass = started_with(FakeSource::osms_offline().with_items(namesakes()));

    let state = press_over(&mut compass, "Red Potion", 1_000);

    assert_eq!(card(&state).map(|card| card.shop), Some(ShopPrice::Sells { mesos: 25 }));
}

#[test]
fn weapons_are_listed_by_their_kind_of_weapon_and_the_rest_by_their_inventory_tab() {
    let items = namesakes()
        .with(use_item(2_000_100, "Hero's Gladius").sells_for(1))
        .with(setup(3_010_000, "Hero's Gladius").sells_for(1))
        .with_scroll(scroll(2_040_100, "Hero's Gladius"))
        .with_cash(cash(5_000_100, "Hero's Gladius", "Pet"));
    let mut compass = started_with(FakeSource::osms_offline().with_items(items));

    let state = press_over(&mut compass, "Hero's Gladius", 1_000);

    let (_, items) = choice(&state).expect("a choice");
    assert_eq!(
        kinds(&items),
        [
            ("01302010", "One-Handed Sword"),
            ("04031002", "Etc"),
            ("02000100", "Use"),
            ("03010000", "Setup"),
            ("02040100", "Scroll"),
            ("05000100", "Cash"),
        ]
    );
}

#[test]
fn each_listed_item_shows_its_picture_once_it_is_in() {
    let source = FakeSource::osms_offline().with_items(namesakes());
    let asked = source.picture_requests.clone();
    let mut compass = started_with(source);
    press_over(&mut compass, "Blue Moon", 1_000);

    let state = compass.tick(1_100).clone();

    let (_, items) = choice(&state).expect("a choice");
    let icons: Vec<Option<&str>> = items.iter().map(|item| item.icon.as_deref()).collect();
    assert_eq!(
        icons,
        [Some("picture:current/images/items/01002067.png"), Some("picture:current/images/items/01032001.png")]
    );
    assert_eq!(*asked.lock().unwrap(), ["current/images/items/01002067.png", "current/images/items/01032001.png"]);
}

/// Quests and recipes that give and need Bronze Ore and Bronze Plate.
fn bronze_work() -> ItemWorld {
    shop()
        .with(etc(4_011_000, "Bronze Plate").sells_for(1_000))
        .with(equip(1_002_001, "Bronze Koif", "Hat").sells_for(2_000))
        .with_quest(quest(1_001, "Ore for the Smith").gives(4_010_000, "Bronze Ore", 5))
        .with_quest(quest(1_002, "A Lucky Find").gives_at_random(4_010_000, "Bronze Ore", 7.14))
        .with_quest(quest(1_003, "Pick a Reward").gives_as_a_pick(4_010_000, "Bronze Ore"))
        .with_quest(quest(1_004, "Plates for the Guard").needs(4_011_000, "Bronze Plate", 15))
        .with_recipe(recipe("Smithing", 2, 4_011_000, "Bronze Plate").from("Bronze Ore", 10))
        .with_recipe(recipe("Smithing", 3, 1_002_001, "Bronze Koif").from("Bronze Plate", 2).from("Red Potion", 1))
}

#[test]
fn the_card_says_which_quests_give_the_item_always_at_random_or_as_a_pick() {
    let card = card_of(bronze_work(), "Bronze Ore");

    assert_eq!(
        card.comes_from,
        Listed {
            shown: vec![
                ItemSource::Quest { name: "Ore for the Smith".into(), reward: QuestReward::Always },
                ItemSource::Quest { name: "A Lucky Find".into(), reward: QuestReward::Chance { hundredths: 714 } },
                ItemSource::Quest { name: "Pick a Reward".into(), reward: QuestReward::Pick },
            ],
            more: 0,
        }
    );
}

#[test]
fn the_card_says_which_recipes_make_the_item_and_which_quests_and_recipes_need_it() {
    let card = card_of(bronze_work(), "Bronze Plate");

    assert_eq!(card.comes_from.shown, [ItemSource::Recipe { discipline: "Smithing".into(), level: 2 }]);
    assert_eq!(
        card.used_for.shown,
        [
            ItemUse::Quest { name: "Plates for the Guard".into(), count: 15 },
            ItemUse::Recipe { makes: "Bronze Koif".into(), count: 2 },
        ]
    );
}

#[test]
fn each_list_shows_three_then_how_many_more() {
    let mut items = shop();
    for (n, name) in ["One", "Two", "Three", "Four", "Five"].into_iter().enumerate() {
        items = items.with_quest(quest(2_000 + n as u64, &format!("Ores, Part {name}")).needs(4_010_000, "Bronze Ore", 10));
    }

    let card = card_of(items, "Bronze Ore");

    let names: Vec<&str> = card
        .used_for
        .shown
        .iter()
        .map(|used| match used {
            ItemUse::Quest { name, .. } => name.as_str(),
            ItemUse::Recipe { makes, .. } => makes.as_str(),
        })
        .collect();
    assert_eq!(names, ["Ores, Part One", "Ores, Part Two", "Ores, Part Three"]);
    assert_eq!(card.used_for.more, 2);
}

#[test]
fn a_quest_that_gives_the_item_twice_over_is_listed_once() {
    let items = shop().with_quest(quest(1_001, "Ore for the Smith").gives(4_010_000, "Bronze Ore", 5).gives(4_010_000, "Bronze Ore", 5));

    let card = card_of(items, "Bronze Ore");

    assert_eq!(card.comes_from.shown, [ItemSource::Quest { name: "Ore for the Smith".into(), reward: QuestReward::Always }]);
}

#[test]
fn an_item_no_quest_or_recipe_gives_or_needs_has_neither_list() {
    let card = card_of(bronze_work(), "Blue Moon");

    assert_eq!(card.comes_from, Listed { shown: vec![], more: 0 });
    assert_eq!(card.used_for, Listed { shown: vec![], more: 0 });
}

/// When a saved copy was made: 2026-10-01 00:00 UTC, in ms since the Unix epoch.
const SAVED_AT: u64 = 1_790_812_800_000;
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// Item files that don't read.
fn changed_format() -> ItemSetAnswer {
    Poll::Ready(Ok(ItemFiles { items: "<html>".into(), ..shop().files() }))
}

/// The made-up item data, but Bronze Ore sells for `mesos`.
fn bronze_ore_at(mesos: u64) -> ItemWorld {
    let mut items = shop();
    items.items.retain(|item| item.name != "Bronze Ore");
    items.with(etc(4_010_000, "Bronze Ore").sells_for(mesos))
}

/// As [`started_with`], with the wall clock at `now_unix_ms` from the start.
fn started_at(source: FakeSource, now_unix_ms: u64) -> Compass {
    let mut compass = Compass::new(source);
    compass.on_wall_clock(now_unix_ms);
    compass.restore(settings());
    compass.set_item_card_size(CARD);
    let frame = ScreenRect { x: GAME.x - 1, y: GAME.y - 31, width: GAME.width + 2, height: GAME.height + 32 };
    compass.on_game_window(Some(GameWindow { client: GAME, frame, screens: vec![SCREEN] }));
    compass.tick(0);
    compass
}

fn bronze_ore_price(compass: &mut Compass, at_ms: u64) -> Option<ShopPrice> {
    card(&press_over(compass, "Bronze Ore", at_ms)).map(|card| card.shop)
}

#[test]
fn until_osmss_item_files_arrive_the_alert_says_they_are_loading() {
    let mut compass = started_with(FakeSource::osms_offline().answering_item_set(vec![Poll::Pending]));

    let state = press_over(&mut compass, "Bronze Ore", 1_000);

    assert_eq!(state.item_card, None);
    assert_eq!(alert(&state), card_alert(ItemLookup::ItemsLoading));
}

#[test]
fn with_osms_unreachable_and_no_item_files_saved_the_alert_says_so_and_they_are_asked_for_a_minute_on() {
    let source = FakeSource::osms_offline();
    let asked = source.item_set_requests.clone();
    let mut compass = started_with(source);
    assert_eq!(asked.load(Ordering::SeqCst), 1);

    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    assert_eq!(alert(&state), card_alert(ItemLookup::ItemsUnavailable));

    compass.tick(59_000);
    assert_eq!(asked.load(Ordering::SeqCst), 1, "not on every tick");
    compass.tick(60_000);
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}

#[test]
fn item_files_that_dont_read_with_none_saved_say_the_card_needs_an_update() {
    let mut compass = started_with(FakeSource::osms_offline().answering_item_set(vec![changed_format()]));

    let state = press_over(&mut compass, "Bronze Ore", 1_000);

    assert_eq!(alert(&state), card_alert(ItemLookup::ItemsFormatChanged));
}

#[test]
fn item_files_that_dont_read_leave_the_map_set_as_it_is() {
    let mut compass = started_with(FakeSource::osms().answering_item_set(vec![changed_format()]));

    assert_eq!(compass.tick(1_000).recognition, Recognition::Ready);
}

#[test]
fn item_files_saved_under_a_day_ago_are_used_at_launch_without_fetching_them() {
    let source = FakeSource::osms_offline().with_items(bronze_ore_at(500)).with_saved_items(bronze_ore_at(450), SAVED_AT);
    let asked = source.item_set_requests.clone();
    let mut compass = started_at(source, SAVED_AT + DAY_MS - 60_000);

    assert_eq!(bronze_ore_price(&mut compass, 1_000), Some(ShopPrice::Sells { mesos: 450 }));
    assert_eq!(asked.load(Ordering::SeqCst), 0);
}

#[test]
fn item_files_saved_over_a_day_ago_are_fetched_afresh_at_launch() {
    let source = FakeSource::osms_offline().with_items(bronze_ore_at(500)).with_saved_items(bronze_ore_at(450), SAVED_AT);
    let mut compass = started_at(source, SAVED_AT + DAY_MS + 60_000);

    assert_eq!(bronze_ore_price(&mut compass, 1_000), Some(ShopPrice::Sells { mesos: 500 }));
}

#[test]
fn with_osms_unreachable_the_item_files_saved_are_used_however_old() {
    let source = FakeSource::osms_offline().with_saved_items(bronze_ore_at(450), SAVED_AT);
    let mut compass = started_at(source, SAVED_AT + 30 * DAY_MS);

    assert_eq!(bronze_ore_price(&mut compass, 1_000), Some(ShopPrice::Sells { mesos: 450 }));
}

#[test]
fn item_files_are_fetched_afresh_every_day_and_swapped_in_once_they_read() {
    let answers = vec![Poll::Ready(Ok(bronze_ore_at(450).files())), changed_format(), Poll::Ready(Ok(bronze_ore_at(500).files()))];
    let source = FakeSource::osms_offline().answering_item_set(answers);
    let asked = source.item_set_requests.clone();
    let mut compass = started_at(source, SAVED_AT);

    compass.tick(DAY_MS - 1_000);
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    compass.tick(DAY_MS);
    assert_eq!(asked.load(Ordering::SeqCst), 2);
    assert_eq!(bronze_ore_price(&mut compass, DAY_MS + 1_000), Some(ShopPrice::Sells { mesos: 450 }), "kept: the new ones didn't read");

    compass.tick(2 * DAY_MS);
    assert_eq!(bronze_ore_price(&mut compass, 2 * DAY_MS + 1_000), Some(ShopPrice::Sells { mesos: 500 }));
}

#[test]
fn a_card_from_item_files_saved_over_7_days_ago_says_it_is_a_saved_copy_from_then() {
    let source = FakeSource::osms_offline().with_saved_items(shop(), SAVED_AT);
    let mut compass = started_at(source, SAVED_AT + 7 * DAY_MS + 60_000);

    let card = card(&press_over(&mut compass, "Bronze Ore", 1_000)).expect("a card");

    assert_eq!(card.saved_copy_from_unix_ms, Some(SAVED_AT));
}

#[test]
fn a_card_from_item_files_saved_under_7_days_ago_says_nothing_of_it() {
    let source = FakeSource::osms_offline().with_saved_items(shop(), SAVED_AT);
    let mut compass = started_at(source, SAVED_AT + 7 * DAY_MS - 60_000);

    let card = card(&press_over(&mut compass, "Bronze Ore", 1_000)).expect("a card");

    assert_eq!(card.saved_copy_from_unix_ms, None);
}

const HOUR_MS: u64 = 60 * 60 * 1000;

/// BasilMarket's made-up prices for the made-up items.
fn market() -> Vec<Price> {
    vec![
        sold(4_010_000, "Bronze Ore", 520, 12),
        sold_in_shops(2_000_000, "Red Potion", 40, 3),
        npc_priced(1_002_067, "Blue Moon", 11_000),
        unpriced(4_010_001, "Steel Ore"),
    ]
}

fn page(name: &str) -> Option<String> {
    Some(format!("https://basilmarket.net/price/{}", common::items::slug(name)))
}

fn sold_at(mesos: u64, sales: u32, name: &str) -> Market {
    Market::Sold { mesos, sales, page: page(name).unwrap(), from_unix_ms: None }
}

/// The made-up item data and BasilMarket's made-up prices in, with the wall clock at
/// [`SAVED_AT`].
fn with_market(source: FakeSource) -> Compass {
    started_at(source.with_items(namesakes().with(etc(4_010_001, "Steel Ore").sells_for(500))), SAVED_AT)
}

fn market_of(compass: &mut Compass, name: &str, at_ms: u64) -> Option<Market> {
    card(&press_over(compass, name, at_ms)).map(|card| card.market)
}

#[test]
fn the_card_says_what_players_paid_for_the_item_and_from_how_many_sales() {
    let mut compass = with_market(FakeSource::osms_offline().with_prices(&market()));

    assert_eq!(market_of(&mut compass, "Bronze Ore", 1_000), Some(sold_at(520, 12, "Bronze Ore")));
}

#[test]
fn a_price_from_sales_seen_in_free_market_shops_counts_too() {
    let mut compass = with_market(FakeSource::osms_offline().with_prices(&market()));

    assert_eq!(market_of(&mut compass, "Red Potion", 1_000), Some(sold_at(40, 3, "Red Potion")));
}

#[test]
fn without_sales_the_card_says_none_were_seen_never_the_shop_price_basilmarket_shows_instead() {
    let mut compass = with_market(FakeSource::osms_offline().with_prices(&market()));

    let unpriced = market_of(&mut compass, "Steel Ore", 1_000);
    assert_eq!(unpriced, Some(Market::NoSales { page: page("Steel Ore"), from_unix_ms: None }));

    press_over(&mut compass, "Blue Moon", 2_000);
    let npc_priced = compass.pick_card_item("01002067").clone();
    assert_eq!(card(&npc_priced).map(|card| card.market), Some(Market::NoSales { page: page("Blue Moon"), from_unix_ms: None }));
}

#[test]
fn an_item_basilmarket_doesnt_list_has_no_sales_and_no_page() {
    let mut compass = with_market(FakeSource::osms_offline().with_prices(&market()));

    assert_eq!(market_of(&mut compass, "Hero's Gladius", 1_000), None, "a choice");
    let state = compass.pick_card_item("01302010").clone();

    assert_eq!(card(&state).map(|card| card.market), Some(Market::NoSales { page: None, from_unix_ms: None }));
}

#[test]
fn prices_are_matched_by_the_games_item_id_so_one_never_lands_on_an_item_sharing_the_name() {
    // BasilMarket lists one Blue Moon, under the Overall's ID.
    let prices = vec![sold(1_002_067, "Blue Moon", 30_000, 4)];
    let mut compass = with_market(FakeSource::osms_offline().with_prices(&prices));
    press_over(&mut compass, "Blue Moon", 1_000);

    let earring = compass.pick_card_item("01032001").clone();
    assert_eq!(card(&earring).map(|card| card.market), Some(Market::NoSales { page: None, from_unix_ms: None }));

    press_over(&mut compass, "Blue Moon", 2_000);
    let overall = compass.pick_card_item("01002067").clone();
    assert_eq!(card(&overall).map(|card| card.market), Some(sold_at(30_000, 4, "Blue Moon")));
}

#[test]
fn items_sharing_a_name_and_a_card_take_the_price_basilmarket_lists_under_either() {
    let prices = vec![sold(2_000_007, "Red Potion", 45, 2)];
    let mut compass = with_market(FakeSource::osms_offline().with_prices(&prices));

    assert_eq!(market_of(&mut compass, "Red Potion", 1_000), Some(sold_at(45, 2, "Red Potion")));
}

#[test]
fn basilmarket_is_asked_only_once_a_card_opens() {
    let source = FakeSource::osms_offline().with_prices(&market());
    let asked = source.price_requests.clone();
    let mut compass = with_market(source);
    compass.tick(10 * HOUR_MS);
    assert_eq!(asked.load(Ordering::SeqCst), 0);

    press_over(&mut compass, "Bronze Ore", 10 * HOUR_MS + 1_000);

    assert_eq!(asked.load(Ordering::SeqCst), 1);
}

#[test]
fn the_price_list_is_asked_for_again_only_once_it_is_over_3_hours_old() {
    let source = FakeSource::osms_offline().with_prices(&market());
    let asked = source.price_requests.clone();
    let mut compass = with_market(source);
    press_over(&mut compass, "Bronze Ore", 1_000);

    compass.on_wall_clock(SAVED_AT + 3 * HOUR_MS - 60_000);
    press_over(&mut compass, "Red Potion", 2_000);
    assert_eq!(asked.load(Ordering::SeqCst), 1);

    compass.on_wall_clock(SAVED_AT + 3 * HOUR_MS + 60_000);
    press_over(&mut compass, "Red Potion", 3_000);
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}

#[test]
fn while_the_price_list_is_fetched_with_none_saved_the_card_says_the_price_is_loading() {
    let answers = vec![Poll::Pending, Poll::Ready(Ok(price_list(&market())))];
    let mut compass = with_market(FakeSource::osms_offline().answering_prices(answers));

    assert_eq!(market_of(&mut compass, "Bronze Ore", 1_000), Some(Market::Loading));

    let state = compass.tick(1_100).clone();
    assert_eq!(card(&state).map(|card| card.market), Some(sold_at(520, 12, "Bronze Ore")));
}

#[test]
fn a_price_list_saved_under_3_hours_ago_is_used_without_asking() {
    let source = FakeSource::osms_offline().with_saved_prices(&market(), SAVED_AT - 2 * HOUR_MS);
    let asked = source.price_requests.clone();
    let mut compass = with_market(source);

    assert_eq!(market_of(&mut compass, "Bronze Ore", 1_000), Some(sold_at(520, 12, "Bronze Ore")));
    assert_eq!(asked.load(Ordering::SeqCst), 0);
}

#[test]
fn an_older_saved_price_list_shows_while_a_fresh_one_is_fetched() {
    let fresh = vec![sold(4_010_000, "Bronze Ore", 600, 13)];
    let answers = vec![Poll::Pending, Poll::Ready(Ok(price_list(&fresh)))];
    let source = FakeSource::osms_offline().with_saved_prices(&market(), SAVED_AT - 5 * HOUR_MS).answering_prices(answers);
    let mut compass = with_market(source);

    assert_eq!(market_of(&mut compass, "Bronze Ore", 1_000), Some(sold_at(520, 12, "Bronze Ore")));

    let state = compass.tick(1_100).clone();
    assert_eq!(card(&state).map(|card| card.market), Some(sold_at(600, 13, "Bronze Ore")));
}

#[test]
fn prices_over_a_day_old_that_cant_be_refreshed_say_when_they_are_from() {
    let saved_at = SAVED_AT - 2 * DAY_MS;
    let mut compass = with_market(FakeSource::osms_offline().with_saved_prices(&market(), saved_at));

    let market = market_of(&mut compass, "Bronze Ore", 1_000);

    assert_eq!(
        market,
        Some(Market::Sold { mesos: 520, sales: 12, page: page("Bronze Ore").unwrap(), from_unix_ms: Some(saved_at) })
    );
}

#[test]
fn with_basilmarket_unreachable_and_no_price_list_saved_the_market_price_is_unavailable() {
    let mut compass = with_market(FakeSource::osms_offline());

    assert_eq!(market_of(&mut compass, "Bronze Ore", 1_000), Some(Market::Unavailable));
}

#[test]
fn a_price_list_that_doesnt_read_keeps_the_one_held() {
    let answers = vec![Poll::Ready(Ok(price_list(&market()))), Poll::Ready(Ok("<html>".into()))];
    let mut compass = with_market(FakeSource::osms_offline().answering_prices(answers));
    press_over(&mut compass, "Bronze Ore", 1_000);
    compass.on_wall_clock(SAVED_AT + 4 * HOUR_MS);

    assert_eq!(market_of(&mut compass, "Bronze Ore", 2_000), Some(sold_at(520, 12, "Bronze Ore")));
}

#[test]
fn after_basilmarket_couldnt_be_reached_it_is_asked_again_no_sooner_than_a_minute_on() {
    let source = FakeSource::osms_offline();
    let asked = source.price_requests.clone();
    let mut compass = with_market(source);
    press_over(&mut compass, "Bronze Ore", 1_000);

    press_over(&mut compass, "Bronze Ore", 30_000);
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    press_over(&mut compass, "Bronze Ore", 61_000);
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}

#[test]
fn the_card_credits_osms_for_its_data_and_basilmarket_for_its_prices() {
    let card = card_of(shop(), "Bronze Ore");

    assert_eq!(card.credit, "Data: OSMS Data Explorer · Prices: BasilMarket.net");
}

/// MeowDB's search, made up in its shape: Bronze Ore, and the two Blue Moons.
fn made_up_search(query: &str) -> Result<Vec<ItemHit>, SourceError> {
    let answer = match query {
        "Bronze Ore" => r#"{"items":[{"id":105,"name":"Bronze Ore","sub":"Ore"},{"id":106,"name":"Bronze Plate","sub":"Ore"}]}"#,
        "Blue Moon" => {
            r#"{"items":[{"id":901,"name":"Blue Moon","sub":"Overall"},{"id":902,"name":"Blue Moon","sub":"Earring"}]}"#
        }
        "Red Potion" => {
            r#"{"items":[{"id":801,"name":"Red Potion","sub":"Potion"},{"id":802,"name":"Red Potion","sub":"Potion"}]}"#
        }
        _ => return Err(SourceError::Unreachable(format!("no made-up answer for {query:?}"))),
    };
    meowdb::read_item_search(answer)
}

/// Everything the shell has been asked to do since it last asked.
fn requests(compass: &mut Compass) -> Vec<Request> {
    std::iter::from_fn(|| compass.take_request()).collect()
}

fn with_meowdb() -> Compass {
    started_with(FakeSource::osms_offline().with_items(namesakes()).with_item_searches(made_up_search))
}

#[test]
fn clicking_the_cards_name_opens_the_items_meowdb_page_through_meowdbs_search() {
    let mut compass = with_meowdb();
    press_over(&mut compass, "Bronze Ore", 1_000);

    compass.open_card_meowdb_page(2_000);

    assert_eq!(requests(&mut compass), [Request::OpenInBrowser { url: meowdb::item_url("105") }]);
}

#[test]
fn of_meowdb_items_sharing_the_name_the_one_of_the_cards_kind_opens() {
    let mut compass = with_meowdb();
    press_over(&mut compass, "Blue Moon", 1_000);
    compass.pick_card_item("01032001");

    compass.open_card_meowdb_page(2_000);

    assert_eq!(requests(&mut compass), [Request::OpenInBrowser { url: meowdb::item_url("902") }]);
}

#[test]
fn of_meowdb_items_sharing_the_name_none_of_the_cards_kind_are_listed_in_the_alert_to_pick() {
    let mut compass = with_meowdb();
    press_over(&mut compass, "Red Potion", 1_000);

    let state = compass.open_card_meowdb_page(2_000).clone();

    assert!(requests(&mut compass).is_empty());
    assert!(matches!(alert(&state), Some((ItemLookup::Choose { .. }, Hotkey::ItemLookup))), "{:?}", state.item_lookup);
}

#[test]
fn clicking_the_market_line_opens_the_items_basilmarket_page() {
    let mut compass = with_market(FakeSource::osms_offline().with_prices(&market()));
    press_over(&mut compass, "Bronze Ore", 1_000);

    assert_eq!(
        compass.open_card_market_page(),
        Some(Request::OpenInBrowser { url: page("Bronze Ore").unwrap() })
    );
}

#[test]
fn without_a_page_on_basilmarket_or_a_card_the_market_line_opens_nothing() {
    let mut compass = with_market(FakeSource::osms_offline().with_prices(&market()));
    assert_eq!(compass.open_card_market_page(), None, "no card");

    press_over(&mut compass, "Hero's Gladius", 1_000);
    compass.pick_card_item("01302010");
    assert_eq!(compass.open_card_market_page(), None, "not on BasilMarket");
}

#[test]
fn items_sharing_a_name_kind_and_shop_price_look_alike_so_show_one_card_with_the_quests_and_recipes_of_each() {
    let items = namesakes()
        .with_quest(quest(1_001, "Potions for the Road").gives(2_000_000, "Red Potion", 10))
        .with_quest(quest(1_002, "A Potion for Olaf").needs(2_000_007, "Red Potion", 1));

    let card = card_of(items, "Red Potion");

    assert_eq!(card.comes_from.shown, [ItemSource::Quest { name: "Potions for the Road".into(), reward: QuestReward::Always }]);
    assert_eq!(card.used_for.shown, [ItemUse::Quest { name: "A Potion for Olaf".into(), count: 1 }]);
}
