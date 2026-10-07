//! Shared test inputs: minimap samples turned into frames, and fake map data sources
//! serving MeowDB's saved map index and map pages (personal-use test data, ADR 0003).
#![allow(dead_code)]

use compass_core::{
    Compass, Fetched, Frame, MapData, MapDataSource, MapEntry, PanelState, Rect, ScreenRect, Size,
    SourceError, TitleRegion, default_title_region, meowdb,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;

pub fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/minimap")
}

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn load(name: &str) -> image::RgbaImage {
    image::open(samples_dir().join(format!("{name}.png")))
        .unwrap_or_else(|e| panic!("sample {name}: {e}"))
        .to_rgba8()
}

pub fn frame(img: &image::RgbaImage, at_ms: u64) -> Frame {
    Frame { width: img.width(), height: img.height(), rgba: img.as_raw().clone(), at_ms }
}

/// A loading screen: the Title Region is plain black.
pub fn black_frame(at_ms: u64) -> Frame {
    frame(&image::RgbaImage::from_pixel(560, 96, image::Rgba([0, 0, 0, 255])), at_ms)
}

/// `--window` samples are whole game-window screenshots including the Windows title
/// bar (25–28 px, hand-cropped). The client area starts below it.
const TITLE_BAR_PX: u32 = 27;

/// The game's client area in a window screenshot: everything below the title bar.
pub fn client_of(window: &image::RgbaImage) -> image::RgbaImage {
    image::imageops::crop_imm(window, 0, TITLE_BAR_PX, window.width(), window.height() - TITLE_BAR_PX)
        .to_image()
}

/// Capture `region` (physical screen px) from a game client area shown on screen at
/// `game`, the way the shell captures the panel state's Title Region.
pub fn capture(client: &image::RgbaImage, game: ScreenRect, region: ScreenRect, at_ms: u64) -> Frame {
    let (x, y) = ((region.x - game.x) as u32, (region.y - game.y) as u32);
    let crop = image::imageops::crop_imm(client, x, y, region.width, region.height).to_image();
    assert_eq!(crop.dimensions(), (region.width, region.height), "region inside the client area");
    frame(&crop, at_ms)
}

/// Crop the default Title Region out of a window screenshot, the way the shell crops
/// it out of the game's client area.
pub fn default_region_frame(window: &image::RgbaImage, at_ms: u64) -> Frame {
    let client = client_of(window);
    let r = default_title_region(client.width(), client.height());
    let crop = image::imageops::crop_imm(&client, r.x, r.y, r.width, r.height).to_image();
    frame(&crop, at_ms)
}

/// The frame the shell would capture for a sample: `--window` samples go through the
/// default Title Region; other samples (a minimap crop, a live Title Region capture)
/// are fed whole.
pub fn sample_frame(name: &str, at_ms: u64) -> Frame {
    let img = load(name);
    if name.ends_with("--window") { default_region_frame(&img, at_ms) } else { frame(&img, at_ms) }
}

/// Two confident reads of a sample: the panel switches to its map.
pub fn arrive(compass: &mut Compass, sample: &str, at_ms: u64) -> PanelState {
    compass.on_frame(sample_frame(sample, at_ms));
    compass.on_frame(sample_frame(sample, at_ms + 333)).clone()
}

