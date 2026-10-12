//! Keeping the market prices the player types into Item Cards, on their Market Lines,
//! between launches: the core's [`SavedMarketPrices`] as pretty JSON beside the settings
//! (`%APPDATA%\com.itechify.minerva\market-prices.json` on Windows), read at startup
//! and written whenever the core's market prices revision changes, but never over a file
//! that was there and didn't read. Beside it, the `market-imports` folder, where a tool of
//! the player's own leaves prices for Minerva to take in (ADR 0013).

use crate::errors::error;
use minerva_core::{MARKET_PRICES_VERSION, MarketImport, SavedMarketPrices};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("market-prices.json"))
}

pub fn imports_path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("market-imports"))
}

/// The Market Imports waiting in `dir` (ADR 0013), in their names' order: every `.json`
/// file in it but a dot file (a tool's, while it writes). Each is moved out of the tool's
/// way before it is read, so one the tool writes meanwhile waits for the next look, and
/// deleted once read. One that doesn't read, or is in another shape, is logged and set
/// aside as `<name>.bad`. None while there is no folder.
pub fn take_imports(dir: &Path) -> Vec<MarketImport> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut waiting: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            let name = path.file_name().map(|name| name.to_string_lossy()).unwrap_or_default();
            name.ends_with(".json") && !name.starts_with('.') && path.is_file()
        })
        .collect();
    waiting.sort();
    waiting.iter().filter_map(|path| take_import(path)).collect()
}

fn take_import(path: &Path) -> Option<MarketImport> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let taken = path.with_extension("json.taken");
    // Still being written, or gone: the next look tries again.
    fs::rename(path, &taken).ok()?;
    let why = match fs::read(&taken) {
        Err(e) => format!("unreadable ({e})"),
        Ok(bytes) => match serde_json::from_slice::<MarketImport>(&bytes) {
            Ok(import) if import.version == MARKET_PRICES_VERSION => {
                if let Err(e) = fs::remove_file(&taken) {
                    error!("market import {name} taken in, but not deleted ({e})");
                }
                return Some(import);
            }
            Ok(import) => format!("in shape {}", import.version),
            Err(e) => format!("unreadable ({e})"),
        },
    };
    match fs::rename(&taken, path.with_extension("json.bad")) {
        Ok(()) => error!("market import {name} {why}; set aside"),
        Err(e) => error!("market import {name} {why}, and not set aside ({e})"),
    }
    None
}

/// The market prices file, as this launch found it.
pub struct MarketPricesFile {
    path: PathBuf,
    /// It was there but didn't read, and couldn't be set aside: it is left as it was, and
    /// the prices aren't saved this launch.
    left: bool,
}

