//! The MeowDB map data source (ADR 0003): fetches pages over HTTPS with an honest
//! user agent, keeps them in a disk cache, and leaves reading them to the core.
//!
//! Politeness: the core asks for the map index once at startup (again only after a
//! failure, at most once a minute). A cached page younger than [`FRESH_FOR`] is used
//! without any request; an older one is refetched, and still used if MeowDB can't be
//! reached. Every page MeowDB serves is cached, even one the core can't read, so a
//! format change costs one request a day rather than one a minute.

use compass_core::{MapDataSource, MapEntry, SourceError, meowdb};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How long a cached page is used without asking MeowDB again. MeowDB's figures will
/// change after the Oct 21 launch, so not much longer than a day.
const FRESH_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// Who is asking: name, version, purpose and where to find out more. Never a browser UA.
const USER_AGENT: &str = concat!(
    "MapleCompass/",
    env!("CARGO_PKG_VERSION"),
    " (personal-use map helper; +https://github.com/itechify/maplecompass)"
);

/// Cache file for the "All Maps A-Z" page.
const INDEX_FILE: &str = "map-index.html";

pub struct MeowDbSource {
    client: reqwest::Client,
    cache_dir: PathBuf,
}

impl MeowDbSource {
    /// A source caching pages under `cache_dir` (created when first written).
    pub fn new(cache_dir: PathBuf) -> reqwest::Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()?;
        Ok(Self { client, cache_dir })
    }

    /// Fetch a page. Called from the compass thread, never from inside the async
    /// runtime, so blocking on it here is safe.
    fn fetch(&self, url: &str) -> Result<String, String> {
        tauri::async_runtime::block_on(async {
            let response = self.client.get(url).send().await?.error_for_status()?;
            response.text().await
        })
        .map_err(|e| e.to_string())
    }

    /// The cached page at `file` read with `read`, else a fresh fetch of `url`, else
    /// a stale cached copy.
    fn page<T>(
        &self,
        file: &str,
        url: &str,
        read: impl Fn(&str) -> Result<T, SourceError>,
    ) -> Result<T, SourceError> {
        let path = self.cache_dir.join(file);
        let cached = std::fs::read_to_string(&path).ok();
        if let Some(page) = &cached {
            if is_fresh(&path) {
                // Even when it can't be read: refetching won't fix a format change
                // until MeowDB changes again, so it waits out the day like any page.
                return read(page);
            }
        }
        match self.fetch(url) {
            Ok(page) => {
                if let Err(e) = write_atomically(&path, &page) {
                    eprintln!("MeowDB cache: couldn't write {}: {e}", path.display());
                }
                read(&page)
            }
            Err(e) => {
                eprintln!("MeowDB: {url}: {e}");
                match cached {
                    Some(page) => read(&page),
                    None => Err(SourceError::Unreachable(e)),
                }
            }
        }
    }
}

impl MapDataSource for MeowDbSource {
    fn map_index(&mut self) -> Result<Vec<MapEntry>, SourceError> {
        let index = self.page(INDEX_FILE, meowdb::MAP_INDEX_URL, meowdb::read_map_index);
        if let Err(e) = &index {
            eprintln!("map index unavailable: {e}");
        }
        index
    }
}

fn is_fresh(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < FRESH_FOR)
}

/// Write via a temporary file and a rename, so a crash never leaves half a page.
fn write_atomically(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}
