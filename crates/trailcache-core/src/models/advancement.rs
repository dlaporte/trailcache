// Allow dead code: API response structs have fields for completeness
#![allow(dead_code)]

use std::cmp::Ordering;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

// ============================================================================
// Rank Ordering
// ============================================================================

/// Classification of a status_display() result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCategory {
    Awarded,
    Completed,
    InProgress,
    None,
}

impl StatusCategory {
    /// Lowercase string representation for serialization to frontend.
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusCategory::Awarded => "awarded",
            StatusCategory::Completed => "completed",
            StatusCategory::InProgress => "in_progress",
            StatusCategory::None => "none",
        }
    }
}

/// Default badge status text when no status is available.
pub const DEFAULT_BADGE_STATUS: &str = "In Progress";

/// Sentinel returned by `format_date()` when the date is missing or unparseable.
pub const UNKNOWN_DATE: &str = "?";

/// Status string constants used for advancement completion checks.
pub const STATUS_AWARDED: &str = "Awarded";
pub const STATUS_LEADER_APPROVED: &str = "Leader Approved";
pub const STATUS_COUNSELOR_APPROVED: &str = "Counselor Approved";
pub const DEFAULT_AWARD_STATUS: &str = "Unknown";

/// Scout rank for sorting purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScoutRank {
    Unknown = 0,
    Scout = 1,
    Tenderfoot = 2,
    SecondClass = 3,
    FirstClass = 4,
    Star = 5,
    Life = 6,
    Eagle = 7,
}

impl ScoutRank {
    /// Parse a rank string into a ScoutRank enum value.
    /// Handles variations like "Eagle Scout", "Life Scout", etc.
    pub fn parse(s: Option<&str>) -> Self {
        match s {
            Some(rank) => {
                let lower = rank.to_lowercase();
                if lower.contains("eagle") {
                    ScoutRank::Eagle
                } else if lower.contains("life") {
                    ScoutRank::Life
                } else if lower.contains("star") {
                    ScoutRank::Star
                } else if lower.contains("first class") {
                    ScoutRank::FirstClass
                } else if lower.contains("second class") {
                    ScoutRank::SecondClass
                } else if lower.contains("tenderfoot") {
                    ScoutRank::Tenderfoot
                } else if lower == "scout" {
                    ScoutRank::Scout
                } else {
                    ScoutRank::Unknown
                }
            }
            None => ScoutRank::Unknown,
        }
    }

    /// Get the numeric order for sorting (0 = Unknown/Crossover, 7 = Eagle).
    pub fn order(&self) -> usize {
        *self as usize
    }

