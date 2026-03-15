use std::cmp::Ordering;

use chrono::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsvpStatus {
    Going,
    NotGoing,
    NoResponse,
}

impl std::fmt::Display for RsvpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RsvpStatus::Going => write!(f, "Going"),
            RsvpStatus::NotGoing => write!(f, "Not Going"),
            RsvpStatus::NoResponse => write!(f, "No Response"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub location: Option<String>,
    #[serde(rename = "eventType")]
    pub event_type: Option<String>,
    #[serde(default)]
    pub rsvp: bool,
    #[serde(rename = "slipsRequired", default)]
    pub slips_required: bool,
    // POST /events returns "invitedUsers", GET /events/{id} returns "users"
    #[serde(rename = "invitedUsers", alias = "users", default)]
    pub invited_users: Vec<InvitedUser>,
    #[serde(default)]
    pub units: Vec<EventUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventUnit {
    #[serde(rename = "unitId")]
    pub unit_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitedUser {
    #[serde(rename = "userId")]
    pub user_id: i64,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    pub rsvp: Option<String>,
    #[serde(rename = "rsvpCode", default)]
    pub rsvp_code: Option<String>,
    #[serde(default)]
    pub attended: bool,
    #[serde(rename = "isAdult", default)]
    pub is_adult: bool,
}

impl InvitedUser {
    pub fn status(&self) -> RsvpStatus {
        // Check rsvpCode first (Y/N), then fall back to rsvp field
        if let Some(code) = &self.rsvp_code {
            if code.eq_ignore_ascii_case("y") || code.eq_ignore_ascii_case("yes") {
                return RsvpStatus::Going;
            }
            if code.eq_ignore_ascii_case("n") || code.eq_ignore_ascii_case("no") {
                return RsvpStatus::NotGoing;
            }
        }
        // Fall back to rsvp field
        if let Some(rsvp) = &self.rsvp {
            let rsvp_lower = rsvp.to_ascii_lowercase();
            if rsvp_lower == "going" || rsvp_lower == "yes" {
                return RsvpStatus::Going;
            }
            if rsvp_lower == "not going" || rsvp_lower == "not_going" || rsvp_lower == "no" {
                return RsvpStatus::NotGoing;
            }
        }
        RsvpStatus::NoResponse
    }

    pub fn display_name(&self) -> String {
        format!("{}, {}", self.last_name, self.first_name)
    }
}

#[allow(dead_code)] // Helper methods - some used, others for future use
impl Event {
    /// For compatibility with code that expects event_id
    pub fn event_id(&self) -> i64 {
        self.id
    }

    /// Get the unit ID from the first unit in the units array
    pub fn unit_id(&self) -> Option<i64> {
        self.units.first().map(|u| u.unit_id)
    }

    pub fn formatted_date(&self) -> String {
        match &self.start_date {
            Some(date) => {
                // Try to parse and format the date nicely
                if let Ok(dt) = DateTime::parse_from_rfc3339(date) {
                    dt.format("%b %d, %Y").to_string()
                } else {
                    // Fall back to raw date string, truncate if too long
                    date.chars().take(10).collect()
                }
            }
            None => "TBD".to_string(),
        }
    }

    pub fn formatted_time(&self) -> Option<String> {
        self.start_date.as_ref().and_then(|date| {
            if let Ok(dt) = DateTime::parse_from_rfc3339(date) {
                Some(dt.format("%H:%M").to_string())
            } else {
                None
            }
        })
    }

    /// Compact date/time for list view: "Jan 26 5:00p"
    pub fn formatted_datetime_short(&self) -> String {
        match &self.start_date {
            Some(date) => {
                if let Ok(dt) = DateTime::parse_from_rfc3339(date) {
                    let hour = dt.format("%I").to_string().trim_start_matches('0').to_string();
                    let minute = dt.format("%M").to_string();
                    let ampm = dt.format("%p").to_string().to_lowercase().chars().next().unwrap_or('a');
                    if minute == "00" {
                        dt.format(&format!("%b %d {}{}",  hour, ampm)).to_string()
                    } else {
                        dt.format(&format!("%b %d {}:{}{}",  hour, minute, ampm)).to_string()
                    }
                } else {
                    date.chars().take(10).collect()
                }
            }
            None => "TBD".to_string(),
        }
    }

    /// Standard date/time format: "MM/DD/YYYY HH:mm"
    pub fn formatted_datetime_standard(&self) -> String {
        match &self.start_date {
            Some(date) => {
                if let Ok(dt) = DateTime::parse_from_rfc3339(date) {
                    dt.format("%m/%d/%Y %H:%M").to_string()
                } else {
                    date.chars().take(16).collect()
                }
            }
            None => "TBD".to_string(),
        }
    }

    /// Formatted start datetime: "Feb 06, 2026 @ 07:00 PM"
    pub fn formatted_start_datetime(&self) -> String {
        Self::format_datetime_nice(&self.start_date)
    }

    /// Formatted end datetime: "Feb 08, 2026 @ 10:00 AM"
    pub fn formatted_end_datetime(&self) -> String {
        Self::format_datetime_nice(&self.end_date)
    }

    fn format_datetime_nice(date_opt: &Option<String>) -> String {
        match date_opt {
            Some(date) => {
                if let Ok(dt) = DateTime::parse_from_rfc3339(date) {
                    dt.format("%b %d, %Y @ %I:%M %p").to_string()
                } else {
                    date.chars().take(16).collect()
                }
            }
            None => "TBD".to_string(),
        }
    }

    /// Short event type for list view
    pub fn event_type_short(&self) -> &str {
        match self.event_type.as_deref() {
            Some("Troop Meeting") => "Mtg",
            Some("Camping") => "Camp",
            Some("Hiking") => "Hike",
            Some("Service") => "Svc",
            Some("Other") => "Other",
            Some(t) if t.len() <= 5 => t,
            Some(t) => &t[..5],
            None => "-",
        }
    }

    /// Derive a meaningful event type from available fields
    /// The API eventType is often just "Other", so we infer from other fields
    pub fn derived_type(&self) -> &str {
        // First check if eventType is something other than "Other"
        if let Some(ref et) = self.event_type {
            if et != "Other" && !et.is_empty() {
                return et;
            }
        }

        // Infer from name
        let name_lower = self.name.to_lowercase();
        if name_lower.contains("meeting") || name_lower.contains("mtg") {
            return "Meeting";
        }
        if name_lower.contains("camp") {
            return "Camping";
        }
        if name_lower.contains("hike") || name_lower.contains("hiking") {
            return "Hike";
        }
        if name_lower.contains("ski") {
            return "Outdoor";
        }
        if name_lower.contains("service") {
            return "Service";
        }

        // Fall back to "Other"
        "Other"
    }

    pub fn going_count(&self) -> i32 {
        self.invited_users.iter()
            .filter(|u| matches!(u.status(), RsvpStatus::Going))
            .count() as i32
    }

    pub fn not_going_count(&self) -> i32 {
        self.invited_users.iter()
            .filter(|u| matches!(u.status(), RsvpStatus::NotGoing))
            .count() as i32
    }

    pub fn no_response_count(&self) -> i32 {
        self.invited_users.iter()
            .filter(|u| matches!(u.status(), RsvpStatus::NoResponse))
            .count() as i32
    }

    /// Adult RSVP counts: (going, not_going)
    pub fn adult_rsvp_counts(&self) -> (i32, i32) {
        let going = self.invited_users.iter()
            .filter(|u| u.is_adult && matches!(u.status(), RsvpStatus::Going))
            .count() as i32;
        let not_going = self.invited_users.iter()
            .filter(|u| u.is_adult && matches!(u.status(), RsvpStatus::NotGoing))
            .count() as i32;
        (going, not_going)
    }

    /// Scout RSVP counts: (going, not_going)
    pub fn scout_rsvp_counts(&self) -> (i32, i32) {
        let going = self.invited_users.iter()
            .filter(|u| !u.is_adult && matches!(u.status(), RsvpStatus::Going))
            .count() as i32;
        let not_going = self.invited_users.iter()
            .filter(|u| !u.is_adult && matches!(u.status(), RsvpStatus::NotGoing))
            .count() as i32;
        (going, not_going)
    }

    /// Check if this event matches a search query (case-insensitive).
    /// Query should already be lowercased.
    pub fn matches_search(&self, query_lowercase: &str) -> bool {
        self.name.to_lowercase().contains(query_lowercase)
            || self.location.as_ref().map(|s| s.to_lowercase().contains(query_lowercase)).unwrap_or(false)
            || self.derived_type().to_lowercase().contains(query_lowercase)
    }

    /// Invited users who have responded (Going or Not Going), split by adult/youth.
    pub fn respondents(&self) -> (Vec<&InvitedUser>, Vec<&InvitedUser>) {
        let (mut adults, mut scouts) = (vec![], vec![]);
        for u in &self.invited_users {
            if matches!(u.status(), RsvpStatus::Going | RsvpStatus::NotGoing) {
                if u.is_adult { adults.push(u); } else { scouts.push(u); }
            }
        }
        (adults, scouts)
    }

    pub fn rsvp_summary(&self) -> String {
        format!("Going: {} | Not: {} | ??: {}",
            self.going_count(),
            self.not_going_count(),
            self.no_response_count())
    }

    /// Compare two events by the given column, with name as tiebreaker.
    pub fn cmp_by_column(a: &Event, b: &Event, column: EventSortColumn) -> Ordering {
        use crate::utils::cmp_ignore_case;

        let name_cmp = || cmp_ignore_case(&a.name, &b.name);

        match column {
            EventSortColumn::Name => name_cmp(),
            EventSortColumn::Date => {
                let date_a = a.start_date.as_deref().unwrap_or("");
                let date_b = b.start_date.as_deref().unwrap_or("");
                date_a.cmp(date_b).then_with(name_cmp)
            }
            EventSortColumn::Location => {
                cmp_ignore_case(
                    a.location.as_deref().unwrap_or(""),
                    b.location.as_deref().unwrap_or(""),
                )
                .then_with(name_cmp)
            }
            EventSortColumn::Type => {
                cmp_ignore_case(a.derived_type(), b.derived_type())
                    .then_with(name_cmp)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventGuest {
    #[serde(rename = "userId")]
    pub user_id: i64,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    #[serde(rename = "rsvpStatus")]
    pub rsvp_status: Option<String>,
    #[serde(rename = "isYouth")]
    pub is_youth: Option<bool>,
}

#[allow(dead_code)] // Helper methods for future guest display improvements
impl EventGuest {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn display_name(&self) -> String {
        format!("{}, {}", self.last_name, self.first_name)
    }

    pub fn status(&self) -> RsvpStatus {
        if let Some(status) = &self.rsvp_status {
            let status_lower = status.to_ascii_lowercase();
            if status_lower == "going" || status_lower == "yes" {
                return RsvpStatus::Going;
            }
            if status_lower == "not going" || status_lower == "not_going" || status_lower == "no" {
                return RsvpStatus::NotGoing;
            }
        }
        RsvpStatus::NoResponse
    }
}

// Sorting options for events table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventSortColumn {
    Name,
    #[default]
    Date,
    Location,
    Type,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_matches_search() {
        let event = Event {
            id: 1,
            name: "Summer Camp".to_string(),
            description: None,
            start_date: None,
            end_date: None,
            location: Some("Camp Parsons".to_string()),
            event_type: Some("Camping".to_string()),
            rsvp: false,
            slips_required: false,
            invited_users: vec![],
            units: vec![],
        };
        assert!(event.matches_search("summer"));
        assert!(event.matches_search("parsons"));
        assert!(event.matches_search("camping"));
        assert!(!event.matches_search("hiking"));
    }
}
