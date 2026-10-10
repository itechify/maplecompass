//! OSMS Data Explorer's pictures (ADR 0008): map pictures and mob animations, each by its
//! path under OSMS's data (`maps/<hash>.webp`). They are named by content hash, so a copy
//! on disk never goes stale: one is fetched the first time it is asked for, one at a time
//! and with MapleCompass's own user agent, and saved under `osms-images/<path>` in the
//! cache. The overlay loads it from there through the [`SCHEME`] URL scheme
//! ([`serve`]): the webview never asks OSMS itself.
//!
//! Every ask for a saved copy marks it used (its modified time), and copies unused for
//! [`UNUSED_FOR`] (30 days) are deleted when the source starts.

use compass_core::SourceError;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::task::Poll;
use std::time::{Duration, SystemTime};

/// The URL scheme the overlay loads OSMS's pictures from: our saved copies of them.
pub const SCHEME: &str = "osmsimage";

/// Where OSMS Data Explorer publishes its data: a picture's path is under it.
const DATA_URL: &str = "https://osmsdataexplorer.com/data/";

/// The pictures' folder in the cache.
const DIR: &str = "osms-images";

/// How long a saved picture is kept without being asked for.
const UNUSED_FOR: Duration = Duration::from_secs(30 * 24 * 60 * 60);

type Answer = Result<String, SourceError>;

/// OSMS's pictures: saved, and fetched when they aren't.
pub struct Pictures {
    client: reqwest::Client,
    dir: PathBuf,
    /// The picture being fetched, by path, and where its address will arrive.
    in_flight: Option<(String, Receiver<Answer>)>,
}

impl Pictures {
    /// Pictures saved under `cache_dir`, fetched with `client`. Those unused for
    /// [`UNUSED_FOR`] are deleted, in the background.
    pub fn new(client: reqwest::Client, cache_dir: &Path) -> Self {
        let dir = cache_dir.join(DIR);
        let unused = dir.clone();
        std::thread::spawn(move || delete_unused(&unused, SystemTime::now()));
        Self { client, dir, in_flight: None }
    }

    /// Where the overlay loads the picture at `path` from: at once when it is saved;
    /// otherwise fetched in the background, the first call starting the fetch and later
    /// ones collecting it. One fetch at a time: while another picture is on its way, the
    /// ask waits for it.
    pub fn poll(&mut self, path: &str) -> Poll<Answer> {
        if !is_picture_path(path) {
            return Poll::Ready(Err(SourceError::FormatChanged(format!("picture path {path:?}"))));
        }
        if let Some((asked, arriving)) = &self.in_flight {
            let finished = match arriving.try_recv() {
                Err(TryRecvError::Empty) => None,
                Ok(answer) => Some(answer),
                Err(TryRecvError::Disconnected) => Some(Err(SourceError::Unreachable("fetch abandoned".into()))),
            };
            match finished {
                Some(answer) if asked == path => {
                    self.in_flight = None;
                    if let Err(e) = &answer {
                        eprintln!("OSMS picture {path}: {e}");
                    }
                    return Poll::Ready(answer);
                }
                // Another picture's fetch finished, unheard: it is saved all the same.
                Some(_) => self.in_flight = None,
                None if asked == path => return Poll::Pending,
                None => {}
            }
        }
        let file = self.dir.join(path);
        if file.is_file() {
            mark_used(&file);
            return Poll::Ready(Ok(url(path)));
        }
        if self.in_flight.is_some() {
            return Poll::Pending;
        }
        let (tx, rx) = mpsc::channel();
        let (client, owned) = (self.client.clone(), path.to_string());
        tauri::async_runtime::spawn(async move {
            let _ = tx.send(fetch(&client, &owned, &file).await);
        });
        self.in_flight = Some((path.to_string(), rx));
        Poll::Pending
    }
}

/// Where the overlay loads the picture at `path` from; [`serve`] answers it.
fn url(path: &str) -> String {
    if cfg!(windows) { format!("http://{SCHEME}.localhost/{path}") } else { format!("{SCHEME}://localhost/{path}") }
}

