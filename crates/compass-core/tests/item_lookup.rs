//! Item Lookup (ADR 0007): the player hovers an item in the game and presses Ctrl+Shift+I.
//! The shell captures the game's client area with the pointer, the Item Tooltip's name is
//! read, and MeowDB's search turns the name into the item's page, which opens in the
//! browser. A name MeowDB doesn't clearly have opens nothing, and the panel says why.
//!
//! The readings here stand for what the Item Tooltip reader made of a capture (its own
//! samples are in `item_tooltip_samples.rs`); MeowDB's answers are its search's, saved on
//! 2026-10-09.
//!
//! Worked example, as in `grind_tracker_window.rs`: the game's client area at (812,110),
//! 1920x1200 physical px, on one 3440x1440 monitor; an expanded panel of 340x72, so at
//! (2376,510). The Item Tooltip's name line is read at (1000,600), well clear of it.

mod common;

use common::{FakeSource, saved_search};
use compass_core::{
    Action, Compass, ExpBarRegion, GameWindow, ItemChoice, ItemHit, ItemLookup, ItemTooltip, ItemTooltipReading,
    OurWindow, PanelSizes, PanelState, Rect, Request, ScreenRect, Section, Settings, Size, SourceError, TitleRegion,
    meowdb,
};

const GAME: ScreenRect = ScreenRect { x: 812, y: 110, width: 1920, height: 1200 };
const SCREEN: ScreenRect = ScreenRect { x: 0, y: 0, width: 3440, height: 1440 };
const CLIENT: Size = Size { width: 1920, height: 1200 };
/// Where the name line of the Item Tooltip read is on screen.
const NAME: ScreenRect = ScreenRect { x: 1000, y: 600, width: 200, height: 19 };
/// Where the expanded panel is.
const PANEL: ScreenRect = ScreenRect { x: 2376, y: 510, width: 340, height: 72 };

fn settings() -> Settings {
    Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client: CLIENT }),
        ..Settings::default()
    }
}

/// A core over the game window, its panel's sizes known, MeowDB's search answered by
/// `source`.
fn started_with(source: FakeSource, settings: Settings) -> Compass {
    let mut compass = Compass::new(source);
    compass.restore(settings);
    compass.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    compass.set_grind_tracker_size(Size { width: 300, height: 150 });
    let frame = ScreenRect { x: GAME.x - 1, y: GAME.y - 31, width: GAME.width + 2, height: GAME.height + 32 };
    compass.on_game_window(Some(GameWindow { client: GAME, frame, screens: vec![SCREEN] }));
    compass
}

/// MeowDB's saved answers, as soon as asked (from the cache).
fn started() -> Compass {
    started_with(FakeSource::offline(), settings())
}

/// MeowDB's search answers whatever is asked with its saved answer for `query`.
fn answering(query: &'static str) -> FakeSource {
    let json = saved_search(query).expect("saved answer");
    FakeSource::offline().with_item_searches(move |_| meowdb::read_item_search(&json))
}

/// MeowDB's search answers whatever is asked with these items.
fn answering_items(items: &[(&str, &str)]) -> FakeSource {
    let hits: Vec<ItemHit> = items
        .iter()
        .map(|(id, name)| ItemHit { id: id.to_string(), name: name.to_string(), category: None })
        .collect();
    FakeSource::offline().with_item_searches(move |_| Ok(hits.clone()))
}

fn found(name: &str, at_ms: u64) -> Option<ItemTooltipReading> {
    Some(ItemTooltipReading { tooltip: ItemTooltip::Found { area: NAME, name: Some(name.into()) }, at_ms })
}

/// The Item Tooltip near the pointer read as `name`, after the player pressed the hotkey.
fn look_up(compass: &mut Compass, name: &str, at_ms: u64) -> PanelState {
    assert_eq!(compass.on_hotkey("ctrl+shift+i"), Some(Request::CaptureItemTooltip));
    compass.on_item_tooltip_reading(found(name, at_ms)).clone()
}

fn line(state: &PanelState) -> Option<ItemLookup> {
    state.item_lookup.as_ref().map(|line| line.outcome.clone())
}

fn opened(name: &str, id: &str) -> Option<ItemLookup> {
    Some(ItemLookup::Opened { name: name.into(), url: meowdb::item_url(id) })
}

