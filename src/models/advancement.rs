// Allow dead code: API response structs have fields for completeness
#![allow(dead_code)]

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
#[derive(Debug, Clone, Deserialize)]
pub struct MeritBadgeWithRequirements {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub requirements: Vec<MeritBadgeRequirement>,
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
