//! Caches earlier versions kept that nothing reads any more, deleted at launch: MeowDB's
//! mob pages (`mob-<id>.html`) and maplestory.io's mob images (`mob-<id>.gif`), which Mob
//! Cards came from before OSMS's set (ADR 0008). Nothing writes them now, so only the
//! first launch of this version finds any to delete.

use std::path::Path;

/// Delete the old caches under `cache_dir`. Settings, Personal Bests and Item Lookup's
/// saved searches aren't among them.
pub fn delete(cache_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(cache_dir) else { return };
    for entry in entries.flatten() {
        let old = entry.file_name().to_str().is_some_and(is_old_mob_cache);
        if old && let Err(e) = std::fs::remove_file(entry.path()) {
            eprintln!("old cache: couldn't delete {}: {e}", entry.path().display());
        }
    }
}

/// Whether a file in the cache is a mob page or mob image of an earlier version's, or one
/// left half written.
fn is_old_mob_cache(name: &str) -> bool {
    let name = name.strip_suffix(".tmp").unwrap_or(name);
    let Some(rest) = name.strip_prefix("mob-") else { return false };
    let id = rest.strip_suffix(".html").or_else(|| rest.strip_suffix(".gif"));
    id.is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mob_pages_and_mob_images_are_deleted_and_everything_else_stays() {
        let cache = std::env::temp_dir().join(format!("maplecompass-old-caches-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&cache);
        std::fs::create_dir_all(cache.join("osms-images")).unwrap();
        let old = ["mob-3.html", "mob-700000.gif", "mob-13.gif.tmp"];
        let kept = ["osms-set.json", "item-search-7265.json", "map-010003070.html", "mob-index.html", "mob-3.png"];
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
}