fn not_found(read: &str) -> Option<ItemLookup> {
    Some(ItemLookup::NotFound { read: read.into() })
}

/// Everything the shell has been asked to do since it last asked.
fn requests(compass: &mut Compass) -> Vec<Request> {
    std::iter::from_fn(|| compass.take_request()).collect()
}

fn open(id: &str) -> Request {
    Request::OpenInBrowser { url: meowdb::item_url(id) }
}

#[test]
fn ctrl_shift_i_over_the_game_asks_the_shell_to_capture_it_with_the_pointer() {
    let mut compass = started();

    assert_eq!(compass.on_hotkey("ctrl+shift+i"), Some(Request::CaptureItemTooltip));
}

#[test]
fn without_a_game_window_ctrl_shift_i_does_nothing() {
    let mut compass = Compass::new(FakeSource::offline());

    assert_eq!(compass.on_hotkey("ctrl+shift+i"), None);
    assert_eq!(compass.panel_state().item_lookup, None);
}

#[test]
fn a_press_while_the_last_capture_is_still_being_read_is_ignored() {
    let mut compass = started();
    compass.on_hotkey("ctrl+shift+i");

    assert_eq!(compass.on_hotkey("ctrl+shift+i"), None, "the first capture is still being read");

    compass.on_item_tooltip_reading(found("Orihalcon Ore", 1_000));
    assert_eq!(compass.on_hotkey("ctrl+shift+i"), Some(Request::CaptureItemTooltip));
}

#[test]
fn a_capture_that_failed_lets_the_next_press_capture_again() {
    let mut compass = started();
    compass.on_hotkey("ctrl+shift+i");

    let state = compass.on_item_tooltip_reading(None).clone();

    assert_eq!(line(&state), Some(ItemLookup::Unreadable));
    assert_eq!(compass.on_hotkey("ctrl+shift+i"), Some(Request::CaptureItemTooltip));
}

#[test]
fn a_name_meowdb_has_opens_its_page() {
    let mut compass = started();

    let state = look_up(&mut compass, "Orihalcon Ore", 1_000);

    assert_eq!(line(&state), opened("Orihalcon Ore", "418"));
    assert_eq!(requests(&mut compass), [open("418")]);
}

#[test]
fn every_seed_item_opens_its_own_page() {
    for (name, id) in [
        ("Orihalcon Ore", "418"),
        ("The Green Relaxer", "522"),
        ("Soap Bubbles", "1552"),
        ("Ribboned Pig Headband", "808"),
        ("Mithril Guards", "690"),
    ] {
        let mut compass = started();
        let state = look_up(&mut compass, name, 1_000);
        assert_eq!(line(&state), opened(name, id), "{name}");
        assert_eq!(requests(&mut compass), [open(id)], "{name}");
    }
}

#[test]
fn while_meowdb_is_asked_the_panel_shows_the_name_read_and_the_page_opens_when_it_answers() {
    let mut compass = started_with(FakeSource::offline().items_taking(2), settings());

    let state = look_up(&mut compass, "Orihalcon Ore", 1_000);
    assert_eq!(line(&state), Some(ItemLookup::LookingUp { read: "Orihalcon Ore".into() }));
    assert!(requests(&mut compass).is_empty());

    compass.tick(1_100);
    let state = compass.tick(1_200).clone();

    assert_eq!(line(&state), opened("Orihalcon Ore", "418"));
    assert_eq!(requests(&mut compass), [open("418")]);
}

#[test]
fn meowdb_is_asked_once_per_lookup() {
    let source = FakeSource::offline().items_taking(2);
    let asked = source.item_requests.clone();
    let mut compass = started_with(source, settings());

    look_up(&mut compass, "Orihalcon Ore", 1_000);
    for at_ms in [1_100, 1_200, 1_300, 1_400] {
        compass.tick(at_ms);
    }

    assert_eq!(*asked.lock().unwrap(), ["Orihalcon Ore"]);
}

#[test]
fn letters_the_recogniser_confuses_and_case_still_find_the_item() {
    for read in ["0rihalcon 0re", "Orihalc0n Ore", "orihalcon ore", "Orihalcon  Ore"] {
        let mut compass = started_with(answering("Orihalcon Ore"), settings());
        let state = look_up(&mut compass, read, 1_000);
        assert_eq!(line(&state), opened("Orihalcon Ore", "418"), "{read:?}");
    }
    let mut compass = started_with(answering("Hero's Gladius"), settings());
    let state = look_up(&mut compass, "Heros GIadius", 1_000);
    assert!(matches!(line(&state), Some(ItemLookup::Choose { .. })), "punctuation and I for l: {state:?}");
}

