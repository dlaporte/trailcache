//! Domain models for unit/troop information.
//!
//! These types represent unit data in a clean domain format,
//! decoupled from the API response structures.

use serde::{Deserialize, Serialize};

/// Key 3 leadership positions for a unit.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Key3Leaders {
    pub scoutmaster: Option<Leader>,
    pub committee_chair: Option<Leader>,
    pub charter_org_rep: Option<Leader>,
}

/// A leader with name information.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// Meeting location address.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
pub struct OrgProfile {
    pub name: Option<String>,
    pub full_name: Option<String>,
    pub charter_org_name: Option<String>,
    pub charter_exp_date: Option<String>,
    pub charter_status: Option<String>,
}

/// A commissioner assigned to the unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
