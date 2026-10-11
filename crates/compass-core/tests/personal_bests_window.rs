//! The Personal Bests window (GLOSSARY): every map's Personal Best side by side, over the
//! game, to compare maps. The Grind Tracker's star and its Best row's "Best" open it while
//! some map has a best, and close it; so does its ×. It shows by the Grind Tracker's rule
//! (with the game in front, not while setup is open), EXP tracking on or not. The core
//! hands the shell its rows as plain data, one map a row, highest EXP/h first, live: the
//! bests as saved, the Visit under way's included, with each map's names, region, mob
//! levels and share of its solo EXP/h ceiling from OSMS's file set, and the map the player
//! is on marked.
//!
//! Where it goes, worked as in `grind_tracker_window.rs`: the game's client area at
//! (812,110), 1920x1200 physical px, on one 3440x1440 monitor, and a Personal Bests window
//! of 920x600, so centred at (1312,410). The Title Region is captured at (882,150)
//! 130x48 and the EXP Bar Region at (1817,1254) 384x32.

mod common;

use common::player::Player;
use common::{CONSTRUCTION_SITE_ID, FakeSource, SUNSET_SKY_ID, osms, sharing_a_title, tracking, tracking_with};
use compass_core::{
    Action, BESTS_VERSION, ClientOffset, Compass, ExpBarRegion, Foreground, GameWindow, LevelRange, MapBests,
    OSMS_AND_MEOWDB_CREDIT, PanelControl, PanelSizes, PanelState, PersonalBestsMap, PersonalBestsWindow, Rect,
    Request, SavedBests, ScreenRect, Settings, Size, TitleRegion, VisitBest, meowdb,
};

const S: u64 = 1000;
const MIN: u64 = 60 * S;

const SUNSET: &str = "victoria-road__sunset-sky--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const ROCKY_ROAD: &str = "victoria-road__rocky-road-i--title-region";

/// The player presses the Grind Tracker's star: the Personal Bests window then.
fn toggle(player: &mut Player) -> PersonalBestsWindow {
    player.press(PanelControl::TogglePersonalBests).personal_bests
}

/// Whether the Grind Tracker's window bar shows the Personal Bests button.
fn button(state: &PanelState) -> bool {
    state.grind_tracker.personal_bests
}

/// The window's rows as (map ID, EXP/h, marked here), in its order.
fn rows(window: &PersonalBestsWindow) -> Vec<(&str, u64, bool)> {
    window.maps.iter().map(|m| (m.id.as_str(), m.exp_per_hour, m.here)).collect()
}

/// The window's row for `map`.
fn row<'a>(window: &'a PersonalBestsWindow, map: &str) -> &'a PersonalBestsMap {
    window.maps.iter().find(|m| m.id == map).unwrap_or_else(|| panic!("no row for {map}"))
}

/// Bests saved at an earlier launch: one Visit's, with no date, on each map.
fn bests_of(maps: &[(&str, u64)]) -> SavedBests {
    let visit = |exp_per_hour| vec![VisitBest { exp_per_hour, set_at_unix_ms: None }];
    let maps = maps.iter().map(|&(id, exp)| MapBests { id: id.into(), visits: visit(exp) }).collect();
    SavedBests { version: BESTS_VERSION, maps }
}

/// Henesys Hunting Ground I's map ID: its Map Summary is the one the spec works out.
const HENESYS_HUNTING_GROUND_I_ID: &str = "010001010";

/// Maple Hill: a Hidden Street map in the Event region.
const MAPLE_HILL: u64 = 90_000_000;
const MAPLE_HILL_ID: &str = "090000000";

/// Sunset Sky's mob levels in the made-up game world: Blue Snail to Green Mushroom.
const SUNSET_SKY_LEVELS: LevelRange = LevelRange { lowest: 2, highest: 14 };

/// The Construction Site's mob levels in the made-up game world: Red Snail to Blue
/// Mushroom.
const CONSTRUCTION_SITE_LEVELS: LevelRange = LevelRange { lowest: 4, highest: 19 };

