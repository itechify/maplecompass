//! What the world crawl learns is kept between launches (ADR 0004): the core hands the
//! shell every map it knows, with the date its data is from, and takes it back at the
//! next launch, so search, Distance and routes know the whole world at once instead of
//! reading hundreds of cached pages again. A map whose data is over a week old still
//! counts as known; it is refreshed in the background, oldest first and no more than
//! one every 30 seconds, while maps never seen are fetched at the crawl's usual pace.

mod common;

use common::{FakeSource, made_up_index};
use compass_core::{Compass, CrawlProgress, MapData, PanelState, SavedWorld, WORLD_VERSION};
use std::task::Poll;

const DAY: u64 = 24 * 60 * 60 * 1000;
/// The wall clock at the second launch: 2026-10-09 00:00 UTC, ms since the Unix epoch.
const NOW: u64 = 1_791_504_000_000;

/// A source serving `n` made-up maps, each a map without mobs or portals, answering
/// crawl requests at once.
fn blank_world(n: usize) -> FakeSource {
    FakeSource::with_index(made_up_index(n)).with_pages(|_| Poll::Ready(Ok(MapData::default())))
}

fn progress(state: &PanelState) -> (u32, u32, bool) {
    let CrawlProgress { known, total, done } = state.search.crawl;
    (known, total, done)
}

/// Tick every 50 ms from `from_ms` up to `to_ms`.
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

/// The world a first launch learned: `n` maps crawled at `crawled_at` (wall clock), as
/// the shell would save it (JSON).
fn crawled_world(n: usize, crawled_at: u64) -> SavedWorld {
    let mut first = Compass::new(blank_world(n));
    first.on_wall_clock(crawled_at);
    tick_through(&mut first, 0, 1_000);
    let json = serde_json::to_string(&first.world()).unwrap();
    serde_json::from_str(&json).unwrap()
}

/// A second launch, whose shell is loading the saved world, at wall clock `now`.
fn relaunch(source: FakeSource, now: u64) -> Compass {
    let mut compass = Compass::new(source);
    compass.on_wall_clock(now);
    compass.await_world();
    compass
}

/// `world` with map `i`'s data dated `as_of_unix_ms`.
fn dated(mut world: SavedWorld, i: usize, as_of_unix_ms: u64) -> SavedWorld {
    world.maps[i].as_of_unix_ms = as_of_unix_ms;
    world
}

#[test]
fn a_launch_with_the_saved_world_knows_every_map_at_once_and_asks_for_none() {
    let world = crawled_world(5, NOW - DAY);
    let source = blank_world(5).crawling_uncached();
    let crawled = source.crawl_requests.clone();
    let mut compass = relaunch(source, NOW);

    compass.restore_world(world);
    let state = tick_through(&mut compass, 0, 60_000);

    assert_eq!(progress(&state), (5, 5, true));
    assert!(crawled.lock().unwrap().is_empty(), "{:?}", crawled.lock().unwrap());
}

#[test]
fn while_the_saved_world_is_loading_the_crawl_waits_for_it() {
    let source = blank_world(5).crawling_uncached();
    let crawled = source.crawl_requests.clone();
    let mut compass = relaunch(source, NOW);

    let state = tick_through(&mut compass, 0, 10_000);
    assert!(crawled.lock().unwrap().is_empty());
    assert_eq!(progress(&state), (0, 5, false));

    // None was saved (a first launch): the crawl starts.
    compass.restore_world(SavedWorld::default());
    tick_through(&mut compass, 10_000, 20_000);
    assert_eq!(crawled.lock().unwrap().len(), 5);
}

#[test]
fn a_world_saved_in_another_format_is_left_and_every_map_is_crawled_again() {
    let world = SavedWorld { version: WORLD_VERSION + 1, ..crawled_world(3, NOW - DAY) };
    let source = blank_world(3);
    let crawled = source.crawl_requests.clone();
    let mut compass = relaunch(source, NOW);

    compass.restore_world(world);
    tick_through(&mut compass, 0, 1_000);

    assert_eq!(crawled.lock().unwrap().len(), 3);
}

