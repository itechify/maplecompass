//! Each mob the panel lists has a Mob Card, shown when the player hovers its name: its
//! image and the figures from its MeowDB page that the row doesn't show (until #58). Once
//! the shown map lists them, its listed mobs' pages (MeowDB) and images (maplestory.io)
//! are asked for one at a time, each site's paced, and each only once. Clicking a mob's name opens its MeowDB page.

mod common;

use common::osms;
use common::{FakeSource, arrive, requests_for, saved_mob_ids, saved_mob_page};
use compass_core::{
    Compass, DamageRange, Elements, HpPerExp, MapDataStatus, MobCard, MobDetails, MobPage, OSMS_AND_IMAGES_CREDIT,
    OSMS_CREDIT, PanelState, Request, SourceError,
};
use std::sync::{Arc, Mutex};

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

/// The figures on a card whose page is in, fresh.
fn details<'a>(state: &'a PanelState, id: &str) -> &'a MobDetails {
    match &card(state, id).page {
        MobPage::Ready(details) => details,
        page => panic!("mob {id}: {page:?}"),
    }
}

fn listed_ids(state: &PanelState) -> Vec<String> {
    let data = state.data.data().expect("the map's data");
    data.listed_mobs.iter().map(|m| m.id.clone()).collect()
}

/// The made-up game world, but Sunset Sky lists these mobs (by ID) and nothing else:
/// each "Mob <id>" on 4 spawn points, levels 10 up in the order given.
fn sunset_listing<S: ToString>(ids: &[S]) -> FakeSource {
    let ids: Vec<u64> = ids.iter().map(|id| id.to_string().parse().unwrap()).collect();
    let mut world = osms::game().changing(osms::SUNSET_SKY, |m| osms::Map { spawns: Vec::new(), ..m });
    for (i, &id) in ids.iter().enumerate() {
        world = world
            .with_mob(osms::mob(id, &format!("Mob {id}"), 10 + i as u32, 100, 10))
            .changing(osms::SUNSET_SKY, |m| m.spawns(id, 4));
    }
    FakeSource::with_world(world)
}

#[test]
fn each_listed_mob_has_a_card_with_the_figures_from_its_meowdb_page() {
    let mut compass = Compass::new(FakeSource::osms());
    arrive(&mut compass, SUNSET, 0);

    let state = tick_through(&mut compass, 400, 20_000);

    // In the order the panel lists them.
    let ids: Vec<&str> = state.mob_cards.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, listed_ids(&state));
    let blue_snail = card(&state, "3");
    assert_eq!(blue_snail.url, "https://meowdb.com/msclassic/monsters/3");
    assert_eq!(blue_snail.image.as_deref(), Some("image:3"));
    assert_eq!(
        *details(&state, "3"),
        MobDetails {
            elements: Elements::default(),
            touch_damage: Some(DamageRange { min: 5, max: 7 }),
            mp: Some(30),
            accuracy: Some(39),
            avoid: None,
            physical_defense: None,
            magic_defense: None,
            speed: "Slow".into(),
            movement: "Stays on its floor".into(),
            hp_per_exp: Some(HpPerExp { hundredths: 1275, rank: 34, ranked: 182 }),
        }
    );
}

#[test]
fn elements_read_as_weak_strong_and_immune() {
    let mut compass = Compass::new(sunset_listing(&["1004", "1005", "13", "6", "700000"]));
    arrive(&mut compass, SUNSET, 0);

    let state = tick_through(&mut compass, 400, 20_000);

    let elements = |id: &str| details(&state, id).elements.clone();
    let names = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
    assert_eq!(elements("1004"), Elements { weak: names(&["Fire"]), strong: names(&["Ice"]), immune: vec![] });
    assert_eq!(elements("1005"), Elements { weak: names(&["Ice"]), strong: names(&["Fire"]), immune: vec![] });
    assert_eq!(elements("13"), Elements { weak: vec![], strong: vec![], immune: names(&["Poison"]) });
    assert_eq!(elements("6"), Elements { weak: names(&["Fire"]), strong: vec![], immune: vec![] });
    // A boss's other tags ("Boss", "Plant", "Jump attack") aren't elements.
    assert_eq!(elements("700000"), Elements { weak: names(&["Fire"]), strong: vec![], immune: vec![] });
}