#[test]
fn a_name_one_letter_off_opens_the_item_far_closer_than_any_other() {
    // MeowDB's real answer to "Orihalcon Oro": Orihalcon Ore, then Orihalcon Ingot and
    // other Orihalcon items.
    let mut compass = started();

    let state = look_up(&mut compass, "Orihalcon Oro", 1_000);

    assert_eq!(line(&state), opened("Orihalcon Ore", "418"));
}

#[test]
fn a_name_as_close_to_two_items_opens_neither() {
    let items = [("126", "Hat MP Scroll: Greater"), ("125", "Hat HP Scroll: Greater")];
    let mut compass = started_with(answering_items(&items), settings());

    let state = look_up(&mut compass, "Hat NP Scroll: Greater", 1_000);

    assert_eq!(line(&state), not_found("Hat NP Scroll: Greater"));
    assert!(requests(&mut compass).is_empty());
}

#[test]
fn a_name_one_letter_closer_to_one_of_two_items_still_opens_neither() {
    let items = [("126", "Hat MP Scroll: Greater"), ("125", "Hat HP Scroll: Greater")];
    let mut compass = started_with(answering_items(&items), settings());

    let state = look_up(&mut compass, "Hat MP Scroll: Greatr", 1_000);

    assert_eq!(line(&state), not_found("Hat MP Scroll: Greatr"));
}

#[test]
fn the_exact_name_wins_over_a_neighbour_one_letter_away() {
    let items = [("125", "Hat HP Scroll: Greater"), ("126", "Hat MP Scroll: Greater")];
    let mut compass = started_with(answering_items(&items), settings());

    let state = look_up(&mut compass, "Hat MP Scroll: Greater", 1_000);

    assert_eq!(line(&state), opened("Hat MP Scroll: Greater", "126"));
}

#[test]
fn a_name_meowdb_answers_only_with_other_items_opens_none() {
    // Opal Ore, Iron Ore, Gold Ore and others, but no Bronze Ore.
    let mut compass = started_with(answering("Orihalcon Ore"), settings());

    let state = look_up(&mut compass, "Bronze Ore", 1_000);

    assert_eq!(line(&state), not_found("Bronze Ore"));
    assert!(requests(&mut compass).is_empty());
}

#[test]
fn a_name_no_meowdb_item_has_says_so() {
    let mut compass = started();

    let state = look_up(&mut compass, "Zqxv Nothing", 1_000);

    assert_eq!(line(&state), not_found("Zqxv Nothing"));
    assert!(requests(&mut compass).is_empty());
}

#[test]
fn a_name_several_items_share_lists_them_with_their_categories_to_pick_from() {
    let mut compass = started();

    let state = look_up(&mut compass, "Hero's Gladius", 1_000);

    let choice = |id: &str, category: &str| ItemChoice {
        id: id.into(),
        name: "Hero's Gladius".into(),
        category: Some(category.into()),
        url: meowdb::item_url(id),
    };
    assert_eq!(
        line(&state),
        Some(ItemLookup::Choose {
            read: "Hero's Gladius".into(),
            items: vec![choice("465", "Quest Item"), choice("556", "One-Handed Sword")],
        }),
        "not Gladius or Old Gladius"
    );
    assert!(requests(&mut compass).is_empty(), "nothing opens until the player picks");

    assert_eq!(compass.pick_item("556"), Some(open("556")));
    assert_eq!(line(compass.panel_state()), opened("Hero's Gladius", "556"));
}

#[test]
fn only_an_item_on_the_list_can_be_picked() {
    let mut compass = started();
    look_up(&mut compass, "Hero's Gladius", 1_000);

    assert_eq!(compass.pick_item("551"), None, "Gladius isn't on the list");
}

#[test]
fn six_items_sharing_a_name_are_all_listed() {
    let mut compass = started();

    let state = look_up(&mut compass, "Omok Table", 1_000);

    let Some(ItemLookup::Choose { items, .. }) = line(&state) else { panic!("a list: {state:?}") };
    let ids: Vec<&str> = items.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(ids, ["484", "487", "490", "493", "496", "499"]);
}

