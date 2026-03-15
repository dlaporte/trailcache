//! Domain models for unit/troop information.
//!
//! These types represent unit data in a clean domain format,
//! decoupled from the API response structures.

use serde::{Deserialize, Serialize};

/// Key 3 leadership positions for a unit.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Key3Leaders {
    pub scoutmaster: Option<Leader>,
    pub committee_chair: Option<Leader>,
    pub charter_org_rep: Option<Leader>,
}

/// A leader with name information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Leader {
    pub first_name: String,
    pub last_name: String,
}

impl Leader {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

/// Unit registration and contact information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnitInfo {
    pub name: Option<String>,
    pub website: Option<String>,
    pub registration_url: Option<String>,
    pub district_name: Option<String>,
    pub council_name: Option<String>,
    pub charter_org_name: Option<String>,
    pub charter_expiry: Option<String>,
    pub meeting_location: Option<MeetingLocation>,
    pub contacts: Vec<UnitContact>,
    /// Pre-computed charter status display text, e.g. "Expires Mar 15, 2026".
    #[serde(default)]
    pub charter_status_display: Option<String>,
    /// Pre-computed flag: true if charter is expired.
    #[serde(default)]
    pub charter_expired: Option<bool>,
}

impl UnitInfo {
    /// Populate computed charter fields from `charter_expiry`.
    pub fn with_computed_fields(mut self) -> Self {
        if let Some(ref expiry) = self.charter_expiry {
            if let Some((status, formatted)) = crate::utils::check_expiration(expiry) {
                self.charter_status_display = Some(status.format_expiry(&formatted));
                self.charter_expired = Some(matches!(status, crate::utils::ExpirationStatus::Expired));
            }
        }
        self
    }
}

/// Meeting location address.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeetingLocation {
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
}

impl MeetingLocation {
    /// Format the address as a single line.
    pub fn formatted(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(ref line1) = self.address_line1 {
            if !line1.is_empty() {
                parts.push(line1.clone());
            }
        }
        if let Some(ref city) = self.city {
            if !city.is_empty() {
                let city_state = match &self.state {
                    Some(state) if !state.is_empty() => format!("{}, {}", city, state),
                    _ => city.clone(),
                };
                parts.push(city_state);
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }
}

/// A unit contact person.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnitContact {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

impl UnitContact {
    pub fn full_name(&self) -> String {
        format!(
            "{} {}",
            self.first_name.as_deref().unwrap_or(""),
            self.last_name.as_deref().unwrap_or("")
        )
        .trim()
        .to_string()
    }
}

/// Organization profile information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrgProfile {
    pub name: Option<String>,
    pub full_name: Option<String>,
    pub charter_org_name: Option<String>,
    pub charter_exp_date: Option<String>,
    pub charter_status: Option<String>,
}

/// A commissioner assigned to the unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Commissioner {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub position: Option<String>,
}

impl Commissioner {
    pub fn full_name(&self) -> String {
        format!(
            "{} {}",
            self.first_name.as_deref().unwrap_or(""),
            self.last_name.as_deref().unwrap_or("")
        )
        .trim()
        .to_string()
    }

    #[allow(dead_code)] // Used in tests
    pub fn position_display(&self) -> &str {
        self.position.as_deref().unwrap_or("Unknown")
    }
}
