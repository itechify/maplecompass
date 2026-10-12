//! Errors that would otherwise go nowhere: a release build has no console. Each is
//! printed, and handed to the core loop for the core's Bug Report (ADR 0009). The same
//! error again is handed on at most once a minute, so one that repeats every frame (a
//! capture failing while the screen is locked) doesn't fill the report.

use crate::Command;
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

/// How long the same error waits before it is handed on again.
const AGAIN_AFTER: Duration = Duration::from_secs(60);

struct Errors {
    /// The core loop's; `None` until it is set up.
    commands: Option<Sender<Command>>,
    /// Errors from before then, to hand on once it is.
    held: Vec<(String, String)>,
    /// The last error handed on, and when.
    last: Option<(String, String, Instant)>,
}

static ERRORS: Mutex<Errors> = Mutex::new(Errors { commands: None, held: Vec::new(), last: None });

/// Hand errors to the core loop from now on, those from before first.
pub fn send_to(commands: Sender<Command>) {
    let Ok(mut errors) = ERRORS.lock() else { return };
    for (from, message) in std::mem::take(&mut errors.held) {
        let _ = commands.send(Command::Error { from, message });
    }
    errors.commands = Some(commands);
}

/// An error that `from` ("shell", or one of our windows by its label) ran into.
pub fn report(from: &str, message: String) {
    eprintln!("{from}: {message}");
    let Ok(mut errors) = ERRORS.lock() else { return };
    let repeated = errors
        .last
        .as_ref()
        .is_some_and(|(f, m, at)| f == from && *m == message && at.elapsed() < AGAIN_AFTER);
    if repeated {
        return;
    }
    errors.last = Some((from.to_string(), message.clone(), Instant::now()));
    match &errors.commands {
        Some(commands) => {
            let _ = commands.send(Command::Error { from: from.to_string(), message });
        }
        None => errors.held.push((from.to_string(), message)),
    }
}

/// An error the shell ran into, formatted as by `format!`: printed, and kept for the Bug
/// Report.
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::errors::report("shell", format!($($arg)*))
    };
}
pub(crate) use error;

/// One of our windows ran into an uncaught error or a rejected promise.
#[tauri::command]
pub fn window_error(window: tauri::Window, message: String) {
    report(window.label(), message);
}
