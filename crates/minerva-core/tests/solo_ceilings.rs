//! The Map Leaderboards' cap (ADR 0015) is 5× each map's solo EXP/h ceiling, from a table
//! `probe ceilings` writes from OSMS's set with the core's own Map Summary arithmetic, so
//! the site never works it out again. Read from made-up worlds in OSMS's shape.
//!
//! Unlike the other tests, these assert on the table rather than on `PanelState`: the
//! table is what the site's Worker loads, and the panel never shows it.

mod common;

use common::osms::{self, map, mob};
use minerva_core::osms::read;
use std::collections::BTreeMap;

/// The ranged-timer world of `map_summary.rs` (269 Ghost Stumps and Pigs an hour, 2,810
/// EXP alone), beside a map whose only mob gives no EXP and a map with no mobs.
fn world() -> osms::World {
    osms::world(
        vec![
            map(100_000_000, "Made-up Street", "Ghost Field")
                .timed(20, 2, "180.00-1800.00 [720.00]")
                .timed(21, 3, "20.00-60.00 [41.76]"),
            map(100_000_001, "Made-up Street", "Training Field").spawns(22, 4),
            map(100_000_002, "Made-up Street", "Empty Field"),
        ],
        vec![mob(20, "Ghost Stump", 19, 300, 100), mob(21, "Pig", 7, 80, 7), mob(22, "Dummy", 1, 10, 0)],
    )
}

#[test]
fn each_map_with_a_solo_ceiling_is_keyed_by_its_nine_digit_id() {
    let set = read(&world().files()).expect("reads");

    assert_eq!(set.solo_ceilings(), BTreeMap::from([("100000000".to_string(), 2810)]));
}

/// Henesys Hunting Ground I's solo ceiling is the spec's 153,216 (`map_summary.rs`), and
/// Kerning City, a Town, has none.
#[test]
fn the_ceiling_is_the_map_summarys_solo_exp_per_hour_and_a_town_has_none() {
    let set = read(&osms::game().files()).expect("reads");

    let ceilings = set.solo_ceilings();

    assert_eq!(ceilings.get("010001010"), Some(&153_216));
    assert_eq!(ceilings.get("010003000"), None);
}
