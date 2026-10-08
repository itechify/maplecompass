//! The icons at the top of the panel do what the hotkeys do: each one is pressed through
//! the core, the same as its hotkey, so a player who forgets a hotkey can click instead.

mod common;

use common::{FakeSource, arrive};
use compass_core::{Compass, PanelControl, PlacementMode, Request, Settings, SetupReason};

const SUNSET: &str = "victoria-road__sunset-sky--window";

#[test]
fn the_collapse_icon_collapses_the_panel_like_ctrl_shift_c() {
    let mut compass = Compass::new(FakeSource::meowdb());

    assert_eq!(compass.press(PanelControl::TogglePanel), None);
    assert!(!compass.panel_state().expanded);
    compass.press(PanelControl::TogglePanel);
    assert!(compass.panel_state().expanded);
}

#[test]
fn the_placement_icon_cycles_the_placement_like_ctrl_shift_p() {
    let mut compass = Compass::new(FakeSource::meowdb());

    compass.press(PanelControl::CyclePlacement);

    assert_eq!(compass.panel_state().placement_mode, PlacementMode::DockedRight);
}

#[test]
fn the_meowdb_icon_opens_the_shown_maps_page_like_ctrl_shift_m_and_nothing_without_a_map() {
    let mut compass = Compass::new(FakeSource::meowdb());
    assert_eq!(compass.press(PanelControl::OpenMeowdbPage), None);

    arrive(&mut compass, SUNSET, 0);

    assert_eq!(
        compass.press(PanelControl::OpenMeowdbPage),
        Some(Request::OpenInBrowser { url: "https://meowdb.com/msclassic/maps/010003070".into() })
    );
}

#[test]
fn the_help_icon_opens_setup_like_ctrl_shift_s() {
    let mut compass = Compass::new(FakeSource::meowdb());
    compass.restore(Settings { title_region: Some(common::kerning_title_region()), ..Settings::default() });

    compass.press(PanelControl::Setup);

    let state = compass.panel_state();
    assert_eq!(state.setup.needed, Some(SetupReason::Requested));
    assert!(state.setup.window_open);
}

#[test]
fn the_search_icon_opens_the_search_view_like_ctrl_shift_f() {
    let mut compass = Compass::new(FakeSource::meowdb());

    assert_eq!(compass.press(PanelControl::Search), None);

    assert!(compass.panel_state().search.view.is_some());
}

#[test]
fn a_button_and_its_hotkey_land_in_the_same_state() {
    let presses = [
        (PanelControl::TogglePanel, "ctrl+shift+c"),
        (PanelControl::CyclePlacement, "ctrl+shift+p"),
        (PanelControl::Setup, "ctrl+shift+s"),
        (PanelControl::ChooseCandidate, "ctrl+shift+n"),
        (PanelControl::Search, "ctrl+shift+f"),
    ];
    for (control, hotkey) in presses {
        let mut clicked = Compass::new(FakeSource::meowdb());
        let mut typed = Compass::new(FakeSource::meowdb());
        arrive(&mut clicked, SUNSET, 0);
        arrive(&mut typed, SUNSET, 0);

        let by_click = clicked.press(control);
        let by_key = typed.on_hotkey(hotkey);

        assert_eq!(by_click, by_key, "{control:?}");
        assert_eq!(clicked.panel_state(), typed.panel_state(), "{control:?}");
    }
}
