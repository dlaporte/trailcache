//! Local caching module for offline data access.
//!
//! This module provides the `CacheManager` for storing and retrieving
//! Scoutbook data locally. Data is cached in JSON format and considered
//! stale after 60 minutes.
//!
//! Cached data types include:
//! - Youth, Adults, Parents
//! - Events
//! - Advancement dashboard and progress data
//! - Patrols

pub mod manager;

pub use manager::CacheManager;
