//! The Item Card (ADR 0010): the player hovers an item in the game and presses
//! Ctrl+Shift+I. The shell captures the game's client area with the pointer, the Item
//! Tooltip's name is read, and the card shows at the pointer what the game's tooltip
//! doesn't: what a shop pays for the item, and the market prices the player keeps for it on
//! their Market Lines (ADR 0011), with links to its MeowDB page and to MesoWatch's market
//! prices. Failures go to the Item Lookup alert.
//!
//! The readings here stand for what the Item Tooltip reader made of a capture. OSMS's item
//! files are made up in their shapes ([`common::items`]).
//!
//! Worked example, as in `item_lookup.rs`: the game's client area at (812,110), 1920x1200
//! physical px, on one 3440x1440 monitor; an expanded panel of 340x72, so at (2376,510); an
//! Item Card of 300x160. The pointer is at (1000,580), with the Item Tooltip's name line
//! read at (1000,600).

mod common;

use common::{FakeSource, ItemSetAnswer};
use common::items::{ItemWorld, cash, equip, etc, item_world, scroll, setup, use_item, weapon};
use compass_core::{
    Action, Compass, ExpBarRegion, Foreground, GameWindow, Hotkey, ItemCard, ItemHit, ItemCardChoice, ItemCardView, ItemLookup, ItemTooltip,
    ItemTooltipReading, MarketEdit, MarketPrice, NameProblem, OurWindow, PanelSizes, PanelState, Rect, Request, ScreenPoint, ScreenRect, Settings, ShopPrice,
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

/// The made-up item data in.
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
    assert_eq!(compass.on_hotkey("ctrl+shift+u"), None, "Item Lookup waits for it too");

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
fn the_cards_market_line_opens_mesowatch_for_the_player_to_look_the_item_up() {
    let mut compass = started();
    assert_eq!(compass.open_card_mesowatch(), None, "no card");

    press_over(&mut compass, "Bronze Ore", 1_000);

    assert_eq!(compass.open_card_mesowatch(), Some(Request::OpenInBrowser { url: "https://meso.watch/".into() }));
}

#[test]
fn the_card_credits_osms_for_its_data_and_the_sites_it_links_to() {
    let card = card_of(shop(), "Bronze Ore");

    assert_eq!(card.credit, "Data: OSMS Data Explorer · Pages: NiaMeowDB, MesoWatch");
}

fn overlaps(a: ScreenRect, b: ScreenRect) -> bool {
    a.x < b.x + b.width as i32 && b.x < a.x + a.width as i32 && a.y < b.y + b.height as i32 && b.y < a.y + a.height as i32
}

#[test]
fn a_card_dragged_by_its_bar_stays_where_the_player_left_it() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);

    compass.act(Action::ItemCardMoved { x: 1500, y: 300 });
    let state = compass.tick(60_000).clone();

    assert_eq!(state.item_card_placement, Some(ScreenRect { x: 1500, y: 300, width: 300, height: 160 }));
}

#[test]
fn a_card_dragged_over_the_minimap_title_moves_off_it() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);

    let state = compass.act(Action::ItemCardMoved { x: 1000, y: 140 }).clone();

    let placed = state.item_card_placement.expect("the card shows");
    assert!(!overlaps(placed, TITLE_REGION), "{placed:?}");
}

#[test]
fn a_dragged_card_moves_with_the_game_window() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);
    compass.act(Action::ItemCardMoved { x: 1500, y: 300 });

    let moved = ScreenRect { x: GAME.x + 100, ..GAME };
    let frame = ScreenRect { x: moved.x - 1, y: moved.y - 31, width: moved.width + 2, height: moved.height + 32 };
    let state = compass.on_game_window(Some(GameWindow { client: moved, frame, screens: vec![SCREEN] })).clone();

    assert_eq!(state.item_card_placement.map(|at| (at.x, at.y)), Some((1600, 300)));
}

#[test]
fn the_next_card_opens_at_the_pointer_again() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);
    compass.act(Action::ItemCardMoved { x: 1500, y: 300 });

    let state = press_over(&mut compass, "Red Potion", 2_000);

    assert_eq!(state.item_card_placement, Some(ScreenRect { x: 1008, y: 588, width: 300, height: 160 }));
}