    /// Returns all ranks in display order (highest to lowest: Eagle → Unknown/Crossover).
    pub fn all_display_order() -> &'static [ScoutRank] {
        &[
            ScoutRank::Eagle,
            ScoutRank::Life,
            ScoutRank::Star,
            ScoutRank::FirstClass,
            ScoutRank::SecondClass,
            ScoutRank::Tenderfoot,
            ScoutRank::Scout,
            ScoutRank::Unknown,
        ]
    }

    /// Get a short abbreviation for this rank.
    pub fn abbreviation(&self) -> &'static str {
        match self {
            ScoutRank::Unknown => "Xovr",
            ScoutRank::Scout => "Sct",
            ScoutRank::Tenderfoot => "TF",
            ScoutRank::SecondClass => "2C",
            ScoutRank::FirstClass => "1C",
            ScoutRank::Star => "Star",
            ScoutRank::Life => "Life",
            ScoutRank::Eagle => "Eagle",
        }
    }

    /// Get the display name for this rank.
    pub fn display_name(&self) -> &'static str {
        match self {
            ScoutRank::Unknown => "Crossover",
            ScoutRank::Scout => "Scout",
            ScoutRank::Tenderfoot => "Tenderfoot",
            ScoutRank::SecondClass => "Second Class",
            ScoutRank::FirstClass => "First Class",
            ScoutRank::Star => "Star",
            ScoutRank::Life => "Life",
            ScoutRank::Eagle => "Eagle",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdvancementDashboard {
    #[serde(rename = "rankStats")]
    pub rank_stats: Option<Vec<RankStats>>,
    #[serde(rename = "meritBadgeCount")]
    pub merit_badge_count: Option<i32>,
    #[serde(rename = "activeYouthCount")]
    pub active_youth_count: Option<i32>,
    #[serde(rename = "readyToAwardCount")]
    pub ready_to_award_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankStats {
    #[serde(rename = "rankName")]
    pub rank_name: String,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyToAward {
    #[serde(rename = "userId")]
    pub user_id: i64,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    #[serde(rename = "advancementType")]
    pub advancement_type: String,
    #[serde(rename = "advancementName")]
    pub advancement_name: String,
    #[serde(rename = "dateCompleted")]
    pub date_completed: Option<String>,
}

impl ReadyToAward {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn display_name(&self) -> String {
        format!("{}, {}", self.last_name, self.first_name)
    }
}

// API response wrapper for ranks endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RanksResponse {
    pub status: Option<String>,
    pub program: Vec<ProgramRanks>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramRanks {
    #[serde(rename = "programId")]
    pub program_id: i32,
    pub program: String,
    #[serde(rename = "totalNumberOfRanks")]
    pub total_number_of_ranks: Option<i32>,
    pub ranks: Vec<RankFromApi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankFromApi {
    pub id: i64,
    #[serde(rename = "versionId")]
    pub version_id: Option<i64>,
    pub name: String,
    #[serde(rename = "dateEarned")]
    pub date_earned: Option<String>,
    pub awarded: Option<bool>,
    #[serde(rename = "awardedDate")]
    pub awarded_date: Option<String>,
    #[serde(rename = "percentCompleted")]
    pub percent_completed: Option<f32>,
    pub level: Option<i32>,
    pub status: Option<String>,
    #[serde(rename = "programId")]
    pub program_id: Option<i32>,
}

// Simplified rank progress for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankProgress {
    pub rank_id: i64,
    pub version_id: Option<i64>,
    pub rank_name: String,
    pub date_completed: Option<String>,
    pub date_awarded: Option<String>,
    pub requirements_completed: Option<i32>,
    pub requirements_total: Option<i32>,
    pub percent_completed: Option<f32>,
    pub level: Option<i32>,
}

impl RankProgress {
    /// Get sort order for rank (higher = more advanced, Eagle = 7, Scout = 1)
    pub fn sort_order(&self) -> i32 {
        // Use level if available, otherwise derive from name using ScoutRank
        self.level.unwrap_or_else(|| {
            ScoutRank::parse(Some(&self.rank_name)).order() as i32
        })
    }

    pub fn from_api(rank: &RankFromApi) -> Self {
        // Convert empty strings to None
        let date_completed = rank.date_earned.clone()
            .filter(|s| !s.is_empty());
        let date_awarded = rank.awarded_date.clone()
            .filter(|s| !s.is_empty());

        Self {
            rank_id: rank.id,
            version_id: rank.version_id,
            rank_name: rank.name.clone(),
            date_completed,
            date_awarded,
            requirements_completed: None,
            requirements_total: None,
            percent_completed: rank.percent_completed,
            level: rank.level,
        }
    }

    /// A rank is completed only if it has a non-empty dateEarned
    pub fn is_completed(&self) -> bool {
        self.date_completed.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
    }

    pub fn is_awarded(&self) -> bool {
        self.date_awarded.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
    }

    pub fn progress_percent(&self) -> Option<i32> {
        self.percent_completed.map(|p| (p * 100.0).round() as i32)
    }

    /// Sort date for ordering: prefers date_awarded, falls back to date_completed.
    pub fn sort_date(&self) -> String {
        self.date_awarded.clone()
            .or_else(|| self.date_completed.clone())
            .unwrap_or_default()
    }

    /// Pre-computed display date for the status column.
    pub fn display_date(&self) -> String {
        if self.is_awarded() {
            let d = format_date(self.date_awarded.as_deref());
            if d == UNKNOWN_DATE { format_date(self.date_completed.as_deref()) } else { d }
        } else {
            format_date(self.date_completed.as_deref())
        }
    }

    /// Classify the rank's display status.
    /// Returns (category, display_text).
    pub fn status_display(&self) -> (StatusCategory, String) {
        if self.is_awarded() {
            let date = format_date(self.date_awarded.as_deref());
            let display = if date == UNKNOWN_DATE { "Awarded".to_string() } else { date };
            (StatusCategory::Awarded, display)
        } else if self.is_completed() {
            let date = format_date(self.date_completed.as_deref());
            let display = if date == UNKNOWN_DATE { "Completed".to_string() } else { date };
            (StatusCategory::Completed, display)
        } else if let Some(pct) = self.progress_percent() {
            if pct > 0 {
                (StatusCategory::InProgress, format!("{}%", pct))
            } else {
                (StatusCategory::None, String::new())
            }
        } else {
            (StatusCategory::None, String::new())
        }
    }
}

// Wrapper for rank with requirements response
#[derive(Debug, Clone, Deserialize)]
pub struct RankWithRequirements {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub requirements: Vec<RankRequirement>,
}

// Wrapper for merit badge with requirements response
// Note: API returns id as string
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeritBadgeWithRequirements {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub requirements: Vec<MeritBadgeRequirement>,
    #[serde(rename = "assignedCounselorUser")]
    pub assigned_counselor: Option<CounselorInfo>,
}

/// Merit badge from the catalog endpoint (/advancements/meritBadges)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeritBadgeCatalogEntry {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub id: Option<String>,
    pub name: String,
    #[serde(rename = "isEagleRequired")]
    pub is_eagle_required: Option<bool>,
    pub category: Option<String>,
    #[serde(rename = "categoryId")]
    pub category_id: Option<String>,
    #[serde(rename = "bsaNumber")]
    pub bsa_number: Option<String>,
    #[serde(rename = "shortName")]
    pub short_name: Option<String>,
    pub version: Option<String>,
}

// Merit badge requirement from API
// Note: API returns many fields as strings (e.g., "True"/"False" for booleans)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeritBadgeRequirement {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub id: Option<String>,
    #[serde(rename = "number")]
    pub requirement_number: Option<String>,
    #[serde(rename = "listNumber")]
    pub list_number: Option<String>,
    pub name: Option<String>,
    pub short: Option<String>,
    #[serde(rename = "dateCompleted")]
    pub date_completed: Option<String>,
    #[serde(rename = "leaderApprovedDate")]
    pub leader_approved_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_bool")]
    pub completed: bool,
    pub status: Option<String>,
    #[serde(rename = "percentCompleted")]
    pub percent_completed: Option<String>,
}

// Helper to deserialize "True"/"False" strings or actual bools
fn deserialize_string_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct BoolVisitor;

    impl<'de> de::Visitor<'de> for BoolVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a boolean or string 'True'/'False'")
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
            Ok(v)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v.to_lowercase().as_str() {
                "true" => Ok(true),
                "false" | "" => Ok(false),
                _ => Ok(false),
            }
        }
    }

    deserializer.deserialize_any(BoolVisitor)
}

