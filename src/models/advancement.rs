// Allow dead code: API response structs have fields for completeness
#![allow(dead_code)]

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

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
            crate::app::ScoutRank::from_str(Some(&self.rank_name)).order() as i32
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
        self.percent_completed.map(|p| (p * 100.0) as i32)
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
    pub fn is_completed(&self) -> bool {
        self.completed
            || self.date_completed.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            || matches!(self.status.as_deref(), Some("Leader Approved") | Some("Awarded") | Some("Counselor Approved"))
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
    pub fn is_completed(&self) -> bool {
        self.completed.unwrap_or(false)
            || self.date_completed.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            || matches!(self.status.as_deref(), Some("Leader Approved") | Some("Awarded"))
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
            Some("Awarded") | Some("Leader Approved")
        )
    }

    pub fn is_awarded(&self) -> bool {
        self.status.as_deref() == Some("Awarded")
    }

    pub fn progress_percent(&self) -> Option<i32> {
        self.percent_completed.map(|p| (p * 100.0) as i32)
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
    /// Returns the award name, or "Unknown Award" if not set
    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("Unknown Award")
    }

    /// Returns true if this award has been awarded (status is "Awarded", awarded=true, or has awarded date)
    pub fn is_awarded(&self) -> bool {
        self.awarded == Some(true)
            || self.status.as_deref() == Some("Awarded")
            || self.date_awarded.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
    }

    /// Returns true if this award is completed (status is "Awarded" or "Leader Approved", or has leader approved date)
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status.as_deref(),
            Some("Awarded") | Some("Leader Approved")
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
        "?".to_string()
    }

    /// Get the award type for display
    pub fn type_display(&self) -> &str {
        self.award_type.as_deref().unwrap_or("")
    }

    /// Get progress percentage if available
    pub fn progress_percent(&self) -> Option<i32> {
        self.percent_completed.map(|p| (p * 100.0) as i32)
    }
}

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
        None => "?".to_string(),
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
}
