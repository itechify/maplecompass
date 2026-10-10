//! OSMS Data Explorer's file set (ADR 0008): the five files under
//! [`compass_core::osms::DATA_URL`], fetched with MapleCompass's own user agent, one
//! request at a time and with no other pace. Nothing from OSMS is bundled.
//!
//! The set is saved on disk as one file, so it is swapped whole. The core asks at startup
//! and every 24 hours after: a saved set younger than [`FRESH_FOR`] is answered as it is,
//! at once; an older one (or none) is fetched afresh. A fetched set is saved only once the
//! core's reader reads it. If any file fails, or the set doesn't read, the saved set is
//! answered instead, as a saved copy with its date, and only with none saved is that an
//! error. The core decides whether the panel says so.

use compass_core::osms::{self, OsmsFiles};
use compass_core::{Fetched, SourceError};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::task::Poll;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long a saved set is used without fetching it again: game data changes only on
/// patch days, and a day is soon enough to catch one. An hour short of the core's 24 hours
/// between asks, so a set saved at one ask is always due at the next, whatever the clocks.
const FRESH_FOR: Duration = Duration::from_secs(23 * 60 * 60);

/// The saved set's file in the cache.
const SET_FILE: &str = "osms-set.json";

type Answer = Result<Fetched<OsmsFiles>, SourceError>;

/// OSMS's file set: saved, and fetched when due.
pub struct OsmsSet {
    client: reqwest::Client,
    file: PathBuf,
    /// The set being fetched (or read) in the background.
    in_flight: Option<Receiver<Answer>>,
}

impl OsmsSet {
    /// A set saved under `cache_dir`, fetched with `client`.
    pub fn new(client: reqwest::Client, cache_dir: &Path) -> Self {
        Self { client, file: cache_dir.join(SET_FILE), in_flight: None }
    }

    /// The set, in the background: the first call starts reading or fetching it, later
    /// ones collect it.
    pub fn poll(&mut self) -> Poll<Answer> {
        if let Some(arriving) = &self.in_flight {
            let answer = match arriving.try_recv() {
                Err(TryRecvError::Empty) => return Poll::Pending,
                Ok(answer) => answer,
                Err(TryRecvError::Disconnected) => Err(SourceError::Unreachable("fetch abandoned".into())),
            };
            self.in_flight = None;
            if let Err(e) = &answer {
                eprintln!("OSMS's file set: {e}");
            }
            return Poll::Ready(answer);
        }
        let (tx, rx) = mpsc::channel();
        let (client, file) = (self.client.clone(), self.file.clone());
        tauri::async_runtime::spawn(async move {
            let _ = tx.send(settle(&client, &file).await);
        });
        self.in_flight = Some(rx);
        Poll::Pending
    }
}

/// The saved set if it is fresh enough; otherwise a set fetched now, saved once it reads;
/// failing that, the saved set as a saved copy.
async fn settle(client: &reqwest::Client, file: &Path) -> Answer {
    let saved = saved_set(file);
    if let Some((files, saved_at)) = &saved
        && SystemTime::now().duration_since(*saved_at).is_ok_and(|age| age < FRESH_FOR)
    {
        return Ok(Fetched::Fresh(files.clone()));
    }
    let error = match fetch(client).await {
        Ok(files) => match osms::read(&files) {
            Ok(_) => {
                save(file, &files);
                return Ok(Fetched::Fresh(files));
            }
            Err(e) => e,
        },
        Err(why) => SourceError::Unreachable(why),
    };
    eprintln!("OSMS: {error}");
    match saved {
        Some((data, saved_at)) => {
            let since_epoch = saved_at.duration_since(UNIX_EPOCH).unwrap_or_default();
            Ok(Fetched::SavedCopy { data, saved_at_unix_ms: since_epoch.as_millis() as u64 })
        }
        None => Err(error),
    }
}

/// Fetch OSMS's five files, one after another: the first that fails fails the set.
pub async fn fetch(client: &reqwest::Client) -> Result<OsmsFiles, String> {
    let mut texts = Vec::new();
    for name in OsmsFiles::NAMES {
        let url = format!("{}{name}", osms::DATA_URL);
        let response = client.get(&url).send().await.and_then(|r| r.error_for_status());
        let text = match response {
            Ok(response) => response.text().await,
            Err(e) => Err(e),
        };
        texts.push(text.map_err(|e| format!("{url}: {e}"))?);
    }
    let [maps, portals, monsters, map_manifest, lookups]: [String; 5] =
        texts.try_into().map_err(|_| "not five files".to_string())?;
    Ok(OsmsFiles { maps, portals, monsters, map_manifest, lookups })
}

/// Fetch OSMS's live set now, whatever is saved, for the probe.
pub fn fetch_live() -> Result<OsmsFiles, String> {
    let client = crate::meowdb::client().map_err(|e| e.to_string())?;
    tauri::async_runtime::block_on(fetch(&client))
}

/// The set saved at `file`, and when it was saved.
fn saved_set(file: &Path) -> Option<(OsmsFiles, SystemTime)> {
    let json = std::fs::read(file).ok()?;
    let files = serde_json::from_slice(&json).ok()?;
    let saved_at = std::fs::metadata(file).and_then(|m| m.modified()).ok()?;
    Some((files, saved_at))
}

/// Save `files` at `file`, in place of the set saved there, whole.
fn save(file: &Path, files: &OsmsFiles) {
    let written = serde_json::to_vec(files)
        .map_err(std::io::Error::other)
        .and_then(|json| crate::files::write_atomically(file, &json));
    if let Err(e) = written {
        eprintln!("OSMS: couldn't save {}: {e}", file.display());
    }
}
