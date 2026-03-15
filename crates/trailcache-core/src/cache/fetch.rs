//! Generic fetch-with-cache pattern.
//!
//! Encapsulates the repeated logic: check cache → fetch API → save → fallback to stale.

use anyhow::Result;
use std::future::Future;

use super::manager::CachedData;

/// Fetch data with cache fallback.
///
/// - `offline` — if true, return cached data only (regardless of staleness)
/// - `load_cache` — loads cached data (returns `Ok(None)` if absent)
/// - `save_cache` — saves fetched data to cache
/// - `fetch_api` — async API fetch
///
/// Returns:
/// - Offline + cached → `Ok(Some(data))`
/// - Offline + no cache → `Ok(None)`
/// - Cache fresh → `Ok(Some(data))`
/// - API success → saves + `Ok(Some(data))`
/// - API error + stale cache → `Ok(Some(stale_data))`
/// - API error + no cache → `Err`
pub async fn fetch_with_cache<T>(
    offline: bool,
    load_cache: impl FnOnce() -> Result<Option<CachedData<T>>>,
    save_cache: impl FnOnce(&T) -> Result<()>,
    fetch_api: impl Future<Output = Result<T>>,
) -> Result<Option<T>> {
    // Offline mode: return whatever we have
    if offline {
        return Ok(load_cache()?.map(|c| c.data));
    }

    // Check cache freshness
    let stale_data = match load_cache()? {
        Some(cached) if !cached.is_stale() => return Ok(Some(cached.data)),
        Some(cached) => Some(cached.data),
        None => None,
    };

    // Fetch from API
    match fetch_api.await {
        Ok(data) => {
            let _ = save_cache(&data);
            Ok(Some(data))
        }
        Err(e) => match stale_data {
            Some(data) => Ok(Some(data)),
            None => Err(e),
        },
    }
}
