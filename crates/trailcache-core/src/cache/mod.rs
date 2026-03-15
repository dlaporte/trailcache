//! Local caching module for offline data access.
//!
//! This module provides the `CacheManager` for storing and retrieving
//! troop data locally. Data is cached in JSON format and considered
//! stale after 60 minutes.
//!
//! Cached data types include:
//! - Youth, Adults, Parents
//! - Events
//! - Advancement dashboard and progress data
//! - Patrols

pub mod fetch;
pub mod manager;
pub mod offline;
pub mod refresh;

pub use fetch::fetch_with_cache;
pub use manager::{CacheAges, CacheManager};
pub use offline::{cache_all_for_offline, CacheProgress};
pub use refresh::{refresh_base_data, RefreshResult as BaseRefreshResult};
