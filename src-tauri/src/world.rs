//! Keeping what the world crawl learns between launches (ADR 0004). The core hands over
//! every map it knows as plain data ([`SavedWorld`]); this writes it as one JSON file in
//! the cache folder and reads it back at the next launch, on a thread of its own. When
//! there is no such file yet, or it was saved in another shape, the world is built again
//! from the MeowDB pages cached on disk, each dated by when it was saved.

use compass_core::{SavedMap, SavedWorld, WORLD_VERSION, meowdb};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// The saved world's file, in the cache folder beside the pages.
const WORLD_FILE: &str = "world.json";

/// The world an earlier launch saved, and whether it came from its file (rather than
/// being built from the cached pages, and so still to be saved).
pub struct Loaded {
    pub world: SavedWorld,
    pub from_file: bool,
}

fn file(cache_dir: &Path) -> PathBuf {
    cache_dir.join(WORLD_FILE)
}

/// The saved world under `cache_dir`, or else one built from the cached map pages
/// there. Slow when it has to build: call it off the compass loop.
pub fn load(cache_dir: &Path) -> Loaded {
    let saved = std::fs::read(file(cache_dir)).ok().and_then(|json| serde_json::from_slice::<SavedWorld>(&json).ok());
    match saved {
        Some(world) if world.version == WORLD_VERSION => Loaded { world, from_file: true },
        _ => Loaded { world: from_cached_pages(cache_dir), from_file: false },
    }
}

/// Every map page cached under `cache_dir` that reads, dated by when it was saved.
fn from_cached_pages(cache_dir: &Path) -> SavedWorld {
    let Ok(entries) = std::fs::read_dir(cache_dir) else { return SavedWorld::default() };
    let maps = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let id = name.strip_prefix("map-")?.strip_suffix(".html")?.to_string();
            // Only map pages: the map index is cached as `map-index.html`.
            if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let saved_at = entry.metadata().and_then(|m| m.modified()).ok()?;
            let page = std::fs::read_to_string(entry.path()).ok()?;
            let data = meowdb::read_map_page(&page, &id).ok()?;
            let as_of_unix_ms = saved_at.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
            Some(SavedMap { id, as_of_unix_ms, data: (&data).into() })
        })
        .collect();
    SavedWorld { maps, ..SavedWorld::default() }
}

/// Save `world` under `cache_dir`, replacing what was there.
pub fn save(cache_dir: &Path, world: &SavedWorld) -> std::io::Result<()> {
    let json = serde_json::to_vec(world).map_err(std::io::Error::other)?;
    crate::files::write_atomically(&file(cache_dir), &json)
}
