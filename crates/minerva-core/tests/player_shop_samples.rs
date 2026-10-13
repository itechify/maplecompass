//! Every Player Shop sample, fed through the core's public inputs as a Deal Finder capture,
//! shows a Deal Badge for each listing not sold out, about the item and price the sample's
//! readings say, or a "?" badge: never another item, never another price (ADR 0014).
//!
//! - Each sample is a frame cropped to a shop's item list, as a Bug Report's
//!   `crops/player-shop-<ms>.png` is, so no seller's name, shop title or chat is committed;
//!   beside it, `<name>.json` lists its listings not sold out, top to bottom, each with the
//!   item's name as the game shows it ("…" and all, for a name it cuts short), the quantity
//!   and the mesos; or `{ "covered": true }` for one the pointer or the game's tooltip
//!   covers. Read on its own, a covered listing's badge says it is covered, or it has none:
//!   with no capture before, a slot the tooltip hides wholly isn't known to hold a listing.
//! - Each frame is fed as the game's client area at the screen's origin, as it is and as
//!   the darker colours of live captures (`prep.rs`), to a core of its own.

mod common;

use common::FakeSource;
use common::items::{ItemWorld, etc, item_world};
use minerva_core::{
    DealBadge, Frame, GameWindow, ListedPrice, Minerva, PanelState, ScreenPoint, ScreenRect, Settings, ShopCapture, Size,
    Verdict,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/player-shop")
}

/// One listing as a sample's readings write it: read, or covered.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Listing {
    Read { name: String, quantity: u64, mesos: u64 },
    Covered { covered: bool },
}

struct Sample {
    file: String,
    listings: Vec<Listing>,
}

fn samples() -> Vec<Sample> {
    let mut files: Vec<String> = std::fs::read_dir(samples_dir())
        .expect("samples/player-shop")
        .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".png"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|file| {
            let readings = samples_dir().join(file.replace(".png", ".json"));
            let text = std::fs::read_to_string(&readings).unwrap_or_else(|e| panic!("{}: {e}", readings.display()));
            let listings: Vec<Listing> = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", readings.display()));
            let written = listings.iter().all(|l| !matches!(l, Listing::Covered { covered: false }));
            assert!(written, "{}: a listing is read, or `\"covered\": true`", readings.display());
            Sample { file, listings }
        })
        .collect()
}

/// The item a listing's name stands for: the name, or for one the game cut short, a
/// made-up name that starts as it does.
fn item_name(listing: &str) -> String {
    match listing.strip_suffix("...").or_else(|| listing.strip_suffix('…')) {
        Some(start) => format!("{start}egs for Made Up"),
        None => listing.to_string(),
    }
}

/// Made-up item data with an item of each listing's name.
fn sample_items(samples: &[Sample]) -> ItemWorld {
    let mut names: Vec<String> = samples
        .iter()
        .flat_map(|s| &s.listings)
        .filter_map(|l| match l {
            Listing::Read { name, .. } => Some(item_name(name)),
            Listing::Covered { .. } => None,
        })
        .collect();
    names.sort();
    names.dedup();
    let mut items = item_world(Vec::new());
    for (n, name) in names.iter().enumerate() {
        items = items.with(etc(4_100_000 + n as u64, name).sells_for(10));
    }
    items
}

/// A core over a 1920x1200 game window at the screen's origin, with the samples' items in
/// and the Deal Finder on.
fn started(samples: &[Sample]) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms_offline().with_items(sample_items(samples)));
    minerva.restore(Settings::default());
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    minerva.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    minerva.set_deal_badges_size(Size { width: 160, height: 0 });
    minerva.tick(0);
    assert!(minerva.on_hotkey("ctrl+shift+d").is_none());
    minerva
}

/// The sample's frame, its colours times `brightness`.
fn frame(sample: &Sample, brightness: f32) -> Frame {
    let img = image::open(samples_dir().join(&sample.file)).unwrap_or_else(|e| panic!("{}: {e}", sample.file)).to_rgba8();
    let mut rgba = img.as_raw().clone();
    for (i, channel) in rgba.iter_mut().enumerate() {
        if i % 4 != 3 {
            *channel = (*channel as f32 * brightness).round() as u8;
        }
    }
    Frame { width: img.width(), height: img.height(), rgba, at_ms: 1_000 }
}

fn badges(state: &PanelState) -> Vec<DealBadge> {
    state.deal_badges.as_ref().map(|shown| shown.badges.clone()).unwrap_or_default()
}

/// What a badge says the listing is: its item's name and price; `None` for "?".
fn read(badge: &DealBadge) -> Option<(String, ListedPrice)> {
    match &badge.verdict {
        Verdict::Deal { item, listed, .. } | Verdict::NoDeal { item, listed, .. } | Verdict::NoPrice { item, listed } => {
            Some((item.name.clone(), *listed))
        }
        Verdict::Unknown => None,
    }
}