/// The Market Lines on the card showing, each as its name and the mesos on it.
fn market(state: &PanelState) -> Vec<(String, Option<u64>)> {
    let card = card(state).expect("a card");
    card.market.into_iter().map(|line| (line.name, line.price.map(|price| price.mesos))).collect()
}

/// What the card's market section has open.
fn editing(state: &PanelState) -> Option<MarketEdit> {
    card(state).expect("a card").market_edit
}

/// The ID of the card's Market Line named `name`.
fn line(state: &PanelState, name: &str) -> String {
    let card = card(state).expect("a card");
    card.market.into_iter().find(|line| line.name == name).map(|line| line.id).unwrap_or_else(|| panic!("no line {name:?}"))
}

/// The player clicked the price on the card's line named `name` and typed `typed`, then
/// Enter.
fn set_price(compass: &mut Compass, state: &PanelState, name: &str, typed: &str) -> PanelState {
    compass.edit_card_market_price(&line(state, name));
    compass.enter_card_market(typed).clone()
}

/// The player clicked the + on the card's market heading and named the new line `name`.
fn add_line(compass: &mut Compass, name: &str) -> PanelState {
    compass.act(Action::AddCardMarketLine);
    compass.enter_card_market_line(name).clone()
}

fn not_set(names: &[&str]) -> Vec<(String, Option<u64>)> {
    names.iter().map(|name| (name.to_string(), None)).collect()
}

#[test]
fn a_card_starts_with_one_market_line_and_no_price_on_it() {
    let mut compass = started();

    let state = press_over(&mut compass, "Bronze Ore", 1_000);

    assert_eq!(market(&state), not_set(&["Your price"]));
    assert_eq!(editing(&state), None);
}

#[test]
fn clicking_a_lines_price_opens_its_field_for_the_player_to_type_in() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    let id = line(&state, "Your price");

    let state = compass.edit_card_market_price(&id).clone();

    assert_eq!(editing(&state), Some(MarketEdit::Price { line: id, not_a_price: None }));
}

#[test]
fn the_price_the_player_types_is_kept_with_the_day_they_set_it() {
    let mut compass = started_at(FakeSource::osms_offline().with_items(shop()), SAVED_AT);
    let state = press_over(&mut compass, "Bronze Ore", 1_000);

    let state = set_price(&mut compass, &state, "Your price", "450,000");

    let card = card(&state).expect("a card");
    assert_eq!(card.market[0].price, Some(MarketPrice { mesos: 450_000, set_at_unix_ms: Some(SAVED_AT) }));
    assert_eq!(card.market_edit, None);
}

#[test]
fn prices_are_typed_as_players_write_them() {
    for (typed, mesos) in [
        ("450000", 450_000),
        ("450k", 450_000),
        ("450K", 450_000),
        ("1.2m", 1_200_000),
        ("1.25M", 1_250_000),
        ("2b", 2_000_000_000),
        (" 1 200 000 mesos ", 1_200_000),
        ("999", 999),
        ("0", 0),
    ] {
        let mut compass = started();
        let state = press_over(&mut compass, "Bronze Ore", 1_000);
        let state = set_price(&mut compass, &state, "Your price", typed);
        assert_eq!(market(&state)[0].1, Some(mesos), "{typed:?}");
    }
}

#[test]
fn what_isnt_a_price_keeps_the_field_open_saying_so_and_the_price_as_it_was() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    let state = set_price(&mut compass, &state, "Your price", "450k");
    let id = line(&state, "Your price");

    for typed in ["lots", "4.5.6k", "12x", "k", "-5"] {
        let state = set_price(&mut compass, &state, "Your price", typed);
        assert_eq!(market(&state)[0].1, Some(450_000), "{typed:?}");
        assert_eq!(editing(&state), Some(MarketEdit::Price { line: id.clone(), not_a_price: Some(typed.into()) }), "{typed:?}");
    }
}

#[test]
fn an_empty_field_clears_the_price() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    let state = set_price(&mut compass, &state, "Your price", "450k");

    let state = set_price(&mut compass, &state, "Your price", "  ");

    assert_eq!(market(&state), not_set(&["Your price"]));
    assert_eq!(editing(&state), None);
}

