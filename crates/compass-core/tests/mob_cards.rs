//! Each mob the panel lists has a Mob Card, shown when the player hovers its name: OSMS's
//! animation of the mob, and the figures from OSMS's set that the row doesn't show. The
//! figures come with the set, so a card is ready as soon as its map is shown. Clicking a
//! mob's name opens its MeowDB page.

mod common;

use common::osms;
use common::{FakeSource, arrive, requests_for};
use compass_core::{
    Compass, DamageRange, HpPerExp, MapDataStatus, MobCard, MobDetails, OSMS_AND_IMAGES_CREDIT, OSMS_CREDIT, PanelState,
    Request, SourceError,
};

const SUNSET: &str = "victoria-road__sunset-sky--window";
const KERNING: &str = "victoria-road__kerning-city--window";
const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";

/// Tick every 50 ms (the shell ticks at least that often) from `from_ms` up to `to_ms`.
fn tick_through(compass: &mut Compass, from_ms: u64, to_ms: u64) -> PanelState {
    let mut at = from_ms;
    loop {
        let state = compass.tick(at).clone();
        if at >= to_ms {
            return state;
        }
        at = (at + 50).min(to_ms);
    }
}

/// The card of the listed mob with this ID.
fn card<'a>(state: &'a PanelState, id: &str) -> &'a MobCard {
    state.mob_cards.iter().find(|c| c.id == id).unwrap_or_else(|| panic!("no card for mob {id}"))
}

/// The figures on the card of the listed mob with this ID.
fn details<'a>(state: &'a PanelState, id: &str) -> &'a MobDetails {
    &card(state, id).details
}

fn listed_ids(state: &PanelState) -> Vec<String> {
    let data = state.data.data().expect("the map's data");
    data.listed_mobs.iter().map(|m| m.id.clone()).collect()
}

/// The made-up game world, but Sunset Sky lists these mobs and nothing else, each on 4
/// spawn points.
fn sunset_listing(mobs: Vec<osms::Mob>) -> FakeSource {
    let mut world = osms::game().changing(osms::SUNSET_SKY, |m| osms::Map { spawns: Vec::new(), ..m });
    for mob in mobs {
        let id = mob.id;
        world = world.with_mob(mob).changing(osms::SUNSET_SKY, |m| m.spawns(id, 4));
    }
    FakeSource::with_world(world)
}

#[test]
fn a_listed_mobs_card_shows_its_figures_from_the_set_as_soon_as_the_map_is_shown() {
    let blue_snail = osms::mob(3, "Blue Snail", 2, 51, 4).p_atk(5).p_def(0).mp(30).acc(39).speed(-50);
    let mushmom = osms::mob(700_000, "Mushmom", 40, 34_440, 340)
        .p_atk(532)
        .p_def(15)
        .m_def(25)
        .mp(200)
        .acc(130)
        .avoid(29)
        .speed(5);
    let mut compass = Compass::new(sunset_listing(vec![blue_snail, mushmom]));

    let state = arrive(&mut compass, SUNSET, 0);

    // In the order the panel lists them.
    let ids: Vec<&str> = state.mob_cards.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["3", "700000"]);
    let blue_snail = details(&state, "3");
    assert_eq!(blue_snail.touch_damage, Some(DamageRange { min: 5, max: 7 }));
    assert_eq!((blue_snail.mp, blue_snail.accuracy), (Some(30), Some(39)));
    // P.DEF is 0, and there's no AVOID or M.DEF at all.
    assert_eq!((blue_snail.avoid, blue_snail.physical_defense, blue_snail.magic_defense), (None, None, None));
    assert_eq!(blue_snail.speed, "Slow");
    let mushmom = details(&state, "700000");
    assert_eq!(mushmom.touch_damage, Some(DamageRange { min: 585, max: 797 }));
    assert_eq!((mushmom.mp, mushmom.accuracy, mushmom.avoid), (Some(200), Some(130), Some(29)));
    assert_eq!((mushmom.physical_defense, mushmom.magic_defense), (Some(15), Some(25)));
    assert_eq!(mushmom.speed, "Fast");
}

