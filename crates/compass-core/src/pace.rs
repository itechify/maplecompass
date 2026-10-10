//! Asking a site politely: no more than one request every [`PACE_MS`], counting only
//! requests that went over the network.

/// The least time between two requests to the same site that went over the network:
/// about one every 2 seconds (ADR 0003, ADR 0007). An answer the source gives at once
/// (from its cache) doesn't count.
pub(crate) const PACE_MS: u64 = 2_000;

/// When the last request to one site that went over the network started.
#[derive(Default)]
pub(crate) struct Pace {
    last_sent_ms: Option<u64>,
}

impl Pace {
    /// Whether a new request may start at `at_ms`.
    pub fn ready(&self, at_ms: u64) -> bool {
        self.last_sent_ms.is_none_or(|t| at_ms.saturating_sub(t) >= PACE_MS)
    }

    /// A request started at `at_ms` and didn't answer at once: it went over the network.
    pub fn sent(&mut self, at_ms: u64) {
        self.last_sent_ms = Some(at_ms);
    }
}