#[test]
fn with_no_best_anywhere_there_is_no_button_and_the_window_doesnt_open() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    // 4 minutes: not a best yet.
    let four = player.hunt(4 * MIN, 20);
    assert!(!button(&four));

    let window = toggle(&mut player);
    assert!(!window.open);
    assert_eq!(window.placement, None);
    assert!(window.maps.is_empty());
}

#[test]
fn a_first_visits_best_is_in_the_window_with_the_maps_names_levels_and_share_marked_here() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    // 7,200 EXP/h, the best set 5 minutes in.
    let five = player.hunt(5 * MIN, 20);
    let set = player.wall_clock();
    assert!(button(&five));
    player.hunt(MIN, 20);

    let window = toggle(&mut player);
    assert!(window.open);
    assert_eq!(window.credit, OSMS_AND_MEOWDB_CREDIT);
    assert_eq!(
        window.maps,
        [PersonalBestsMap {
            id: SUNSET_SKY_ID.into(),
            street_name: "Victoria Road".into(),
            map_name: "Sunset Sky".into(),
            region: "Victoria Island".into(),
            mark: None,
            url: meowdb::map_url(SUNSET_SKY_ID),
            mob_levels: Some(SUNSET_SKY_LEVELS),
            exp_per_hour: 7200,
            // Of its solo ceiling, 213,571 EXP/h: 3.37%.
            solo_exp_ceiling: Some(213_571),
            ceiling_share_hundredths: Some(337),
            set_at_unix_ms: set,
            here: true,
        }]
    );
}

#[test]
fn maps_are_highest_first_and_only_the_one_the_player_is_on_is_marked_here() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(6 * MIN, 30);

    let window = toggle(&mut player);
    assert_eq!(rows(&window), [(CONSTRUCTION_SITE_ID, 10_800, true), (SUNSET_SKY_ID, 7200, false)]);
    // Of its solo ceiling, 281,786 EXP/h: 3.83%.
    let site = row(&window, CONSTRUCTION_SITE_ID);
    assert_eq!(
        (site.mob_levels, site.solo_exp_ceiling, site.ceiling_share_hundredths),
        (Some(CONSTRUCTION_SITE_LEVELS), Some(281_786), Some(383))
    );
}

#[test]
fn open_it_follows_the_bests_and_the_player_as_they_hunt() {
    // Open on Sunset Sky's first best.
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    assert_eq!(rows(&toggle(&mut player)), [(SUNSET_SKY_ID, 7200, true)]);

    // Off to the Construction Site, where a best comes without opening it again.
    let arrived = player.arrive(CONSTRUCTION_SITE);
    assert_eq!(rows(&arrived.personal_bests), [(SUNSET_SKY_ID, 7200, false)]);
    let hunted = player.hunt(6 * MIN, 30);
    assert_eq!(rows(&hunted.personal_bests), [(CONSTRUCTION_SITE_ID, 10_800, true), (SUNSET_SKY_ID, 7200, false)]);
}

#[test]
fn a_best_on_a_map_the_player_has_not_been_on_has_its_names_region_levels_and_share_at_once() {
    // Bests saved at an earlier launch: Henesys Hunting Ground I (Victoria Island) and
    // Maple Hill, an Event map given Blue Snails.
    let world = osms::game().changing(MAPLE_HILL, |m| m.spawns(3, 4));
    let mut compass = tracking_with(FakeSource::with_world(world));
    compass.restore_bests(bests_of(&[(HENESYS_HUNTING_GROUND_I_ID, 7661), (MAPLE_HILL_ID, 2857)]));
    let mut player = Player::clocked(compass);
    player.arrive(SUNSET);

    let window = toggle(&mut player);
    assert_eq!(
        window.maps,
        [
            PersonalBestsMap {
                id: HENESYS_HUNTING_GROUND_I_ID.into(),
                street_name: "Victoria Road".into(),
                map_name: "Henesys Hunting Ground I".into(),
                region: "Victoria Island".into(),
                mark: None,
                url: meowdb::map_url(HENESYS_HUNTING_GROUND_I_ID),
                // Blue Snail and Shroom to Green Mushroom.
                mob_levels: Some(LevelRange { lowest: 2, highest: 14 }),
                exp_per_hour: 7661,
                // Of its solo ceiling, 153,216 EXP/h: 5.0%.
                solo_exp_ceiling: Some(153_216),
                ceiling_share_hundredths: Some(500),
                set_at_unix_ms: None,
                here: false,
            },
            PersonalBestsMap {
                id: MAPLE_HILL_ID.into(),
                street_name: "Hidden Street".into(),
                map_name: "Maple Hill".into(),
                region: "Event".into(),
                mark: None,
                url: meowdb::map_url(MAPLE_HILL_ID),
                mob_levels: Some(LevelRange { lowest: 2, highest: 2 }),
                exp_per_hour: 2857,
                // Of its solo ceiling, 4 Blue Snails' 5,714 EXP/h: 50%.
                solo_exp_ceiling: Some(5714),
                ceiling_share_hundredths: Some(5000),
                set_at_unix_ms: None,
                here: false,
            },
        ]
    );
}

