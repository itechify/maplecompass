//! The Personal Bests page (GLOSSARY): every map's Personal Best side by side, opened in
//! the player's browser from the Grind Tracker's window bar, to compare maps. The core
//! hands the shell the page as plain data, one map a row, highest EXP/h first: the bests
//! as saved, the Visit under way's included, with each map's names, mob levels and share
//! of its solo EXP/h ceiling, and the map the player is on marked.

mod common;

use common::player::Player;
use common::{CONSTRUCTION_SITE_ID, SUNSET_SKY_ID, sharing_a_title, tracking, tracking_with};
use compass_core::{
    BESTS_VERSION, BestsPage, BestsPageMap, LevelRange, MEOWDB_CREDIT, MapBests, PanelControl, PanelState, Request,
    SavedBests, VisitBest, meowdb,
};

const S: u64 = 1000;
const MIN: u64 = 60 * S;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const ROCKY_ROAD: &str = "victoria-road__rocky-road-i--title-region";

/// The player presses the Grind Tracker's Personal Bests button: the page the shell is to
/// open, if any.
fn open_page(player: &mut Player) -> Option<BestsPage> {
    match player.compass.press(PanelControl::OpenBestsPage) {
        Some(Request::OpenBestsPage { page }) => Some(page),
        None => None,
        other => panic!("not the Personal Bests page: {other:?}"),
    }
}

/// Whether the Grind Tracker's window bar shows the Personal Bests button.
fn button(state: &PanelState) -> bool {
    state.grind_tracker.personal_bests
}

/// The page's rows as (map ID, EXP/h, marked here), in its order.
fn rows(page: &BestsPage) -> Vec<(&str, u64, bool)> {
    page.maps.iter().map(|m| (m.id.as_str(), m.exp_per_hour, m.here)).collect()
}

/// The page's row for `map`.
fn row<'a>(page: &'a BestsPage, map: &str) -> &'a BestsPageMap {
    page.maps.iter().find(|m| m.id == map).unwrap_or_else(|| panic!("no row for {map}"))
}

/// Time passes with the player standing still, long enough for the world crawl to read
/// every saved page (each is at hand, so taken at once).
fn crawl_the_world(player: &mut Player) {
    for _ in 0..60 {
        player.compass.tick(player.at);
    }
}

/// Sunset Sky's mob levels, from its saved page: Blue Snail to Green Mushroom.
const SUNSET_SKY_LEVELS: LevelRange = LevelRange { lowest: 2, highest: 14 };

/// The Construction Site's mob levels, from its saved page: Red Snail to Blue Mushroom.
const CONSTRUCTION_SITE_LEVELS: LevelRange = LevelRange { lowest: 4, highest: 19 };

#[test]
fn with_no_best_anywhere_there_is_no_button_and_no_page() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    // 4 minutes: not a best yet.
    let four = player.hunt(4 * MIN, 20);
    assert!(!button(&four));
    assert_eq!(open_page(&mut player), None);
}

#[test]
fn a_first_visits_best_is_on_the_page_with_the_maps_names_levels_and_share_marked_here() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    // 7,200 EXP/h, the best set 5 minutes in.
    let five = player.hunt(5 * MIN, 20);
    let set = player.wall_clock();
    assert!(button(&five));
    player.hunt(MIN, 20);

    let page = open_page(&mut player).expect("a page");
    assert_eq!(
        page,
        BestsPage {
            as_of_unix_ms: player.wall_clock(),
            maps: vec![BestsPageMap {
                id: SUNSET_SKY_ID.into(),
                street_name: "Victoria Road".into(),
                map_name: "Sunset Sky".into(),
                region: "Victoria Island".into(),
                url: meowdb::map_url(SUNSET_SKY_ID),
                mob_levels: Some(SUNSET_SKY_LEVELS),
                exp_per_hour: 7200,
                // Of its solo ceiling, 213,571 EXP/h: 3.37%.
                solo_exp_ceiling: Some(213_571),
                ceiling_share_hundredths: Some(337),
                set_at_unix_ms: set,
                here: true,
            }],
            credit: MEOWDB_CREDIT,
        }
    );
}

#[test]
fn maps_are_highest_first_and_only_the_one_the_player_is_on_is_marked_here() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(6 * MIN, 30);

    let page = open_page(&mut player).expect("a page");
    assert_eq!(rows(&page), [(CONSTRUCTION_SITE_ID, 10_800, true), (SUNSET_SKY_ID, 7200, false)]);
    // Of its solo ceiling, 281,786 EXP/h: 3.83%.
    let site = row(&page, CONSTRUCTION_SITE_ID);
    assert_eq!(
        (site.mob_levels, site.solo_exp_ceiling, site.ceiling_share_hundredths),
        (Some(CONSTRUCTION_SITE_LEVELS), Some(281_786), Some(383))
    );
}