/// Whether `path` looks like a picture's path under OSMS's data: folders and a file name
/// of letters, digits, `-`, `_` and `.` (none starting with a dot), ending in a picture's
/// extension. Nothing else is fetched, saved or served.
fn is_picture_path(path: &str) -> bool {
    let safe = |part: &str| {
        !part.is_empty()
            && !part.starts_with('.')
            && part.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    let picture = [".webp", ".png", ".gif"].iter().any(|ext| path.ends_with(ext));
    path.len() <= 200 && path.contains('/') && path.split('/').all(safe) && picture
}

/// Mark the saved picture at `file` used now, so it isn't deleted as unused.
fn mark_used(file: &Path) {
    let marked = std::fs::File::options().write(true).open(file).and_then(|f| f.set_modified(SystemTime::now()));
    if let Err(e) = marked {
        eprintln!("couldn't mark {} used: {e}", file.display());
    }
}

/// Fetch the picture at `path` from OSMS and save it at `file`.
async fn fetch(client: &reqwest::Client, path: &str, file: &Path) -> Answer {
    let response = client.get(format!("{DATA_URL}{path}")).send().await.map_err(|e| SourceError::Unreachable(e.to_string()))?;
    let status = response.status();
    let is_image = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|t| t.to_str().ok())
        .is_some_and(|t| t.starts_with("image/"));
    if status.is_client_error() || (status.is_success() && !is_image) {
        return Err(SourceError::FormatChanged(format!("OSMS has no picture {path} ({status})")));
    }
    let bytes = response.error_for_status().map_err(|e| SourceError::Unreachable(e.to_string()))?;
    let bytes = bytes.bytes().await.map_err(|e| SourceError::Unreachable(e.to_string()))?;
    crate::files::write_atomically(file, &bytes)
        .map_err(|e| SourceError::Unreachable(format!("couldn't save {}: {e}", file.display())))?;
    Ok(url(path))
}

/// Delete the pictures under `dir` last used longer than [`UNUSED_FOR`] before `now`.
fn delete_unused(dir: &Path, now: SystemTime) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            delete_unused(&path, now);
        } else if meta.modified().is_ok_and(|used| now.duration_since(used).is_ok_and(|age| age > UNUSED_FOR)) {
            if let Err(e) = std::fs::remove_file(&path) {
                eprintln!("couldn't delete unused {}: {e}", path.display());
            }
        }
    }
}

/// Answer the overlay's request for a picture ([`url`]) from the pictures saved under
/// `cache_dir`.
pub fn serve(cache_dir: &Path, request: &tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode, header};
    let path = request.uri().path().trim_start_matches('/');
    let picture = if is_picture_path(path) { std::fs::read(cache_dir.join(DIR).join(path)).ok() } else { None };
    let kind = match path.rsplit('.').next() {
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        _ => "image/webp",
    };
    let response = match picture {
        Some(bytes) => Response::builder().header(header::CONTENT_TYPE, kind).body(bytes),
        None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
    };
    response.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_paths_of_pictures_under_osmss_data_are_taken() {
        for path in ["maps/75cadba2900f5b76b7582ab69585f026.webp", "images/monsters/0a1b2c.webp", "images/monsters/0a1b2c.png"] {
            assert!(is_picture_path(path), "{path}");
        }
        for path in [
            "",
            "75cadba2900f5b76b7582ab69585f026.webp",
            "maps/../settings.json",
            "maps/../../x.webp",
            "maps/.hidden.webp",
            "maps//x.webp",
            "/maps/x.webp",
            "maps\\x.webp",
            "maps/x.html",
            "maps/x y.webp",
            "C:/x.webp",
        ] {
            assert!(!is_picture_path(path), "{path}");
        }
    }

    #[test]
    fn pictures_unused_for_30_days_are_deleted_and_the_rest_kept() {
        let dir = std::env::temp_dir().join(format!("maplecompass-pictures-test-{}", std::process::id()));
        let (old, recent) = (dir.join("maps/old.webp"), dir.join("images/monsters/recent.webp"));
        for file in [&old, &recent] {
            crate::files::write_atomically(file, b"webp").unwrap();
        }
        let now = SystemTime::now();
        let set_used = |file: &Path, ago_days: u64| {
            let used = now - Duration::from_secs(ago_days * 24 * 60 * 60);
            std::fs::File::options().write(true).open(file).unwrap().set_modified(used).unwrap();
        };
        set_used(&old, 31);
        set_used(&recent, 29);

        delete_unused(&dir, now);

        assert!(!old.exists());
        assert!(recent.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
