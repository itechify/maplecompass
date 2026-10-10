//! OSMS's pictures (ADR 0008), by their path below [`crate::osms::PICTURES_URL`]: the Mob
//! Cards' animations. The shell keeps them on disk and answers with where the overlay
//! loads each from.
//!
//! The pictures the panel wants are asked for one at a time, with no other pace: the next
//! as soon as the one before is in. A path names a picture by its content, so an answer is
//! kept for the run and a picture wanted again isn't asked for again; one that couldn't be
//! fetched is asked for again a few minutes later, while it is still wanted.

use crate::map_index::{MapDataSource, SourceError};
use std::collections::HashMap;
use std::task::Poll;

/// How long an unreachable picture waits before it is asked for again.
const RETRY_MS: u64 = 5 * 60_000;

/// How many answers given at once (from the shell's saved pictures) are taken in one tick.
const AT_ONCE_PER_TICK: usize = 8;

/// The pictures asked for, and the one being fetched.
#[derive(Default)]
pub(crate) struct Pictures {
    /// Each picture asked for, by path: its latest answer, and when it came.
    answers: HashMap<String, (Result<String, SourceError>, u64)>,
    in_flight: Option<String>,
}

impl Pictures {
    /// Where the overlay loads the picture at `path` from, once it is in.
    pub fn url(&self, path: &str) -> Option<&str> {
        match self.answers.get(path) {
            Some((Ok(url), _)) => Some(url),
            _ => None,
        }
    }

    /// Time passed: check on the picture being fetched (even one no longer wanted), and
    /// ask for the next of the `wanted` ones once it is in.
    pub fn poll(&mut self, wanted: &[String], source: &mut dyn MapDataSource, at_ms: u64) {
        for _ in 0..AT_ONCE_PER_TICK {
            let Some(path) = self.in_flight.take().or_else(|| self.next(wanted, at_ms)) else { return };
            match source.picture(&path) {
                Poll::Pending => {
                    self.in_flight = Some(path);
                    return;
                }
                Poll::Ready(answer) => {
                    self.answers.insert(path, (answer, at_ms));
                }
            }
        }
    }

    /// The first wanted picture never asked for, else the first that couldn't be fetched
    /// and is due to be asked for again.
    fn next(&self, wanted: &[String], at_ms: u64) -> Option<String> {
        let unasked = || wanted.iter().find(|path| !self.answers.contains_key(*path));
        let retry = || {
            wanted.iter().find(|path| {
                matches!(self.answers.get(*path), Some((Err(SourceError::Unreachable(_)), came_ms))
                    if at_ms.saturating_sub(*came_ms) >= RETRY_MS)
            })
        };
        unasked().or_else(retry).cloned()
    }
}
