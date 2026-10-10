//! The settings view: the gear at the top of the panel opens it, and there the player
//! rebinds a hotkey by clicking its Change button and pressing the new combination. It
//! needs Ctrl, Alt or Win, and can't be another hotkey's (Swap trades the two); Esc
//! cancels, and Backspace, Delete or Turn off turns the hotkey off. While the player
//! rebinds, the hotkeys aren't live, so the combination reaches the overlay rather than
//! acting. Every change is saved at once and acts at once; Restore defaults can be undone
//! until the player changes another.

mod common;

use common::FakeSource;
use compass_core::{
    Action, Compass, Foreground, Hotkey, Hotkeys, KeyPress, PanelControl, RebindProblem, Settings, SettingsNotice,
    SettingsView,
};

/// The key `code` (a `KeyboardEvent.code`) pressed with the modifiers in `held`, such as
/// `"ctrl+shift"`.
fn press(held: &str, code: &str) -> KeyPress {
    let has = |modifier: &str| held.split('+').any(|m| m == modifier);
    KeyPress { code: code.into(), ctrl: has("ctrl"), alt: has("alt"), shift: has("shift"), win: has("win") }
}

/// A core with the game in front and the settings view open.
fn settings_open() -> Compass {
    let mut compass = Compass::new(FakeSource::osms());
    compass.on_foreground(Foreground::Game);
    compass.press(PanelControl::Settings);
    compass
}

fn view(compass: &Compass) -> SettingsView {
    compass.panel_state().settings_view.clone().expect("the settings view is open")
}

/// The hotkey waiting for its new combination, and why the last one pressed wasn't taken.
fn waiting(compass: &Compass) -> (Option<Hotkey>, Option<RebindProblem>) {
    let view = view(compass);
    (view.rebinding, view.problem)
}

fn saved(hotkey: Hotkey) -> Option<SettingsNotice> {
    Some(SettingsNotice::Saved { hotkey, swapped_with: None })
}

#[test]
fn the_gear_opens_the_settings_view_on_the_expanded_panel() {
    let mut compass = Compass::new(FakeSource::osms());
    assert_eq!(compass.panel_state().settings_view, None);
    compass.on_hotkey("ctrl+shift+c");
    assert!(!compass.panel_state().expanded);

    assert_eq!(compass.press(PanelControl::Settings), None);

    let state = compass.panel_state();
    assert!(state.expanded);
    assert_eq!(
        state.settings_view,
        Some(SettingsView { rebinding: None, problem: None, notice: None, at_defaults: true })
    );
}

#[test]
fn the_settings_view_and_the_search_view_take_each_others_place() {
    let mut compass = settings_open();

    compass.press(PanelControl::Search);
    assert!(compass.panel_state().search.view.is_some());
    assert_eq!(compass.panel_state().settings_view, None);

    compass.press(PanelControl::Settings);
    assert_eq!(compass.panel_state().search.view, None);
    assert!(compass.panel_state().settings_view.is_some());
}

#[test]
fn the_back_arrow_or_collapsing_the_panel_closes_the_settings_view() {
    let mut compass = settings_open();
    compass.act(Action::CloseSettings);
    assert_eq!(compass.panel_state().settings_view, None);

    compass.press(PanelControl::Settings);
    compass.on_hotkey("ctrl+shift+c");
    assert_eq!(compass.panel_state().settings_view, None);
}

#[test]
fn the_new_combination_acts_at_once_and_the_old_one_does_nothing() {
    let mut compass = settings_open();

    compass.act(Action::BeginRebind(Hotkey::TogglePanel));
    assert_eq!(waiting(&compass), (Some(Hotkey::TogglePanel), None));
    let state = compass.rebind_key(press("ctrl+shift", "KeyK")).clone();

    assert_eq!(state.hotkeys.toggle_panel, "ctrl+shift+k");
    let view = state.settings_view.expect("still open");
    assert_eq!((view.rebinding, view.notice), (None, saved(Hotkey::TogglePanel)), "and says it is saved");
    assert_eq!(compass.on_hotkey("ctrl+shift+c"), None);
    assert!(compass.panel_state().expanded);
    compass.on_hotkey("ctrl+shift+k");
    assert!(!compass.panel_state().expanded);
}

