//! The tray icon: the gold compass in the notification area. Its menu opens setup (like
//! the panel's Help & setup) and quits MapleCompass, whether or not the game is open,
//! the panel shows or the hotkeys are live. Nothing here touches the game (ADR 0001).

use crate::Command;
use compass_core::PanelControl;
use std::sync::mpsc::Sender;
use tauri::App;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;

/// Put the compass in the tray, with its menu on either click; each choice goes to the
/// compass loop.
pub fn install(app: &App, commands: Sender<Command>) -> tauri::Result<()> {
    let setup = MenuItem::with_id(app, "setup", "Help & setup", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit MapleCompass", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&setup, &PredefinedMenuItem::separator(app)?, &quit])?;
    let mut tray = TrayIconBuilder::with_id("maplecompass")
        .tooltip("MapleCompass")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |_app, event| {
            let command = match event.id().as_ref() {
                "setup" => Command::Press(PanelControl::Setup),
                "quit" => Command::Quit,
                _ => return,
            };
            let _ = commands.send(command);
        });
    // The app icon, the compass, as the windows use it.
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
