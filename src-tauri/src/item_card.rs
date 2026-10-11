//! The Item Card's window (ADR 0010): the hovered item's card, at the pointer where the
//! core puts it. Borderless like the overlay, and like it on top of everything while it
//! shows (only while the game or one of our windows is in front), and never activated by a
//! click (`focusable: false` in `tauri.conf.json`, ADR 0001). Like the Item Lookup alert's
//! window it is declared in `tauri.conf.json` and never hidden: without a card its page
//! draws nothing, and clicks pass through it. While a card shows it takes the clicks on it
//! (its ×, its name, its market line, an item to pick), and none reach the game. Every
//! decision is the core's; this only places the window, sends its page what it renders,
//! and moves it out of the way of a capture.

use crate::view::{LastView, ViewSender};
use crate::{Command, keep_on_top, physical, place};
use compass_core::{PanelState, ShownItemCard, Size};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;
use tauri::{AppHandle, Manager, PhysicalPosition, State, WebviewWindow};

pub const ITEM_CARD: &str = "item-card";

/// The window's width in CSS px (logical), which `src/item-card.css` lays the card out to.
/// Its height follows the card, as the page reports it, within these bounds.
const WIDTH: f64 = 280.0;
const DEFAULT_HEIGHT: f64 = 160.0;
const MIN_HEIGHT: f64 = 40.0;
const MAX_HEIGHT: f64 = 480.0;

/// Where the card waits while a capture is taken: off every screen, as Windows parks a
/// minimised window.
const OUT_OF_SIGHT: PhysicalPosition<i32> = PhysicalPosition::new(-32_000, -32_000);
/// How long a move of the card takes to leave the screen: two frames at 60 Hz, and a
/// little more, for the desktop's compositor to draw the screen without it.
const OFF_SCREEN_SETTLE: Duration = Duration::from_millis(50);

/// The view last sent to the card's page, for when it loads after it was sent.
#[derive(Default)]
pub struct ItemCardView(pub LastView);

/// What the card's page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CardView<'a> {
    /// The window shows: otherwise the page draws nothing.
    shown: bool,
    card: Option<&'a ShownItemCard>,
}

/// The Item Card's window, kept in line with the panel state.
pub struct ItemCardWindow {
    app: AppHandle,
    window: WebviewWindow,
    /// It shows a card, where the core placed it.
    shown: bool,
    /// Tells its watcher the core shows it, so it keeps it on top while the game or one
    /// of our windows is in front.
    shows_on_top: Arc<AtomicBool>,
    /// The card's height in CSS px, as the page last reported it.
    height: f64,
    view: ViewSender,
}

impl ItemCardWindow {
    pub fn new(app: &AppHandle, commands: Sender<Command>) -> Self {
        let window = app.get_webview_window(ITEM_CARD).expect("item-card window in tauri.conf.json");
        app.manage(ItemCardView::default());
        let shows_on_top = keep_on_top(&window, "on-top-item-card", commands, &[]);
        // Clicks pass through until it shows a card.
        let _ = window.set_ignore_cursor_events(true);
        Self {
            app: app.clone(),
            window,
            shown: false,
            shows_on_top,
            height: DEFAULT_HEIGHT,
            view: ViewSender::new(ITEM_CARD, "item-card-state"),
        }
    }

    /// The page's card is `height` CSS px tall.
    pub fn set_height(&mut self, height: f64) {
        self.height = height.clamp(MIN_HEIGHT, MAX_HEIGHT);
    }

    /// The window's size in physical px, for the display it is on.
    pub fn size(&self) -> Option<Size> {
        let scale = self.window.scale_factor().ok()?;
        Some(physical((WIDTH, self.height), scale))
    }

    /// Show the card where the core says, or nothing; and take clicks only while it shows.
    pub fn sync(&mut self, state: &PanelState) {
        let placement = state.item_card_placement;
        let card = state.item_card.as_ref().filter(|_| placement.is_some());
        let view = CardView { shown: card.is_some(), card };
        self.view.send(&self.app, &view, &self.app.state::<ItemCardView>().0);
        self.shows_on_top.store(placement.is_some(), Ordering::Relaxed);
        if let Some(placement) = placement {
            place(&self.window, placement, false);
        }
        let shown = card.is_some();
        if self.shown != shown {
            self.shown = shown;
            if let Err(e) = self.window.set_ignore_cursor_events(!shown) {
                eprintln!("item card click-through toggle failed: {e}");
            }
        }
    }

    /// Before a capture of the game while the card shows: move it off the screen now, and
    /// wait for the screen to be drawn without it, so it is never read in place of the
    /// Item Tooltip it may cover. The next sync puts it back where the core says.
    pub fn out_of_the_capture(&mut self) {
        if !self.shown {
            return;
        }
        if self.window.set_position(OUT_OF_SIGHT).is_ok() {
            std::thread::sleep(OFF_SCREEN_SETTLE);
        }
    }
}

/// The page reports how tall its card is, so the window fits it.
#[tauri::command]
pub fn item_card_height(height: f64, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::ItemCardHeight(height));
}

/// What the card's page renders, as last sent (`None` before the first).
#[tauri::command]
pub async fn item_card_view(view: State<'_, ItemCardView>) -> Result<Option<serde_json::Value>, String> {
    Ok(view.0.get())
}

/// The player clicked the card's ×.
#[tauri::command]
pub fn item_card_close(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::CloseItemCard);
}

/// The player picked the item with this game ID from those the card lists.
#[tauri::command]
pub fn item_card_pick(id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::PickCardItem(id));
}

/// The player clicked the card's name: its MeowDB page.
#[tauri::command]
pub fn item_card_open_meowdb(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::OpenCardMeowdbPage);
}

/// The player clicked the card's market line: its BasilMarket page.
#[tauri::command]
pub fn item_card_open_market(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::OpenCardMarketPage);
}
