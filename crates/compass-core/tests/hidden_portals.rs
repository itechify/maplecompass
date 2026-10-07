//! Once the panel has a map's MeowDB page, it lists the map's Hidden Portals: the exits
//! the player can't see, where each leads and roughly where it is. Pages are the ones
//! saved on 2026-10-07 (personal-use test data, ADR 0003), served by a fake map data
//! source.

mod common;

use common::{FakeSource, sample_frame};
use compass_core::{Compass, HiddenPortal, MapDataStatus, PanelState};

const CONSTRUCTION_SITE: &str = "victoria-road__construction-site-north-of-kerning-city--window";
const KERNING: &str = "victoria-road__kerning-city--window";

/// Two confident reads of a sample: the panel switches to its map.
fn arrive(compass: &mut Compass, sample: &str, at_ms: u64) -> PanelState {
    compass.on_frame(sample_frame(sample, at_ms));
    compass.on_frame(sample_frame(sample, at_ms + 333)).clone()
}

fn hidden_portals(state: &PanelState) -> &[HiddenPortal] {
    match &state.data {
        MapDataStatus::Ready(data) => &data.hidden_portals,
        other => panic!("no map data: {other:?}"),
    }
}

#[test]
fn the_construction_site_north_of_kerning_city_has_a_hidden_portal_up_on_the_right_to_a_hidden_street() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = arrive(&mut compass, CONSTRUCTION_SITE, 0);

    assert_eq!(
        hidden_portals(&state),
        [HiddenPortal {
            name: "hiden00".into(),
            to_map_id: "010003081".into(),
            to_map_name: "Northern Top of Construction Site".into(),
            to_street_name: "Hidden Street".into(),
            to_hidden_street: true,
            quest: false,
            position: "right, upper".into(),
        }]
    );
}

/// Kerning City has 14 invisible exits (type 1): its building doors and the way down to
/// The Swamp of Despair. On a Town they are Doors, drawn into the scenery, not hidden.
#[test]
fn the_doors_of_a_town_are_not_hidden_portals() {
    let mut compass = Compass::new(FakeSource::meowdb());

    let state = arrive(&mut compass, KERNING, 0);

    assert_eq!(hidden_portals(&state), []);
}