#[test]
fn the_window_has_this_visits_best_though_the_best_row_shows_the_earlier_visits() {
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
    let window = toggle(&mut player);
    assert_eq!(rows(&window), [(SUNSET_SKY_ID, 10_800, true)]);
    assert_eq!(row(&window, SUNSET_SKY_ID).set_at_unix_ms, set);
}

#[test]
fn while_candidates_wait_for_a_choice_no_map_is_marked_here() {
    // A first Visit, choosing the Construction Site past Sunset Sky: 3,600 EXP/h there.
    let mut player = Player::clocked(tracking_with(sharing_a_title()));
    player.arrive(KERNING);
    player.hunt(6 * MIN, 10);
    player.choose_next();
    player.choose_next();
    assert_eq!(rows(&toggle(&mut player)), [(CONSTRUCTION_SITE_ID, 3600, true)]);

    // Back by Rocky Road I, which doesn't settle which map the title is: the panel shows
    // the Construction Site, but the player may be on Sunset Sky.
    player.arrive(ROCKY_ROAD);
    player.hunt(MIN, 10);
    let back = player.arrive(KERNING);
    assert_eq!(back.candidates.map(|c| c.chosen), Some(false));
    assert_eq!(rows(&back.personal_bests), [(CONSTRUCTION_SITE_ID, 3600, false)]);
}

