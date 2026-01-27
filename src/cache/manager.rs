// Allow dead code: Infrastructure methods for future use
#![allow(dead_code)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::debug;

use crate::models::{
    Adult, AdvancementDashboard, Commissioner, Event, Key3Leaders, MeritBadgeProgress,
    OrgProfile, Parent, Patrol, RankProgress, ReadyToAward, UnitInfo, Youth,
};

/// Consider cache stale after 1 hour.
/// Balances freshness with reducing unnecessary API calls for slowly-changing data.
const CACHE_STALE_MINUTES: i64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedData<T> {
    pub data: T,
    pub cached_at: DateTime<Utc>,
}

impl<T> CachedData<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            cached_at: Utc::now(),
        }
    }

    pub fn age_minutes(&self) -> i64 {
        let now = Utc::now();
        (now - self.cached_at).num_minutes()
    }

    pub fn age_display(&self) -> String {
        let minutes = self.age_minutes();
        if minutes < 0 {
            // Handle clock skew gracefully
            "just now".to_string()
        } else if minutes < 1 {
            "just now".to_string()
        } else if minutes < 60 {
            format!("{}m ago", minutes)
        } else if minutes < 1440 {
            let hours = minutes / 60;
            let remaining_mins = minutes % 60;
            if remaining_mins >= 30 {
                // Round up: 1h 30m+ becomes 2h
                format!("{}h ago", hours + 1)
            } else {
                format!("{}h ago", hours)
            }
        } else {
            let days = minutes / 1440;
            let remaining_hours = (minutes % 1440) / 60;
            if remaining_hours >= 12 {
                // Round up: 1d 12h+ becomes 2d
                format!("{}d ago", days + 1)
            } else {
                format!("{}d ago", days)
            }
        }
    }

    pub fn is_stale(&self) -> bool {
        self.age_minutes() > CACHE_STALE_MINUTES
    }
}

pub struct CacheManager {
    cache_dir: PathBuf,
}

impl CacheManager {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    fn cache_path(&self, name: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", name))
    }

    fn load<T: DeserializeOwned>(&self, name: &str) -> Result<Option<CachedData<T>>> {
        let path = self.cache_path(name);
        if !path.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read cache file: {}", name))?;

        let cached: CachedData<T> = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse cache file: {}", name))?;

        Ok(Some(cached))
    }

