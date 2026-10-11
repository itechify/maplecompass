//! OSMS Data Explorer's file set (ADR 0008): the five files under
//! [`compass_core::osms::DATA_URL`], fetched with MapleCompass's own user agent, one
//! request at a time and with no other pace. Nothing from OSMS is bundled.
//!
//! The set is saved on disk as one file, so it is swapped whole, and the file's
//! modification time says when it was saved. The core decides when to fetch it afresh: it
//! looks at the saved set at startup, and asks for a fresh one when that is too old, and
//! every day after. A fetched set is saved only once the core's reader reads it. If any
//! file fails, or the set doesn't read, nothing is saved, and the core keeps the set it
//! has.

use crate::errors::error;
use crate::http;
use compass_core::SourceError;
use compass_core::osms::{self, OsmsFiles, SavedFiles};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::task::Poll;
use std::time::UNIX_EPOCH;

/// The saved set's file in the cache.
const SET_FILE: &str = "osms-set.json";

type Answer = Result<OsmsFiles, SourceError>;

/// OSMS's file set: saved, and fetched when the core asks.
pub struct OsmsSet {
    client: reqwest::Client,
    file: PathBuf,
    /// The set being fetched in the background.
    in_flight: Option<Receiver<Answer>>,
}

impl OsmsSet {
    /// A set saved under `cache_dir`, fetched with `client`.
    pub fn new(client: reqwest::Client, cache_dir: &Path) -> Self {
        Self { client, file: cache_dir.join(SET_FILE), in_flight: None }
    }

    /// The set saved, and when; `None` if none is, or it can't be read back.
    pub fn saved(&self) -> Option<SavedFiles> {
        let json = std::fs::read(&self.file).ok()?;
        let files = serde_json::from_slice(&json).ok()?;
        let saved_at = std::fs::metadata(&self.file).and_then(|m| m.modified()).ok()?;
        let since_epoch = saved_at.duration_since(UNIX_EPOCH).unwrap_or_default();
        Some(SavedFiles { files, saved_at_unix_ms: since_epoch.as_millis() as u64 })
    }

    /// A set fetched afresh, in the background: the first call starts fetching it, later
    /// ones collect it.
    pub fn fetch(&mut self) -> Poll<Answer> {
        if let Some(arriving) = &self.in_flight {
            let Poll::Ready(answer) = http::collect(arriving) else { return Poll::Pending };
            self.in_flight = None;
            if let Err(e) = &answer {
                error!("OSMS's file set: {e}");
            }
            return Poll::Ready(answer);
        }
        let (client, file) = (self.client.clone(), self.file.clone());
        self.in_flight = Some(http::in_background(async move { fetch_and_save(&client, &file).await }));
        Poll::Pending
    }
}

/// Fetch OSMS's five files, and save them at `file` once they read.
async fn fetch_and_save(client: &reqwest::Client, file: &Path) -> Answer {
    let files = fetch(client).await.map_err(SourceError::Unreachable)?;
    osms::read(&files)?;
    save(file, &files);
    Ok(files)
}

/// Fetch OSMS's five files, one after another: the first that fails fails the set.
async fn fetch(client: &reqwest::Client) -> Result<OsmsFiles, String> {
    let mut texts = Vec::new();
    for name in OsmsFiles::NAMES {
        texts.push(http::text(client, &format!("{}{name}", osms::DATA_URL)).await?);
    }
    let [maps, portals, monsters, map_manifest, lookups]: [String; 5] =
        texts.try_into().map_err(|_| "not five files".to_string())?;
    Ok(OsmsFiles { maps, portals, monsters, map_manifest, lookups })
}

/// Fetch OSMS's live set now, whatever is saved, for the probe.
pub fn fetch_live() -> Result<OsmsFiles, String> {
    let client = http::client().map_err(|e| e.to_string())?;
    tauri::async_runtime::block_on(fetch(&client))
}

/// Save `files` at `file`, in place of the set saved there, whole.
fn save(file: &Path, files: &OsmsFiles) {
    let written = serde_json::to_vec(files)
        .map_err(std::io::Error::other)
        .and_then(|json| crate::files::write_atomically(file, &json));
    if let Err(e) = written {
        error!("OSMS: couldn't save {}: {e}", file.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn the_saved_set_is_read_back_with_when_it_was_saved() {
        let dir = std::env::temp_dir().join(format!("maplecompass-osms-set-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let set = OsmsSet::new(reqwest::Client::new(), &dir);
        assert_eq!(set.saved(), None);

        let files = OsmsFiles { maps: "{}".into(), ..OsmsFiles::default() };
        save(&dir.join(SET_FILE), &files);
        let saved_at = UNIX_EPOCH + Duration::from_millis(1_790_812_800_000);
        let file = std::fs::File::options().write(true).open(dir.join(SET_FILE)).unwrap();
        file.set_modified(saved_at).unwrap();
        drop(file);

        assert_eq!(set.saved(), Some(SavedFiles { files, saved_at_unix_ms: 1_790_812_800_000 }));
        std::fs::write(dir.join(SET_FILE), b"not json").unwrap();
        assert_eq!(set.saved(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