// Helper to deserialize string or number as Option<String>
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct StringOrNumberVisitor;

    impl<'de> de::Visitor<'de> for StringOrNumberVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
            if v.is_empty() {
                Ok(None)
            } else {
                Ok(Some(v.to_string()))
            }
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }

    deserializer.deserialize_any(StringOrNumberVisitor)
}

impl MeritBadgeRequirement {
    /// Count (completed, total) requirements in a slice.
    pub fn completion_count(reqs: &[MeritBadgeRequirement]) -> (usize, usize) {
        let completed = reqs.iter().filter(|r| r.is_completed()).count();
        (completed, reqs.len())
    }

    pub fn is_completed(&self) -> bool {
        self.completed
            || self.date_completed.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            || matches!(self.status.as_deref(), Some(STATUS_LEADER_APPROVED) | Some(STATUS_AWARDED) | Some(STATUS_COUNSELOR_APPROVED))
    }

    pub fn number(&self) -> String {
        self.list_number.clone()
            .filter(|s| !s.is_empty())
            .or_else(|| self.requirement_number.clone().filter(|s| !s.is_empty()))
            .unwrap_or_else(|| "-".to_string())
    }

    pub fn text(&self) -> String {
        self.short.clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.name.clone().unwrap_or_default())
    }

    pub fn full_text(&self) -> String {
        self.name.clone().unwrap_or_default()
    }
}