#[test]
fn hp_per_exp_is_ranked_among_all_the_sets_mobs() {
    let mut compass = Compass::new(sunset_listing(vec![
        // 0.125, rounded half up.
        osms::mob(9001, "Lightest", 10, 1, 8),
        osms::mob(9002, "Tied", 11, 20, 10),
        osms::mob(9003, "Also Tied", 12, 40, 20),
        // 101.2941…
        osms::mob(700_000, "Mushmom", 40, 34_440, 340),
        // No EXP, so no HP per EXP: below every other mob.
        osms::mob(9004, "No EXP", 13, 500, 0),
    ]));

    let state = arrive(&mut compass, SUNSET, 0);

    // The set's mobs: the made-up game's, all between 8 and 14 HP per EXP, and these 5.
    let ranked = osms::game_mobs().len() as u32 + 5;
    let hp_per_exp = |id: &str| details(&state, id).hp_per_exp;
    assert_eq!(hp_per_exp("9001"), Some(HpPerExp { hundredths: 13, rank: 1, ranked }));
    assert_eq!(hp_per_exp("9002"), Some(HpPerExp { hundredths: 200, rank: 2, ranked }));
    assert_eq!(hp_per_exp("9003"), Some(HpPerExp { hundredths: 200, rank: 2, ranked }));
    assert_eq!(hp_per_exp("700000"), Some(HpPerExp { hundredths: 10129, rank: ranked - 1, ranked }));
    assert_eq!(hp_per_exp("9004"), None);
}

#[test]
fn a_mob_without_an_image_still_has_its_card() {
    let source = FakeSource::osms().with_images(|id| {
        if id == "3" { Err(SourceError::FormatChanged("no such mob".into())) } else { Ok(format!("image:{id}")) }
    });
    let mut compass = Compass::new(source);
    arrive(&mut compass, SUNSET, 0);

    let state = tick_through(&mut compass, 400, 20_000);

    assert_eq!(card(&state, "3").image, None);
    assert_eq!(details(&state, "3").speed, "Normal");
    assert_eq!(card(&state, "6").image.as_deref(), Some("image:6"));
}

#[test]
fn only_the_listed_mobs_images_are_asked_for_and_each_only_once() {
    let source = FakeSource::osms();
    let images = source.image_requests.clone();
    let mut compass = Compass::new(source);

    let state = arrive(&mut compass, CONSTRUCTION_SITE, 0);
    let construction_site = listed_ids(&state);
    let all_mobs: Vec<String> =
        state.data.data().expect("data").mobs.iter().map(|m| m.id.clone()).collect();
    assert!(all_mobs.len() > construction_site.len(), "a map with more mobs than the panel lists");
    tick_through(&mut compass, 400, 20_000);
    arrive(&mut compass, KERNING, 20_050);
    tick_through(&mut compass, 20_400, 40_000);
    let state = arrive(&mut compass, SUNSET, 40_050);
    let sunset = listed_ids(&state);
    tick_through(&mut compass, 40_400, 60_000);
    arrive(&mut compass, CONSTRUCTION_SITE, 60_050);
    tick_through(&mut compass, 60_400, 80_000);

    for id in &all_mobs {
        let expected = usize::from(construction_site.contains(id) || sunset.contains(id));
        assert_eq!(requests_for(&images, id), expected, "mob {id}'s image");
    }
    for id in &sunset {
        assert_eq!(requests_for(&images, id), 1, "mob {id}");
    }
}

#[test]
fn without_a_map_or_its_mobs_there_are_no_cards() {
    let mut compass = Compass::new(FakeSource::osms());
    assert!(compass.tick(0).mob_cards.is_empty());

    // Kerning City is a town.
    let state = arrive(&mut compass, KERNING, 100);
    assert!(matches!(state.data, MapDataStatus::Ready(_)));
    assert!(state.mob_cards.is_empty());
}

#[test]
fn clicking_a_listed_mobs_name_opens_its_meowdb_page() {
    let mut compass = Compass::new(FakeSource::osms());
    arrive(&mut compass, SUNSET, 0);

    assert_eq!(card(compass.panel_state(), "3").url, "https://meowdb.com/msclassic/monsters/3");
    assert_eq!(
        compass.open_mob_page("3"),
        Some(Request::OpenInBrowser { url: "https://meowdb.com/msclassic/monsters/3".into() })
    );
    // Only the mobs the panel lists.
    assert_eq!(compass.open_mob_page("700000"), None);
}

#[test]
fn the_panel_credits_maplestory_io_for_the_mob_images() {
    let mut compass = Compass::new(FakeSource::osms());

    let state = arrive(&mut compass, SUNSET, 0);
    assert_eq!(state.credit, Some(OSMS_AND_IMAGES_CREDIT));

    let state = arrive(&mut compass, KERNING, 1_000);
    assert_eq!(state.credit, Some(OSMS_CREDIT));
}
