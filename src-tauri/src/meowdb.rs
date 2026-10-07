//! The MeowDB map data source (ADR 0003): fetches pages over HTTPS with an honest
//! user agent, keeps them in a disk cache, and leaves reading them to the core.
//!
//! Politeness: the core asks for the map index once at startup (again only after a
//! failure, at most once a minute), and for a map's page only once the player is
//! confirmed on that map (or, for a title several maps share, on that title). A cached
//! page younger than [`FRESH_FOR`] is used without any request, so revisits within a
//! day cost nothing; an older one is refetched, and if MeowDB can't be reached it is
//! still used, as a saved copy the panel labels with its date. Every page MeowDB
//! serves is cached, even one the core can't read, so a format change costs one
//! request a day rather than one a minute.
//!
//! Fetches run on the async runtime and never block the caller: the first call starts
//! one and answers `Pending`, later calls collect it.

use compass_core::{Fetched, MapData, MapDataSource, MapEntry, SourceError, meowdb};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::task::Poll;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

type Answer<T> = Result<Fetched<T>, SourceError>;

pub struct MeowDbSource {
    client: reqwest::Client,
    cache_dir: PathBuf,
    /// The map index being fetched in the background.
    index_in_flight: Option<Receiver<Answer<Vec<MapEntry>>>>,
    /// The map page being fetched in the background, and where its data will arrive.
    map_in_flight: Option<(String, Receiver<Answer<MapData>>)>,
}

impl MeowDbSource {
    /// A source caching pages under `cache_dir` (created when first written).
    pub fn new(cache_dir: PathBuf) -> reqwest::Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()?;
        Ok(Self { client, cache_dir, index_in_flight: None, map_in_flight: None })
    }

    fn map_file(&self, id: &str) -> PathBuf {
        self.cache_dir.join(format!("map-{id}.html"))
    }

    /// Start fetching `url` in the background, caching it at `path`; what `read` makes
    /// of it (or of the saved copy, if MeowDB can't be reached) arrives on the receiver.
    fn fetch<T: Send + 'static>(
        &self,
        url: String,
        path: PathBuf,
        saved: Option<SavedCopy>,
        read: impl Fn(&str) -> Result<T, SourceError> + Send + 'static,
    ) -> Receiver<Answer<T>> {
        let (tx, rx) = mpsc::channel();
        let client = self.client.clone();
        tauri::async_runtime::spawn(async move {
            let fetched = fetch(&client, &url).await;
            let _ = tx.send(settle(&path, &url, fetched, saved, read));
        });
        rx
    }
}

impl MapDataSource for MeowDbSource {
    fn map_index(&mut self) -> Poll<Answer<Vec<MapEntry>>> {
        if let Some(arriving) = &self.index_in_flight {
            let Poll::Ready(index) = collect(arriving) else { return Poll::Pending };
            self.index_in_flight = None;
            if let Err(e) = &index {
                eprintln!("map index unavailable: {e}");
            }
            return Poll::Ready(index);
        }
        let path = self.cache_dir.join(INDEX_FILE);
        match cached(&path) {
            Cached::Fresh(page) => Poll::Ready(meowdb::read_map_index(&page).map(Fetched::Fresh)),
            Cached::Stale(saved) => {
                let url = meowdb::MAP_INDEX_URL.to_string();
                self.index_in_flight = Some(self.fetch(url, path, saved, meowdb::read_map_index));
                Poll::Pending
            }
        }
    }

    fn map_data(&mut self, id: &str) -> Poll<Answer<MapData>> {
        if let Some((asked, arriving)) = &self.map_in_flight {
            if asked == id {
                let Poll::Ready(data) = collect(arriving) else { return Poll::Pending };
                self.map_in_flight = None;
                if let Err(e) = &data {
                    eprintln!("map {id}: {e}");
                }
                return Poll::Ready(data);
            }
        }
        // A new map: an earlier fetch still in flight finishes into the cache unheard.
        let path = self.map_file(id);
        match cached(&path) {
            Cached::Fresh(page) => Poll::Ready(meowdb::read_map_page(&page, id).map(Fetched::Fresh)),
            Cached::Stale(saved) => {
                let owned = id.to_string();
                let read = move |page: &str| meowdb::read_map_page(page, &owned);
                self.map_in_flight = Some((id.to_string(), self.fetch(meowdb::map_url(id), path, saved, read)));
                Poll::Pending
            }
        }
    }
}

/// What a background fetch sent, if it has finished.
fn collect<T>(arriving: &Receiver<Answer<T>>) -> Poll<Answer<T>> {
    match arriving.try_recv() {
        Err(TryRecvError::Empty) => Poll::Pending,
        Ok(answer) => Poll::Ready(answer),
        Err(TryRecvError::Disconnected) => Poll::Ready(Err(SourceError::Unreachable("fetch abandoned".into()))),
    }
}

/// A cached page too old to use as it is, and when it was saved.
struct SavedCopy {
    page: String,
    saved_at_unix_ms: u64,
}

/// A cached page young enough to use as it is, or an older copy (if any) to fall back
/// on when MeowDB can't be reached.
enum Cached {
    Fresh(String),
    Stale(Option<SavedCopy>),
}

fn cached(path: &Path) -> Cached {
    let Ok(page) = std::fs::read_to_string(path) else { return Cached::Stale(None) };
    let saved_at = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let age = saved_at.and_then(|t| SystemTime::now().duration_since(t).ok());
    match (age, saved_at) {
        // Even when it can't be read: refetching won't fix a format change until
        // MeowDB changes again, so it waits out the day like any page.
        (Some(age), _) if age < FRESH_FOR => Cached::Fresh(page),
        (_, saved_at) => {
            let since_epoch = saved_at.and_then(|t| t.duration_since(UNIX_EPOCH).ok()).unwrap_or_default();
            Cached::Stale(Some(SavedCopy { page, saved_at_unix_ms: since_epoch.as_millis() as u64 }))
        }
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
/// read the saved copy if there is one.
fn settle<T>(
    path: &Path,
    url: &str,
    fetched: Result<String, String>,
    saved: Option<SavedCopy>,
    read: impl Fn(&str) -> Result<T, SourceError>,
) -> Answer<T> {
    match fetched {
        Ok(page) => {
            if let Err(e) = crate::files::write_atomically(path, page.as_bytes()) {
                eprintln!("MeowDB cache: couldn't write {}: {e}", path.display());
            }
            read(&page).map(Fetched::Fresh)
        }
        Err(e) => {
            eprintln!("MeowDB: {url}: {e}");
            match saved {
                Some(SavedCopy { page, saved_at_unix_ms }) => {
                    read(&page).map(|data| Fetched::SavedCopy { data, saved_at_unix_ms })
                }
                None => Err(SourceError::Unreachable(e)),
            }
        }
    }
}