#[test]
fn every_samples_listings_read_as_its_readings_say_or_as_unreadable_never_wrong() {
    let samples = samples();
    assert!(samples.len() >= 3, "expected the Player Shop samples, found {}", samples.len());
    let mut wrong = Vec::new();
    let mut unreadable = 0;
    for sample in &samples {
        for brightness in [1.0, 0.935] {
            let mut minerva = started(&samples);
            let capture = ShopCapture { frame: frame(sample, brightness), origin: ScreenPoint { x: 0, y: 0 } };
            let badges = badges(minerva.on_player_shop_capture(capture));
            let (covered, uncovered): (Vec<&DealBadge>, Vec<&DealBadge>) = badges.iter().partition(|b| b.covered);
            let covers = sample.listings.iter().filter(|l| matches!(l, Listing::Covered { .. })).count();
            let reads: Vec<(&String, u64, u64)> = sample
                .listings
                .iter()
                .filter_map(|l| match l {
                    Listing::Read { name, quantity, mesos } => Some((name, *quantity, *mesos)),
                    Listing::Covered { .. } => None,
                })
                .collect();
            if uncovered.len() != reads.len() || covered.len() > covers {
                let found = format!("{} listings found and {} covered", uncovered.len(), covered.len());
                wrong.push(format!("{} ×{brightness}: {found}, not {} and up to {covers}", sample.file, reads.len()));
                continue;
            }
            for (badge, (name, quantity, mesos)) in uncovered.into_iter().zip(reads) {
                let expected = (item_name(name), ListedPrice { quantity, mesos });
                match read(badge) {
                    Some(read) if read == expected => {}
                    None => unreadable += 1,
                    Some(read) => wrong.push(format!("{} ×{brightness}: {read:?}, not {expected:?}", sample.file)),
                }
            }
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
    // The samples were all read whole when they were added: a reader that reads less of them
    // has got worse, though it says nothing wrong.
    assert_eq!(unreadable, 0, "listings now unreadable");
}

/// `frame` with `rect` (x, y, w, h) painted as `paint` says, over what was there.
fn painted(mut frame: Frame, (x0, y0, w, h): (u32, u32, u32, u32), paint: impl Fn(u32, u32) -> Option<[u8; 3]>) -> Frame {
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            if let Some(rgb) = paint(x - x0, y - y0) {
                let at = ((y * frame.width + x) * 4) as usize;
                frame.rgba[at..at + 3].copy_from_slice(&rgb);
            }
        }
    }
    frame
}

/// The game's pointer, a white glove outlined in near-black, its cuff light blue.
fn pointer(x: u32, y: u32) -> Option<[u8; 3]> {
    let inside = |x: i32, y: i32| (x - 10).pow(2) * 9 + (y - 13).pow(2) * 4 <= 9 * 4 * 64;
    let (x, y) = (x as i32, y as i32);
    match (inside(x, y), inside(x - 1, y) && inside(x + 1, y) && inside(x, y - 1) && inside(x, y + 1)) {
        (true, true) if y > 19 => Some([170, 214, 246]),
        (true, true) => Some([255, 255, 255]),
        (true, false) => Some([24, 24, 32]),
        _ => None,
    }
}

/// The game's tooltip by the pointer: its dark slate fill, with a line of white text.
fn tooltip(_: u32, y: u32) -> Option<[u8; 3]> {
    Some(if (40..52).contains(&y) { [218, 218, 223] } else { [38, 46, 74] })
}

#[test]
fn a_listing_the_pointer_or_its_tooltip_covers_keeps_what_was_read_there_marked_covered() {
    let samples = samples();
    let sample = samples.iter().find(|s| s.file == "gems-and-ores.png").expect("gems-and-ores");
    let mut minerva = started(&samples);
    let capture = |frame: Frame| ShopCapture { frame, origin: ScreenPoint { x: 0, y: 0 } };
    let shown = |state: &PanelState| -> Vec<(bool, Option<String>)> {
        badges(state).iter().map(|b| (b.covered, read(b).map(|(name, _)| name))).collect()
    };
    let before = shown(minerva.on_player_shop_capture(capture(frame(sample, 1.0))));

    // The pointer on Garnet Ore's name.
    let state = minerva.on_player_shop_capture(capture(painted(frame(sample, 1.0), (110, 78, 21, 27), pointer)));
    let mut expected = before.clone();
    expected[1].0 = true;
    assert_eq!(shown(state), expected);

    // The tooltip by it over Topaz and Emerald, wholly.
    let state = minerva.on_player_shop_capture(capture(painted(frame(sample, 1.0), (40, 125, 290, 140), tooltip)));
    let mut expected = before.clone();
    expected[2].0 = true;
    expected[3].0 = true;
    assert_eq!(shown(state), expected);

    // Uncovered, all read again.
    assert_eq!(shown(minerva.on_player_shop_capture(capture(frame(sample, 1.0)))), before);
}