#[test]
fn a_card_says_how_its_mob_moves_and_shows_figures_meowdb_leaves_blank_as_none() {
    let mut compass = Compass::new(sunset_listing(&["17", "7", "700000", "13"]));
    arrive(&mut compass, SUNSET, 0);

    let state = tick_through(&mut compass, 400, 20_000);

    let stirge = details(&state, "17");
    assert_eq!((stirge.speed.as_str(), stirge.movement.as_str()), ("Normal", "Flies"));
    let slime = details(&state, "7");
    assert_eq!((slime.speed.as_str(), slime.movement.as_str()), ("Slow", "Jumps between floors"));
    assert_eq!((slime.physical_defense, slime.magic_defense), (Some(10), Some(20)));
    let mushmom = details(&state, "700000");
    assert_eq!(mushmom.speed, "Fast");
    // Mushmom's page has no MP line.
    assert_eq!(mushmom.mp, None);
    assert_eq!(mushmom.touch_damage, Some(DamageRange { min: 585, max: 797 }));
    assert_eq!(mushmom.hp_per_exp, Some(HpPerExp { hundredths: 10129, rank: 176, ranked: 182 }));
    let green_mushroom = details(&state, "13");
    assert_eq!((green_mushroom.accuracy, green_mushroom.avoid), (Some(79), Some(8)));
}

#[test]
fn a_mob_page_without_other_attributes_reads_with_no_mp() {
    // Pig's page lists no Other Attributes at all: no MP, no tags.
    let mut compass = Compass::new(sunset_listing(&["8", "10"]));
    arrive(&mut compass, SUNSET, 0);

    let state = tick_through(&mut compass, 400, 20_000);

    let pig = details(&state, "8");
    assert_eq!(pig.mp, None);
    assert_eq!(pig.touch_damage, Some(DamageRange { min: 16, max: 22 }));
    assert_eq!((pig.accuracy, pig.avoid), (Some(54), None));
    assert_eq!((pig.speed.as_str(), pig.movement.as_str()), ("Normal", "Jumps between floors"));
    assert_eq!(pig.hp_per_exp, Some(HpPerExp { hundredths: 985, rank: 15, ranked: 182 }));
    details(&state, "10");
}

#[test]
fn every_saved_mob_page_reads_into_a_card() {
    let ids = saved_mob_ids();
    assert!(ids.len() >= 12, "{ids:?}");
    // The panel lists 5 mobs at a time.
    for chunk in ids.chunks(5) {
        let mut compass = Compass::new(sunset_listing(chunk));
        arrive(&mut compass, SUNSET, 0);

        let state = tick_through(&mut compass, 400, 20_000);

        for id in listed_ids(&state) {
            details(&state, &id);
        }
    }
}

#[test]
fn a_mob_page_missing_one_of_the_cards_figures_reads_as_changed_never_as_a_partial_card() {
    let page = saved_mob_page("3").expect("Blue Snail's page");
    for (label, renamed) in [(">AVOID<", ">EVA<"), (">Touch DMG<", ">Contact<"), (">HP per EXP<", ">HP/EXP<")] {
        assert!(page.contains(label), "{label}");
        let source = FakeSource::osms().with_mob_page("3", page.replace(label, renamed));
        let mut compass = Compass::new(source);
        arrive(&mut compass, SUNSET, 0);

        let state = tick_through(&mut compass, 400, 20_000);

        assert_eq!(card(&state, "3").page, MobPage::FormatChanged, "{label} renamed");
        details(&state, "6");
    }
}