// Rank requirement from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankRequirement {
    pub id: Option<i64>,
    #[serde(rename = "requirementNumber")]
    pub requirement_number: Option<String>,
    #[serde(rename = "listNumber")]
    pub list_number: Option<String>,
    /// The full requirement text (API uses "name" field)
    pub name: Option<String>,
    /// Short description
    pub short: Option<String>,
    #[serde(rename = "dateCompleted")]
    pub date_completed: Option<String>,
    #[serde(rename = "leaderApprovedDate")]
    pub leader_approved_date: Option<String>,
    #[serde(rename = "leaderApprovedFirstName")]
    pub leader_approved_first_name: Option<String>,
    #[serde(rename = "leaderApprovedLastName")]
    pub leader_approved_last_name: Option<String>,
    pub completed: Option<bool>,
    pub status: Option<String>,
}

impl RankRequirement {
    /// Count (completed, total) requirements in a slice.
    pub fn completion_count(reqs: &[RankRequirement]) -> (usize, usize) {
        let completed = reqs.iter().filter(|r| r.is_completed()).count();
        (completed, reqs.len())
    }

    pub fn is_completed(&self) -> bool {
        self.completed.unwrap_or(false)
            || self.date_completed.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            || matches!(self.status.as_deref(), Some(STATUS_LEADER_APPROVED) | Some(STATUS_AWARDED))
    }

    pub fn number(&self) -> String {
        self.list_number.clone()
            .or_else(|| self.requirement_number.clone())
            .unwrap_or_else(|| "-".to_string())
    }

    pub fn text(&self) -> String {
        // Use short description if available, otherwise name (full text)
        self.short.clone().unwrap_or_else(||
            self.name.clone().unwrap_or_default()
        )
    }

    pub fn full_text(&self) -> String {
        self.name.clone().unwrap_or_default()
    }
}

/// Merit badge counselor information from API
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CounselorInfo {
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "middleName")]
    pub middle_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    #[serde(rename = "homePhone")]
    pub home_phone: Option<String>,
    #[serde(rename = "mobilePhone")]
    pub mobile_phone: Option<String>,
    pub email: Option<String>,
    pub picture: Option<String>,
}

impl CounselorInfo {
    /// Get the counselor's full name
    pub fn full_name(&self) -> String {
        let first = self.first_name.as_deref().unwrap_or("");
        let last = self.last_name.as_deref().unwrap_or("");
        format!("{} {}", first, last).trim().to_string()
    }

    /// Get the best phone number (prefer mobile, fall back to home)
    pub fn phone(&self) -> Option<&str> {
        self.mobile_phone.as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.home_phone.as_deref().filter(|s| !s.is_empty()))
    }
}

/// Summary of badge completion counts.
#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BadgeSummary {
    pub completed: usize,
    pub in_progress: usize,
    pub eagle_completed: usize,
    pub eagle_required_total: usize,
}

// Merit badge from API (flat array)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeritBadgeProgress {
    pub id: i64,
    pub name: String,
    #[serde(rename = "dateStarted")]
    pub date_started: Option<String>,
    #[serde(rename = "dateCompleted")]
    pub date_completed: Option<String>,
    #[serde(rename = "awardedDate")]
    pub awarded_date: Option<String>,
    #[serde(rename = "percentCompleted")]
    pub percent_completed: Option<f32>,
    #[serde(rename = "isEagleRequired")]
    pub is_eagle_required: Option<bool>,
    pub status: Option<String>,
    #[serde(rename = "assignedCounselorUser")]
    pub assigned_counselor: Option<CounselorInfo>,
    // Keep old fields for compatibility
    #[serde(skip)]
    pub requirements_completed: Option<i32>,
    #[serde(skip)]
    pub requirements_total: Option<i32>,
}