#[test]
fn a_rebound_hotkey_is_saved_and_the_others_are_kept() {
    let mut compass = settings_open();

    compass.act(Action::BeginRebind(Hotkey::ItemLookup));
    compass.rebind_key(press("alt", "F5"));

    let saved = compass.settings().hotkeys;
    assert_eq!(saved, Hotkeys { item_lookup: "alt+f5".into(), ..Hotkeys::default() });
    let mut later = Compass::new(FakeSource::osms());
    later.restore(serde_json::from_str(&serde_json::to_string(&compass.settings()).unwrap()).unwrap());
    assert_eq!(later.panel_state().hotkeys.item_lookup, "alt+f5", "and kept at the next launch");
}

#[test]
fn the_hotkeys_are_not_live_while_the_player_rebinds_one() {
    let mut compass = settings_open();
    assert!(compass.panel_state().hotkeys_live);

    compass.act(Action::BeginRebind(Hotkey::Search));
    assert!(!compass.panel_state().hotkeys_live, "the combination pressed must reach the overlay");
    compass.rebind_key(press("ctrl+alt", "KeyF"));
    assert!(compass.panel_state().hotkeys_live, "live again, the new one with them");

    compass.act(Action::BeginRebind(Hotkey::Search));
    compass.act(Action::CancelRebind);
    assert!(compass.panel_state().hotkeys_live);

    compass.act(Action::BeginRebind(Hotkey::Search));
    compass.on_foreground(Foreground::MapleCompass);
    assert!(!compass.panel_state().hotkeys_live, "whatever is in front");
}

#[test]
fn a_modifier_pressed_on_its_own_waits_for_the_key() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::Setup));

    compass.rebind_key(press("ctrl", "ControlLeft"));
    compass.rebind_key(press("ctrl+shift", "ShiftLeft"));

    assert_eq!(waiting(&compass), (Some(Hotkey::Setup), None));
    assert_eq!(compass.panel_state().hotkeys.setup, "ctrl+shift+s");
}

#[test]
fn a_combination_without_ctrl_alt_or_win_is_refused_and_another_can_be_pressed() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::Setup));

    // The game would never get Shift+K, or K, while the hotkeys are live: not even in chat.
    compass.rebind_key(press("shift", "KeyK"));
    assert_eq!(
        waiting(&compass),
        (Some(Hotkey::Setup), Some(RebindProblem::NoModifier { pressed: "shift+k".into() }))
    );
    compass.rebind_key(press("", "KeyK"));
    assert_eq!(view(&compass).problem, Some(RebindProblem::NoModifier { pressed: "k".into() }));
    assert_eq!(compass.panel_state().hotkeys.setup, "ctrl+shift+s");

    compass.rebind_key(press("win+shift", "KeyK"));
    assert_eq!(waiting(&compass), (None, None));
    assert_eq!(compass.panel_state().hotkeys.setup, "super+shift+k");
}

#[test]
fn another_hotkeys_combination_is_refused_naming_that_hotkey() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::TogglePanel));

    compass.rebind_key(press("ctrl+shift", "KeyF"));

    assert_eq!(
        waiting(&compass),
        (
            Some(Hotkey::TogglePanel),
            Some(RebindProblem::InUse { pressed: "ctrl+shift+f".into(), by: Hotkey::Search })
        )
    );
    assert_eq!(compass.panel_state().hotkeys, Hotkeys::default());
}

#[test]
fn swap_gives_the_combination_to_the_hotkey_being_rebound_and_its_old_one_to_the_other() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::TogglePanel));
    compass.rebind_key(press("ctrl+shift", "KeyF"));

    let state = compass.act(Action::SwapHotkeys).clone();

    assert_eq!(state.hotkeys.toggle_panel, "ctrl+shift+f");
    assert_eq!(state.hotkeys.search, "ctrl+shift+c");
    let view = state.settings_view.expect("still open");
    assert_eq!((view.rebinding, view.problem), (None, None));
    assert_eq!(view.notice, Some(SettingsNotice::Saved { hotkey: Hotkey::TogglePanel, swapped_with: Some(Hotkey::Search) }));
    compass.on_hotkey("ctrl+shift+f");
    assert!(!compass.panel_state().expanded, "Ctrl+Shift+F collapses the panel now");
}

