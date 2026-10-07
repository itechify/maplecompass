//! Shared test inputs: minimap samples turned into frames, and fake map data sources
//! serving MeowDB's saved map index (personal-use test data, ADR 0003).
#![allow(dead_code)]

use compass_core::{Frame, MapDataSource, MapEntry, SourceError, default_title_region};
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::path::{Path, PathBuf};

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

/// Crop the default Title Region out of a window screenshot, the way the shell crops
/// it out of the game's client area.
pub fn default_region_frame(window: &image::RgbaImage, at_ms: u64) -> Frame {
    let client = image::imageops::crop_imm(
        window,
        0,
        TITLE_BAR_PX,
        window.width(),
        window.height() - TITLE_BAR_PX,
    )
    .to_image();
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

/// A map data source that answers from memory and counts index requests.
pub struct FakeSource {
    index: Result<Vec<MapEntry>, SourceError>,
    pub index_requests: Arc<AtomicU32>,
}

impl FakeSource {
    pub fn with_index(index: Vec<MapEntry>) -> Self {
        Self { index: Ok(index), index_requests: Arc::default() }
    }

    /// The saved MeowDB index.
    pub fn meowdb() -> Self {
        Self::with_index(saved_index())
    }

    /// MeowDB can't be reached.
    pub fn offline() -> Self {
        Self { index: Err(SourceError::Unreachable("offline".into())), index_requests: Arc::default() }
    }
}

impl MapDataSource for FakeSource {
    fn map_index(&mut self) -> Result<Vec<MapEntry>, SourceError> {
        self.index_requests.fetch_add(1, Ordering::SeqCst);
        self.index.clone()
    }
}