impl MeritBadgeProgress {
    /// A merit badge is completed if status is "Awarded" or "Leader Approved"
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status.as_deref(),
            Some(STATUS_AWARDED) | Some(STATUS_LEADER_APPROVED)
        )
    }

    pub fn is_awarded(&self) -> bool {
        self.status.as_deref() == Some(STATUS_AWARDED)
    }

    pub fn progress_percent(&self) -> Option<i32> {
        self.percent_completed.map(|p| (p * 100.0).round() as i32)
    }

    /// Summarize a slice of badges into completed/in-progress/eagle counts.
    pub fn summarize(badges: &[MeritBadgeProgress]) -> BadgeSummary {
        let mut completed = 0usize;
        let mut in_progress = 0usize;
        let mut eagle_completed = 0usize;
        for b in badges {
            if b.is_completed() {
                completed += 1;
                if b.is_eagle_required.unwrap_or(false) {
                    eagle_completed += 1;
                }
            } else {
                in_progress += 1;
            }
        }
        BadgeSummary { completed, in_progress, eagle_completed, eagle_required_total: EAGLE_REQUIRED_COUNT }
    }

    /// Compare two badges for display sorting: in-progress first (by % desc), then completed/awarded (by date desc).
    /// Awarded items sort before merely completed within the completed group.
    pub fn cmp_by_progress(a: &MeritBadgeProgress, b: &MeritBadgeProgress) -> Ordering {
        // Status order: in-progress (0) < completed (1) < awarded (2)
        let status_order = |m: &MeritBadgeProgress| -> u8 {
            if m.is_awarded() { 2 }
            else if m.is_completed() { 1 }
            else { 0 }
        };
        let a_status = status_order(a);
        let b_status = status_order(b);

        if a_status == 0 && b_status == 0 {
            // Both in-progress: sort by percent desc
            let pct_a = a.percent_completed.unwrap_or(0.0);
            let pct_b = b.percent_completed.unwrap_or(0.0);
            pct_b.partial_cmp(&pct_a).unwrap_or(Ordering::Equal)
        } else if a_status == 0 || b_status == 0 {
            // In-progress comes first
            a_status.cmp(&b_status)
        } else {
            // Both completed/awarded: awarded first, then by sort date desc
            let sort_date = |m: &MeritBadgeProgress| -> String {
                m.awarded_date.clone()
                    .or_else(|| m.date_completed.clone())
                    .unwrap_or_default()
            };
            b_status.cmp(&a_status)
                .then_with(|| sort_date(b).cmp(&sort_date(a)))
        }
    }

    /// Classify the badge's display status.
    /// Returns (category, display_text).
    pub fn status_display(&self) -> (StatusCategory, String) {
        if self.is_awarded() {
            let date = format_date(self.awarded_date.as_deref());
            let display = if date == UNKNOWN_DATE {
                // Fall back to date_completed if awarded_date is missing
                let fallback = format_date(self.date_completed.as_deref());
                if fallback == UNKNOWN_DATE { "Awarded".to_string() } else { fallback }
            } else {
                date
            };
            (StatusCategory::Awarded, display)
        } else if self.is_completed() {
            let date = format_date(self.date_completed.as_deref());
            let display = if date == UNKNOWN_DATE { "Completed".to_string() } else { date };
            (StatusCategory::Completed, display)
        } else if let Some(pct) = self.progress_percent() {
            (StatusCategory::InProgress, format!("{}%", pct))
        } else {
            (StatusCategory::None, self.status.clone().unwrap_or_else(|| DEFAULT_BADGE_STATUS.to_string()))
        }
    }

    /// Sort date for ordering: prefers awarded_date, falls back to date_completed.
    pub fn sort_date(&self) -> String {
        self.awarded_date.clone()
            .or_else(|| self.date_completed.clone())
            .unwrap_or_default()
    }

    /// Check if this badge has an assigned counselor
    pub fn has_counselor(&self) -> bool {
        self.assigned_counselor.as_ref()
            .map(|c| !c.full_name().is_empty())
            .unwrap_or(false)
    }
}

// Leadership position history from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadershipPosition {
    pub position: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(rename = "numberOfDaysInPosition")]
    pub days_served: Option<i32>,
    pub patrol: Option<String>,
    pub rank: Option<String>,
}

impl LeadershipPosition {
    /// Returns the position name, or "Unknown Position" if not set
    pub fn name(&self) -> &str {
        self.position.as_deref().unwrap_or("Unknown Position")
    }

    /// Returns true if this is a current position (no end date or end date in future)
    pub fn is_current(&self) -> bool {
        self.end_date.is_none() || self.end_date.as_deref() == Some("")
    }

    /// Format the date range for display
    pub fn date_range(&self) -> String {
        let start = format_date(self.start_date.as_deref());
        if self.is_current() {
            format!("{} - Present", start)
        } else {
            let end = format_date(self.end_date.as_deref());
            format!("{} - {}", start, end)
        }
    }