#[test]
fn cancelling_closes_the_field_and_keeps_the_price_as_it_was() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    let state = set_price(&mut compass, &state, "Your price", "450k");
    compass.edit_card_market_price(&line(&state, "Your price"));

    let state = compass.act(Action::CancelCardMarket).clone();

    assert_eq!(market(&state), [("Your price".to_string(), Some(450_000))]);
    assert_eq!(editing(&state), None);
}

#[test]
fn each_item_keeps_its_own_price_and_shows_it_on_its_next_card() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    set_price(&mut compass, &state, "Your price", "450k");

    let other = press_over(&mut compass, "Red Potion", 2_000);
    assert_eq!(market(&other), not_set(&["Your price"]));

    let again = press_over(&mut compass, "Bronze Ore", 3_000);
    assert_eq!(market(&again), [("Your price".to_string(), Some(450_000))]);
}

#[test]
fn items_that_look_alike_keep_one_price() {
    let mut compass = started_with(FakeSource::osms_offline().with_items(namesakes()));
    let state = press_over(&mut compass, "Red Potion", 1_000);
    set_price(&mut compass, &state, "Your price", "40");

    let state = press_over(&mut compass, "Red Potion", 2_000);

    assert_eq!(market(&state), [("Your price".to_string(), Some(40))]);
}

#[test]
fn the_prices_and_lines_are_saved_and_restored_at_the_next_launch() {
    let mut compass = started_at(FakeSource::osms_offline().with_items(shop()), SAVED_AT);
    let before = compass.market_prices_revision();
    press_over(&mut compass, "Bronze Ore", 1_000);
    let state = add_line(&mut compass, "Lowest ask");
    let state = set_price(&mut compass, &state, "Lowest ask", "450k");
    assert_ne!(compass.market_prices_revision(), before, "a change to save");

    let saved = serde_json::to_string(&compass.market_prices()).expect("saves as JSON");
    let mut next = started_at(FakeSource::osms_offline().with_items(shop()), SAVED_AT + DAY_MS);
    next.restore_market_prices(serde_json::from_str(&saved).expect("reads back"));
    let restored = press_over(&mut next, "Bronze Ore", 1_000);

    assert_eq!(card(&restored).expect("a card").market, card(&state).expect("a card").market);
    assert_eq!(market(&restored), [("Your price".to_string(), None), ("Lowest ask".to_string(), Some(450_000))]);
}

#[test]
fn a_price_kept_before_there_were_lines_is_on_the_first_line() {
    let mut compass = started();
    // As MapleCompass 0.8.0 saved them: one price per item.
    let kept = r#"{"version":1,"items":{"04010000":{"mesos":450000,"setAtUnixMs":1790812800000}}}"#;
    compass.restore_market_prices(serde_json::from_str(kept).expect("reads"));

    let state = press_over(&mut compass, "Bronze Ore", 1_000);

    assert_eq!(market(&state), [("Your price".to_string(), Some(450_000))]);
    assert_eq!(compass.market_prices().version, 2, "saved in the shape with lines from then on");
}

#[test]
fn the_first_lines_field_opens_as_the_one_price_did_in_older_bug_reports() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);

    let state = compass.act(Action::EditCardMarket).clone();

    assert_eq!(editing(&state), Some(MarketEdit::Price { line: line(&state, "Your price"), not_a_price: None }));
}

#[test]
fn a_new_card_or_closing_the_card_closes_its_field() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    let id = line(&state, "Your price");
    compass.edit_card_market_price(&id);

    let state = press_over(&mut compass, "Bronze Ore", 2_000);
    assert_eq!(editing(&state), None, "a new card");

    compass.edit_card_market_price(&id);
    let state = compass.close_item_card().clone();
    assert_eq!(state.item_card, None);
    compass.enter_card_market("450k");
    let state = press_over(&mut compass, "Bronze Ore", 3_000);
    assert_eq!(market(&state), not_set(&["Your price"]), "nothing was being typed once it closed");
}

#[test]
fn without_a_card_there_is_no_price_to_type() {
    let mut compass = started();
    compass.edit_card_market_price("1");
    compass.enter_card_market("450k");

    let state = press_over(&mut compass, "Bronze Ore", 1_000);

    assert_eq!(market(&state), not_set(&["Your price"]));
}

