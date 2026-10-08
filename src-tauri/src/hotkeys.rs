//! Global hotkeys. On Windows these are `RegisterHotKey`: the system tells us when the
//! combination is pressed, whichever window has focus. No keyboard hook and no polling
//! of key state, and nothing is sent to the game (ADR 0001). They act while the game
//! keeps the keyboard, which a click on the overlay never takes from it.
//!
//! A registered combination reaches no other app, so they are registered only while the
//! core says they are live (the game or one of our windows is in front) and released
//! otherwise. The core lists the hotkeys and decides what each press does; this only
//! registers them and hands presses over.

use crate::Command;
use compass_core::Hotkeys;
use std::sync::mpsc::Sender;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// The player's hotkeys, registered with the system or not.
pub struct GlobalHotkeys {
    /// Every hotkey that parsed, with its accelerator string.
    bindings: Vec<(Shortcut, String)>,
    /// The ones that didn't parse: always reported unavailable.
    invalid: Vec<String>,
    /// The ones registered now; empty while the hotkeys aren't live.
    registered: Vec<Shortcut>,
    live: bool,
}

/// Set up the player's hotkeys, registering none yet; once registered, each press sends
/// its accelerator string to the compass loop.
pub fn install(app: &tauri::App, hotkeys: &Hotkeys, commands: Sender<Command>) -> tauri::Result<GlobalHotkeys> {
    let mut invalid = Vec::new();
    let mut bindings: Vec<(Shortcut, String)> = Vec::new();
    // Every hotkey the core lists (the search one included), so none is forgotten.
    for text in hotkeys.all() {
        match text.parse::<Shortcut>() {
            Ok(shortcut) => bindings.push((shortcut, text.to_string())),
            Err(e) => {
                eprintln!("hotkey {text:?} invalid: {e}");
                invalid.push(text.to_string());
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
    Ok(GlobalHotkeys { bindings, invalid, registered: Vec::new(), live: false })
}

impl GlobalHotkeys {
    /// Register the hotkeys when they go live and release them when they stop. On
    /// registering, returns the ones that are unavailable (invalid, or taken by another
    /// app), for the core to show the player; `None` when nothing was registered.
    pub fn sync(&mut self, app: &AppHandle, live: bool) -> Option<Vec<String>> {
        if live == self.live {
            return None;
        }
        self.live = live;
        let shortcuts = app.global_shortcut();
        if !live {
            for shortcut in self.registered.drain(..) {
                if let Err(e) = shortcuts.unregister(shortcut) {
                    eprintln!("releasing a hotkey failed: {e}");
                }
            }
            return None;
        }
        let mut unavailable = self.invalid.clone();
        for (shortcut, text) in &self.bindings {
            match shortcuts.register(*shortcut) {
                Ok(()) => self.registered.push(*shortcut),
                Err(e) => {
                    eprintln!("hotkey {text} unavailable: {e}");
                    unavailable.push(text.clone());
                }
            }
        }
        Some(unavailable)
    }
}