    fn save<T: Serialize>(&self, name: &str, data: &T) -> Result<()> {
        let cached = CachedData::new(data);
        let path = self.cache_path(name);
        let contents = serde_json::to_string_pretty(&cached)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    // ===== Youth =====

    pub fn load_youth(&self) -> Result<Option<CachedData<Vec<Youth>>>> {
        self.load("youth")
    }

    pub fn save_youth(&self, youth: &[Youth]) -> Result<()> {
        self.save("youth", &youth)
    }

    // ===== Adults =====

    pub fn load_adults(&self) -> Result<Option<CachedData<Vec<Adult>>>> {
        self.load("adults")
    }

    pub fn save_adults(&self, adults: &[Adult]) -> Result<()> {
        self.save("adults", &adults)
    }

    // ===== Parents =====

    pub fn load_parents(&self) -> Result<Option<CachedData<Vec<Parent>>>> {
        self.load("parents")
    }

    pub fn save_parents(&self, parents: &[Parent]) -> Result<()> {
        self.save("parents", &parents)
    }

    // ===== Patrols =====

    pub fn load_patrols(&self) -> Result<Option<CachedData<Vec<Patrol>>>> {
        self.load("patrols")
    }

    pub fn save_patrols(&self, patrols: &[Patrol]) -> Result<()> {
        self.save("patrols", &patrols)
    }

    // ===== Advancement Dashboard =====

    pub fn load_advancement_dashboard(&self) -> Result<Option<CachedData<AdvancementDashboard>>> {
        self.load("advancement_dashboard")
    }

    pub fn save_advancement_dashboard(&self, dashboard: &AdvancementDashboard) -> Result<()> {
        self.save("advancement_dashboard", dashboard)
    }

    // ===== Ready to Award =====

    pub fn load_ready_to_award(&self) -> Result<Option<CachedData<Vec<ReadyToAward>>>> {
        self.load("ready_to_award")
    }

    pub fn save_ready_to_award(&self, awards: &[ReadyToAward]) -> Result<()> {
        self.save("ready_to_award", &awards)
    }

    // ===== Events =====

    pub fn load_events(&self) -> Result<Option<CachedData<Vec<Event>>>> {
        self.load("events")
    }

    pub fn save_events(&self, events: &[Event]) -> Result<()> {
        self.save("events", &events)
    }

    // ===== Individual Youth Progress =====

    pub fn load_youth_ranks(&self, user_id: i64) -> Result<Option<CachedData<Vec<RankProgress>>>> {
        self.load(&format!("ranks_{}", user_id))
    }

    pub fn save_youth_ranks(&self, user_id: i64, ranks: &[RankProgress]) -> Result<()> {
        self.save(&format!("ranks_{}", user_id), &ranks)
    }

    pub fn load_youth_merit_badges(
        &self,
        user_id: i64,
    ) -> Result<Option<CachedData<Vec<MeritBadgeProgress>>>> {
        self.load(&format!("merit_badges_{}", user_id))
    }

    pub fn save_youth_merit_badges(
        &self,
        user_id: i64,
        badges: &[MeritBadgeProgress],
    ) -> Result<()> {
        self.save(&format!("merit_badges_{}", user_id), &badges)
    }

    // ===== Unit Info =====

    pub fn load_unit_info(&self) -> Result<Option<CachedData<UnitInfo>>> {
        self.load("unit_info")
    }

    pub fn save_unit_info(&self, info: &UnitInfo) -> Result<()> {
        self.save("unit_info", info)
    }

    // ===== Key3 =====

    pub fn load_key3(&self) -> Result<Option<CachedData<Key3Leaders>>> {
        self.load("key3")
    }

    pub fn save_key3(&self, key3: &Key3Leaders) -> Result<()> {
        self.save("key3", key3)
    }

    // ===== Org Profile =====

    pub fn load_org_profile(&self) -> Result<Option<CachedData<OrgProfile>>> {
        self.load("org_profile")
    }

    pub fn save_org_profile(&self, profile: &OrgProfile) -> Result<()> {
        self.save("org_profile", profile)
    }

    // ===== Commissioners =====

    pub fn load_commissioners(&self) -> Result<Option<CachedData<Vec<Commissioner>>>> {
        self.load("commissioners")
    }

    pub fn save_commissioners(&self, commissioners: &[Commissioner]) -> Result<()> {
        self.save("commissioners", &commissioners)
    }

    // ===== Rank Requirements =====

    pub fn load_rank_requirements(
        &self,
        user_id: i64,
        rank_id: i64,
    ) -> Result<Option<CachedData<Vec<crate::models::RankRequirement>>>> {
        self.load(&format!("rank_reqs_{}_{}", user_id, rank_id))
    }

    pub fn save_rank_requirements(
        &self,
        user_id: i64,
        rank_id: i64,
        requirements: &[crate::models::RankRequirement],
    ) -> Result<()> {
        self.save(&format!("rank_reqs_{}_{}", user_id, rank_id), &requirements)
    }

    // ===== Badge Requirements =====

    pub fn load_badge_requirements(
        &self,
        user_id: i64,
        badge_id: i64,
    ) -> Result<Option<CachedData<(Vec<crate::models::MeritBadgeRequirement>, Option<String>)>>> {
        self.load(&format!("badge_reqs_{}_{}", user_id, badge_id))
    }

    pub fn save_badge_requirements(
        &self,
        user_id: i64,
        badge_id: i64,
        requirements: &[crate::models::MeritBadgeRequirement],
        version: &Option<String>,
    ) -> Result<()> {
        self.save(&format!("badge_reqs_{}_{}", user_id, badge_id), &(requirements, version))
    }

    // ===== Cache Age Information =====

    /// Helper to load cache and log errors without failing
    fn load_age<T>(&self, name: &str, loader: impl FnOnce() -> Result<Option<CachedData<T>>>) -> Option<String> {
        match loader() {
            Ok(Some(cached)) => Some(cached.age_display()),
            Ok(None) => None,
            Err(e) => {
                debug!(cache = name, error = %e, "Failed to load cache for age display");
                None
            }
        }
    }

    pub fn get_cache_ages(&self) -> CacheAges {
        CacheAges {
            youth: self.load_age("youth", || self.load_youth()),
            adults: self.load_age("adults", || self.load_adults()),
            parents: self.load_age("parents", || self.load_parents()),
            patrols: self.load_age("patrols", || self.load_patrols()),
            events: self.load_age("events", || self.load_events()),
            advancement: self.load_age("advancement", || self.load_advancement_dashboard()),
        }
    }

    /// Helper to check staleness and log errors without failing
    fn is_cache_stale<T>(&self, name: &str, loader: impl FnOnce() -> Result<Option<CachedData<T>>>) -> bool {
        match loader() {
            Ok(Some(cached)) => cached.is_stale(),
            Ok(None) => true, // No cache = stale
            Err(e) => {
                debug!(cache = name, error = %e, "Failed to load cache for staleness check");
                true // Error reading = treat as stale
            }
        }
    }

    /// Check if any of the core cached data is stale
    pub fn any_stale(&self) -> bool {
        // Check all main cache types for staleness
        let stale_checks = [
            self.is_cache_stale("youth", || self.load_youth()),
            self.is_cache_stale("adults", || self.load_adults()),
            self.is_cache_stale("events", || self.load_events()),
            self.is_cache_stale("patrols", || self.load_patrols()),
            self.is_cache_stale("advancement", || self.load_advancement_dashboard()),
        ];
        stale_checks.iter().any(|&stale| stale)
    }
}

#[derive(Debug, Default)]
pub struct CacheAges {
    pub youth: Option<String>,
    pub adults: Option<String>,
    pub parents: Option<String>,
    pub patrols: Option<String>,
    pub events: Option<String>,
    pub advancement: Option<String>,
}

impl CacheAges {
    pub fn roster_age(&self) -> String {
        self.youth
            .clone()
            .or_else(|| self.adults.clone())
            .unwrap_or_else(|| "never".to_string())
    }

