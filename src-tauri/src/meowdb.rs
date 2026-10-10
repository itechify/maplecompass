//! The shell's data source: OSMS Data Explorer's file set (ADR 0008, in [`crate::osms`]),
//! and MeowDB's pages and search (ADR 0003), fetched over HTTPS with an honest user agent
//! and kept in a disk cache. Reading them is left to the core.
//!
//! The core's world crawl (ADR 0004, until #57) keeps what it learns itself (saved
//! between launches by `world.rs`) and decides when a map's MeowDB page is due, so its
//! requests always go to MeowDB, paced to about one every 2 seconds, falling back on the
//! saved copy when MeowDB can't be reached. Every page MeowDB serves is cached, even one
//! the core can't read.
//!
//! Mob Cards (ADR 0006, until #58): the core asks for the shown map's listed mobs' images
//! from maplestory.io, one at a time. A cached image is good for [`IMAGE_FRESH_FOR`] (30
//! days). Images are saved as files and served to the overlay from them
//! ([`serve_mob_image`]): the webview never asks maplestory.io itself.
//!
//! Item Lookup (ADR 0007): the core asks MeowDB's search for an item's name once per
//! lookup the player makes. An answer is cached for [`ITEM_SEARCH_FRESH_FOR`] (a week),
//! so looking the same item up again asks MeowDB nothing.
//!
//! Fetches run on the async runtime and never block the caller: the first call starts
//! one and answers `Pending`, later calls collect it. A map page is read from the cache
//! there too (a page is a few hundred KB to read).

use crate::osms::OsmsSet;
use compass_core::osms::OsmsFiles;
use compass_core::{Fetched, ItemHit, MapData, MapDataSource, SourceError, meowdb};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::task::Poll;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long a cached mob image is used without asking maplestory.io again: sprites
/// practically never change.
const IMAGE_FRESH_FOR: Duration = Duration::from_secs(30 * 24 * 60 * 60);

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
    /// The world crawl's page being fetched in the background, and where its data will
    /// arrive.
    crawl_page_in_flight: Option<(String, Receiver<Answer<MapData>>)>,
    /// The mob image being fetched from maplestory.io, and where its address will arrive.
    mob_image_in_flight: Option<(String, Receiver<Result<String, SourceError>>)>,
    /// The item name being searched for, and where MeowDB's items will arrive.
    item_search_in_flight: Option<(String, Receiver<Answer<Vec<ItemHit>>>)>,
}

impl MeowDbSource {
    /// A source caching under `cache_dir` (created when first written).
    pub fn new(cache_dir: PathBuf) -> reqwest::Result<Self> {
        let client = client()?;
        Ok(Self {
            osms: OsmsSet::new(client.clone(), &cache_dir),
            client,
            cache_dir,
            crawl_page_in_flight: None,
            mob_image_in_flight: None,
            item_search_in_flight: None,
        })
    }