#[test]
fn maps_over_a_week_old_still_count_as_known_and_refresh_oldest_first_one_every_30_seconds() {
    let index = made_up_index(3);
    let world = crawled_world(3, NOW - DAY);
    let world = dated(dated(world, 0, NOW - 8 * DAY), 1, NOW - 10 * DAY);
    let source = blank_world(3).crawling_uncached();
    let crawled = source.crawl_requests.clone();
    let mut compass = relaunch(source, NOW);
    compass.restore_world(world);

    // Known from the start, and the panel doesn't say it is mapping the world.
    let mut starts = Vec::new();
    for at in (0..=90_000).step_by(50) {
        let before = crawled.lock().unwrap().len();
        let state = compass.tick(at);
        assert_eq!(progress(state), (3, 3, true), "at {at} ms");
        starts.extend(std::iter::repeat_n(at, crawled.lock().unwrap().len() - before));
    }

    // The 10-day-old map first, then the 8-day-old one; the day-old one waits its turn.
    assert_eq!(*crawled.lock().unwrap(), [index[1].id.clone(), index[0].id.clone()]);
    assert!(starts[1] - starts[0] >= 30_000, "{starts:?}");
    let saved = compass.world();
    let as_of = |id: &str| saved.maps.iter().find(|m| m.id == id).unwrap().as_of_unix_ms;
    assert_eq!((as_of(&index[0].id), as_of(&index[1].id), as_of(&index[2].id)), (NOW, NOW, NOW - DAY));
}

#[test]
fn maps_never_seen_come_at_the_crawl_s_pace_before_any_refresh() {
    let index = made_up_index(4);
    let world = SavedWorld { maps: vec![crawled_world(1, NOW - 10 * DAY).maps.remove(0)], ..SavedWorld::default() };
    let source = blank_world(4).crawling_uncached();
    let crawled = source.crawl_requests.clone();
    let mut compass = relaunch(source, NOW);
    compass.restore_world(world);

    let mapping = tick_through(&mut compass, 0, 2_100);
    assert_eq!(progress(&mapping), (3, 4, false), "the panel says it is mapping the world");
    let state = tick_through(&mut compass, 2_100, 40_000);

    assert_eq!(progress(&state), (4, 4, true));
    let ids: Vec<String> = index.into_iter().map(|m| m.id).collect();
    assert_eq!(*crawled.lock().unwrap(), [ids[1].clone(), ids[2].clone(), ids[3].clone(), ids[0].clone()]);
}

#[test]
fn a_refresh_that_fails_keeps_the_map_known_and_waits_before_trying_again() {
    let world = dated(crawled_world(1, NOW), 0, NOW - 8 * DAY);
    let source = FakeSource::with_index(made_up_index(1)).with_pages_offline().crawling_uncached();
    let crawled = source.crawl_requests.clone();
    let mut compass = relaunch(source, NOW);
    compass.restore_world(world);

    let state = tick_through(&mut compass, 0, 4 * 60_000);
    assert_eq!(progress(&state), (1, 1, true));
    assert_eq!(crawled.lock().unwrap().len(), 1, "not asked again within 5 minutes");

    tick_through(&mut compass, 4 * 60_000, 6 * 60_000);
    assert_eq!(crawled.lock().unwrap().len(), 2);
}

#[test]
fn a_saved_world_arriving_after_the_crawl_started_keeps_the_newer_of_each_map() {
    let source = blank_world(2).crawling_uncached();
    let crawled = source.crawl_requests.clone();
    let mut compass = relaunch(source, NOW);
    compass.restore_world(SavedWorld::default());
    tick_through(&mut compass, 0, 3_000);
    assert_eq!(crawled.lock().unwrap().len(), 2);

    // An older saved world for both: the maps just fetched stay as they are.
    let state = compass.restore_world(crawled_world(2, NOW - 3 * DAY)).clone();
    assert_eq!(progress(&state), (2, 2, true));
    assert!(compass.world().maps.iter().all(|m| m.as_of_unix_ms == NOW));
}

#[test]
fn crawl_progress_reaches_the_panel_at_most_about_once_a_second() {
    // Answered at once, eight maps a tick.
    let mut compass = Compass::new(blank_world(60));

    let mut changes = Vec::new();
    let mut last = None;
    for at in (0..=3_000).step_by(50) {
        let crawl = compass.tick(at).search.crawl;
        if last != Some(crawl) {
            changes.push((at, crawl));
            last = Some(crawl);
        }
    }

    let (end, finished) = *changes.last().unwrap();
    assert!(finished.done && finished.known == 60, "{changes:?}");
    for pair in changes.windows(2) {
        let ((from, _), (to, crawl)) = (pair[0], pair[1]);
        assert!(to - from >= 1_000 || crawl.done, "{changes:?}");
    }
    assert!(end < 1_000, "being done shows at once: {changes:?}");
}
