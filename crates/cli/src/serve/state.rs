//! Application state and rate limiting.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Instant;

use tokio::sync::{Mutex, RwLock};

use super::RATE_LIMIT_WINDOW_SECS;

/// Per-IP request tracker: (request count, window start time).
type IpTracker = HashMap<IpAddr, (u64, Instant)>;

/// In-memory per-IP rate limiter.
pub(crate) struct RateLimiter {
    /// Request counts per IP per window.
    tracker: Mutex<IpTracker>,
    /// Maximum requests per window.
    pub(crate) max_requests: u64,
}

impl RateLimiter {
    pub(crate) fn new(max_requests: u64) -> Self {
        Self {
            tracker: Mutex::new(HashMap::new()),
            max_requests,
        }
    }

    /// Check if a request from the given IP is allowed.
    /// Returns Ok(()) if allowed, Err(retry_after_secs) if rate limited.
    pub(crate) async fn check(&self, ip: IpAddr) -> Result<(), u64> {
        let mut tracker = self.tracker.lock().await;
        let now = Instant::now();

        let entry = tracker.entry(ip).or_insert((0, now));

        // Reset window if expired
        let elapsed = now.duration_since(entry.1).as_secs();
        if elapsed >= RATE_LIMIT_WINDOW_SECS {
            entry.0 = 0;
            entry.1 = now;
        }

        entry.0 += 1;
        if entry.0 > self.max_requests {
            let retry_after = RATE_LIMIT_WINDOW_SECS.saturating_sub(elapsed);
            Err(retry_after)
        } else {
            Ok(())
        }
    }
}

/// Application state shared across request handlers.
pub(crate) struct AppState {
    /// Loaded contract bundles keyed by bundle ID.
    pub(crate) contracts: RwLock<HashMap<String, serde_json::Value>>,
    /// Per-IP rate limiter.
    pub(crate) rate_limiter: RateLimiter,
    /// Optional API key for authentication. None = no auth required.
    pub(crate) api_key: Option<String>,
}
