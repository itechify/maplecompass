//! The files earlier versions kept in the cache folder and this one no longer uses,
//! deleted at startup: MeowDB's map index and map pages (`map-index.html`,
//! `map-<id>.html`) and the world crawl's saved world (`world.json`), with any temporary
//! copy a crash left of them. Nothing writes them any more, so deleting whichever are
//! there deletes them on the first launch of the new version.
//!
//! Everything else stays: OSMS's saved set, Item Lookup's saved searches, the Mob Cards'
//! pages and images, and (in their own folder) the settings and Personal Bests.

use std::path::Path;

/// Delete the old files in `cache_dir`. Slow on the first launch (hundreds of pages):
/// call it off the UI thread.
pub fn clear(cache_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(cache_dir) else { return };
    for entry in entries.filter_map(Result::ok) {
        let old = entry.file_name().to_str().is_some_and(is_old);
        if old && let Err(e) = std::fs::remove_file(entry.path()) {
            eprintln!("couldn't delete the old {}: {e}", entry.path().display());
        }
    }
}

/// Whether the file named `name` is one of the old ones, or a temporary copy of one.
fn is_old(name: &str) -> bool {
    let name = name.strip_suffix(".tmp").unwrap_or(name);
    let map_page = name.strip_prefix("map-").and_then(|rest| rest.strip_suffix(".html"));
    name == "world.json" || map_page.is_some_and(|id| id == "index" || is_map_id(id))
}

/// Whether `id` is a map ID as the map pages were named: digits only.
fn is_map_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// A cache folder of `test`'s own under the temp folder, holding these files.
    fn cache_with(test: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("maplecompass-old-caches-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for file in files {
            fs::write(dir.join(file), "saved").unwrap();
        }
        dir
    }

    /// The files left in `dir`, by name.
    fn left(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> =
            fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        names.sort();
        names
    }

    #[test]
    fn the_map_index_the_map_pages_and_the_saved_world_go_and_everything_else_stays() {
        let dir = cache_with(
            "mixed",
            &[
                "map-index.html",
                "map-010000000.html",
                "map-100000000.html.tmp",
                "world.json",
                "world.json.tmp",
                "osms-set.json",
                "item-search-6f72.json",
                "mob-3.html",
                "mob-3.gif",
                "map-notes.html",
            ],
        );

        clear(&dir);

        assert_eq!(left(&dir), ["item-search-6f72.json", "map-notes.html", "mob-3.gif", "mob-3.html", "osms-set.json"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_cache_folder_that_isnt_there_yet_is_left_alone() {
        let dir = std::env::temp_dir().join(format!("maplecompass-old-caches-none-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);

        clear(&dir);

        assert!(!dir.exists());
    }
}
