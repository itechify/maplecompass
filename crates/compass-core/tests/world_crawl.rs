//! MapleCompass crawls the MeowDB page of every map in OSMS's map list in the background
//! (ADR 0004, until #57), politely: about one page every 2 seconds, unless the source has
//! it cached. The panel says how many of the list's maps are known so far, and
//! recognition and the shown map's own data keep working throughout.

mod common;

use common::osms::made_up_world;
use common::{FakeSource, arrive, fresh, requests_for as requests_in};
use compass_core::{Compass, CrawlProgress, MapData, MapDataStatus, PanelState, SourceError};
use std::sync::Mutex;
use std::task::Poll;

const KERNING: &str = "victoria-road__kerning-city--window";
const KERNING_CITY: &str = "010003000";

/// A source serving `n` made-up maps, each without mobs or portals.
fn blank_world(n: u64) -> FakeSource {
    FakeSource::with_world(made_up_world(n))
}

/// The made-up maps' IDs.
fn ids(n: u64) -> Vec<String> {
    made_up_world(n).maps.iter().map(|m| format!("{:09}", m.id)).collect()
}

/// How many of the map list's maps are known, out of how many.
fn known(state: &PanelState) -> (u32, u32) {
    let CrawlProgress { known, total, .. } = state.search.crawl;
    (known, total)
}

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

#[test]
fn once_the_map_list_arrives_every_map_in_it_is_crawled_once_and_counted_as_known() {
    let source = blank_world(5);
    let crawled = source.crawl_requests.clone();
    let mut compass = Compass::new(source);

    let state = tick_through(&mut compass, 0, 5_000);

    assert_eq!(known(&state), (5, 5));
    assert!(state.search.crawl.done);
    assert_eq!(*crawled.lock().unwrap(), ids(5));
}

/// When each crawl fetch started, ticking every 50 ms from 0 up to `to_ms`.
fn crawl_start_times(compass: &mut Compass, crawled: &Mutex<Vec<String>>, to_ms: u64) -> Vec<u64> {
    let mut starts = Vec::new();
    for at in (0..=to_ms).step_by(50) {
        let before = crawled.lock().unwrap().len();
        compass.tick(at);
        let after = crawled.lock().unwrap().len();
        starts.extend(std::iter::repeat_n(at, after - before));
    }
    starts
}

#[test]
fn pages_that_come_over_the_network_are_asked_for_no_more_than_once_every_2_seconds() {
    let source = blank_world(6).crawling_uncached();
    let crawled = source.crawl_requests.clone();
    let mut compass = Compass::new(source);

    let starts = crawl_start_times(&mut compass, &crawled, 20_000);

    assert_eq!(starts.len(), 6, "every map asked for once: {starts:?}");
    assert!(starts.windows(2).all(|w| w[1] - w[0] >= 2_000), "paced: {starts:?}");
    assert_eq!(*crawled.lock().unwrap(), ids(6));
    let state = compass.tick(20_050);
    assert_eq!(known(state), (6, 6));
    assert!(state.search.crawl.done);
}

#[test]
fn while_the_crawl_runs_the_panel_says_how_many_maps_are_known_so_far() {
    let mut compass = Compass::new(blank_world(6).crawling_uncached());

    let state = tick_through(&mut compass, 0, 4_500);

    assert_eq!(known(&state), (3, 6));
    assert!(!state.search.crawl.done);
}

#[test]
fn before_the_map_list_arrives_no_maps_are_known_of_none() {
    let mut compass = Compass::new(FakeSource::answering_osms(vec![Poll::Pending, Poll::Pending, fresh(made_up_world(6))]));

    let state = compass.tick(0);

    assert_eq!(known(state), (0, 0));
    assert!(!state.search.crawl.done);
}

#[test]
fn the_shown_maps_data_never_waits_for_the_crawl() {
    let source = FakeSource::osms().crawling_uncached();
    let crawled = source.crawl_requests.clone();
    let mut compass = Compass::new(source);
    // A crawl fetch starts at 2 s and is still in flight when the player arrives.
    tick_through(&mut compass, 0, 2_000);

    let state = arrive(&mut compass, KERNING, 2_010);

    assert_eq!(state.map.as_ref().map(|m| m.id.as_str()), Some(KERNING_CITY));
    assert!(matches!(state.data, MapDataStatus::Ready(_)), "{:?}", state.data);

    let state = tick_through(&mut compass, 2_400, 10_000);
    assert!(matches!(state.data, MapDataStatus::Ready(_)));
    // Fetches started at 0, 2, 4, 6, 8 and 10 s.
    assert_eq!(crawled.lock().unwrap().len(), 6, "the crawl went on at its pace");
}

#[test]
fn an_unreachable_page_is_asked_for_again_later_and_a_page_in_a_changed_format_is_skipped() {
    let index = ids(4);
    let (flaky, changed) = (index[1].clone(), index[2].clone());
    let mut asked_flaky = 0;
    let source = blank_world(4).with_pages(move |id| {
        Poll::Ready(if id == changed {
            Err(SourceError::FormatChanged("new layout".into()))
        } else if id == flaky && {
            asked_flaky += 1;
            asked_flaky == 1
        } {
            Err(SourceError::Unreachable("timed out".into()))
        } else {
            Ok(MapData::default())
        })
    });
    let crawled = source.crawl_requests.clone();
    let mut compass = Compass::new(source);

    let state = tick_through(&mut compass, 0, 10_000);
    assert_eq!(known(&state), (2, 4));
    assert!(!state.search.crawl.done);
    assert_eq!(requests_in(&crawled, &index[1]), 1, "not retried straight away");

    let state = tick_through(&mut compass, 10_000, 15 * 60_000);
    assert_eq!(known(&state), (3, 4));
    assert!(state.search.crawl.done);
    assert_eq!(requests_in(&crawled, &index[1]), 2);
    assert_eq!(requests_in(&crawled, &index[2]), 1, "a changed page isn't asked for again");
    assert_eq!(crawled.lock().unwrap().len(), 5);
}

#[test]
fn an_unreachable_meowdb_is_not_asked_over_and_over() {
    let source = blank_world(3).with_pages_offline().crawling_uncached();
    let crawled = source.crawl_requests.clone();
    let mut compass = Compass::new(source);

    let state = tick_through(&mut compass, 0, 60_000);

    assert_eq!(crawled.lock().unwrap().len(), 3);
    assert_eq!(known(&state), (0, 3));
    assert!(!state.search.crawl.done);
}

/// When a saved copy was made: 2026-10-01 00:00 UTC, in ms since the Unix epoch.
const SAVED_AT: u64 = 1_790_812_800_000;

#[test]
fn saved_copies_count_as_known_so_the_crawl_works_offline() {
    let mut compass = Compass::new(blank_world(4).serving_saved_copies(SAVED_AT));

    let state = tick_through(&mut compass, 0, 1_000);

    assert_eq!(known(&state), (4, 4));
    assert!(state.search.crawl.done);
}

#[test]
fn pages_the_source_has_cached_let_the_crawl_move_on_without_waiting_out_2_seconds() {
    let mut compass = Compass::new(blank_world(30));

    let state = tick_through(&mut compass, 0, 500);

    assert_eq!(known(&state), (30, 30));
}