    pub fn events_age(&self) -> String {
        self.events.clone().unwrap_or_else(|| "never".to_string())
    }

    /// Returns the most recent update time across all cache types
    pub fn last_updated(&self) -> String {
        // Return the most recent (smallest age) from all cache types
        let ages = [
            &self.youth,
            &self.adults,
            &self.events,
        ];

        // Find any that has a value
        for a in ages.iter().copied().flatten() {
            return a.clone();
        }

        "never".to_string()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_cached_data_age_display_just_now() {
        let cached = CachedData::new(vec![1, 2, 3]);
        // Just created, should be "just now"
        assert_eq!(cached.age_display(), "just now");
    }

    #[test]
    fn test_cached_data_is_stale() {
        let fresh = CachedData::new(vec![1]);
        assert!(!fresh.is_stale());

        // Create a cached data that's 61 minutes old
        let mut old = CachedData::new(vec![1]);
        old.cached_at = Utc::now() - Duration::minutes(61);
        assert!(old.is_stale());
    }

    #[test]
    fn test_cached_data_age_minutes() {
        let cached = CachedData::new(vec![1]);
        // Should be 0 or very close to 0
        assert!(cached.age_minutes() <= 1);
    }

    #[test]
    fn test_cache_ages_last_updated_with_values() {
        let ages = CacheAges {
            youth: Some("5m ago".to_string()),
            adults: None,
            parents: None,
            patrols: None,
            events: None,
            advancement: None,
        };
        assert_eq!(ages.last_updated(), "5m ago");
    }

    #[test]
    fn test_cache_ages_last_updated_empty() {
        let ages = CacheAges::default();
        assert_eq!(ages.last_updated(), "never");
    }
}