#[test]
fn the_list_goes_after_half_a_minute() {
    let mut compass = started();
    look_up(&mut compass, "Hero's Gladius", 1_000);

    assert!(line(compass.tick(30_999)).is_some());
    assert_eq!(line(compass.tick(31_000)), None);
}

#[test]
fn the_next_lookup_replaces_the_list() {
    let mut compass = started();
    look_up(&mut compass, "Hero's Gladius", 1_000);

    let state = look_up(&mut compass, "Orihalcon Ore", 2_000);

    assert_eq!(line(&state), opened("Orihalcon Ore", "418"));
    assert_eq!(compass.pick_item("556"), None, "the list has gone");
}

#[test]
fn meowdb_unreachable_says_so() {
    let source = FakeSource::offline().with_item_searches(|_| Err(SourceError::Unreachable("offline".into())));
    let mut compass = started_with(source, settings());

    let state = look_up(&mut compass, "Orihalcon Ore", 1_000);

    assert_eq!(line(&state), Some(ItemLookup::Unreachable));
    assert!(requests(&mut compass).is_empty());
}

#[test]
fn a_search_answer_in_another_shape_says_lookup_is_unavailable() {
    for answer in ["<!DOCTYPE html><html></html>", r#"{"results":[]}"#, r#"{"items":[{"id":418}]}"#] {
        let source = FakeSource::offline().with_item_searches(move |_| meowdb::read_item_search(answer));
        let mut compass = started_with(source, settings());
        let state = look_up(&mut compass, "Orihalcon Ore", 1_000);
        assert_eq!(line(&state), Some(ItemLookup::FormatChanged), "{answer}");
        assert!(requests(&mut compass).is_empty(), "{answer}");
    }
}

#[test]
fn no_item_tooltip_near_the_pointer_says_so_and_nothing_is_looked_up() {
    let source = FakeSource::offline();
    let asked = source.item_requests.clone();
    let mut compass = started_with(source, settings());
    compass.on_hotkey("ctrl+shift+i");

    let state = compass.on_item_tooltip_reading(Some(ItemTooltipReading { tooltip: ItemTooltip::None, at_ms: 1_000 }));

    assert_eq!(line(state), Some(ItemLookup::NoTooltip));
    assert!(asked.lock().unwrap().is_empty());
}

#[test]
fn an_item_tooltip_whose_name_cant_be_read_says_so() {
    let mut compass = started();
    compass.on_hotkey("ctrl+shift+i");

    let unreadable = ItemTooltip::Found { area: NAME, name: None };
    let state = compass.on_item_tooltip_reading(Some(ItemTooltipReading { tooltip: unreadable, at_ms: 1_000 }));

    assert_eq!(line(state), Some(ItemLookup::Unreadable));
}

#[test]
fn a_name_read_as_too_little_to_be_one_is_unreadable() {
    let mut compass = started();

    let state = look_up(&mut compass, " . ", 1_000);

    assert_eq!(line(&state), Some(ItemLookup::Unreadable));
}

#[test]
fn our_panel_over_the_name_says_so_and_nothing_is_looked_up() {
    let source = FakeSource::offline();
    let asked = source.item_requests.clone();
    let mut compass = started_with(source, settings());
    assert_eq!(compass.panel_state().placement, Some(PANEL));
    compass.on_hotkey("ctrl+shift+i");

    // The name runs on under the panel's left edge.
    let under_panel = ScreenRect { x: PANEL.x - 150, y: PANEL.y + 20, width: 170, height: 19 };
    let tooltip = ItemTooltip::Found { area: under_panel, name: Some("Orihalcon".into()) };
    let state = compass.on_item_tooltip_reading(Some(ItemTooltipReading { tooltip, at_ms: 1_000 }));

    assert_eq!(line(state), Some(ItemLookup::Covered { by: OurWindow::Panel }));
    assert!(asked.lock().unwrap().is_empty(), "a name cut short isn't looked up");
}

