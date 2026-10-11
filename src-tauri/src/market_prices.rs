//! Keeping the market prices the player types into Item Cards between launches: the core's
//! [`SavedMarketPrices`] as pretty JSON beside the settings
//! (`%APPDATA%\com.itechify.maplecompass\market-prices.json` on Windows), read at startup
//! and written whenever the core's market prices revision changes, but never over a file
//! that was there and didn't read.

use crate::errors::error;
use compass_core::{MARKET_PRICES_VERSION, SavedMarketPrices};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("market-prices.json"))
}

/// The market prices file, as this launch found it.
pub struct MarketPricesFile {
    path: PathBuf,
    /// It was there but didn't read, and couldn't be set aside: it is left as it was, and
    /// the prices aren't saved this launch.
    left: bool,
}

impl MarketPricesFile {
    /// The prices saved at `path`, or none when there is no file yet. A file that is there
    /// but doesn't read (reading it fails, it isn't JSON as the core saves it, or it was
    /// saved in another shape, by a later version) is logged and set aside as
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
    use compass_core::MarketPrice;

    /// `market-prices.json` in a folder of `test`'s own under the temp folder, emptied first.
    fn file(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("maplecompass-market-prices-{test}-{}", std::process::id()));
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
        prices.items.insert("04010000".into(), MarketPrice { mesos: 450_000, set_at_unix_ms: Some(1_791_720_000_000) });
        first.save(&prices).unwrap();

        assert_eq!(MarketPricesFile::load(path).1, prices);
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
