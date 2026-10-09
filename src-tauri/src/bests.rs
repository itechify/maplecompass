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

#[cfg(test)]
mod tests {
    use super::*;
    use compass_core::{MapBests, VisitBest};

    /// `personal-bests.json` in a folder of `test`'s own under the temp folder, emptied
    /// first.
    fn file(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("maplecompass-bests-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("personal-bests.json")
    }

    /// Sunset Sky's two best Visits, one set before the shell told the wall clock.
    fn sunset_sky_bests() -> SavedBests {
        let visits = vec![
            VisitBest { exp_per_hour: 7200, set_at_unix_ms: Some(1_791_547_200_000) },
            VisitBest { exp_per_hour: 3600, set_at_unix_ms: None },
        ];
        SavedBests { maps: vec![MapBests { id: "010003070".into(), visits }], ..SavedBests::default() }
    }

    #[test]
    fn saved_bests_load_as_they_were_saved() {
        let path = file("saved");
        save(&path, &sunset_sky_bests()).unwrap();
        assert_eq!(load(&path), sunset_sky_bests());
    }

    #[test]
    fn with_no_file_yet_there_are_no_bests() {
        let path = file("none");
        assert_eq!(load(&path), SavedBests::default());
        assert!(!path.with_extension("json.bad").exists());
    }

    #[test]
    fn an_unreadable_file_is_set_aside_and_there_are_no_bests() {
        let path = file("unreadable");
        let unreadable = r#"{ "version": 1, "maps": ["#;
        fs::write(&path, unreadable).unwrap();
        assert_eq!(load(&path), SavedBests::default());
        assert!(!path.exists());
        assert_eq!(fs::read_to_string(path.with_extension("json.bad")).unwrap(), unreadable);
    }

    #[test]
    fn a_file_saved_in_another_shape_is_set_aside_and_there_are_no_bests() {
        let path = file("another-shape");
        save(&path, &SavedBests { version: BESTS_VERSION + 1, ..sunset_sky_bests() }).unwrap();
        assert_eq!(load(&path), SavedBests::default());
        assert!(!path.exists());
        assert!(path.with_extension("json.bad").exists());
    }
}