#[test]
fn swap_does_nothing_unless_the_combination_pressed_is_another_hotkeys() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::TogglePanel));
    compass.rebind_key(press("shift", "KeyF"));

    compass.act(Action::SwapHotkeys);

    assert_eq!(compass.panel_state().hotkeys, Hotkeys::default());
    assert_eq!(view(&compass).rebinding, Some(Hotkey::TogglePanel), "still waiting");
}

#[test]
fn pressing_a_hotkeys_own_combination_keeps_it() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::GrindTracker));

    compass.rebind_key(press("ctrl+shift", "KeyG"));

    assert_eq!(waiting(&compass), (None, None));
    assert_eq!(compass.panel_state().hotkeys, Hotkeys::default());
}

#[test]
fn a_key_no_hotkey_can_use_is_refused() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::Search));

    for code in ["IntlBackslash", "CapsLock", "ContextMenu", "Key", "KeyAB", "F25", "Digit10"] {
        compass.rebind_key(press("ctrl", code));
        assert_eq!(view(&compass).problem, Some(RebindProblem::UnsupportedKey { code: code.into() }), "{code}");
    }
    assert_eq!(view(&compass).rebinding, Some(Hotkey::Search));
    assert_eq!(compass.panel_state().hotkeys, Hotkeys::default());
}

#[test]
fn keys_beyond_letters_are_named_as_hotkey_registration_names_them() {
    let cases = [
        ("ctrl", "Digit1", "ctrl+1"),
        ("ctrl+alt", "F12", "ctrl+alt+f12"),
        ("alt", "Numpad3", "alt+numpad3"),
        ("ctrl", "NumpadAdd", "ctrl+numpadadd"),
        ("ctrl+shift", "BracketLeft", "ctrl+shift+bracketleft"),
        ("win+ctrl+alt+shift", "Space", "super+ctrl+alt+shift+space"),
        ("ctrl", "ArrowUp", "ctrl+arrowup"),
        ("ctrl", "Backspace", "ctrl+backspace"),
    ];
    for (held, code, combination) in cases {
        let mut compass = settings_open();
        compass.act(Action::BeginRebind(Hotkey::ChooseCandidate));

        compass.rebind_key(press(held, code));

        assert_eq!(compass.panel_state().hotkeys.choose_candidate, combination, "{held} {code}");
    }
}

#[test]
fn esc_stops_waiting_and_changes_nothing() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::Search));
    compass.rebind_key(press("shift", "KeyQ"));

    compass.rebind_key(press("", "Escape"));

    assert_eq!(waiting(&compass), (None, None), "the view stays, its problem gone");
    assert_eq!(view(&compass).notice, None, "nothing was saved");
    assert_eq!(compass.panel_state().hotkeys, Hotkeys::default());
}

#[test]
fn closing_the_settings_view_mid_rebind_changes_nothing_and_the_hotkeys_are_live_again() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::Search));

    compass.act(Action::CloseSettings);
    compass.rebind_key(press("ctrl", "KeyQ"));

    let state = compass.panel_state();
    assert_eq!(state.settings_view, None);
    assert_eq!(state.hotkeys, Hotkeys::default());
    assert!(state.hotkeys_live);
}

#[test]
fn keys_pressed_while_no_hotkey_waits_change_nothing() {
    let mut compass = settings_open();
    let before = compass.panel_state().clone();

    compass.rebind_key(press("ctrl", "KeyQ"));
    compass.rebind_key(press("", "Backspace"));

    assert_eq!(*compass.panel_state(), before);
}

#[test]
fn backspace_delete_or_turn_off_turns_a_hotkey_off_and_its_old_combination_does_nothing() {
    for turn_off in ["Backspace", "Delete", "button"] {
        let mut compass = settings_open();
        compass.act(Action::BeginRebind(Hotkey::TogglePanel));

        let state = if turn_off == "button" {
            compass.act(Action::TurnOffHotkey).clone()
        } else {
            compass.rebind_key(press("", turn_off)).clone()
        };

        assert_eq!(state.hotkeys.toggle_panel, "", "{turn_off}");
        assert!(!state.hotkeys.all().contains(&"ctrl+shift+c"), "{turn_off}: not registered");
        assert_eq!(state.hotkeys.all().len(), 7);
        let view = state.settings_view.expect("still open");
        assert_eq!((view.rebinding, view.notice), (None, saved(Hotkey::TogglePanel)));
        assert_eq!(compass.on_hotkey("ctrl+shift+c"), None);
        assert!(compass.panel_state().expanded, "{turn_off}: the old combination does nothing");
        compass.press(PanelControl::TogglePanel);
        assert!(!compass.panel_state().expanded, "{turn_off}: its button still works");
    }
}

