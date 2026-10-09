//! Sending one of our pages (setup, the Grind Tracker's) the view it renders: only when it changed,
//! since the loop wakes up 20 times a second, and kept for a page that loads after it
//! was sent.

use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// The view last sent to a page, for the page to ask for when it loads late.
#[derive(Default)]
pub struct LastView(Mutex<Option<Value>>);

impl LastView {
    pub fn get(&self) -> Option<Value> {
        self.0.lock().unwrap().clone()
    }
}

/// Sends one page its view as an event.
pub struct ViewSender {
    label: &'static str,
    event: &'static str,
    sent: Option<Value>,
}

impl ViewSender {
    pub fn new(label: &'static str, event: &'static str) -> Self {
        Self { label, event, sent: None }
    }

    /// Send `view` if it differs from the last one sent, and keep it in `last`. Whether
    /// it was sent.
    pub fn send(&mut self, app: &AppHandle, view: &impl Serialize, last: &LastView) -> bool {
        let Ok(view) = serde_json::to_value(view) else { return false };
        if self.sent.as_ref() == Some(&view) {
            return false;
        }
        if let Err(e) = app.emit_to(self.label, self.event, &view) {
            eprintln!("emit failed: {e}");
        }
        *last.0.lock().unwrap() = Some(view.clone());
        self.sent = Some(view);
        true
    }
}