#[test]
fn a_best_on_a_map_not_in_the_map_list_is_kept_and_goes_by_its_id() {
    let gone = "999999999";
    let mut compass = tracking();
    compass.restore_bests(bests_of(&[(gone, 5000)]));
    let mut player = Player::clocked(compass);
    player.arrive(SUNSET);

    let window = toggle(&mut player);
    assert_eq!(
        window.maps,
        [PersonalBestsMap {
            id: gone.into(),
            street_name: String::new(),
            map_name: String::new(),
            region: String::new(),
            mark: None,
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
fn removing_the_last_best_closes_the_window_and_the_button_goes() {
    // Sunset Sky's only best, 7,200 EXP/h, then back on it with the window open.
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SITE);
    player.hunt(MIN, 30);
    let back = player.arrive(SUNSET);
    assert!(button(&back));
    assert!(toggle(&mut player).open);

    let removed = player.press(PanelControl::RemoveBest);
    assert!(!button(&removed));
    assert!(!removed.personal_bests.open);
    assert_eq!(removed.personal_bests.placement, None);
}

#[test]
fn without_the_wall_clock_its_bests_have_no_date() {
    let mut player = Player::new(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);

    let window = toggle(&mut player);
    assert_eq!(row(&window, SUNSET_SKY_ID).set_at_unix_ms, None);
}

#[test]
fn pressing_the_star_again_closes_it_and_closed_it_holds_no_rows() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    assert!(toggle(&mut player).open);

    let closed = toggle(&mut player);
    assert!(!closed.open);
    assert!(closed.maps.is_empty());

    assert!(toggle(&mut player).open);
    let closed = player.press(PanelControl::ClosePersonalBests).personal_bests;
    assert!(!closed.open);
}

#[test]
fn clicking_a_maps_name_opens_its_meowdb_page() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    player.hunt(6 * MIN, 20);
    assert_eq!(player.compass.open_bests_map_page(SUNSET_SKY_ID), None, "closed, it lists none");
    toggle(&mut player);

    let url = meowdb::map_url(SUNSET_SKY_ID);
    assert_eq!(player.compass.open_bests_map_page(SUNSET_SKY_ID), Some(Request::OpenInBrowser { url }));
    assert_eq!(player.compass.open_bests_map_page(CONSTRUCTION_SITE_ID), None);
}

#[test]
fn the_shell_gets_the_window_and_the_button_as_they_render_them() {
    let mut player = Player::clocked(tracking());
    player.arrive(SUNSET);
    let state = serde_json::to_value(player.hunt(6 * MIN, 20)).unwrap();
    assert_eq!(state["grindTracker"]["personalBests"], true);

    let opened = player.press(PanelControl::TogglePersonalBests);
    // Its rows go to its own window, not with the overlay's state.
    assert!(serde_json::to_value(&opened).unwrap().get("personalBests").is_none());
    let window = serde_json::to_value(&opened.personal_bests).unwrap();
    assert_eq!(window["open"], true);
    assert_eq!(window["credit"], OSMS_AND_MEOWDB_CREDIT);
    let sunset = &window["maps"][0];
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

// Where it goes.

const GAME: ScreenRect = ScreenRect { x: 812, y: 110, width: 1920, height: 1200 };
const SCREEN: ScreenRect = ScreenRect { x: 0, y: 0, width: 3440, height: 1440 };
const CLIENT: Size = Size { width: 1920, height: 1200 };
const WINDOW: Size = Size { width: 920, height: 600 };

fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect { x, y, width, height }
}

/// The game window with this client area, framed as measured, on `SCREEN`.
fn game(client: ScreenRect) -> Option<GameWindow> {
    let frame = rect(client.x - 1, client.y - 31, client.width + 2, client.height + 32);
    Some(GameWindow { client, frame, screens: vec![SCREEN] })
}

/// Settings with both regions marked on the game's client area.
fn tracking_settings() -> Settings {
    Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client: CLIENT }),
        exp_bar_region: Some(ExpBarRegion { marked: Rect { x: 1005, y: 1144, width: 240, height: 32 }, client: CLIENT }),
        ..Settings::default()
    }
}

/// A core restored from `settings` with a best on Sunset Sky, the game window found and
/// the windows' sizes known.
fn started(settings: Settings) -> Compass {
    started_on(GAME, settings)
}

/// As [`started`], the game's client area at `client`.
fn started_on(client: ScreenRect, settings: Settings) -> Compass {
    let mut compass = Compass::new(FakeSource::osms_offline());
    compass.restore(settings);
    compass.restore_bests(bests_of(&[(SUNSET_SKY_ID, 7200)]));
    compass.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    compass.set_grind_tracker_size(Size { width: 300, height: 150 });
    compass.set_personal_bests_size(WINDOW);
    compass.on_game_window(game(client));
    compass
}

/// As [`started`], with the Personal Bests window opened.
fn opened(settings: Settings) -> Compass {
    let mut compass = started(settings);
    compass.press(PanelControl::TogglePersonalBests);
    compass
}

fn placed(state: &PanelState) -> Option<ScreenRect> {
    state.personal_bests.placement
}

/// What the shell does on restart: save the settings as JSON, load them into a new core.
fn restart(compass: &Compass) -> Compass {
    let json = serde_json::to_string(&compass.settings()).unwrap();
    started(serde_json::from_str(&json).unwrap())
}

#[test]
fn it_opens_centred_over_the_game() {
    let compass = opened(tracking_settings());

    assert_eq!(placed(compass.panel_state()), Some(rect(1312, 410, 920, 600)));
}

#[test]
fn as_its_rows_grow_it_stays_centred() {
    let mut compass = opened(tracking_settings());

    let state = compass.set_personal_bests_size(Size { width: 920, height: 200 });

    assert_eq!(placed(state), Some(rect(1312, 610, 920, 200)));
}