    /// Sort positions for display: current first, then by start_date desc.
    pub fn sort_for_display(positions: &mut [LeadershipPosition]) {
        positions.sort_by(|a, b| {
            match (a.is_current(), b.is_current()) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => b.start_date.cmp(&a.start_date),
            }
        });
    }

    /// Format days served for display
    pub fn days_display(&self) -> String {
        match self.days_served {
            Some(days) if days > 0 => format!("{} days", days),
            _ => String::new(),
        }
    }
}

// Youth award from API (e.g., Eagle Palm, 50-miler, etc.)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Award {
    #[serde(alias = "awardId")]
    pub id: Option<i64>,
    pub name: Option<String>,
    #[serde(rename = "dateStarted")]
    pub date_started: Option<String>,
    #[serde(rename = "dateCompleted", alias = "markedCompletedDate")]
    pub date_completed: Option<String>,
    #[serde(rename = "dateEarned")]
    pub date_earned: Option<String>,
    #[serde(rename = "dateAwarded", alias = "awardedDate")]
    pub date_awarded: Option<String>,
    #[serde(rename = "awardType")]
    pub award_type: Option<String>,
    pub status: Option<String>,
    /// v2 API uses boolean `awarded` field
    pub awarded: Option<bool>,
    #[serde(rename = "percentCompleted")]
    pub percent_completed: Option<f32>,
    #[serde(rename = "leaderApprovedDate")]
    pub leader_approved_date: Option<String>,
}

impl Award {
    /// Sort awards for display: awarded first, then by date_awarded desc.
    pub fn sort_for_display(awards: &mut [Award]) {
        awards.sort_by(|a, b| {
            match (a.is_awarded(), b.is_awarded()) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => b.date_awarded.cmp(&a.date_awarded),
            }
        });
    }

    /// Returns the award name, or "Unknown Award" if not set
    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("Unknown Award")
    }

    /// Returns true if this award has been awarded (status is "Awarded", awarded=true, or has awarded date)
    pub fn is_awarded(&self) -> bool {
        self.awarded == Some(true)
            || self.status.as_deref() == Some(STATUS_AWARDED)
            || self.date_awarded.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
    }

    /// Returns true if this award is completed (status is "Awarded" or "Leader Approved", or has leader approved date)
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status.as_deref(),
            Some(STATUS_AWARDED) | Some(STATUS_LEADER_APPROVED)
        ) || self.date_completed.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            || self.leader_approved_date.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
    }

    /// Format the date for display
    pub fn date_display(&self) -> String {
        if let Some(ref date) = self.date_awarded {
            if !date.is_empty() {
                return format_date(Some(date));
            }
        }
        if let Some(ref date) = self.date_completed {
            if !date.is_empty() {
                return format_date(Some(date));
            }
        }
        if let Some(ref date) = self.date_earned {
            if !date.is_empty() {
                return format!("{} (earned)", format_date(Some(date)));
            }
        }
        if let Some(ref date) = self.date_started {
            if !date.is_empty() {
                return format!("{} (started)", format_date(Some(date)));
            }
        }
        UNKNOWN_DATE.to_string()
    }

    /// Get the award type for display
    pub fn type_display(&self) -> &str {
        self.award_type.as_deref().unwrap_or("")
    }

    /// Get progress percentage if available
    pub fn progress_percent(&self) -> Option<i32> {
        self.percent_completed.map(|p| (p * 100.0).round() as i32)
    }
}

/// Number of Eagle-required merit badges in the Scouts BSA program.
pub const EAGLE_REQUIRED_COUNT: usize = 13;

