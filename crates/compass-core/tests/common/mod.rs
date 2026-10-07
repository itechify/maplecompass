//! Shared test inputs: minimap samples turned into frames, and fake map data sources
//! serving MeowDB's saved map index and map pages (personal-use test data, ADR 0003).
#![allow(dead_code)]

use compass_core::{
    Frame, MapData, MapDataSource, MapEntry, ScreenRect, SourceError, default_title_region, meowdb,
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

/// A map data source that answers from memory and counts index and map data requests.
pub struct FakeSource {
    /// Answers for successive index requests; the last one repeats.
    answers: Vec<Result<Vec<MapEntry>, SourceError>>,
    pub index_requests: Arc<AtomicU32>,
    /// Answers a map data request for a map ID. Defaults to the saved pages, read with
    /// the core's MeowDB reader; a map without a saved page is unreachable.
    pages: Box<dyn FnMut(&str) -> PageAnswer + Send>,
    /// The map ID of every map data request, in order.
    pub map_requests: Arc<Mutex<Vec<String>>>,
}

impl FakeSource {
    fn answering(answers: Vec<Result<Vec<MapEntry>, SourceError>>) -> Self {
        Self {
            answers,
            index_requests: Arc::default(),
            pages: Box::new(|id| Poll::Ready(read_saved_page(id))),
            map_requests: Arc::default(),
        }
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

    pub fn with_index(index: Vec<MapEntry>) -> Self {
        Self::answering(vec![Ok(index)])
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
        Self::answering(vec![Err(error)])
    }

    /// MeowDB can't be reached at first, then serves this index.
    pub fn offline_then(index: Vec<MapEntry>) -> Self {
        Self::answering(vec![Err(unreachable()), Ok(index)])
    }
}

fn unreachable() -> SourceError {
    SourceError::Unreachable("offline".into())
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
    fn map_index(&mut self) -> Result<Vec<MapEntry>, SourceError> {
        let n = self.index_requests.fetch_add(1, Ordering::SeqCst) as usize;
        self.answers[n.min(self.answers.len() - 1)].clone()
    }

    fn map_data(&mut self, id: &str) -> PageAnswer {
        self.map_requests.lock().unwrap().push(id.to_string());
        (self.pages)(id)
    }
}
