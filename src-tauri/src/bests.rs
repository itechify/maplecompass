//! Keeping the Personal Bests between launches: the core's [`SavedBests`] as pretty JSON
//! beside the settings (`%APPDATA%\com.itechify.maplecompass\personal-bests.json` on
//! Windows), read at startup and written whenever the core's bests revision changes.

use compass_core::{BESTS_VERSION, SavedBests};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("personal-bests.json"))
}

/// The saved bests, or none when there are none yet. A file that doesn't read, or was
/// saved in another shape (by a later version), is kept beside it as
/// `personal-bests.json.bad` for diagnosis, rather than written over.
pub fn load(path: &Path) -> SavedBests {
    let Ok(text) = fs::read_to_string(path) else { return SavedBests::default() };
    match serde_json::from_str::<SavedBests>(&text) {
        Ok(bests) if bests.version == BESTS_VERSION => bests,
        read => {
            match read {
                Ok(bests) => eprintln!("personal bests saved in shape {}; starting with none", bests.version),
                Err(e) => eprintln!("personal bests unreadable ({e}); starting with none"),
            }
            let _ = fs::rename(path, path.with_extension("json.bad"));
            SavedBests::default()
        }
    }
}

/// Write the bests atomically: a temp file renamed over the old one.
pub fn save(path: &Path, bests: &SavedBests) -> std::io::Result<()> {
    crate::files::write_atomically(path, serde_json::to_string_pretty(bests)?.as_bytes())
}