#[test]
fn shift_with_backspace_is_a_combination_to_refuse_not_a_way_to_turn_off() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::Search));

    compass.rebind_key(press("shift", "Backspace"));

    assert_eq!(view(&compass).problem, Some(RebindProblem::NoModifier { pressed: "shift+backspace".into() }));
    assert_eq!(compass.panel_state().hotkeys.search, "ctrl+shift+f");
}

#[test]
fn a_hotkey_turned_off_stays_off_at_the_next_launch_and_can_be_turned_on_again() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::ItemLookup));
    compass.rebind_key(press("", "Delete"));

    let mut later = Compass::new(FakeSource::osms());
    later.restore(serde_json::from_str(&serde_json::to_string(&compass.settings()).unwrap()).unwrap());
    assert_eq!(later.panel_state().hotkeys.item_lookup, "", "still off");
    assert!(later.panel_state().hotkeys.all().iter().all(|hotkey| !hotkey.is_empty()));

    later.press(PanelControl::Settings);
    later.act(Action::BeginRebind(Hotkey::ItemLookup));
    later.rebind_key(press("ctrl+shift", "KeyL"));
    assert_eq!(later.panel_state().hotkeys.item_lookup, "ctrl+shift+l");
}

#[test]
fn hotkeys_turned_off_are_never_in_the_way_of_another_combination() {
    let mut compass = settings_open();
    for hotkey in [Hotkey::Search, Hotkey::Setup] {
        compass.act(Action::BeginRebind(hotkey));
        compass.rebind_key(press("", "Backspace"));
    }

    compass.act(Action::BeginRebind(Hotkey::TogglePanel));
    compass.rebind_key(press("ctrl+shift", "KeyF"));

    assert_eq!(waiting(&compass), (None, None), "Search's old combination is free");
    assert_eq!(compass.panel_state().hotkeys.toggle_panel, "ctrl+shift+f");
}

#[test]
fn swapping_with_a_hotkey_that_was_off_turns_the_other_off() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::Setup));
    compass.rebind_key(press("", "Backspace"));

    compass.act(Action::BeginRebind(Hotkey::Setup));
    compass.rebind_key(press("ctrl+shift", "KeyF"));
    compass.act(Action::SwapHotkeys);

    let hotkeys = &compass.panel_state().hotkeys;
    assert_eq!((hotkeys.setup.as_str(), hotkeys.search.as_str()), ("ctrl+shift+f", ""));
}

#[test]
fn starting_another_change_clears_the_last_ones_notice() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::Search));
    compass.rebind_key(press("ctrl+alt", "KeyF"));
    assert_eq!(view(&compass).notice, saved(Hotkey::Search));

    compass.act(Action::BeginRebind(Hotkey::Setup));

    assert_eq!(view(&compass).notice, None);
}

#[test]
fn restore_defaults_puts_every_hotkey_back_and_on() {
    let mut compass = Compass::new(FakeSource::osms());
    let hotkeys = Hotkeys { toggle_panel: "ctrl+shift+x".into(), search: "".into(), ..Hotkeys::default() };
    compass.restore(Settings { hotkeys, ..Settings::default() });
    compass.press(PanelControl::Settings);
    assert!(!view(&compass).at_defaults);
    compass.act(Action::BeginRebind(Hotkey::Setup));

    let state = compass.act(Action::ResetHotkeys).clone();

    assert_eq!(state.hotkeys, Hotkeys::default());
    assert_eq!(
        state.settings_view,
        Some(SettingsView { rebinding: None, problem: None, notice: Some(SettingsNotice::Restored), at_defaults: true })
    );
    assert_eq!(compass.settings().hotkeys, Hotkeys::default(), "and saved");
}

