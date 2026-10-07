//! Global hotkeys. On Windows these are `RegisterHotKey`: the system tells us when the
//! combination is pressed, whichever window has focus. No keyboard hook and no polling
//! of key state, and nothing is sent to the game (ADR 0001). They are the only way out
//! of click-through, because a click-through overlay never sees the mouse.
//!
//! The core lists the hotkeys and decides what each press does; this only registers
//! them and hands presses over.

use crate::Command;
use compass_core::Hotkeys;
use std::sync::mpsc::Sender;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Register the player's hotkeys; each press sends its accelerator string to the
/// compass loop. Returns the ones that couldn't be registered (invalid, or taken by
/// another app), for the core to show the player.
pub fn install(app: &tauri::App, hotkeys: &Hotkeys, commands: Sender<Command>) -> tauri::Result<Vec<String>> {
    let all = [
        &hotkeys.toggle_panel,
        &hotkeys.edit_mode,
        &hotkeys.cycle_placement,
        &hotkeys.open_meowdb_page,
        &hotkeys.setup,
        &hotkeys.choose_candidate,
    ];
    let mut unavailable = Vec::new();
    let mut bindings: Vec<(Shortcut, String)> = Vec::new();
    for text in all {
        match text.parse::<Shortcut>() {
            Ok(shortcut) => bindings.push((shortcut, text.clone())),
            Err(e) => {
                eprintln!("hotkey {text:?} invalid: {e}");
                unavailable.push(text.clone());
            }
        }
    }
    let pressed = bindings.clone();
    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, shortcut, event| {
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                if let Some((_, text)) = pressed.iter().find(|(s, _)| s == shortcut) {
                    let _ = commands.send(Command::Hotkey(text.clone()));
                }
            })
            .build(),
    )?;
    for (shortcut, text) in bindings {
        if let Err(e) = app.global_shortcut().register(shortcut) {
            eprintln!("hotkey {text} unavailable: {e}");
            unavailable.push(text);
        }
    }
    Ok(unavailable)
}
