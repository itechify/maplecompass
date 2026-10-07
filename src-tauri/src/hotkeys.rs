//! Global hotkeys. On Windows these are `RegisterHotKey`: the system tells us when the
//! combination is pressed, whichever window has focus. No keyboard hook, and nothing is
//! sent to the game (ADR 0001). They are the only way out of click-through, because a
//! click-through overlay never sees the mouse.

use crate::Command;
use compass_core::{Action, Hotkeys};
use std::sync::mpsc::Sender;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Register the player's hotkeys; each press sends its command to the compass loop.
/// A hotkey that can't be registered (taken by another app) is reported, not fatal.
pub fn install(app: &tauri::App, hotkeys: &Hotkeys, commands: Sender<Command>) -> tauri::Result<()> {
    let defaults = Hotkeys::default();
    let bindings = [
        (parse(&hotkeys.toggle_panel, &defaults.toggle_panel), Command::Act(Action::ToggleExpanded)),
        (parse(&hotkeys.edit_mode, &defaults.edit_mode), Command::Act(Action::ToggleEditMode)),
        (parse(&hotkeys.cycle_placement, &defaults.cycle_placement), Command::Act(Action::CyclePlacement)),
        (parse(&hotkeys.open_meowdb_page, &defaults.open_meowdb_page), Command::OpenMeowDbPage),
    ];
    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, shortcut, event| {
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                if let Some((_, command)) = bindings.iter().find(|(s, _)| s == shortcut) {
                    let _ = commands.send(*command);
                }
            })
            .build(),
    )?;
    for (shortcut, _) in bindings {
        if let Err(e) = app.global_shortcut().register(shortcut) {
            eprintln!("hotkey {shortcut} unavailable: {e}");
        }
    }
    Ok(())
}

/// The hotkey in `text`, or `fallback` when `text` isn't a valid accelerator.
fn parse(text: &str, fallback: &str) -> Shortcut {
    text.parse().unwrap_or_else(|e| {
        eprintln!("hotkey {text:?} invalid ({e}); using {fallback}");
        fallback.parse().expect("default hotkey parses")
    })
}
