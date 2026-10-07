//! The MeowDB map data source (ADR 0003): fetches pages over HTTPS with an honest
//! user agent, keeps them in a disk cache, and leaves reading them to the core.
//!
//! Politeness: the core asks for the map index once at startup (again only after a
//! failure, at most once a minute), and for a map's page only once the player is
//! confirmed on that map. A cached page younger than [`FRESH_FOR`] is used without any
//! request, so revisits within a day cost nothing; an older one is refetched, and
//! still used if MeowDB can't be reached. Every page MeowDB serves is cached, even one
//! the core can't read, so a format change costs one request a day rather than one a
//! minute.

use compass_core::{MapData, MapDataSource, MapEntry, SourceError, meowdb};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::task::Poll;
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
    /// The map page being fetched in the background, and where its data will arrive.
    in_flight: Option<(String, Receiver<Result<MapData, SourceError>>)>,
}

impl MeowDbSource {
    /// A source caching pages under `cache_dir` (created when first written).
    pub fn new(cache_dir: PathBuf) -> reqwest::Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()?;
        Ok(Self { client, cache_dir, in_flight: None })
    }

    /// Start fetching map `id`'s page in the background; its data arrives on the receiver.
    fn fetch_map(&self, id: &str, stale: Option<String>) -> Receiver<Result<MapData, SourceError>> {
        let (tx, rx) = mpsc::channel();
        let (client, id, path) = (self.client.clone(), id.to_string(), self.map_file(id));
        tauri::async_runtime::spawn(async move {
            let url = meowdb::map_url(&id);
            let fetched = fetch(&client, &url).await;
            let _ = tx.send(settle(&path, &url, fetched, stale, |page| meowdb::read_map_page(page, &id)));
        });
        rx
    }

    fn map_file(&self, id: &str) -> PathBuf {
        self.cache_dir.join(format!("map-{id}.html"))
    }
}

impl MapDataSource for MeowDbSource {
    fn map_index(&mut self) -> Result<Vec<MapEntry>, SourceError> {
        let path = self.cache_dir.join(INDEX_FILE);
        let index = match cached(&path) {
            Cached::Fresh(page) => meowdb::read_map_index(&page),
            Cached::Stale(page) => {
                // Called from the compass thread, never from inside the async runtime,
                // so blocking on it here is safe.
                let fetched = tauri::async_runtime::block_on(fetch(&self.client, meowdb::MAP_INDEX_URL));
                settle(&path, meowdb::MAP_INDEX_URL, fetched, page, meowdb::read_map_index)
            }
        };
        if let Err(e) = &index {
            eprintln!("map index unavailable: {e}");
        }
        index
    }

    fn map_data(&mut self, id: &str) -> Poll<Result<MapData, SourceError>> {
        if let Some((asked, arriving)) = &self.in_flight {
            if asked == id {
                let data = match arriving.try_recv() {
                    Err(TryRecvError::Empty) => return Poll::Pending,
                    Ok(data) => data,
                    Err(TryRecvError::Disconnected) => Err(SourceError::Unreachable("fetch abandoned".into())),
                };
                self.in_flight = None;
                if let Err(e) = &data {
                    eprintln!("map {id}: {e}");
                }
                return Poll::Ready(data);
            }
        }
        // A new map: an earlier fetch still in flight finishes into the cache unheard.
        match cached(&self.map_file(id)) {
            Cached::Fresh(page) => Poll::Ready(meowdb::read_map_page(&page, id)),
            Cached::Stale(page) => {
                self.in_flight = Some((id.to_string(), self.fetch_map(id, page)));
                Poll::Pending
            }
        }
    }
}

/// A cached page young enough to use as it is, or an older copy (if any) to fall back
/// on when MeowDB can't be reached.
enum Cached {
    Fresh(String),
    Stale(Option<String>),
}

fn cached(path: &Path) -> Cached {
    match std::fs::read_to_string(path) {
        // Even when it can't be read: refetching won't fix a format change until
        // MeowDB changes again, so it waits out the day like any page.
        Ok(page) if is_fresh(path) => Cached::Fresh(page),
        Ok(page) => Cached::Stale(Some(page)),
        Err(_) => Cached::Stale(None),
    }
}

async fn fetch(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let response = client.get(url).send().await.and_then(|r| r.error_for_status());
    let page = match response {
        Ok(response) => response.text().await,
        Err(e) => Err(e),
    };
    page.map_err(|e| e.to_string())
}

/// After a fetch: cache and read what MeowDB served, or if it couldn't be reached,
/// read the stale copy if there is one.
fn settle<T>(
    path: &Path,
    url: &str,
    fetched: Result<String, String>,
    stale: Option<String>,
    read: impl Fn(&str) -> Result<T, SourceError>,
) -> Result<T, SourceError> {
    match fetched {
        Ok(page) => {
            if let Err(e) = write_atomically(path, &page) {
                eprintln!("MeowDB cache: couldn't write {}: {e}", path.display());
            }
            read(&page)
        }
        Err(e) => {
            eprintln!("MeowDB: {url}: {e}");
            match stale {
                Some(page) => read(&page),
                None => Err(SourceError::Unreachable(e)),
            }
        }
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