#[test]
fn a_map_the_player_is_not_on_has_its_levels_and_share_once_the_crawl_has_read_its_page() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 30);

    // The crawl hasn't got to Sunset Sky: its names, but no levels or share.
    let early = open_page(&mut player).expect("a page");
    let sunset = row(&early, SUNSET_SKY_ID);
    assert_eq!(
        (sunset.map_name.as_str(), sunset.mob_levels, sunset.solo_exp_ceiling, sunset.ceiling_share_hundredths),
        ("Sunset Sky", None, None, None)
    );

    crawl_the_world(&mut player);
    let crawled = open_page(&mut player).expect("a page");
    let sunset = row(&crawled, SUNSET_SKY_ID);
    assert_eq!(
        (sunset.mob_levels, sunset.solo_exp_ceiling, sunset.ceiling_share_hundredths),
        (Some(SUNSET_SKY_LEVELS), Some(213_571), Some(337))
    );
}

#[test]
fn the_page_has_this_visits_best_though_the_best_row_shows_the_earlier_visits() {
    // Sunset Sky at 7,200 EXP/h, then 2 minutes on the Construction Site, and back.
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(2 * MIN, 30);
    player.arrive(SUNSET);

    // 10,800 EXP/h beats it: the Best row stays at the earlier Visit's.
    let beaten = player.hunt(5 * MIN, 30);
    let set = player.wall_clock();
    assert_eq!(beaten.exp_tracking.personal_best, Some(7200));
    let page = open_page(&mut player).expect("a page");
    assert_eq!(rows(&page), [(SUNSET_SKY_ID, 10_800, true)]);
    assert_eq!(row(&page, SUNSET_SKY_ID).set_at_unix_ms, set);
}

#[test]
fn while_candidates_wait_for_a_choice_no_map_is_marked_here() {
    // A first Visit, choosing the Construction Site past Sunset Sky: 3,600 EXP/h there.
    let mut player = Player::clocked(tracking_with(sharing_a_title()));
    player.arrive(KERNING);
    player.hunt(6 * MIN, 10);
    player.choose_next();
    player.choose_next();
    assert_eq!(rows(&open_page(&mut player).expect("a page")), [(CONSTRUCTION_SITE_ID, 3600, true)]);

    // Back by Rocky Road I, which doesn't settle which map the title is: the panel shows
    // the Construction Site, but the player may be on Sunset Sky.
    player.arrive(ROCKY_ROAD);
    player.hunt(MIN, 10);
    let back = player.arrive(KERNING);
    assert_eq!(back.candidates.map(|c| c.chosen), Some(false));
    assert_eq!(rows(&open_page(&mut player).expect("a page")), [(CONSTRUCTION_SITE_ID, 3600, false)]);
}

#[test]
fn a_best_on_a_map_the_map_index_no_longer_has_goes_by_its_id() {
    let gone = "999999999";
    let bests = SavedBests {
        version: BESTS_VERSION,
        maps: vec![MapBests { id: gone.into(), visits: vec![VisitBest { exp_per_hour: 5000, set_at_unix_ms: None }] }],
    };
    let mut compass = tracking();
    compass.restore_bests(bests);
    let mut player = Player::clocked(compass);
    player.arrive(SUNSET);

    let page = open_page(&mut player).expect("a page");
    assert_eq!(
        page.maps,
        [BestsPageMap {
            id: gone.into(),
            street_name: String::new(),
            map_name: String::new(),
            region: String::new(),
            url: meowdb::map_url(gone),
            mob_levels: None,
            exp_per_hour: 5000,
            solo_exp_ceiling: None,
            ceiling_share_hundredths: None,
            set_at_unix_ms: None,
            here: false,
        }]
    );
}

#[test]
fn a_best_removed_from_the_history_leaves_the_page_and_with_none_left_the_button_goes() {
    // Sunset Sky's only best, 7,200 EXP/h, then back on it.
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 30);
    let back = player.arrive(SUNSET);
    assert!(button(&back));

    let removed = player.press(PanelControl::RemoveBest);
    assert!(!button(&removed));
    assert_eq!(open_page(&mut player), None);
}

#[test]
fn without_the_wall_clock_the_page_and_its_bests_have_no_date() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);

    let page = open_page(&mut player).expect("a page");
    assert_eq!(page.as_of_unix_ms, None);
    assert_eq!(row(&page, SUNSET_SKY_ID).set_at_unix_ms, None);
}

#[test]
fn the_shell_gets_the_page_and_the_button_as_the_browser_and_window_render_them() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    let state = serde_json::to_value(player.hunt(6 * MIN, 20)).unwrap();
    assert_eq!(state["grindTracker"]["personalBests"], true);

    let page = serde_json::to_value(open_page(&mut player).expect("a page")).unwrap();
    assert_eq!(page["asOfUnixMs"], player.wall_clock().unwrap());
    assert_eq!(page["credit"], MEOWDB_CREDIT);
    let sunset = &page["maps"][0];
    assert_eq!(sunset["id"], SUNSET_SKY_ID);
    assert_eq!(sunset["streetName"], "Victoria Road");
    assert_eq!(sunset["mapName"], "Sunset Sky");
    assert_eq!(sunset["region"], "Victoria Island");
    assert_eq!(sunset["url"], meowdb::map_url(SUNSET_SKY_ID));
    assert_eq!(sunset["mobLevels"], serde_json::json!({ "lowest": 2, "highest": 14 }));
    assert_eq!(sunset["expPerHour"], 7200);
    assert_eq!(sunset["soloExpCeiling"], 213_571);
    assert_eq!(sunset["ceilingShareHundredths"], 337);
    assert!(sunset["setAtUnixMs"].is_u64());
    assert_eq!(sunset["here"], true);
}
