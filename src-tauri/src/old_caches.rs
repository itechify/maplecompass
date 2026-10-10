//! Caches earlier versions kept that nothing reads any more, deleted at launch: MeowDB's
//! map index and map pages (`map-index.html`, `map-<id>.html`) and the world crawl's saved
//! world (`world.json`), which search knew the world from, and MeowDB's mob pages
//! (`mob-<id>.html`) and maplestory.io's mob images (`mob-<id>.gif`), which Mob Cards came
//! from, before OSMS's set (ADR 0008). Nothing writes them now, so only the first launch
//! of this version finds any to delete.

use std::path::Path;

/// Delete the old caches under `cache_dir`. Settings, Personal Bests and Item Lookup's
/// saved searches aren't among them.
pub fn delete(cache_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(cache_dir) else { return };
    for entry in entries.flatten() {
        let old = entry.file_name().to_str().is_some_and(is_old_cache);
        if old && let Err(e) = std::fs::remove_file(entry.path()) {
            eprintln!("old cache: couldn't delete {}: {e}", entry.path().display());
        }
    }
}

/// Whether a file in the cache is one of an earlier version's, or one left half written.
fn is_old_cache(name: &str) -> bool {
    let name = name.strip_suffix(".tmp").unwrap_or(name);
    name == "world.json" || is_old_map_cache(name) || is_old_mob_cache(name)
}

/// A MeowDB map page, or its map index.
fn is_old_map_cache(name: &str) -> bool {
    let page = name.strip_prefix("map-").and_then(|rest| rest.strip_suffix(".html"));
    page.is_some_and(|id| id == "index" || is_id(id))
}

/// A MeowDB mob page, or a maplestory.io mob image.
fn is_old_mob_cache(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("mob-") else { return false };
    let id = rest.strip_suffix(".html").or_else(|| rest.strip_suffix(".gif"));
    id.is_some_and(is_id)
}

/// Whether `id` is a map or mob ID as the old caches were named: digits only.
fn is_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_and_mob_caches_and_the_saved_world_are_deleted_and_everything_else_stays() {
        let cache = std::env::temp_dir().join(format!("maplecompass-old-caches-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&cache);
        std::fs::create_dir_all(cache.join("osms-images")).unwrap();
        let old = [
            "mob-3.html",
            "mob-700000.gif",
            "mob-13.gif.tmp",
            "map-index.html",
            "map-010003070.html",
            "map-100000000.html.tmp",
            "world.json",
            "world.json.tmp",
        ];
        let kept = ["osms-set.json", "item-search-7265.json", "mob-index.html", "mob-3.png", "map-notes.html"];
        for name in old.iter().chain(&kept) {
            std::fs::write(cache.join(name), b"x").unwrap();
        }

        delete(&cache);

        for name in old {
            assert!(!cache.join(name).exists(), "{name}");
        }
        for name in kept {
            assert!(cache.join(name).exists(), "{name}");
        }
        assert!(cache.join("osms-images").is_dir());
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn a_cache_folder_that_isnt_there_yet_is_left_alone() {
        let cache = std::env::temp_dir().join(format!("maplecompass-old-caches-none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&cache);

        delete(&cache);

        assert!(!cache.exists());
    }
}
