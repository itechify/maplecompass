//! The Deal Badges' window (ADR 0014): a badge beside each listing of the Player Shop the
//! Deal Finder read, left of the shop's window where the core puts it. Borderless like the
//! overlay, and like it on top of everything while it shows (only while the game or one of
//! our windows is in front), and never activated by a click (`focusable: false` in
//! `tauri.conf.json`, ADR 0001). Like the Item Lookup alert's window it is declared in
//! `tauri.conf.json` and never hidden: without badges its page draws nothing and clicks pass
//! through it. With them it takes clicks: one on a badge shows its item's card. Every
//! decision is the core's; this only places the window, sends its page what it renders,
//! and lets clicks through or not.

use crate::errors::error;
use crate::item_card::OUT_OF_SIGHT;
use crate::view::{LastView, ViewSender};
use crate::{Command, keep_on_top, physical, place};
use minerva_core::{Action, DealBadge, PanelState, Size};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Manager, State, WebviewWindow};

pub const DEAL_BADGES: &str = "deal-badges";

/// The window's width in CSS px (logical); `src/deal-badges.css` right-aligns the badges in
/// it, towards the shop. Its height is the shop's list's, as the core places it.
const WIDTH: f64 = 150.0;

/// The view last sent to the badges' page, for when it loads after it was sent.
#[derive(Default)]
pub struct DealBadgesView(pub LastView);

/// What the badges' page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BadgesView<'a> {
    /// The badges, each level with its listing (physical px from the window's top);
    /// empty while the window draws nothing.
    badges: &'a [DealBadge],
    /// The Deal Line's name, for each badge's words.
    deal_line: Option<&'a str>,
}

/// The Deal Badges' window, kept in line with the panel state.
pub struct DealBadgesWindow {
    app: AppHandle,
    window: WebviewWindow,
    /// It takes clicks: it shows badges.
    clickable: bool,
    /// Tells its watcher the core shows it, so it keeps it on top while the game or one
    /// of our windows is in front.
    shows_on_top: Arc<AtomicBool>,
    view: ViewSender,
}

impl DealBadgesWindow {
    pub fn new(app: &AppHandle, commands: Sender<Command>) -> Self {
        let window = app.get_webview_window(DEAL_BADGES).expect("deal-badges window in tauri.conf.json");
        let shows_on_top = keep_on_top(&window, "on-top-deal-badges", commands, &[]);
        // Clicks pass through until it shows badges.
        let _ = window.set_ignore_cursor_events(true);
        Self { app: app.clone(), window, clickable: false, shows_on_top, view: ViewSender::new(DEAL_BADGES, "deal-badges-state") }
    }

    /// The window's size in physical px, for the display it is on: its width counts.
    pub fn size(&self) -> Option<Size> {
        let scale = self.window.scale_factor().ok()?;
        Some(physical((WIDTH, 0.0), scale))
    }

    /// Before a capture of the game for an Item Card or Item Lookup press while badges show:
    /// move the window off the screen now, as the Item Card is, since it would be read in
    /// place of an Item Tooltip it covers; whether it moved. The core shows no badges until
    /// the capture is read, and the next sync after puts it back.
    pub fn out_of_the_capture(&mut self) -> bool {
        self.clickable && self.window.set_position(OUT_OF_SIGHT).is_ok()
    }

    /// Show the badges where the core says, or nothing; and take clicks only while there
    /// are badges.
    pub fn sync(&mut self, state: &PanelState) {
        let shown = state.deal_badges.as_ref();
        let view = BadgesView {
            badges: shown.map_or(&[], |shown| &shown.badges),
            deal_line: state.deal_finder.deal_line.as_deref(),
        };
        self.view.send(&self.app, &view, &self.app.state::<DealBadgesView>().0);
        self.shows_on_top.store(shown.is_some(), Ordering::Relaxed);
        if let Some(shown) = shown {
            place(&self.window, shown.placement, false);
        }
        let clickable = shown.is_some_and(|shown| !shown.badges.is_empty());
        if self.clickable != clickable {
            self.clickable = clickable;
            if let Err(e) = self.window.set_ignore_cursor_events(!clickable) {
                error!("deal badges click-through toggle failed: {e}");
            }
        }
    }
}

/// What the badges' page renders, as last sent (`None` before the first).
#[tauri::command]
pub async fn deal_badges_view(view: State<'_, DealBadgesView>) -> Result<Option<serde_json::Value>, String> {
    Ok(view.0.get())
}

/// The player clicked listing `row`'s badge: its item's card shows beside it.
#[tauri::command]
pub fn deal_badge_open(row: u32, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Act(Action::OpenDealItem { row }));
}

/// The player set the Deal Margin in the settings view, in percent.
#[tauri::command]
pub fn deal_margin_set(percent: u32, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Act(Action::SetDealMargin { percent }));
}
