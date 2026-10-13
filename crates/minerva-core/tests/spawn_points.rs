//! The Map Preview dots every Spawn Point on the map, where OSMS puts it in the picture's
//! pixels (`mob_positions` in `maps.json`), with the mob that spawns there, in OSMS's
//! order. A Spawn Point without a position still counts towards the map's figures, but has
//! no dot. Served from made-up worlds in OSMS's shape.

mod common;

use common::FakeSource;
use common::osms::{self, KERNING_CITY, map};
use minerva_core::{MapDataStatus, Minerva, PanelState, PicturePoint, SpawnMark, TitleLines, TitleReading};

/// A made-up map whose spawn points the tests give, on a street of its own.
const FIELD: u64 = 100_000_000;
const BLUE_SNAIL: u64 = 3;
const SLIME: u64 = 7;

fn read_as(at_ms: u64) -> TitleReading {
    TitleReading { lines: Some(TitleLines { street: "Made-up Street".into(), map: "Field".into() }), at_ms }
}

/// The panel on the made-up Field, with the game's maps and mobs around it.
fn on_field(change: impl FnOnce(osms::Map) -> osms::Map) -> PanelState {
    let world = osms::game().with(change(map(FIELD, "Made-up Street", "Field").portal("east00", 2, KERNING_CITY)));
    let mut minerva = Minerva::new(FakeSource::with_world(world));
    minerva.on_reading(read_as(0));
    minerva.on_reading(read_as(333)).clone()
}

fn spawns(state: &PanelState) -> &[SpawnMark] {
    &state.map_preview.as_ref().expect("a Map Preview").spawns
}

fn dot(x: i32, y: i32, mob: u64) -> SpawnMark {
    SpawnMark { at: PicturePoint { x, y }, mob: mob.to_string() }
}

#[test]
fn each_spawn_point_is_dotted_where_osms_puts_it_with_its_mob() {
    let state = on_field(|m| m.spawn_at(BLUE_SNAIL, 412, 520).spawn_at(SLIME, 903, 611).spawn_at(BLUE_SNAIL, 1188, 520));

    assert_eq!(spawns(&state), [dot(412, 520, BLUE_SNAIL), dot(903, 611, SLIME), dot(1188, 520, BLUE_SNAIL)]);
}

#[test]
fn timed_spawn_points_are_dotted_as_the_others() {
    let state = on_field(|m| m.spawn_at(BLUE_SNAIL, 412, 520).timed(SLIME, 2, "300.00"));

    assert_eq!(spawns(&state).iter().filter(|s| s.mob == SLIME.to_string()).count(), 2);
}

#[test]
fn a_spawn_point_without_a_position_counts_but_has_no_dot() {
    let state = on_field(|m| m.spawn_at(BLUE_SNAIL, 412, 520).spawn_unplaced(BLUE_SNAIL));

    assert_eq!(spawns(&state), [dot(412, 520, BLUE_SNAIL)]);
    let MapDataStatus::Ready(data) = &state.data else { panic!("no map data: {:?}", state.data) };
    assert_eq!(data.mobs[0].spawn_count, 2);
}

#[test]
fn a_map_without_mobs_has_no_dots() {
    let state = on_field(|m| m);

    assert_eq!(spawns(&state), []);
}