#[test]
fn the_grind_tracker_over_the_name_says_so() {
    let tracking = Settings {
        exp_bar_region: Some(ExpBarRegion { marked: Rect { x: 1005, y: 1144, width: 240, height: 32 }, client: CLIENT }),
        ..settings()
    };
    let mut compass = started_with(FakeSource::offline(), tracking);
    let tracker = compass.panel_state().grind_tracker.placement.expect("the Grind Tracker shows");
    compass.on_hotkey("ctrl+shift+i");

    let under_tracker = ScreenRect { x: tracker.x + 10, y: tracker.y + 10, width: 170, height: 19 };
    let tooltip = ItemTooltip::Found { area: under_tracker, name: Some("Orihalcon Ore".into()) };
    let state = compass.on_item_tooltip_reading(Some(ItemTooltipReading { tooltip, at_ms: 1_000 }));

    assert_eq!(line(state), Some(ItemLookup::Covered { by: OurWindow::GrindTracker }));
}

#[test]
fn a_scrolled_items_upgrade_count_isnt_looked_up_with_its_name() {
    let source = FakeSource::offline();
    let asked = source.item_requests.clone();
    let mut compass = started_with(source, settings());

    let state = look_up(&mut compass, "Mithril Guards (+3)", 1_000);

    assert_eq!(*asked.lock().unwrap(), ["Mithril Guards"]);
    assert_eq!(line(&state), opened("Mithril Guards", "690"));
}

#[test]
fn the_line_goes_after_six_seconds() {
    let mut compass = started();
    look_up(&mut compass, "Zqxv Nothing", 1_000);

    assert!(line(compass.tick(6_999)).is_some());
    assert_eq!(line(compass.tick(7_000)), None);
}

#[test]
fn each_lookup_is_numbered_so_the_same_line_twice_shows_afresh() {
    let mut compass = started();

    let first = look_up(&mut compass, "Zqxv Nothing", 1_000).item_lookup.unwrap();
    let second = look_up(&mut compass, "Zqxv Nothing", 2_000).item_lookup.unwrap();

    assert_eq!(first.outcome, second.outcome);
    assert_ne!(first.lookup, second.lookup);
}

/// A core whose panel the player collapsed to its icon.
fn collapsed() -> Compass {
    let mut compass = started();
    compass.act(Action::ToggleExpanded);
    assert!(!compass.panel_state().expanded);
    compass
}

#[test]
fn a_failure_expands_a_collapsed_panel_until_its_line_goes() {
    let mut compass = collapsed();

    let state = look_up(&mut compass, "Zqxv Nothing", 1_000);

    assert!(state.expanded, "so the player sees why nothing opened");
    assert!(!compass.settings().expanded, "the player's collapsed panel is what's saved");
    assert!(!compass.tick(7_000).expanded, "collapsed again once the line goes");
}

#[test]
fn an_opened_page_leaves_a_collapsed_panel_collapsed() {
    let mut compass = collapsed();

    let state = look_up(&mut compass, "Orihalcon Ore", 1_000);

    assert!(!state.expanded);
    assert_eq!(line(&state), opened("Orihalcon Ore", "418"));
}

#[test]
fn a_panel_the_player_uses_while_expanded_for_a_line_stays_expanded() {
    let mut compass = collapsed();
    look_up(&mut compass, "Zqxv Nothing", 1_000);

    compass.act(Action::ToggleSection(Section::Mobs));

    assert!(compass.tick(7_000).expanded);
    assert!(compass.settings().expanded);
}

#[test]
fn collapsing_a_panel_expanded_for_a_line_keeps_it_collapsed() {
    let mut compass = collapsed();
    look_up(&mut compass, "Zqxv Nothing", 1_000);

    let state = compass.act(Action::ToggleExpanded).clone();

    assert!(!state.expanded);
    assert!(line(&state).is_some(), "the line is still there, behind the icon");
    assert!(!compass.tick(2_000).expanded);
    assert!(!compass.tick(7_000).expanded);
}

#[test]
fn picking_from_the_list_collapses_the_panel_again() {
    let mut compass = collapsed();

    assert!(look_up(&mut compass, "Hero's Gladius", 1_000).expanded, "a list to pick from shows");
    compass.pick_item("465");

    assert!(!compass.panel_state().expanded);
}

#[test]
fn an_expanded_panel_stays_expanded_after_a_failure_goes() {
    let mut compass = started();

    look_up(&mut compass, "Zqxv Nothing", 1_000);

    assert!(compass.tick(7_000).expanded);
}