#[test]
fn while_a_mobs_page_is_on_its_way_its_card_says_so() {
    let mut compass = Compass::new(FakeSource::osms().mobs_uncached());
    let state = arrive(&mut compass, SUNSET, 0);

    assert!(!state.mob_cards.is_empty());
    assert!(state.mob_cards.iter().all(|c| c.page == MobPage::Loading && c.image.is_none()));

    let state = tick_through(&mut compass, 400, 30_000);
    assert!(state.mob_cards.iter().all(|c| matches!(c.page, MobPage::Ready(_)) && c.image.is_some()));
}

#[test]
fn when_meowdb_is_unreachable_a_card_says_so_or_shows_its_saved_copy_with_the_date() {
    let source = FakeSource::osms().with_mob_pages(|_| Err(SourceError::Unreachable("offline".into())));
    let mut compass = Compass::new(source);
    arrive(&mut compass, SUNSET, 0);
    let state = tick_through(&mut compass, 400, 20_000);
    assert!(state.mob_cards.iter().all(|c| c.page == MobPage::Unavailable), "{:?}", state.mob_cards);

    let mut compass = Compass::new(FakeSource::osms().serving_saved_copies(1_759_900_000_000));
    arrive(&mut compass, SUNSET, 0);
    let state = tick_through(&mut compass, 400, 20_000);
    let MobPage::SavedCopy { saved_at_unix_ms, details } = &card(&state, "3").page else {
        panic!("{:?}", card(&state, "3").page)
    };
    assert_eq!(*saved_at_unix_ms, 1_759_900_000_000);
    assert_eq!(details.accuracy, Some(39));
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
    details(&state, "3");
    assert_eq!(card(&state, "6").image.as_deref(), Some("image:6"));
}

#[test]
fn only_the_listed_mobs_pages_and_images_are_asked_for_and_each_only_once() {
    let source = FakeSource::osms();
    let (mobs, images) = (source.mob_requests.clone(), source.image_requests.clone());
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
        assert_eq!(requests_for(&mobs, id), expected, "mob {id}'s page");
        assert_eq!(requests_for(&images, id), expected, "mob {id}'s image");
    }
    for id in &sunset {
        assert_eq!((requests_for(&mobs, id), requests_for(&images, id)), (1, 1), "mob {id}");
    }
}

/// When each fetch started, by kind, ticking every 50 ms from `from_ms` up to `to_ms`.
fn fetch_starts(
    compass: &mut Compass,
    logs: &[(&'static str, &Arc<Mutex<Vec<String>>>)],
    from_ms: u64,
    to_ms: u64,
) -> Vec<(u64, &'static str)> {
    let mut starts = Vec::new();
    for at in (from_ms..=to_ms).step_by(50) {
        let before: Vec<usize> = logs.iter().map(|(_, log)| log.lock().unwrap().len()).collect();
        compass.tick(at);
        for ((kind, log), before) in logs.iter().zip(before) {
            let after = log.lock().unwrap().len();
            starts.extend(std::iter::repeat_n((at, *kind), after - before));
        }
    }
    starts
}

#[test]
fn mob_pages_come_one_at_a_time_and_every_meowdb_request_is_paced() {
    let source = FakeSource::osms().mobs_uncached();
    let logs = [("mob", &source.mob_requests.clone()), ("image", &source.image_requests.clone())];
    let mut compass = Compass::new(source);
    let state = arrive(&mut compass, SUNSET, 0);
    let listed = listed_ids(&state).len();

    let starts = fetch_starts(&mut compass, &logs, 400, 40_000);

    let meowdb: Vec<_> = starts.iter().filter(|(_, kind)| *kind == "mob").collect();
    assert_eq!(meowdb.len(), listed);
    assert!(meowdb.windows(2).all(|w| w[1].0 - w[0].0 >= 2_000), "MeowDB paced: {meowdb:?}");
    let images: Vec<_> = starts.iter().filter(|(_, kind)| *kind == "image").collect();
    assert_eq!(images.len(), listed);
    assert!(images.windows(2).all(|w| w[1].0 - w[0].0 >= 2_000), "maplestory.io paced: {images:?}");
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
