//! The shell's data source: OSMS Data Explorer's file set and pictures (ADR 0008, in
//! [`crate::osms`] and [`crate::pictures`]), and MeowDB's search (ADR 0007), fetched over
//! HTTPS with an honest user agent and kept in a disk cache. Reading them is left to the
//! core. No MeowDB page is fetched.
//!
//! Item Lookup (ADR 0007): the core asks MeowDB's search for an item's name once per
//! lookup the player makes. An answer is cached for [`ITEM_SEARCH_FRESH_FOR`] (a week),
//! so looking the same item up again asks MeowDB nothing. Every answer MeowDB gives is
//! cached, even one the core can't read.
//!
//! Fetches run on the async runtime and never block the caller: the first call starts
//! one and answers `Pending`, later calls collect it.

use crate::osms::OsmsSet;
use crate::pictures::Pictures;
use compass_core::osms::OsmsFiles;
use compass_core::{Fetched, ItemHit, MapDataSource, SourceError, meowdb};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::task::Poll;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long a cached answer of MeowDB's search is used without asking again: the items a
/// name finds hardly change.
const ITEM_SEARCH_FRESH_FOR: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Who is asking: name, version, purpose and where to find out more. Never a browser UA.
const USER_AGENT: &str = concat!(
    "MapleCompass/",
    env!("CARGO_PKG_VERSION"),
    " (personal-use map helper; +https://github.com/itechify/maplecompass)"
);

type Answer<T> = Result<Fetched<T>, SourceError>;

/// An HTTP client that says who is asking, with MapleCompass's timeouts.
pub(crate) fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
}

pub struct MeowDbSource {
    client: reqwest::Client,
    cache_dir: PathBuf,
    /// OSMS's file set.
    osms: OsmsSet,
    /// OSMS's pictures.
    pictures: Pictures,
    /// The item name being searched for, and where MeowDB's items will arrive.
    item_search_in_flight: Option<(String, Receiver<Answer<Vec<ItemHit>>>)>,
}

impl MeowDbSource {
    /// A source caching under `cache_dir` (created when first written).
    pub fn new(cache_dir: PathBuf) -> reqwest::Result<Self> {
        let client = client()?;
        Ok(Self {
            osms: OsmsSet::new(client.clone(), &cache_dir),
            pictures: Pictures::new(client.clone(), &cache_dir),
            client,
            cache_dir,
            item_search_in_flight: None,
        })
    }

    /// In the background: fetch `url` and cache it at `path`. What `read` makes of it
    /// (or of the saved copy, if MeowDB can't be reached) arrives on the receiver.
    fn fetch<T: Send + 'static>(
        &self,
        url: String,
        path: PathBuf,
        read: impl Fn(&str) -> Result<T, SourceError> + Send + 'static,
    ) -> Receiver<Answer<T>> {
        let (tx, rx) = mpsc::channel();
        let client = self.client.clone();
        tauri::async_runtime::spawn(async move {
            let fetched = fetch(&client, &url).await;
            let _ = tx.send(settle(&path, &url, fetched, read));
        });
        rx
    }
}

impl MapDataSource for MeowDbSource {
    fn osms_files(&mut self) -> Poll<Answer<OsmsFiles>> {
        self.osms.poll()
    }

    fn picture(&mut self, path: &str) -> Poll<Result<String, SourceError>> {
        self.pictures.poll(path)
    }

    fn item_search(&mut self, name: &str) -> Poll<Answer<Vec<ItemHit>>> {
        if let Some((asked, arriving)) = &self.item_search_in_flight {
            if asked == name {
                let Poll::Ready(items) = collect(arriving) else { return Poll::Pending };
                self.item_search_in_flight = None;
                if let Err(e) = &items {
                    eprintln!("item search {name:?}: {e}");
                }
                return Poll::Ready(items);
            }
        }
        // Read here, so a cached answer is given at once and the page opens right away.
        let path = self.item_search_file(name);
        if let Some(answer) = cached(&path, ITEM_SEARCH_FRESH_FOR) {
            return Poll::Ready(meowdb::read_item_search(&answer).map(Fetched::Fresh));
        }
        let url = match reqwest::Url::parse_with_params(meowdb::ITEM_SEARCH_URL, [("q", name), ("locale", "en")]) {
            Ok(url) => url.to_string(),
            Err(e) => return Poll::Ready(Err(SourceError::FormatChanged(format!("search URL: {e}")))),
        };
        let arriving = self.fetch(url, path, meowdb::read_item_search);
        self.item_search_in_flight = Some((name.to_string(), arriving));
        Poll::Pending
    }
}

impl MeowDbSource {
    fn item_search_file(&self, name: &str) -> PathBuf {
        // The name as hex: any name makes a safe file name, and no two share one.
        let key: String = name.to_lowercase().bytes().map(|b| format!("{b:02x}")).collect();
        self.cache_dir.join(format!("item-search-{key}.json"))
    }
}

/// What a background fetch sent, if it has finished.
fn collect<T>(arriving: &Receiver<Result<T, SourceError>>) -> Poll<Result<T, SourceError>> {
    match arriving.try_recv() {
        Err(TryRecvError::Empty) => Poll::Pending,
        Ok(answer) => Poll::Ready(answer),
        Err(TryRecvError::Disconnected) => Poll::Ready(Err(SourceError::Unreachable("fetch abandoned".into()))),
    }
}

/// The page cached at `path`, if it was saved less than `fresh_for` ago. Even one that
/// can't be read: refetching won't fix a format change until MeowDB changes again, so
/// it waits out its freshness like any page.
fn cached(path: &Path, fresh_for: Duration) -> Option<String> {
    let saved_at = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let age = SystemTime::now().duration_since(saved_at).ok()?;
    if age < fresh_for { std::fs::read_to_string(path).ok() } else { None }
}

/// The page cached at `path`, however old, and when it was saved (ms since the Unix
/// epoch): what to fall back on when MeowDB can't be reached.
fn saved_copy(path: &Path) -> Option<(String, u64)> {
    let page = std::fs::read_to_string(path).ok()?;
    let saved_at = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let since_epoch = saved_at.and_then(|t| t.duration_since(UNIX_EPOCH).ok()).unwrap_or_default();
    Some((page, since_epoch.as_millis() as u64))
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
/// read the copy saved at `path` if there is one.
fn settle<T>(
    path: &Path,
    url: &str,
    fetched: Result<String, String>,
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
            match saved_copy(path) {
                Some((page, saved_at_unix_ms)) => read(&page).map(|data| Fetched::SavedCopy { data, saved_at_unix_ms }),
                None => Err(SourceError::Unreachable(e)),
            }
        }
    }
}
