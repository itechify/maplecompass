//! Global hotkeys. On Windows these are `RegisterHotKey`: the system tells us when the
//! combination is pressed, whichever window has focus. No keyboard hook and no polling
//! of key state, and nothing is sent to the game (ADR 0001). They act while the game
//! keeps the keyboard, which a click on the overlay never takes from it.
//!
//! A registered combination reaches no other app, so they are registered only while the
//! core says they are live (the game or one of our windows is in front, and the player
//! isn't rebinding one) and released otherwise. The core lists the hotkeys and decides
//! what each press does; this only registers them and hands presses over. When the player
//! rebinds one, the new list replaces the old.

use crate::errors::error;
use crate::Command;
use compass_core::Hotkeys;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Every hotkey that parsed, with its accelerator string: what a press is sent as.
type Bindings = Arc<Mutex<Vec<(Shortcut, String)>>>;

/// The player's hotkeys, registered with the system or not.
pub struct GlobalHotkeys {
    /// The hotkeys as the core last listed them.
    hotkeys: Hotkeys,
    /// Every one of them that parsed, shared with the press handler.
    bindings: Bindings,
    /// The ones that didn't parse: always reported unavailable.
    invalid: Vec<String>,
    /// The ones registered now; empty while the hotkeys aren't live.
    registered: Vec<Shortcut>,
    live: bool,
}

/// Set up the player's hotkeys, registering none yet; once registered, each press sends
/// its accelerator string to the compass loop.
pub fn install(app: &tauri::App, hotkeys: &Hotkeys, commands: Sender<Command>) -> tauri::Result<GlobalHotkeys> {
    let (parsed, invalid) = parse(hotkeys);
    let bindings: Bindings = Arc::new(Mutex::new(parsed));
    let pressed = bindings.clone();
    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, shortcut, event| {
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                let text = pressed.lock().unwrap().iter().find(|(s, _)| s == shortcut).map(|(_, text)| text.clone());
                if let Some(text) = text {
                    let _ = commands.send(Command::Hotkey(text));
                }
            })
            .build(),
    )?;
    Ok(GlobalHotkeys { hotkeys: hotkeys.clone(), bindings, invalid, registered: Vec::new(), live: false })
}

/// Every hotkey the core lists (the search one included), so none is forgotten: those
/// that parse, and those that don't.
fn parse(hotkeys: &Hotkeys) -> (Vec<(Shortcut, String)>, Vec<String>) {
    let mut invalid = Vec::new();
    let mut bindings = Vec::new();
    for text in hotkeys.all() {
        match text.parse::<Shortcut>() {
            Ok(shortcut) => bindings.push((shortcut, text.to_string())),
            Err(e) => {
                error!("hotkey {text:?} invalid: {e}");
                invalid.push(text.to_string());
            }
        }
    }
    (bindings, invalid)
}

impl GlobalHotkeys {
    /// Register `hotkeys` when they go live and release them when they stop; when the
    /// player rebound one, release the old ones and register the new. On registering,
    /// returns the ones that are unavailable, for the core to show the player: those
    /// taken by another app, then those that aren't valid; `None` when nothing was
    /// registered.
    pub fn sync(&mut self, app: &AppHandle, hotkeys: &Hotkeys, live: bool) -> Option<(Vec<String>, Vec<String>)> {
        let rebound = *hotkeys != self.hotkeys;
        if !rebound && live == self.live {
            return None;
        }
        let shortcuts = app.global_shortcut();
        for shortcut in self.registered.drain(..) {
            if let Err(e) = shortcuts.unregister(shortcut) {
                error!("releasing a hotkey failed: {e}");
            }
        }
        if rebound {
            // Before registering, so a press of a new one is known.
            let (parsed, invalid) = parse(hotkeys);
            *self.bindings.lock().unwrap() = parsed;
            self.invalid = invalid;
            self.hotkeys = hotkeys.clone();
        }
        self.live = live;
        if !live {
            return None;
        }
        let mut taken = Vec::new();
        for (shortcut, text) in self.bindings.lock().unwrap().iter() {
            match shortcuts.register(*shortcut) {
                Ok(()) => self.registered.push(*shortcut),
                Err(e) => {
                    error!("hotkey {text} unavailable: {e}");
                    taken.push(text.clone());
                }
            }
        }
        Some((taken, self.invalid.clone()))
    }
}