/// A Title Region marked around the minimap title of the Kerning City window sample,
/// whose client area is 1916x1197.
pub fn kerning_title_region() -> TitleRegion {
    TitleRegion {
        marked: Rect { x: 70, y: 40, width: 130, height: 48 },
        client: Size { width: 1916, height: 1197 },
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedEntry {
    id: String,
    street_name: String,
    map_name: String,
    region: String,
}

/// MeowDB's map index as saved on 2026-10-07 (422 maps).
pub fn saved_index() -> Vec<MapEntry> {
    let json = std::fs::read_to_string(fixtures_dir().join("meowdb-map-index.json")).expect("fixture");
    let saved: Vec<SavedEntry> = serde_json::from_str(&json).expect("map index json");
    saved
        .into_iter()
        .map(|e| MapEntry { id: e.id, street_name: e.street_name, map_name: e.map_name, region: e.region })
        .collect()
}

/// The saved index, except that the title `street` / `map` belongs to the maps `ids`
/// and to no other. A sample showing that title then stands in for those maps'
/// minimap: there are no samples of titles MeowDB really shares.
pub fn index_with_title(street: &str, map: &str, ids: &[&str]) -> Vec<MapEntry> {
    saved_index()
        .into_iter()
        .filter(|e| !(e.street_name == street && e.map_name == map) || ids.contains(&e.id.as_str()))
        .map(|e| {
            if ids.contains(&e.id.as_str()) {
                MapEntry { street_name: street.into(), map_name: map.into(), ..e }
            } else {
                e
            }
        })
        .collect()
}

/// A MeowDB map page saved on 2026-10-07: `tests/fixtures/meowdb-maps/<id>__<slug>.html`.
pub fn saved_page(id: &str) -> Option<String> {
    let dir = fixtures_dir().join("meowdb-maps");
    let file = std::fs::read_dir(dir)
        .expect("fixture dir")
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(&format!("{id}__")))?;
    Some(std::fs::read_to_string(file.path()).expect("fixture"))
}

type PageAnswer = Poll<Result<MapData, SourceError>>;
type IndexAnswer = Poll<Result<Vec<MapEntry>, SourceError>>;

/// A map data source that answers from memory and counts index and map data requests.
pub struct FakeSource {
    /// Answers for successive index requests; the last one repeats.
    answers: Vec<IndexAnswer>,
    pub index_requests: Arc<AtomicU32>,
    /// Answers a map data request for a map ID. Defaults to the saved pages, read with
    /// the core's MeowDB reader; a map without a saved page is unreachable.
    pages: Box<dyn FnMut(&str) -> PageAnswer + Send>,
    /// The map ID of every map data request, in order.
    pub map_requests: Arc<Mutex<Vec<String>>>,
    /// The map ID of every crawl fetch started, in order: a crawl request for a map
    /// whose fetch is still in flight checks on it and isn't logged again.
    pub crawl_requests: Arc<Mutex<Vec<String>>>,
    /// Crawl answers aren't in the cache: each arrives only on the poll after its fetch
    /// started, as from the network.
    crawl_uncached: bool,
    /// The crawl fetch in flight, and its answer once it has arrived.
    crawl_in_flight: Option<(String, Option<Result<MapData, SourceError>>)>,
    /// MeowDB is unreachable and the answers are copies saved at this time (ms since the
    /// Unix epoch); `None`: they are fresh.
    saved_at: Option<u64>,
}

impl FakeSource {
    fn answering(answers: Vec<IndexAnswer>) -> Self {
        Self {
            answers,
            index_requests: Arc::default(),
            pages: Box::new(|id| Poll::Ready(read_saved_page(id))),
            map_requests: Arc::default(),
            crawl_requests: Arc::default(),
            crawl_uncached: false,
            crawl_in_flight: None,
            saved_at: None,
        }
    }

    /// MeowDB can't be reached, and every answer (index and pages) is a copy saved at
    /// `saved_at_unix_ms`.
    pub fn serving_saved_copies(self, saved_at_unix_ms: u64) -> Self {
        Self { saved_at: Some(saved_at_unix_ms), ..self }
    }

    /// The map index arrives only on the `polls`-th request for it; until then the
    /// fetch is still in flight.
    pub fn with_slow_index(index: Vec<MapEntry>, polls: usize) -> Self {
        let mut answers: Vec<IndexAnswer> = (1..polls).map(|_| Poll::Pending).collect();
        answers.push(Poll::Ready(Ok(index)));
        Self::answering(answers)
    }

    fn fetched<T>(&self, answer: Poll<Result<T, SourceError>>) -> Poll<Result<Fetched<T>, SourceError>> {
        answer.map(|result| {
            result.map(|data| match self.saved_at {
                Some(saved_at_unix_ms) => Fetched::SavedCopy { data, saved_at_unix_ms },
                None => Fetched::Fresh(data),
            })
        })
    }

    /// Answer map data requests with `pages` instead of the saved pages.
    pub fn with_pages(mut self, pages: impl FnMut(&str) -> PageAnswer + Send + 'static) -> Self {
        self.pages = Box::new(pages);
        self
    }

    /// Serve `html` as the MeowDB page of map `id`, and the saved pages for other maps.
    pub fn with_page(self, id: &str, html: String) -> Self {
        let id = id.to_string();
        self.with_pages(move |asked| {
            Poll::Ready(if asked == id { meowdb::read_map_page(&html, asked) } else { read_saved_page(asked) })
        })
    }

    /// Map pages can't be had: MeowDB is unreachable (the map index still is).
    pub fn with_pages_offline(self) -> Self {
        self.with_pages(|_| Poll::Ready(Err(unreachable())))
    }

    /// Each map's saved page arrives only on the `polls`-th request for it; until then
    /// the fetch is still in flight.
    pub fn with_slow_pages(self, polls: usize) -> Self {
        let mut asked = std::collections::HashMap::<String, usize>::new();
        self.with_pages(move |id| {
            let n = asked.entry(id.to_string()).or_default();
            *n += 1;
            if *n < polls { Poll::Pending } else { Poll::Ready(read_saved_page(id)) }
        })
    }

    /// Crawl answers come over the network, not from the cache: each arrives on the
    /// poll after its fetch started. (By default the crawl is answered at once, as from
    /// the cache.)
    pub fn crawling_uncached(self) -> Self {
        Self { crawl_uncached: true, ..self }
    }

    pub fn with_index(index: Vec<MapEntry>) -> Self {
        Self::answering(vec![Poll::Ready(Ok(index))])
    }

    /// The saved MeowDB index.
    pub fn meowdb() -> Self {
        Self::with_index(saved_index())
    }

    /// MeowDB can't be reached.
    pub fn offline() -> Self {
        Self::failing(unreachable())
    }

    /// Every index request fails this way.
    pub fn failing(error: SourceError) -> Self {
        Self::answering(vec![Poll::Ready(Err(error))])
    }

    /// MeowDB can't be reached at first, then serves this index.
    pub fn offline_then(index: Vec<MapEntry>) -> Self {
        Self::answering(vec![Poll::Ready(Err(unreachable())), Poll::Ready(Ok(index))])
    }
}

fn unreachable() -> SourceError {
    SourceError::Unreachable("offline".into())
}

/// A small index of `n` made-up maps ("100000000", "100000001", …) on one street.
pub fn made_up_index(n: usize) -> Vec<MapEntry> {
    (0..n)
        .map(|i| MapEntry {
            id: format!("{:09}", 100_000_000 + i),
            street_name: "Made-up Street".into(),
            map_name: format!("Made-up Map {i}"),
            region: "Victoria Island".into(),
        })
        .collect()
}

/// How many map data requests a [`FakeSource`] had for map `id`.
pub fn requests_for(requests: &Mutex<Vec<String>>, id: &str) -> usize {
    requests.lock().unwrap().iter().filter(|r| *r == id).count()
}

fn read_saved_page(id: &str) -> Result<MapData, SourceError> {
    match saved_page(id) {
        Some(html) => meowdb::read_map_page(&html, id),
        None => Err(SourceError::Unreachable(format!("no saved page for {id}"))),
    }
}

impl MapDataSource for FakeSource {
    fn map_index(&mut self) -> Poll<Result<Fetched<Vec<MapEntry>>, SourceError>> {
        let n = self.index_requests.fetch_add(1, Ordering::SeqCst) as usize;
        let answer = self.answers[n.min(self.answers.len() - 1)].clone();
        self.fetched(answer)
    }

    fn map_data(&mut self, id: &str) -> Poll<Result<Fetched<MapData>, SourceError>> {
        self.map_requests.lock().unwrap().push(id.to_string());
        let answer = (self.pages)(id);
        self.fetched(answer)
    }

    /// Answered from the same pages as [`FakeSource::map_data`], but logged in
    /// `crawl_requests`.
    fn crawl_data(&mut self, id: &str) -> Poll<Result<Fetched<MapData>, SourceError>> {
        let answer = match self.crawl_in_flight.take() {
            Some((asked, Some(arrived))) if asked == id => Poll::Ready(arrived),
            Some((asked, None)) if asked == id => (self.pages)(id),
            _ => {
                self.crawl_requests.lock().unwrap().push(id.to_string());
                match (self.pages)(id) {
                    Poll::Ready(arrived) if self.crawl_uncached => {
                        self.crawl_in_flight = Some((id.to_string(), Some(arrived)));
                        return Poll::Pending;
                    }
                    answer => answer,
                }
            }
        };
        if answer.is_pending() {
            self.crawl_in_flight = Some((id.to_string(), None));
        }
        self.fetched(answer)
    }
}
