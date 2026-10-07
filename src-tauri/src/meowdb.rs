//! The MeowDB map data source (ADR 0003, ADR 0004): fetches pages over HTTPS with an
//! honest user agent, keeps them in a disk cache, and leaves reading them to the core.
//!
//! Politeness: the core asks for the map index once at startup (again only after a
//! failure, at most once a minute), and for the shown map's page once the player is
//! confirmed on that map (or, for a title several maps share, on that title). A cached
//! page younger than [`FRESH_FOR`] is used for it without any request, so revisits
//! within a day cost nothing; an older one is refetched, and if MeowDB can't be reached
//! it is still used, as a saved copy the panel labels with its date. Every page MeowDB
//! serves is cached, even one the core can't read, so a format change costs one
//! request a day rather than one a minute.
//!
//! The core's world crawl (ADR 0004) asks for every map's page in turn, paced to about
//! one request every 2 seconds. For it a cached page is good for [`CRAWL_FRESH_FOR`]
//! (a week), and is answered at once so the crawl doesn't wait. Its fetches have their
//! own slot, so they never abandon or hold up the shown map's. Once a page fetch finds
//! MeowDB unreachable, the crawl uses older saved copies as they are for a while
//! ([`OFFLINE_RETRY`]) rather than wait on a failing fetch for each.
//!
//! Fetches run on the async runtime and never block the caller: the first call starts
//! one and answers `Pending`, later calls collect it.

use compass_core::{Fetched, MapData, MapDataSource, MapEntry, SourceError, meowdb};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::task::Poll;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How long a cached page is used without asking MeowDB again. MeowDB's figures will
/// change after the Oct 21 launch, so not much longer than a day.
const FRESH_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// How long a cached page is used for the world crawl without asking MeowDB again
/// (ADR 0004): a week, so the crawl refetches each page at most once a week.
const CRAWL_FRESH_FOR: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// How long after MeowDB was found unreachable the crawl uses older saved copies as
/// they are, without trying MeowDB for each page first.
const OFFLINE_RETRY: Duration = Duration::from_secs(5 * 60);

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
    /// The shown map's page fetches.
    shown_map_pages: PageFetches,
    /// The world crawl's page fetches: their own, so asking for one never abandons the
    /// shown map's fetch.
    crawl_pages: PageFetches,
    /// When a map page fetch last found MeowDB unreachable; `None` once one got through.
    unreachable_at: Option<Instant>,
}

impl MeowDbSource {
    /// A source caching pages under `cache_dir` (created when first written).
    pub fn new(cache_dir: PathBuf) -> reqwest::Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()?;
        Ok(Self {
            client,
            cache_dir,
            index_in_flight: None,
            shown_map_pages: PageFetches::new(FRESH_FOR),
            crawl_pages: PageFetches::new(CRAWL_FRESH_FOR),
            unreachable_at: None,
        })
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
        match cached(&path, FRESH_FOR) {
            Cached::Fresh(page) => Poll::Ready(meowdb::read_map_index(&page).map(Fetched::Fresh)),
            Cached::Stale(saved) => {
                let url = meowdb::MAP_INDEX_URL.to_string();
                self.index_in_flight = Some(self.fetch(url, path, saved, meowdb::read_map_index));
                Poll::Pending
            }
        }
    }

    fn map_data(&mut self, id: &str) -> Poll<Answer<MapData>> {
        self.map_page(PageAsker::ShownMap, id)
    }

    fn crawl_data(&mut self, id: &str) -> Poll<Answer<MapData>> {
        self.map_page(PageAsker::Crawl, id)
    }
}

/// Who asks for a map page: the panel, for the map it shows, or the world crawl.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PageAsker {
    ShownMap,
    Crawl,
}

/// One asker's map page fetches. Each asker has its own, so the crawl never abandons or
/// holds up the shown map's fetch.
struct PageFetches {
    /// How long a cached page is used for this asker without asking MeowDB again.
    fresh_for: Duration,
    /// The page being fetched in the background, and where its data will arrive.
    in_flight: Option<(String, Receiver<Answer<MapData>>)>,
}

impl PageFetches {
    fn new(fresh_for: Duration) -> Self {
        Self { fresh_for, in_flight: None }
    }
}

impl MeowDbSource {
    fn fetches(&mut self, asker: PageAsker) -> &mut PageFetches {
        match asker {
            PageAsker::ShownMap => &mut self.shown_map_pages,
            PageAsker::Crawl => &mut self.crawl_pages,
        }
    }

    /// Whether MeowDB couldn't be reached lately: the crawl then takes its saved copies
    /// as they are, rather than wait on a fetch for each that would likely fail too.
    fn offline(&self) -> bool {
        self.unreachable_at.is_some_and(|at| at.elapsed() < OFFLINE_RETRY)
    }

    /// Map `id`'s data for `asker`: from the cache if it's young enough for them,
    /// otherwise fetched in the background.
    fn map_page(&mut self, asker: PageAsker, id: &str) -> Poll<Answer<MapData>> {
        let fetches = self.fetches(asker);
        let fresh_for = fetches.fresh_for;
        if let Some((asked, arriving)) = &fetches.in_flight {
            if asked == id {
                let Poll::Ready(data) = collect(arriving) else { return Poll::Pending };
                fetches.in_flight = None;
                match &data {
                    Ok(Fetched::Fresh(_)) => self.unreachable_at = None,
                    Ok(Fetched::SavedCopy { .. }) | Err(SourceError::Unreachable(_)) => {
                        self.unreachable_at = Some(Instant::now());
                    }
                    Err(SourceError::FormatChanged(_)) => {}
                }
                if let Err(e) = &data {
                    eprintln!("map {id}: {e}");
                }
                return Poll::Ready(data);
            }
        }
        // A new map: an earlier fetch still in flight finishes into the cache unheard.
        let path = self.map_file(id);
        match cached(&path, fresh_for) {
            Cached::Fresh(page) => Poll::Ready(meowdb::read_map_page(&page, id).map(Fetched::Fresh)),
            // Offline, the crawl moves on with the saved copy, and tries MeowDB again
            // once OFFLINE_RETRY has passed.
            Cached::Stale(Some(SavedCopy { page, saved_at_unix_ms }))
                if asker == PageAsker::Crawl && self.offline() =>
            {
                Poll::Ready(meowdb::read_map_page(&page, id).map(|data| Fetched::SavedCopy { data, saved_at_unix_ms }))
            }
            Cached::Stale(saved) => {
                let owned = id.to_string();
                let read = move |page: &str| meowdb::read_map_page(page, &owned);
                let arriving = self.fetch(meowdb::map_url(id), path, saved, read);
                self.fetches(asker).in_flight = Some((id.to_string(), arriving));
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

/// The page cached at `path`: fresh if it was saved less than `fresh_for` ago.
fn cached(path: &Path, fresh_for: Duration) -> Cached {
    let Ok(page) = std::fs::read_to_string(path) else { return Cached::Stale(None) };
    let saved_at = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let age = saved_at.and_then(|t| SystemTime::now().duration_since(t).ok());
    match (age, saved_at) {
        // Even when it can't be read: refetching won't fix a format change until
        // MeowDB changes again, so it waits out its freshness like any page.
        (Some(age), _) if age < fresh_for => Cached::Fresh(page),
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