#[test]
fn a_choice_has_no_market_section_to_open() {
    let mut compass = started_with(FakeSource::osms_offline().with_items(namesakes()));
    press_over(&mut compass, "Blue Moon", 1_000);

    compass.edit_card_market_price("1");
    compass.act(Action::AddCardMarketLine);
    let state = compass.pick_card_item("01002067").clone();

    assert_eq!(editing(&state), None);
}

#[test]
fn the_plus_opens_a_new_lines_name_field() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);

    let state = compass.act(Action::AddCardMarketLine).clone();

    assert_eq!(editing(&state), Some(MarketEdit::Name { line: None, problem: None }));
}

#[test]
fn a_line_added_shows_last_on_every_card_with_no_prices_yet() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);

    let state = add_line(&mut compass, "  Lowest ask ");
    assert_eq!(market(&state), not_set(&["Your price", "Lowest ask"]));
    assert_eq!(editing(&state), None);

    let other = press_over(&mut compass, "Red Potion", 2_000);
    assert_eq!(market(&other), not_set(&["Your price", "Lowest ask"]));
}

#[test]
fn each_line_keeps_its_own_price_for_the_item() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);
    add_line(&mut compass, "Typical");
    let state = add_line(&mut compass, "Lowest ask");

    let state = set_price(&mut compass, &state, "Typical", "500k");
    let state = set_price(&mut compass, &state, "Lowest ask", "450k");

    assert_eq!(
        market(&state),
        [("Your price".to_string(), None), ("Typical".to_string(), Some(500_000)), ("Lowest ask".to_string(), Some(450_000))]
    );
}

#[test]
fn a_line_needs_a_name_no_other_line_has() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);
    compass.act(Action::AddCardMarketLine);

    let state = compass.enter_card_market_line("   ").clone();
    assert_eq!(editing(&state), Some(MarketEdit::Name { line: None, problem: Some(NameProblem::Empty) }));

    let state = compass.enter_card_market_line("YOUR PRICE").clone();
    let taken = NameProblem::Taken { name: "YOUR PRICE".into() };
    assert_eq!(editing(&state), Some(MarketEdit::Name { line: None, problem: Some(taken) }));
    assert_eq!(market(&state), not_set(&["Your price"]), "nothing added");
}

#[test]
fn clicking_a_lines_name_opens_its_name_field() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    let id = line(&state, "Your price");

    let state = compass.edit_card_market_line(&id).clone();

    assert_eq!(editing(&state), Some(MarketEdit::Name { line: Some(id), problem: None }));
}

#[test]
fn a_line_renamed_keeps_its_prices_and_may_change_only_its_case() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    let state = set_price(&mut compass, &state, "Your price", "450k");
    let id = line(&state, "Your price");

    compass.edit_card_market_line(&id);
    let state = compass.enter_card_market_line("Typical").clone();
    assert_eq!(market(&state), [("Typical".to_string(), Some(450_000))]);
    assert_eq!(line(&state, "Typical"), id, "the same line");

    compass.edit_card_market_line(&id);
    let state = compass.enter_card_market_line("TYPICAL").clone();
    assert_eq!(market(&state), [("TYPICAL".to_string(), Some(450_000))]);
    assert_eq!(editing(&state), None);
}

#[test]
fn a_line_cant_take_another_lines_name() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);
    let state = add_line(&mut compass, "Typical");
    let id = line(&state, "Typical");

    compass.edit_card_market_line(&id);
    let state = compass.enter_card_market_line("your price").clone();

    let taken = NameProblem::Taken { name: "your price".into() };
    assert_eq!(editing(&state), Some(MarketEdit::Name { line: Some(id), problem: Some(taken) }));
    assert_eq!(market(&state), not_set(&["Your price", "Typical"]));
}

#[test]
fn removing_a_line_asks_first_saying_how_many_prices_go_with_it() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);
    let state = add_line(&mut compass, "Typical");
    set_price(&mut compass, &state, "Typical", "500k");
    let state = press_over(&mut compass, "Red Potion", 2_000);
    set_price(&mut compass, &state, "Typical", "30");
    let id = line(&state, "Typical");

    compass.edit_card_market_line(&id);
    let state = compass.act(Action::AskRemoveCardMarketLine).clone();

    assert_eq!(editing(&state), Some(MarketEdit::Remove { line: id, prices: 2 }));
    assert_eq!(market(&state), [("Your price".to_string(), None), ("Typical".to_string(), Some(30))], "nothing gone yet");
}