#[test]
fn undo_after_restore_defaults_brings_the_hotkeys_from_before_back() {
    let mut compass = Compass::new(FakeSource::osms());
    let mine = Hotkeys { toggle_panel: "ctrl+shift+x".into(), search: "".into(), ..Hotkeys::default() };
    compass.restore(Settings { hotkeys: mine.clone(), ..Settings::default() });
    compass.press(PanelControl::Settings);
    compass.act(Action::ResetHotkeys);

    let state = compass.act(Action::UndoResetHotkeys).clone();

    assert_eq!(state.hotkeys, mine);
    let view = state.settings_view.expect("still open");
    assert_eq!((view.notice, view.at_defaults), (Some(SettingsNotice::RestoreUndone), false));
    assert_eq!(compass.settings().hotkeys, mine, "and saved");
    compass.act(Action::UndoResetHotkeys);
    assert_eq!(compass.panel_state().hotkeys, mine, "once");
}

#[test]
fn restore_defaults_can_no_longer_be_undone_once_another_change_starts_or_the_view_closes() {
    for after in [Action::BeginRebind(Hotkey::Search), Action::CloseSettings] {
        let mut compass = Compass::new(FakeSource::osms());
        let mine = Hotkeys { toggle_panel: "ctrl+shift+x".into(), ..Hotkeys::default() };
        compass.restore(Settings { hotkeys: mine, ..Settings::default() });
        compass.press(PanelControl::Settings);
        compass.act(Action::ResetHotkeys);

        compass.act(after);
        compass.press(PanelControl::Settings);
        compass.act(Action::UndoResetHotkeys);

        assert_eq!(compass.panel_state().hotkeys, Hotkeys::default(), "{after:?}");
    }
}

#[test]
fn restore_defaults_with_every_hotkey_at_its_default_changes_and_says_nothing() {
    let mut compass = settings_open();
    assert!(view(&compass).at_defaults);

    compass.act(Action::ResetHotkeys);

    assert_eq!(view(&compass).notice, None);
}

#[test]
fn the_overlay_gets_the_settings_view_with_the_names_it_reads() {
    let mut compass = settings_open();
    compass.act(Action::BeginRebind(Hotkey::TogglePanel));
    compass.rebind_key(press("ctrl+shift", "KeyF"));
    let refused = serde_json::to_string(&compass.panel_state().settings_view).unwrap();
    compass.act(Action::SwapHotkeys);
    let swapped = serde_json::to_string(&compass.panel_state().settings_view).unwrap();
    compass.act(Action::ResetHotkeys);
    let restored = serde_json::to_string(&compass.panel_state().settings_view).unwrap();

    assert!(refused.contains(r#""problem":{"kind":"inUse","pressed":"ctrl+shift+f","by":"search"}"#), "{refused}");
    assert!(refused.contains(r#""rebinding":"togglePanel""#), "{refused}");
    assert!(
        swapped.contains(r#""notice":{"kind":"saved","hotkey":"togglePanel","swappedWith":"search"}"#),
        "{swapped}"
    );
    assert!(swapped.contains(r#""atDefaults":false"#), "{swapped}");
    assert!(restored.contains(r#""notice":{"kind":"restored"}"#), "{restored}");
    assert!(restored.contains(r#""atDefaults":true"#), "{restored}");
}

#[test]
fn a_combination_rebound_away_from_is_no_longer_reported_unavailable() {
    let mut compass = Compass::new(FakeSource::osms());
    let hotkeys = Hotkeys { setup: "ctrl+shift+banana".into(), ..Hotkeys::default() };
    compass.restore(Settings { hotkeys, ..Settings::default() });
    compass.hotkeys_unavailable(vec!["ctrl+shift+n".into()], vec!["ctrl+shift+banana".into()]);
    compass.press(PanelControl::Settings);

    compass.act(Action::BeginRebind(Hotkey::ChooseCandidate));
    compass.rebind_key(press("ctrl+alt", "KeyN"));
    let state = compass.panel_state();
    assert_eq!(state.unavailable_hotkeys, ["ctrl+shift+banana"], "the shell reports on Ctrl+Alt+N once it registers it");
    assert_eq!(state.invalid_hotkeys, ["ctrl+shift+banana"]);

    let state = compass.act(Action::ResetHotkeys);
    assert!(state.unavailable_hotkeys.is_empty());
    assert!(state.invalid_hotkeys.is_empty());
}
