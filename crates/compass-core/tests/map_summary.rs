//! Once the panel is sure of a map, it fills in that map's hunting information from its
//! MeowDB page: the Map Summary and the mobs, sorted by level. Pages are the ones saved
//! on 2026-10-07 (personal-use test data, ADR 0003), served by a fake map data source.

mod common;

use common::{FakeSource, sample_frame, saved_index, saved_page};
use compass_core::{
    Compass, MapDataStatus, MapEntry, MapSummary, Mob, PanelState, PerParty,
};

const SUNSET: &str = "victoria-road__sunset-sky--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const SUNSET_SKY: &str = "010003070";
const DEAD_MINE_IV: &str = "020001067";

/// Two confident reads of a sample: the panel switches to its map.
fn arrive(compass: &mut Compass, sample: &str, at_ms: u64) -> PanelState {
    compass.on_frame(sample_frame(sample, at_ms));
    compass.on_frame(sample_frame(sample, at_ms + 333)).clone()
}

fn mob(name: &str, level: u32, hp: u64, exp: u64, spawn_count: u32, spawn_share_percent: u32) -> Mob {
    Mob { name: name.into(), level, hp, exp, spawn_count, spawn_share_percent }
}

#[test]
fn arriving_on_sunset_sky_shows_its_map_summary_and_mobs_sorted_by_level() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = arrive(&mut compass, SUNSET, 0);

    let MapDataStatus::Ready(data) = &state.data else { panic!("{:?}", state.data) };
    assert_eq!(
        data.summary,
        Some(MapSummary {
            mobs_per_hour: PerParty { solo: 15714, full_party: 20952 },
            exp_per_hour: PerParty { solo: 223928, full_party: 298572 },
        })
    );
    assert_eq!(
        data.mobs,
        [
            mob("Blue Snail", 2, 51, 4, 7, 16),
            mob("Stump", 5, 133, 10, 15, 34),
            mob("Orange Mushroom", 8, 142, 15, 8, 18),
            mob("Dark Stump", 11, 236, 21, 7, 16),
            mob("Green Mushroom", 14, 233, 26, 7, 16),
        ]
    );
}

#[test]
fn arriving_on_construction_site_north_of_kerning_city_shows_its_map_summary_and_mobs() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = arrive(&mut compass, "victoria-road__construction-site-north-of-kerning-city--window", 0);

    let MapDataStatus::Ready(data) = &state.data else { panic!("{:?}", state.data) };
    assert_eq!(
        data.summary,
        Some(MapSummary {
            mobs_per_hour: PerParty { solo: 19285, full_party: 25714 },
            exp_per_hour: PerParty { solo: 297499, full_party: 396667 },
        })
    );
    assert_eq!(
        data.mobs,
        [
            mob("Red Snail", 4, 68, 8, 23, 43),
            mob("Orange Mushroom", 8, 142, 15, 9, 17),
            mob("Dark Stump", 11, 236, 21, 11, 20),
            mob("Octopus", 12, 191, 23, 8, 15),
            mob("Axe Stump", 17, 371, 32, 2, 4),
            mob("Blue Mushroom", 19, 342, 35, 1, 2),
        ]
    );
}

#[test]
fn a_town_has_no_mobs_and_no_map_summary_which_is_not_an_error() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = arrive(&mut compass, KERNING, 0);

    let MapDataStatus::Ready(data) = &state.data else { panic!("{:?}", state.data) };
    assert_eq!(data.summary, None);
    assert_eq!(data.mobs, []);
}

/// Dead Mine IV has no minimap sample, so the index here files the Sunset Sky title
/// under Dead Mine IV's ID.
#[test]
fn hp_in_the_thousands_is_read_in_full() {
    let index = saved_index()
        .into_iter()
        .map(|m| if m.map_name == "Sunset Sky" { MapEntry { id: DEAD_MINE_IV.into(), ..m } } else { m })
        .collect();
    let mut compass = Compass::new(FakeSource::with_index(index));

    let state = arrive(&mut compass, SUNSET, 0);

    let MapDataStatus::Ready(data) = &state.data else { panic!("{:?}", state.data) };
    assert_eq!(
        data.mobs,
        vec![mob("Flyeye", 41, 1419, 99, 18, 64), mob("Minor Zombie", 58, 3619, 197, 10, 36)]
    );
}

#[test]
fn a_page_whose_format_changed_is_reported_as_such_with_no_partial_data_and_the_map_still_shown() {
    let page = saved_page(SUNSET_SKY).unwrap();
    let half = (0..=page.len() / 2).rev().find(|&n| page.is_char_boundary(n)).unwrap();
    let altered = [
        ("no embedded data", page.replace("self.__next_f.push(", "self.__next_f.add(")),
        ("summary's mob figures renamed", page.replace("perMob", "mobFigures")),
        ("no Map Summary although mobs spawn", page.replace(r#"\"rates\""#, r#"\"yields\""#)),
        ("no Monsters table", page.replace(">Monsters (", ">Mobs (")),
        ("a mob's HP not a number", page.replace(">133<", ">lots<")),
        ("cut off halfway", page[..half].to_string()),
        ("another map's page", saved_page("010003080").unwrap()),
    ];
    for (how, html) in altered {
        let mut compass = Compass::new(FakeSource::meowdb().with_page(SUNSET_SKY, html));

        let state = arrive(&mut compass, SUNSET, 0);

        assert_eq!(state.data, MapDataStatus::FormatChanged, "{how}");
        assert_eq!(state.map.as_ref().map(|m| m.map_name.as_str()), Some("Sunset Sky"), "{how}");
    }
}
