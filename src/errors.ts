import { invoke } from "@tauri-apps/api/core";

// Uncaught errors and rejected promises in this window go to the shell, which keeps them
// for the Bug Report (ADR 0009): a release build has no console to show them. Imported
// first by each window's script, so it listens before the rest of the script runs.

function send(message: string): void {
  invoke("window_error", { message }).catch(() => {
    // The shell is gone, or not listening: there is nowhere else to tell.
  });
}

window.addEventListener("error", (event) => {
  const at = event.filename ? ` (${event.filename}:${event.lineno}:${event.colno})` : "";
  send(`${event.message}${at}`);
});

window.addEventListener("unhandledrejection", (event) => {
  const reason = event.reason instanceof Error ? (event.reason.stack ?? event.reason.message) : String(event.reason);
  send(`unhandled rejection: ${reason}`);
});