    fn map_file(&self, id: &str) -> PathBuf {
        self.cache_dir.join(format!("map-{id}.html"))
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

    fn crawl_data(&mut self, id: &str) -> Poll<Answer<MapData>> {
        if let Some((asked, arriving)) = &self.crawl_page_in_flight {
            if asked == id {
                let Poll::Ready(data) = collect(arriving) else { return Poll::Pending };
                self.crawl_page_in_flight = None;
                if let Err(e) = &data {
                    eprintln!("map {id}: {e}");
                }
                return Poll::Ready(data);
            }
        }
        // A new map: an earlier fetch still in flight finishes into the cache unheard.
        let owned = id.to_string();
        let read = move |page: &str| meowdb::read_map_page(page, &owned);
        let arriving = self.fetch(meowdb::map_url(id), self.map_file(id), read);
        self.crawl_page_in_flight = Some((id.to_string(), arriving));
        Poll::Pending
    }

    fn mob_image(&mut self, id: &str) -> Poll<Result<String, SourceError>> {
        if let Some((asked, arriving)) = &self.mob_image_in_flight {
            if asked == id {
                let Poll::Ready(image) = collect(arriving) else { return Poll::Pending };
                self.mob_image_in_flight = None;
                if let Err(e) = &image {
                    eprintln!("mob {id}'s image: {e}");
                }
                return Poll::Ready(image);
            }
        }
        if !is_mob_id(id) {
            return Poll::Ready(Err(SourceError::FormatChanged(format!("mob ID {id:?}"))));
        }
        let path = mob_image_file(&self.cache_dir, id);
        if age(&path).is_some_and(|age| age < IMAGE_FRESH_FOR) {
            return Poll::Ready(Ok(mob_image_url(id)));
        }
        let (tx, rx) = mpsc::channel();
        let (client, owned) = (self.client.clone(), id.to_string());
        tauri::async_runtime::spawn(async move {
            let _ = tx.send(fetch_mob_image(&client, &owned, &path).await);
        });
        self.mob_image_in_flight = Some((id.to_string(), rx));
        Poll::Pending
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

/// The URL scheme the overlay loads mob images from: our saved copies of maplestory.io's.
pub const MOB_IMAGE_SCHEME: &str = "mobimage";

/// Where the overlay loads mob `id`'s image from; [`serve_mob_image`] answers it.
fn mob_image_url(id: &str) -> String {
    if cfg!(windows) {
        format!("http://{MOB_IMAGE_SCHEME}.localhost/{id}")
    } else {
        format!("{MOB_IMAGE_SCHEME}://localhost/{id}")
    }
}

/// maplestory.io's still image of mob `id` standing, from MapleStory Classic World's CBT2
/// data, whose mob IDs are MeowDB's.
fn maplestory_io_url(id: &str) -> String {
    format!("https://maplestory.io/api/MCW/CBT2/mob/{id}/render/stand")
}

/// Whether `id` looks like a mob ID: digits only, so it is safe in a file name.
fn is_mob_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}

fn mob_image_file(cache_dir: &Path, id: &str) -> PathBuf {
    cache_dir.join(format!("mob-{id}.gif"))
}

/// How long ago the file at `path` was saved; `None` if there is none.
fn age(path: &Path) -> Option<Duration> {
    let saved_at = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    Some(SystemTime::now().duration_since(saved_at).unwrap_or_default())
}

/// Fetch mob `id`'s image from maplestory.io and save it at `path`. If maplestory.io
/// can't be reached, an older saved image still does: sprites practically never change.
async fn fetch_mob_image(client: &reqwest::Client, id: &str, path: &Path) -> Result<String, SourceError> {
    let response = match client.get(maplestory_io_url(id)).send().await {
        Ok(response) => response,
        Err(e) => return saved_mob_image(id, path, e.to_string()),
    };
    let status = response.status();
    let is_image = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|t| t.to_str().ok())
        .is_some_and(|t| t.starts_with("image/"));
    // maplestory.io answers 404 for a mob it doesn't have.
    if status.is_client_error() || (status.is_success() && !is_image) {
        return Err(SourceError::FormatChanged(format!("maplestory.io has no image ({status})")));
    }
    let bytes = match response.error_for_status() {
        Ok(response) => response.bytes().await,
        Err(e) => Err(e),
    };
    match bytes {
        Ok(bytes) => match crate::files::write_atomically(path, &bytes) {
            Ok(()) => Ok(mob_image_url(id)),
            Err(e) => Err(SourceError::Unreachable(format!("couldn't save {}: {e}", path.display()))),
        },
        Err(e) => saved_mob_image(id, path, e.to_string()),
    }
}

/// maplestory.io couldn't be reached: the image saved earlier, if there is one.
fn saved_mob_image(id: &str, path: &Path, why: String) -> Result<String, SourceError> {
    eprintln!("maplestory.io: mob {id}: {why}");
    if path.exists() { Ok(mob_image_url(id)) } else { Err(SourceError::Unreachable(why)) }
}

/// Answer the overlay's request for a mob image ([`mob_image_url`]) from the images
/// saved under `cache_dir`.
pub fn serve_mob_image(cache_dir: &Path, request: &tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode, header};
    let id = request.uri().path().trim_start_matches('/');
    let image = if is_mob_id(id) { std::fs::read(mob_image_file(cache_dir, id)).ok() } else { None };
    let response = match image {
        Some(bytes) => Response::builder().header(header::CONTENT_TYPE, "image/gif").body(bytes),
        None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
    };
    response.unwrap_or_default()
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
