//! The Item Lookup alert's window (ADR 0007): what the latest lookup came to, at the top
//! centre of the game window where the core puts it. Borderless like the overlay, and
//! like it on top of everything while it shows (only while the game or one of our
//! windows is in front), and never activated by a click (`focusable: false` in
//! `tauri.conf.json`, ADR 0001). Like the Grind Tracker's window
//! it is declared in `tauri.conf.json` and never hidden: without an alert its page draws
//! nothing. Clicks pass through it to the game, except while it lists items to pick
//! from. Every decision is the core's; this only places the window, sends its page what
//! it renders, and lets clicks through or not.

use crate::view::{LastView, ViewSender};
use crate::{Command, keep_on_top, physical, place};
use compass_core::{ItemLookup, ItemLookupLine, PanelState, Size};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Manager, State, WebviewWindow};

pub const ITEM_LOOKUP: &str = "item-lookup";

/// The window's width in CSS px (logical); `src/item-lookup.css` centres the alert in it.
/// Its height follows the alert, as the page reports it, within these bounds.
const WIDTH: f64 = 440.0;
const DEFAULT_HEIGHT: f64 = 40.0;
const MIN_HEIGHT: f64 = 24.0;
const MAX_HEIGHT: f64 = 400.0;

/// The view last sent to the alert's page, for when it loads after it was sent.
#[derive(Default)]
pub struct ItemLookupView(pub LastView);

/// What the alert's page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AlertView<'a> {
    /// The window shows: otherwise the page draws nothing.
    shown: bool,
    line: Option<&'a ItemLookupLine>,
    /// The Item Lookup hotkey, for the alert to name.
    hotkey: &'a str,
}

/// The Item Lookup alert's window, kept in line with the panel state.
pub struct ItemLookupWindow {
    app: AppHandle,
    window: WebviewWindow,
    /// It takes clicks: it lists items to pick from.
    clickable: bool,
    /// Tells its watcher the core shows it, so it keeps it on top while the game or one
    /// of our windows is in front.
    shows_on_top: Arc<AtomicBool>,
    /// The alert's height in CSS px, as the page last reported it.
    height: f64,
    view: ViewSender,
}

impl ItemLookupWindow {
    pub fn new(app: &AppHandle, commands: Sender<Command>) -> Self {
        let window = app.get_webview_window(ITEM_LOOKUP).expect("item-lookup window in tauri.conf.json");
        app.manage(ItemLookupView::default());
        let shows_on_top = keep_on_top(&window, "on-top-item-lookup", commands);
        // Clicks pass through until it lists items to pick from.
        let _ = window.set_ignore_cursor_events(true);
        Self {
            app: app.clone(),
            window,
            clickable: false,
            shows_on_top,
            height: DEFAULT_HEIGHT,
            view: ViewSender::new(ITEM_LOOKUP, "item-lookup-state"),
        }
    }

    /// The page's alert is `height` CSS px tall.
    pub fn set_height(&mut self, height: f64) {
        self.height = height.clamp(MIN_HEIGHT, MAX_HEIGHT);
    }

    /// The window's size in physical px, for the display it is on.
    pub fn size(&self) -> Option<Size> {
        let scale = self.window.scale_factor().ok()?;
        Some(physical((WIDTH, self.height), scale))
    }

    /// Show the alert where the core says, or nothing; and let clicks through unless it
    /// lists items to pick from.
    pub fn sync(&mut self, state: &PanelState) {
        let placement = state.item_lookup_placement;
        let line = state.item_lookup.as_ref().filter(|_| placement.is_some());
        let view = AlertView { shown: line.is_some(), line, hotkey: &state.hotkeys.item_lookup };
        self.view.send(&self.app, &view, &self.app.state::<ItemLookupView>().0);
        self.shows_on_top.store(placement.is_some(), Ordering::Relaxed);
        if let Some(placement) = placement {
            place(&self.window, placement, false);
        }
        let clickable = line.is_some_and(|line| matches!(line.outcome, ItemLookup::Choose { .. }));
        if self.clickable != clickable {
            self.clickable = clickable;
            if let Err(e) = self.window.set_ignore_cursor_events(!clickable) {
                eprintln!("item lookup click-through toggle failed: {e}");
            }
        }
    }
}

/// The page reports how tall its alert is, so the window fits it.
#[tauri::command]
pub fn item_lookup_height(height: f64, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::ItemLookupHeight(height));
}

/// What the alert's page renders, as last sent (`None` before the first).
#[tauri::command]
pub async fn item_lookup_view(view: State<'_, ItemLookupView>) -> Result<Option<serde_json::Value>, String> {
    Ok(view.0.get())
}
