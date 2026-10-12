//! Once the panel is sure of a map, it fills in that map's hunting information from
//! OSMS's file set: the mobs, sorted by level, and the Map Summary, worked out as MeowDB
//! works it out. Served from made-up worlds in OSMS's shape.

mod common;

use common::osms::{self, HENESYS_HUNTING_GROUND_I, map, mob};
use common::{FakeSource, arrive};
use minerva_core::{
    Minerva, MapData, MapDataStatus, MapSummary, Mob, PanelState, PerParty, Recognition, TitleLines, TitleReading,
};

const KERNING: &str = "victoria-road__kerning-city--window";

fn read_as(street: &str, map: &str, at_ms: u64) -> TitleReading {
    TitleReading { lines: Some(TitleLines { street: street.into(), map: map.into() }), at_ms }
}

/// Two confident readings of a title the minimap samples don't show.
fn arrive_at(world: osms::World, street: &str, name: &str) -> PanelState {
    let mut minerva = Minerva::new(FakeSource::with_world(world));
    minerva.on_reading(read_as(street, name, 0));
    minerva.on_reading(read_as(street, name, 333)).clone()
}

fn data(state: &PanelState) -> &MapData {
    match &state.data {
        MapDataStatus::Ready(data) => data,
        other => panic!("no map data: {other:?}"),
    }
}

fn mob_row(id: &str, name: &str, level: u32, hp: u64, exp: u64, spawn_count: u32, spawn_share_percent: u32) -> Mob {
    Mob { id: id.into(), name: name.into(), level, hp, exp, spawn_count, spawn_share_percent }
}

fn summary(solo: (u64, u64), full_party: (u64, u64)) -> Option<MapSummary> {
    Some(MapSummary {
        mobs_per_hour: PerParty { solo: solo.0, full_party: full_party.0 },
        exp_per_hour: PerParty { solo: solo.1, full_party: full_party.1 },
    })
}

/// Henesys Hunting Ground I's spawn points, untimed: Blue Snail 4, Shroom 10, Red Snail
/// 6, Stump 4, Slime 5, Orange Mushroom 5 and Green Mushroom 5, of 39. Spawn Shares are
/// rounded half up.
#[test]
fn a_maps_mobs_are_its_spawn_points_grouped_by_mob_with_their_figures_from_the_mobs_record() {
    let state = arrive_at(osms::game(), "Victoria Road", "Henesys Hunting Ground I");

    assert_eq!(
        data(&state).mobs,
        [
            mob_row("3", "Blue Snail", 2, 50, 4, 4, 10),
            mob_row("4", "Shroom", 2, 80, 6, 10, 26),
            mob_row("5", "Red Snail", 4, 68, 8, 6, 15),
            mob_row("6", "Stump", 5, 133, 10, 4, 10),
            mob_row("7", "Slime", 6, 120, 12, 5, 13),
            mob_row("9", "Orange Mushroom", 8, 142, 15, 5, 13),
            mob_row("13", "Green Mushroom", 14, 233, 26, 5, 13),
        ]
    );
}

/// The spec's example. An untimed spawn point gives a full party 3600 / 7.56 mobs an
/// hour, and a player alone 0.75 of that. Each mob's figures are rounded (Blue Snail's 4
/// points: 1,429 and 1,905 mobs, 5,714 and 7,619 EXP), and the map's are their sums, so
/// 13,930 mobs alone rather than the 13,929 that 39 points would round to. The map's ×1.5
/// Mob Rate isn't applied.
#[test]
fn the_map_summary_counts_every_untimed_spawn_point_as_meowdb_does() {
    let state = arrive_at(osms::game(), "Victoria Road", "Henesys Hunting Ground I");

    assert_eq!(data(&state).summary, summary((13_930, 153_216), (18_572, 204_285)));
}

/// Henesys Hunting Ground I, with an Iron Hog (EXP 52) on a spawn point timed to an hour:
/// one more mob and 52 more EXP an hour, alone or in a party.
#[test]
fn a_timed_spawn_point_gives_3600_over_its_timer_mobs_an_hour_alone_or_in_a_party() {
    let world = osms::game()
        .with_mob(mob(1_100_000, "Iron Hog", 27, 1000, 52))
        .changing(HENESYS_HUNTING_GROUND_I, |m| m.timed(1_100_000, 1, "3600.00"));

    let state = arrive_at(world, "Victoria Road", "Henesys Hunting Ground I");

    assert_eq!(data(&state).summary, summary((13_931, 153_268), (18_573, 204_337)));
}

/// For a ranged timer OSMS gives the average in brackets: 2 points on 180–1800 s,
/// averaging 720 s, give 10 Ghost Stumps (EXP 100) an hour; 3 on 20–60 s, averaging
/// 41.76 s, give 258.6 Pigs (EXP 7), rounded to 259 and 1,810 EXP.
#[test]
fn a_ranged_timer_counts_as_the_average_osms_gives() {
    let world = osms::world(
        vec![
            map(100_000_000, "Made-up Street", "Ghost Field")
                .timed(20, 2, "180.00-1800.00 [720.00]")
                .timed(21, 3, "20.00-60.00 [41.76]"),
        ],
        vec![mob(20, "Ghost Stump", 19, 300, 100), mob(21, "Pig", 7, 80, 7)],
    );

    let state = arrive_at(world, "Made-up Street", "Ghost Field");

    assert_eq!(data(&state).summary, summary((269, 2810), (269, 2810)));
}

#[test]
fn a_town_has_no_mobs_and_no_map_summary_which_is_not_an_error() {
    let mut minerva = Minerva::new(FakeSource::osms());

    let state = arrive(&mut minerva, KERNING, 0);

    let data = data(&state);
    assert_eq!(data.summary, None);
    assert_eq!(data.mobs, []);
}

/// Never partial figures: a file set whose mobs aren't as the reader expects isn't read
/// at all.
#[test]
fn a_file_set_whose_mobs_are_not_as_expected_is_a_format_change_with_no_map_shown() {
    let unlisted_mob = osms::game().changing(HENESYS_HUNTING_GROUND_I, |m| m.spawns(99_999, 2));
    let not_a_timer = osms::game().changing(HENESYS_HUNTING_GROUND_I, |m| m.timed(3, 1, "soon"));
    let no_timer = osms::game().changing(HENESYS_HUNTING_GROUND_I, |m| m.timed(3, 1, "0.00"));
    let files = osms::game().files();
    let altered = [
        ("a spawn point of a mob monsters.json doesn't have", unlisted_mob.files()),
        ("a timer that isn't one", not_a_timer.files()),
        ("a timer of nothing", no_timer.files()),
        ("monsters.json isn't JSON", minerva_core::osms::OsmsFiles { monsters: "<html>".into(), ..files.clone() }),
        ("a mob's EXP renamed", minerva_core::osms::OsmsFiles { monsters: files.monsters.replace("\"exp\"", "\"xp\""), ..files.clone() }),
        ("a spawn point's mob renamed", minerva_core::osms::OsmsFiles { maps: files.maps.replace("{\"id\":3,", "{\"mob\":3,"), ..files.clone() }),
    ];
    for (how, files) in altered {
        let mut minerva = Minerva::new(FakeSource::answering_osms(vec![common::fresh_files(files)]));

        let state = arrive(&mut minerva, KERNING, 0);

        assert_eq!(state.recognition, Recognition::FormatChanged, "{how}");
        assert_eq!(state.map, None, "{how}");
    }
}