#[test]
fn over_a_game_narrower_than_it_it_is_as_wide_as_the_game() {
    // A 900x700 client area, its Title Region captured at (882,150) 130x48.
    let narrow = rect(812, 110, 900, 700);
    let marked = Rect { x: 70, y: 40, width: 130, height: 48 };
    let title_region = Some(TitleRegion { marked, client: Size { width: 900, height: 700 } });
    let mut compass = started_on(narrow, Settings { title_region, ..Settings::default() });

    compass.press(PanelControl::TogglePersonalBests);

    // Centred at (812,160), 900 wide, it would cover the Title Region: it moves below it.
    assert_eq!(placed(compass.panel_state()), Some(rect(812, 198, 900, 600)));
}

#[test]
fn dragged_it_keeps_its_spot_relative_to_the_game_window_after_a_restart_too() {
    let mut compass = opened(tracking_settings());

    let state = compass.act(Action::PersonalBestsMoved { x: 1000, y: 300 });
    assert_eq!(placed(state), Some(rect(1000, 300, 920, 600)));

    // The game window moving takes it along.
    let moved = rect(GAME.x + 100, GAME.y + 50, GAME.width, GAME.height);
    assert_eq!(placed(compass.on_game_window(game(moved))), Some(rect(1100, 350, 920, 600)));

    assert_eq!(compass.settings().personal_bests_at, Some(ClientOffset { x: 188, y: 190 }));
    let mut restarted = restart(&compass);
    restarted.press(PanelControl::TogglePersonalBests);
    assert_eq!(placed(restarted.panel_state()), Some(rect(1000, 300, 920, 600)));
}

#[test]
fn dragged_back_to_the_middle_it_stays_centred_again() {
    let mut compass = opened(tracking_settings());
    compass.act(Action::PersonalBestsMoved { x: 1000, y: 300 });

    // Let go within an eighth of its width (115 px) of the middle (1312,410).
    let state = compass.act(Action::PersonalBestsMoved { x: 1312 + 100, y: 410 - 60 });
    assert_eq!(placed(state), Some(rect(1312, 410, 920, 600)));
    assert_eq!(compass.settings().personal_bests_at, None);
}

#[test]
fn dragged_onto_a_captured_region_it_moves_off_it() {
    let mut compass = opened(tracking_settings());

    // Over the EXP Bar Region (1817..2201, 1254..1286): our windows show in the capture,
    // so it moves the shortest way off it, here above it.
    let state = compass.act(Action::PersonalBestsMoved { x: 1700, y: 700 });

    assert_eq!(placed(state), Some(rect(1700, 654, 920, 600)));
}

#[test]
fn it_hides_without_the_game_while_setup_is_open_and_while_another_app_is_in_front() {
    let mut compass = opened(tracking_settings());
    let shown = placed(compass.panel_state());
    assert!(shown.is_some());

    assert_eq!(placed(compass.on_game_window(None)), None);
    assert!(compass.panel_state().personal_bests.open, "hidden, not closed");
    compass.on_game_window(game(GAME));

    compass.press(PanelControl::Setup);
    assert_eq!(placed(compass.panel_state()), None);
    compass.act(Action::CancelSetup);
    assert_eq!(placed(compass.panel_state()), shown);

    assert_eq!(placed(compass.on_foreground(Foreground::OtherApp)), None);
    assert_eq!(placed(compass.on_foreground(Foreground::MapleCompass)), shown);
}

#[test]
fn it_shows_with_exp_tracking_off() {
    let mut compass = opened(tracking_settings());

    let state = compass.act(Action::ClearExpBarRegion);

    assert_eq!(state.grind_tracker.placement, None);
    assert_eq!(placed(state), Some(rect(1312, 410, 920, 600)));
}

#[test]
fn each_launch_starts_with_it_closed() {
    let compass = opened(tracking_settings());

    let restarted = restart(&compass);

    assert!(!restarted.panel_state().personal_bests.open);
    assert_eq!(placed(restarted.panel_state()), None);
}