/// Format a date string from "YYYY-MM-DD" to "Month DD, YYYY"
pub fn format_date(date: Option<&str>) -> String {
    match date {
        Some(d) if d.len() >= 10 => {
            if let Ok(parsed) = NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d") {
                parsed.format("%b %d, %Y").to_string()
            } else {
                d.to_string()
            }
        }
        Some(d) => d.to_string(),
        None => UNKNOWN_DATE.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_award_v2_deserialization() {
        let json = r#"{"awardId": 33, "name": "Honor Medal", "status": "Started", "awarded": false, "percentCompleted": 0}"#;
        let award: Award = serde_json::from_str(json).expect("Failed to parse");
        assert_eq!(award.id, Some(33));
        assert_eq!(award.name, Some("Honor Medal".to_string()));
        assert_eq!(award.status, Some("Started".to_string()));
        assert_eq!(award.awarded, Some(false));
    }

    #[test]
    fn test_merit_badge_with_counselor() {
        let json = r#"{
            "id": 24,
            "name": "Citizenship in the Community",
            "dateStarted": "2024-12-01",
            "percentCompleted": 0.43,
            "isEagleRequired": true,
            "status": "Started",
            "assignedCounselorUser": {
                "userId": "1234567",
                "firstName": "John",
                "middleName": "Q",
                "lastName": "Smith",
                "homePhone": "5551234567",
                "mobilePhone": "5559876543",
                "email": "john.smith@example.com",
                "picture": null
            }
        }"#;
        let badge: MeritBadgeProgress = serde_json::from_str(json).expect("Failed to parse");
        assert_eq!(badge.id, 24);
        assert_eq!(badge.name, "Citizenship in the Community");
        assert!(badge.has_counselor());

        let counselor = badge.assigned_counselor.as_ref().unwrap();
        assert_eq!(counselor.full_name(), "John Smith");
        assert_eq!(counselor.phone(), Some("5559876543"));
        assert_eq!(counselor.email.as_deref(), Some("john.smith@example.com"));
    }

    #[test]
    fn test_merit_badge_without_counselor() {
        let json = r#"{
            "id": 25,
            "name": "Swimming",
            "status": "Started"
        }"#;
        let badge: MeritBadgeProgress = serde_json::from_str(json).expect("Failed to parse");
        assert_eq!(badge.id, 25);
        assert!(!badge.has_counselor());
        assert!(badge.assigned_counselor.is_none());
    }

    fn make_badge(name: &str, status: &str, pct: Option<f32>, date_completed: Option<&str>, eagle: bool) -> MeritBadgeProgress {
        MeritBadgeProgress {
            id: 1,
            name: name.to_string(),
            date_started: None,
            date_completed: date_completed.map(|s| s.to_string()),
            awarded_date: None,
            percent_completed: pct,
            is_eagle_required: Some(eagle),
            status: Some(status.to_string()),
            assigned_counselor: None,
            requirements_completed: None,
            requirements_total: None,
        }
    }

    #[test]
    fn test_badge_summary() {
        let badges = vec![
            make_badge("Swimming", "Awarded", None, Some("2025-01-01"), false),
            make_badge("Citizenship", "Awarded", None, Some("2025-02-01"), true),
            make_badge("Camping", "Started", Some(0.5), None, true),
            make_badge("Cooking", "Started", Some(0.2), None, false),
        ];
        let summary = MeritBadgeProgress::summarize(&badges);
        assert_eq!(summary.completed, 2);
        assert_eq!(summary.in_progress, 2);
        assert_eq!(summary.eagle_completed, 1);
    }

    #[test]
    fn test_cmp_by_progress() {
        let in_progress_high = make_badge("A", "Started", Some(0.8), None, false);
        let in_progress_low = make_badge("B", "Started", Some(0.2), None, false);
        let completed_new = make_badge("C", "Awarded", None, Some("2025-06-01"), false);
        let completed_old = make_badge("D", "Awarded", None, Some("2024-01-01"), false);

        // In-progress comes before completed
        assert_eq!(MeritBadgeProgress::cmp_by_progress(&in_progress_high, &completed_new), Ordering::Less);
        // Higher percent comes first among in-progress
        assert_eq!(MeritBadgeProgress::cmp_by_progress(&in_progress_high, &in_progress_low), Ordering::Less);
        // Newer date comes first among completed
        assert_eq!(MeritBadgeProgress::cmp_by_progress(&completed_new, &completed_old), Ordering::Less);
    }

    #[test]
    fn test_leadership_sort_for_display() {
        let mut positions = vec![
            LeadershipPosition {
                position: Some("Patrol Leader".to_string()),
                start_date: Some("2024-06-01".to_string()),
                end_date: Some("2025-01-01".to_string()),
                days_served: Some(214),
                patrol: None,
                rank: None,
            },
            LeadershipPosition {
                position: Some("SPL".to_string()),
                start_date: Some("2025-01-01".to_string()),
                end_date: None,
                days_served: None,
                patrol: None,
                rank: None,
            },
            LeadershipPosition {
                position: Some("Scribe".to_string()),
                start_date: Some("2023-01-01".to_string()),
                end_date: Some("2023-06-01".to_string()),
                days_served: Some(151),
                patrol: None,
                rank: None,
            },
        ];
        LeadershipPosition::sort_for_display(&mut positions);
        // Current (SPL) first, then most recent ended
        assert_eq!(positions[0].position.as_deref(), Some("SPL"));
        assert_eq!(positions[1].position.as_deref(), Some("Patrol Leader"));
        assert_eq!(positions[2].position.as_deref(), Some("Scribe"));
    }

    #[test]
    fn test_award_sort_for_display() {
        let mut awards = vec![
            Award { name: Some("50-Miler".to_string()), awarded: Some(false), date_awarded: None, ..Default::default() },
            Award { name: Some("Eagle Palm".to_string()), awarded: Some(true), date_awarded: Some("2025-03-01".to_string()), ..Default::default() },
            Award { name: Some("Honor Medal".to_string()), awarded: Some(true), date_awarded: Some("2024-06-01".to_string()), ..Default::default() },
        ];
        Award::sort_for_display(&mut awards);
        // Awarded first (newer date first), then not awarded
        assert_eq!(awards[0].name.as_deref(), Some("Eagle Palm"));
        assert_eq!(awards[1].name.as_deref(), Some("Honor Medal"));
        assert_eq!(awards[2].name.as_deref(), Some("50-Miler"));
    }

    #[test]
    fn test_rank_status_display() {
        let awarded = RankProgress {
            rank_id: 1, version_id: None, rank_name: "Eagle".to_string(),
            date_completed: Some("2025-01-15".to_string()),
            date_awarded: Some("2025-02-01".to_string()),
            requirements_completed: None, requirements_total: None,
            percent_completed: None, level: Some(7),
        };
        let (cat, text) = awarded.status_display();
        assert_eq!(cat, StatusCategory::Awarded);
        assert!(text.contains("2025")); // formatted date

        let in_progress = RankProgress {
            rank_id: 2, version_id: None, rank_name: "Star".to_string(),
            date_completed: None, date_awarded: None,
            requirements_completed: None, requirements_total: None,
            percent_completed: Some(0.65), level: Some(5),
        };
        let (cat, text) = in_progress.status_display();
        assert_eq!(cat, StatusCategory::InProgress);
        assert_eq!(text, "65%");
    }

    #[test]
    fn test_badge_status_display() {
        let awarded = make_badge("Swimming", "Awarded", None, Some("2025-03-01"), false);
        let (cat, _) = awarded.status_display();
        assert_eq!(cat, StatusCategory::Awarded);

        let in_progress = make_badge("Cooking", "Started", Some(0.5), None, false);
        let (cat, text) = in_progress.status_display();
        assert_eq!(cat, StatusCategory::InProgress);
        assert_eq!(text, "50%");

        let not_started = make_badge("Hiking", "Started", Some(0.0), None, false);
        let (cat, text) = not_started.status_display();
        assert_eq!(cat, StatusCategory::InProgress);
        assert_eq!(text, "0%");
    }

    #[test]
    fn test_scout_rank_all_display_order() {
        let order = ScoutRank::all_display_order();
        assert_eq!(order.len(), 8);
        assert_eq!(order[0], ScoutRank::Eagle);
        assert_eq!(order[7], ScoutRank::Unknown);
    }

    #[test]
    fn test_cmp_by_progress_awarded_before_completed() {
        let awarded = make_badge("A", "Awarded", None, Some("2025-01-01"), false);
        let completed = make_badge("B", "Leader Approved", None, Some("2025-06-01"), false);
        // Awarded should come before merely completed
        assert_eq!(MeritBadgeProgress::cmp_by_progress(&awarded, &completed), Ordering::Less);
    }

    #[test]
    fn test_badge_sort_date() {
        let with_awarded = make_badge("A", "Awarded", None, Some("2025-01-01"), false);
        // sort_date should prefer awarded_date, but our make_badge doesn't set it
        assert_eq!(with_awarded.sort_date(), "2025-01-01");
    }
}
