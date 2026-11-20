use crate::Result;
use std::time::Duration;

/// Default timeout for outbound HTTP calls.
///
/// Every external call must be bounded. Without one, a peer that accepts the
/// connection but never answers keeps a sync (or an RPC handler) alive for as
/// long as the socket survives — we have seen a single `sync_submitted_places`
/// run for 47 minutes because of an untimed Gitea request.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

pub fn client() -> Result<reqwest::Client> {
    client_with_timeout(DEFAULT_TIMEOUT)
}

pub fn client_with_timeout(timeout: Duration) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder().timeout(timeout).build()?)
}
