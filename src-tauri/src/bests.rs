//! Keeping the Personal Bests between launches: the core's [`SavedBests`] as pretty JSON
//! beside the settings (`%APPDATA%\com.itechify.minerva\personal-bests.json` on
//! Windows), read at startup and written whenever the core's bests revision changes, but
//! never over a file that was there and didn't read.

use crate::errors::error;
use minerva_core::{BESTS_VERSION, SavedBests};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("personal-bests.json"))
}

/// The bests file, as this launch found it.
pub struct BestsFile {
    path: PathBuf,
    /// It was there but didn't read, and couldn't be set aside: it is left as it was, and
    /// the bests aren't saved this launch.
    left: bool,
}

impl BestsFile {
    /// The bests saved at `path`, or none when there is no file yet. A file that is there
    /// but doesn't read (reading it fails, it isn't UTF-8 JSON as the core saves it, or
    /// it was saved in another shape, by a later version) is logged and set aside as
    /// `personal-bests.json.bad` for diagnosis, and there are none. If it can't be set
    /// aside, it is left as it was: [`BestsFile::save`] never writes over it.
    pub fn load(path: PathBuf) -> (Self, SavedBests) {
        let why = match fs::read(&path) {
            Err(e) if e.kind() == ErrorKind::NotFound => return (Self { path, left: false }, SavedBests::default()),
            Err(e) => format!("unreadable ({e})"),
            Ok(bytes) => match serde_json::from_slice::<SavedBests>(&bytes) {
                Ok(bests) if bests.version == BESTS_VERSION => return (Self { path, left: false }, bests),
                Ok(bests) => format!("saved in shape {}", bests.version),
                Err(e) => format!("unreadable ({e})"),
            },
        };
        let left = match fs::rename(&path, path.with_extension("json.bad")) {
            Ok(()) => {
                error!("personal bests {why}; set aside, starting with none");
                false
            }
            Err(e) => {
                error!("personal bests {why}, and not set aside ({e}); starting with none, saving none");
                true
            }
        };
        (Self { path, left }, SavedBests::default())
    }

    /// Write the bests atomically: a temp file renamed over the old one. Nothing, if the
    /// file was left as it was at [`BestsFile::load`].
    pub fn save(&self, bests: &SavedBests) -> std::io::Result<()> {
        if self.left {
            return Ok(());
        }
        crate::files::write_atomically(&self.path, serde_json::to_string_pretty(bests)?.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minerva_core::{MapBests, VisitBest};
    use std::path::Path;

    /// `personal-bests.json` in a folder of `test`'s own under the temp folder, emptied
    /// first.
    fn file(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("minerva-bests-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("personal-bests.json")
    }

    /// The bests a launch finds at `path`.
    fn load(path: &Path) -> SavedBests {
        BestsFile::load(path.to_path_buf()).1
    }

    /// A launch that finds no file at `path` saves `bests` there.
    fn save(path: &Path, bests: &SavedBests) -> std::io::Result<()> {
        BestsFile::load(path.to_path_buf()).0.save(bests)
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
    fn a_file_that_is_not_utf8_is_set_aside_and_there_are_no_bests() {
        // Re-saved as UTF-16 (PowerShell 5.1's `>`, say): a byte-order mark, then two
        // bytes a character.
        let path = file("utf-16");
        let json = serde_json::to_string_pretty(&sunset_sky_bests()).unwrap();
        let utf16: Vec<u8> = [0xFF, 0xFE].into_iter().chain(json.encode_utf16().flat_map(u16::to_le_bytes)).collect();
        fs::write(&path, &utf16).unwrap();
        assert_eq!(load(&path), SavedBests::default());
        assert!(!path.exists());
        assert_eq!(fs::read(path.with_extension("json.bad")).unwrap(), utf16);
    }

    #[test]
    fn a_file_that_cannot_be_read_is_set_aside_and_there_are_no_bests() {
        // A folder where the file should be: it is there, but reading it fails.
        let path = file("not-a-file");
        fs::create_dir(&path).unwrap();
        assert_eq!(load(&path), SavedBests::default());
        assert!(!path.exists());
        assert!(path.with_extension("json.bad").is_dir());
    }

    #[test]
    fn an_unreadable_file_that_cannot_be_set_aside_is_not_written_over() {
        let path = file("kept");
        let unreadable = r#"{ "version": 1, "maps": ["#;
        fs::write(&path, unreadable).unwrap();
        // A folder in the way of setting it aside.
        fs::create_dir(path.with_extension("json.bad")).unwrap();
        let (launch, bests) = BestsFile::load(path.clone());
        assert_eq!(bests, SavedBests::default());

        // The launch's bests aren't saved over it.
        launch.save(&sunset_sky_bests()).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), unreadable);
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