#[test]
fn keep_leaves_the_line_as_it_was() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    let state = set_price(&mut compass, &state, "Your price", "450k");
    compass.edit_card_market_line(&line(&state, "Your price"));
    compass.act(Action::AskRemoveCardMarketLine);

    let state = compass.act(Action::CancelCardMarket).clone();

    assert_eq!(market(&state), [("Your price".to_string(), Some(450_000))]);
    assert_eq!(editing(&state), None);
}

#[test]
fn a_line_removed_goes_from_every_card_with_its_prices() {
    let mut compass = started();
    press_over(&mut compass, "Bronze Ore", 1_000);
    let state = add_line(&mut compass, "Typical");
    let state = set_price(&mut compass, &state, "Typical", "500k");
    compass.edit_card_market_line(&line(&state, "Typical"));
    compass.act(Action::AskRemoveCardMarketLine);

    let state = compass.act(Action::RemoveCardMarketLine).clone();
    assert_eq!(market(&state), not_set(&["Your price"]));
    assert_eq!(editing(&state), None);

    let state = add_line(&mut compass, "Typical");
    assert_eq!(market(&state), not_set(&["Your price", "Typical"]), "a line of that name again starts with no prices");
}

#[test]
fn only_a_lines_own_name_field_asks_to_remove_it() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    let id = line(&state, "Your price");

    let state = compass.act(Action::RemoveCardMarketLine).clone();
    assert_eq!(market(&state), not_set(&["Your price"]), "never without asking");

    compass.edit_card_market_price(&id);
    let state = compass.act(Action::AskRemoveCardMarketLine).clone();
    assert_eq!(editing(&state), Some(MarketEdit::Price { line: id, not_a_price: None }), "not from its price field");

    compass.act(Action::AddCardMarketLine);
    let state = compass.act(Action::AskRemoveCardMarketLine).clone();
    assert_eq!(editing(&state), Some(MarketEdit::Name { line: None, problem: None }), "not from a new line's");
}

#[test]
fn with_every_line_removed_the_card_has_none_until_the_player_adds_one() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    compass.edit_card_market_line(&line(&state, "Your price"));
    compass.act(Action::AskRemoveCardMarketLine);

    let state = compass.act(Action::RemoveCardMarketLine).clone();
    assert_eq!(market(&state), []);
    let saved = serde_json::to_string(&compass.market_prices()).expect("saves as JSON");
    let mut next = started();
    next.restore_market_prices(serde_json::from_str(&saved).expect("reads back"));
    assert_eq!(market(&press_over(&mut next, "Bronze Ore", 1_000)), [], "and none at the next launch");

    let state = add_line(&mut compass, "Typical");
    assert_eq!(market(&state), not_set(&["Typical"]));
}

#[test]
fn a_line_keeps_the_id_it_was_saved_with_and_a_new_line_takes_none_of_theirs() {
    let mut compass = started();
    // Lines as a tool of the player's own might write them, beside the player's.
    let kept = r#"{"version":2,"lines":[
        {"id":"1","name":"Your price","prices":{}},
        {"id":"mesowatch-typical","name":"Typical","prices":{"04010000":{"mesos":500000}}},
        {"id":"4","name":"Mine","prices":{}}]}"#;
    compass.restore_market_prices(serde_json::from_str(kept).expect("reads"));
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    assert_eq!(line(&state, "Typical"), "mesowatch-typical");

    let state = add_line(&mut compass, "Lowest ask");

    let ids: Vec<String> = card(&state).expect("a card").market.into_iter().map(|line| line.id).collect();
    assert_eq!(ids, ["1", "mesowatch-typical", "4", "5"]);
    assert_eq!(market(&state)[1], ("Typical".to_string(), Some(500_000)));
}

#[test]
fn a_field_left_open_on_a_line_that_goes_closes() {
    let mut compass = started();
    let state = press_over(&mut compass, "Bronze Ore", 1_000);
    compass.edit_card_market_price(&line(&state, "Your price"));

    let state = compass.restore_market_prices(serde_json::from_str(r#"{"version":2,"lines":[]}"#).unwrap()).clone();

    assert_eq!(editing(&state), None);
}