#[test]
fn listings_read_before_the_pointer_and_its_tooltip_came_over_them_keep_their_badges_covered() {
    let samples = samples();
    let sample = |file: &str| samples.iter().find(|s| s.file == file).unwrap_or_else(|| panic!("no {file}"));
    let mut minerva = started(&samples);
    let capture = |frame: Frame| ShopCapture { frame, origin: ScreenPoint { x: 0, y: 0 } };
    let shown = |state: &PanelState| -> Vec<(bool, Option<String>)> {
        badges(state).iter().map(|b| (b.covered, read(b).map(|(name, _)| name))).collect()
    };
    let name = |name: &str| Some(name.to_string());

    minerva.on_player_shop_capture(capture(frame(sample("equips-two-sold-out.png"), 1.0)));
    let state = minerva.on_player_shop_capture(capture(frame(sample("equips-under-the-pointer-and-its-tooltip.png"), 1.0)));

    assert_eq!(
        shown(state),
        [
            (false, name("Red Guise")),
            (true, name("Green Knucklevest")),
            (true, name("Black Knucklevest")),
            (true, name("Black Amoria Skirt")),
        ],
        "none for the sold-out listings, and none between the listings"
    );
}

#[test]
fn a_sold_out_listing_gets_no_badge_whatever_stray_pixels_a_capture_holds_on_it() {
    let samples = samples();
    let sample = |file: &str| samples.iter().find(|s| s.file == file).unwrap_or_else(|| panic!("no {file}"));
    let mut minerva = started(&samples);
    let capture = |frame: Frame| ShopCapture { frame, origin: ScreenPoint { x: 0, y: 0 } };
    // Every slot holds a listing here, so every slot is known.
    minerva.on_player_shop_capture(capture(frame(sample("ores-and-bundles-of-800.png"), 0.935)));

    // Another shop where the same window was, its third and fifth listings sold out, with a
    // row of the rule between listings' colour through each, as a capture a few px off
    // might hold.
    let rule = |_: u32, _: u32| Some([171, 190, 212]);
    let frame = frame(sample("scroll-equips-mats-two-sold-out.png"), 0.935);
    let frame = painted(painted(frame, (70, 140, 240, 2), rule), (70, 265, 240, 2), rule);
    let state = minerva.on_player_shop_capture(capture(frame));

    let shown: Vec<(bool, Option<String>)> = badges(state).iter().map(|b| (b.covered, read(b).map(|(name, _)| name))).collect();
    let name = |name: &str| Some(name.to_string());
    assert_eq!(
        shown,
        [
            (false, name("Blue Axe")),
            (false, Some(item_name("Claw Attack Scroll: Interm..."))),
            (false, name("Brown Bamboo Hat")),
            (false, name("Evil Eye Tail")),
        ]
    );
}

#[test]
fn a_sold_out_listing_is_never_taken_for_a_covered_one_without_the_pointer_or_its_tooltip_on_it() {
    let samples = samples();
    let sample = |file: &str| samples.iter().find(|s| s.file == file).unwrap_or_else(|| panic!("no {file}"));
    let mut minerva = started(&samples);
    let capture = |frame: Frame| ShopCapture { frame, origin: ScreenPoint { x: 0, y: 0 } };
    // Every slot holds a listing here, so every slot is known.
    minerva.on_player_shop_capture(capture(frame(sample("ores-and-bundles-of-800.png"), 0.935)));

    // Four listings sold out, in a capture that darkens the shop's blue as live captures do
    // but leaves its light greys (the sold-out listings, the window) nearly as they were: they
    // match no palette the reader expects at that brightness, yet nothing covers them.
    let img = image::open(samples_dir().join("scrolls-and-ores-four-sold-out.png")).expect("the sample").to_rgba8();
    let mut rgba = img.as_raw().clone();
    for p in rgba.chunks_exact_mut(4) {
        let (lo, hi) = (p[..3].iter().min().copied().unwrap_or(0), p[..3].iter().max().copied().unwrap_or(0));
        let light_grey = hi >= 205 && hi - lo <= 8;
        let k = if light_grey { 0.99 } else { 0.935 };
        for c in &mut p[..3] {
            *c = (*c as f32 * k).round() as u8;
        }
    }
    let frame = Frame { width: img.width(), height: img.height(), rgba, at_ms: 1_000 };
    let state = minerva.on_player_shop_capture(capture(frame));

    let shown: Vec<(bool, Option<String>)> = badges(state).iter().map(|b| (b.covered, read(b).map(|(name, _)| name))).collect();
    assert_eq!(shown, [(false, Some("Silver Ore".to_string())), (false, Some("Emerald Ore".to_string()))]);
}