impl MarketPricesFile {
    /// The prices saved at `path`, or none when there is no file yet. One saved before
    /// there were Market Lines (shape 1, 0.8.0) reads as the first line's prices. A file
    /// that is there but doesn't read (reading it fails, it isn't JSON as the core saves
    /// it, or it was saved in a later shape, by a later version) is logged and set aside as
    /// `market-prices.json.bad`, and there are none. If it can't be set aside, it is left as
    /// it was: [`MarketPricesFile::save`] never writes over it.
    pub fn load(path: PathBuf) -> (Self, SavedMarketPrices) {
        let why = match fs::read(&path) {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return (Self { path, left: false }, SavedMarketPrices::default());
            }
            Err(e) => format!("unreadable ({e})"),
            Ok(bytes) => match serde_json::from_slice::<SavedMarketPrices>(&bytes) {
                Ok(prices) if prices.version == MARKET_PRICES_VERSION => return (Self { path, left: false }, prices),
                Ok(prices) => format!("saved in shape {}", prices.version),
                Err(e) => format!("unreadable ({e})"),
            },
        };
        let left = match fs::rename(&path, path.with_extension("json.bad")) {
            Ok(()) => {
                error!("market prices {why}; set aside, starting with none");
                false
            }
            Err(e) => {
                error!("market prices {why}, and not set aside ({e}); starting with none, saving none");
                true
            }
        };
        (Self { path, left }, SavedMarketPrices::default())
    }

    /// Whether [`MarketPricesFile::save`] writes them: not if the file was left as it was
    /// at [`MarketPricesFile::load`].
    pub fn saves(&self) -> bool {
        !self.left
    }

    /// Write the prices atomically: a temp file renamed over the old one. Nothing, if the
    /// file was left as it was at [`MarketPricesFile::load`].
    pub fn save(&self, prices: &SavedMarketPrices) -> std::io::Result<()> {
        if self.left {
            return Ok(());
        }
        crate::files::write_atomically(&self.path, serde_json::to_string_pretty(prices)?.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minerva_core::{MarketPrice, SavedMarketLine};

    /// `market-prices.json` in a folder of `test`'s own under the temp folder, emptied first.
    fn file(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("minerva-market-prices-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("market-prices.json")
    }

    #[test]
    fn prices_saved_are_read_back_at_the_next_launch() {
        let path = file("round-trip");
        let (first, none) = MarketPricesFile::load(path.clone());
        assert_eq!(none, SavedMarketPrices::default());

        let mut prices = SavedMarketPrices::default();
        prices.lines[0].prices.insert("04010000".into(), MarketPrice { mesos: 450_000, set_at_unix_ms: Some(1_791_720_000_000) });
        prices.lines.push(SavedMarketLine { id: "2".into(), name: "Lowest ask".into(), prices: Default::default() });
        first.save(&prices).unwrap();

        assert_eq!(MarketPricesFile::load(path).1, prices);
    }

    #[test]
    fn prices_saved_before_there_were_lines_are_read_as_the_first_lines() {
        let path = file("shape-1");
        fs::write(&path, br#"{"version":1,"items":{"04010000":{"mesos":450000}}}"#).unwrap();

        let (_, prices) = MarketPricesFile::load(path.clone());

        assert_eq!(prices.version, MARKET_PRICES_VERSION);
        assert_eq!(prices.lines.len(), 1);
        assert_eq!(prices.lines[0].prices["04010000"].mesos, 450_000);
        assert!(path.exists(), "not set aside");
    }

    #[test]
    fn prices_saved_in_a_later_shape_are_set_aside() {
        let path = file("later-shape");
        fs::write(&path, br#"{"version":3,"lines":[]}"#).unwrap();

        let (_, prices) = MarketPricesFile::load(path.clone());

        assert_eq!(prices, SavedMarketPrices::default());
        assert!(path.with_extension("json.bad").exists());
    }

    /// The `market-imports` folder in a folder of `test`'s own under the temp folder, made
    /// afresh, holding `files` (name, contents).
    fn imports(test: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("minerva-market-imports-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let imports = dir.join("market-imports");
        fs::create_dir_all(&imports).unwrap();
        for (name, contents) in files {
            fs::write(imports.join(name), contents).unwrap();
        }
        imports
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> =
            fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    const TYPICAL: &str = r#"{"version":2,"lines":[{"id":"tool-typical","name":"Typical","prices":{"04010000":{"mesos":500000,"setAtUnixMs":1791720000000}}}]}"#;
    const ASK: &str = r#"{"version":2,"lines":[{"id":"tool-ask","name":"Lowest ask","prices":{}}]}"#;

    #[test]
    fn imports_waiting_are_taken_in_their_names_order_and_deleted() {
        let dir = imports("taken", &[("tool-b.json", ASK), ("tool-a.json", TYPICAL)]);

        let taken = take_imports(&dir);

        let ids: Vec<&str> = taken.iter().map(|import| import.lines[0].id.as_str()).collect();
        assert_eq!(ids, ["tool-typical", "tool-ask"]);
        assert_eq!(taken[0].lines[0].prices["04010000"].mesos, 500_000);
        assert_eq!(names_in(&dir), Vec::<String>::new());
        assert!(take_imports(&dir).is_empty(), "each once");
    }

    #[test]
    fn a_tools_dot_files_and_files_of_other_kinds_are_left_alone() {
        let dir = imports("left", &[(".tool.json", TYPICAL), ("tool.json.tmp", TYPICAL), ("notes.txt", "hi")]);

        assert!(take_imports(&dir).is_empty());
        assert_eq!(names_in(&dir), [".tool.json", "notes.txt", "tool.json.tmp"]);
    }

    #[test]
    fn an_import_that_doesnt_read_or_is_in_another_shape_is_set_aside() {
        let dir = imports("bad", &[("broken.json", "not json"), ("later.json", r#"{"version":3,"lines":[]}"#), ("tool.json", TYPICAL)]);

        let taken = take_imports(&dir);

        assert_eq!(taken.len(), 1, "the one that reads");
        assert_eq!(names_in(&dir), ["broken.json.bad", "later.json.bad"]);
        assert_eq!(fs::read(dir.join("broken.json.bad")).unwrap(), b"not json");
    }

    #[test]
    fn without_the_folder_there_are_no_imports() {
        let dir = imports("none", &[]);
        fs::remove_dir(&dir).unwrap();

        assert!(take_imports(&dir).is_empty());
    }

    #[test]
    fn a_file_that_doesnt_read_is_set_aside_and_none_are_kept() {
        let path = file("unreadable");
        fs::write(&path, b"not json").unwrap();

        let (_, prices) = MarketPricesFile::load(path.clone());

        assert_eq!(prices, SavedMarketPrices::default());
        assert_eq!(fs::read(path.with_extension("json.bad")).unwrap(), b"not json");
        assert!(!path.exists());
    }
}
