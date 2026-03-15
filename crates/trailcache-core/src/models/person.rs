// Allow dead code: API response structs have fields for completeness
#![allow(dead_code)]

use std::cmp::Ordering;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, Utc, Datelike};

pub const PROGRAM_SCOUTS_BSA: &str = "Scouts BSA";
pub const PROGRAM_ID_SCOUTS_BSA: i32 = 2;
pub const UNIT_TYPE_ID_SCOUTS_BSA: i32 = 2;
pub const POSITION_SCOUT: &str = "Scout";
pub const POSITION_PATROL_LEADER: &str = "Patrol Leader";
pub const POSITION_ASST_PATROL_LEADER: &str = "Assistant Patrol Leader";
pub const DISPLAY_NOT_TRAINED: &str = "Not Trained";
pub const DEFAULT_ADULT_ROLE: &str = "Adult Leader";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonType {
    Youth,
    Adult,
    Parent,
}

impl std::fmt::Display for PersonType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersonType::Youth => write!(f, "Youth"),
            PersonType::Adult => write!(f, "Adult"),
            PersonType::Parent => write!(f, "Parent"),
        }
    }
}

// API Response wrappers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationInfo {
    #[serde(rename = "organizationFullName")]
    pub organization_full_name: Option<String>,
    #[serde(rename = "organizationGuid")]
    pub organization_guid: Option<String>,
    #[serde(rename = "organizationName")]
    pub organization_name: Option<String>,
    #[serde(rename = "unitType")]
    pub unit_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgYouthsResponse {
    #[serde(rename = "organizationInfo")]
    pub organization_info: Option<OrganizationInfo>,
    pub members: Vec<Youth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgAdultsResponse {
    #[serde(rename = "organizationInfo")]
    pub organization_info: Option<OrganizationInfo>,
    pub members: Vec<Adult>,
}

// Response from /units/{guid}/youths endpoint (has patrol & rank info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitYouthsResponse {
    pub id: Option<i64>,
    pub number: Option<String>,
    #[serde(rename = "unitType")]
    pub unit_type: Option<String>,
    #[serde(rename = "fullName")]
    pub full_name: Option<String>,
    pub users: Vec<UnitYouth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitYouth {
    #[serde(rename = "userId")]
    pub user_id: Option<i64>,
    #[serde(rename = "memberId")]
    pub member_id: Option<i64>,
    #[serde(rename = "personGuid")]
    pub person_guid: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "middleName")]
    pub middle_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: String,
    #[serde(rename = "nickName")]
    pub nick_name: Option<String>,
    #[serde(rename = "personFullName")]
    pub person_full_name: Option<String>,
    #[serde(rename = "dateOfBirth")]
    pub date_of_birth: Option<String>,
    pub age: Option<i32>,
    pub grade: Option<i32>,
    pub gender: Option<String>,
    // Contact info
    pub email: Option<String>,
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    #[serde(rename = "homePhone")]
    pub home_phone: Option<String>,
    #[serde(rename = "mobilePhone")]
    pub mobile_phone: Option<String>,
    // Positions (contains patrol info)
    #[serde(default)]
    pub positions: Vec<YouthPosition>,
    // Ranks
    #[serde(rename = "highestRanksAwarded", default)]
    pub highest_ranks_awarded: Vec<YouthRank>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouthPosition {
    #[serde(rename = "positionId")]
    pub position_id: Option<i64>,
    pub position: Option<String>,
    #[serde(rename = "patrolId")]
    pub patrol_id: Option<i64>,
    #[serde(rename = "patrolName")]
    pub patrol_name: Option<String>,
    #[serde(rename = "dateStarted")]
    pub date_started: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouthRank {
    pub id: Option<i64>,
    pub rank: Option<String>,
    pub level: Option<i32>,
    #[serde(rename = "programId")]
    pub program_id: Option<i32>,
    pub program: Option<String>,
    #[serde(rename = "unitTypeId")]
    pub unit_type_id: Option<i32>,
    #[serde(rename = "dateEarned")]
    pub date_earned: Option<String>,
    pub awarded: Option<bool>,
}

impl UnitYouth {
    /// Convert to Youth struct for compatibility with existing code
    pub fn to_youth(&self) -> Youth {
        // Find primary position (first one with a patrol)
        let primary_pos = self.positions.iter()
            .find(|p| p.patrol_name.is_some());

        // Find highest Scouts BSA rank (programId 2, unitTypeId 2)
        let scouts_bsa_rank = self.highest_ranks_awarded.iter()
            .filter(|r| r.program_id == Some(PROGRAM_ID_SCOUTS_BSA) && r.unit_type_id == Some(UNIT_TYPE_ID_SCOUTS_BSA))
            .max_by_key(|r| r.level);

        // Find position of responsibility (not "Scouts BSA" which is just member)
        let por = self.positions.iter()
            .find(|p| p.position.as_deref().map(|s| s != PROGRAM_SCOUTS_BSA).unwrap_or(false))
            .and_then(|p| p.position.clone());

        Youth {
            person_guid: self.person_guid.clone(),
            member_id: self.member_id.map(|id| id.to_string()),
            person_full_name: self.person_full_name.clone(),
            first_name: self.first_name.clone(),
            middle_name: self.middle_name.clone(),
            last_name: self.last_name.clone(),
            nick_name: self.nick_name.clone(),
            gender: self.gender.clone(),
            name_suffix: None,
            ethnicity: None,
            grade: self.grade,
            grade_id: None,
            position: por,
            position_id: None,
            program_id: Some(PROGRAM_ID_SCOUTS_BSA),
            program: Some(PROGRAM_SCOUTS_BSA.to_string()),
            registrar_info: Some(RegistrarInfo {
                date_of_birth: self.date_of_birth.clone(),
                registration_id: None,
                registration_status_id: None,
                registration_status: None,
                registration_effective_dt: None,
                registration_expire_dt: None,
                renewal_status: None,
                is_yearly_membership: None,
                is_manually_ended: None,
                is_auto_renewal_opted_out: None,
            }),
            primary_email_info: self.email.as_ref().map(|e| PrimaryEmailInfo {
                email_id: None,
                email_type: None,
                email_address: Some(e.clone()),
            }),
            primary_phone_info: self.mobile_phone.as_ref().or(self.home_phone.as_ref()).map(|phone| {
                // Parse phone into components if possible
                let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() == 10 {
                    PrimaryPhoneInfo {
                        phone_id: None,
                        phone_type: None,
                        phone_area_code: Some(digits[0..3].to_string()),
                        phone_prefix: Some(digits[3..6].to_string()),
                        phone_line_number: Some(digits[6..10].to_string()),
                    }
                } else {
                    PrimaryPhoneInfo {
                        phone_id: None,
                        phone_type: None,
                        phone_area_code: None,
                        phone_prefix: None,
                        phone_line_number: None,
                    }
                }
            }),
            primary_address_info: if self.address1.is_some() || self.city.is_some() {
                Some(PrimaryAddressInfo {
                    id: None,
                    address_type: None,
                    address1: self.address1.clone(),
                    address2: self.address2.clone(),
                    city: self.city.clone(),
                    state: self.state.clone(),
                    zip_code: self.zip.clone(),
                })
            } else {
                None
            },
            user_id: self.user_id,
            email: self.email.clone(),
            phone_number: self.mobile_phone.clone().or(self.home_phone.clone()),
            patrol_name: primary_pos.and_then(|p| p.patrol_name.clone()),
            patrol_guid: None,
            is_patrol_leader: None, // Could check for "Patrol Leader" position
            current_rank: scouts_bsa_rank.and_then(|r| r.rank.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryEmailInfo {
    #[serde(rename = "emailId")]
    pub email_id: Option<String>,
    #[serde(rename = "emailType")]
    pub email_type: Option<String>,
    #[serde(rename = "emailAddress")]
    pub email_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryPhoneInfo {
    #[serde(rename = "phoneId")]
    pub phone_id: Option<String>,
    #[serde(rename = "phoneType")]
    pub phone_type: Option<String>,
    #[serde(rename = "phoneAreaCode")]
    pub phone_area_code: Option<String>,
    #[serde(rename = "phonePrefix")]
    pub phone_prefix: Option<String>,
    #[serde(rename = "phoneLineNumber")]
    pub phone_line_number: Option<String>,
}

impl PrimaryPhoneInfo {
    pub fn formatted(&self) -> Option<String> {
        match (&self.phone_area_code, &self.phone_prefix, &self.phone_line_number) {
            (Some(area), Some(prefix), Some(line)) if !area.is_empty() && !prefix.is_empty() && !line.is_empty() => {
                Some(format!("({}) {}-{}", area, prefix, line))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryAddressInfo {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub address_type: Option<String>,
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "zipCode")]
    pub zip_code: Option<String>,
}

impl PrimaryAddressInfo {
    pub fn formatted(&self) -> Option<String> {
        let addr1 = self.address1.as_deref().unwrap_or("").trim();
        let city = self.city.as_deref().unwrap_or("");
        let state = self.state.as_deref().unwrap_or("");
        let zip = self.zip_code.as_deref().unwrap_or("");

        if addr1.is_empty() && city.is_empty() {
            return None;
        }

        Some(format!("{}, {}, {} {}", addr1, city, state, zip).trim().to_string())
    }

    pub fn city_state(&self) -> Option<String> {
        let city = self.city.as_deref().unwrap_or("");
        let state = self.state.as_deref().unwrap_or("");
        if city.is_empty() {
            return None;
        }
        Some(format!("{}, {}", city, state))
    }

    /// Format city, state, and zip as a single line.
    pub fn city_state_zip(&self) -> Option<String> {
        let cs = self.city_state()?;
        let zip = self.zip_code.as_deref().unwrap_or("");
        Some(format!("{} {}", cs, zip).trim().to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrarInfo {
    #[serde(rename = "dateOfBirth")]
    pub date_of_birth: Option<String>,
    #[serde(rename = "registrationId")]
    pub registration_id: Option<i64>,
    #[serde(rename = "registrationStatusId")]
    pub registration_status_id: Option<i32>,
    #[serde(rename = "registrationStatus")]
    pub registration_status: Option<String>,
    #[serde(rename = "registrationEffectiveDt")]
    pub registration_effective_dt: Option<String>,
    #[serde(rename = "registrationExpireDt")]
    pub registration_expire_dt: Option<String>,
    #[serde(rename = "renewalStatus")]
    pub renewal_status: Option<String>,
    #[serde(rename = "isYearlyMembership")]
    pub is_yearly_membership: Option<bool>,
    #[serde(rename = "isManuallyEnded")]
    pub is_manually_ended: Option<bool>,
    #[serde(rename = "isAutoRenewalOptedOut")]
    pub is_auto_renewal_opted_out: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Youth {
    #[serde(rename = "personGuid")]
    pub person_guid: Option<String>,
    #[serde(rename = "memberId")]
    pub member_id: Option<String>,
    #[serde(rename = "personFullName")]
    pub person_full_name: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "middleName")]
    pub middle_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: String,
    #[serde(rename = "nickName")]
    pub nick_name: Option<String>,
    pub gender: Option<String>,
    #[serde(rename = "nameSuffix")]
    pub name_suffix: Option<String>,
    pub ethnicity: Option<String>,
    pub grade: Option<i32>,
    #[serde(rename = "gradeId")]
    pub grade_id: Option<i32>,
    pub position: Option<String>,
    #[serde(rename = "positionId")]
    pub position_id: Option<i64>,
    #[serde(rename = "programId")]
    pub program_id: Option<i32>,
    pub program: Option<String>,
    #[serde(rename = "registrarInfo")]
    pub registrar_info: Option<RegistrarInfo>,
    #[serde(rename = "primaryEmailInfo")]
    pub primary_email_info: Option<PrimaryEmailInfo>,
    #[serde(rename = "primaryPhoneInfo")]
    pub primary_phone_info: Option<PrimaryPhoneInfo>,
    #[serde(rename = "primaryAddressInfo")]
    pub primary_address_info: Option<PrimaryAddressInfo>,
    // Legacy fields for compatibility
    #[serde(rename = "userId")]
    pub user_id: Option<i64>,
    pub email: Option<String>,
    #[serde(rename = "phoneNumber")]
    pub phone_number: Option<String>,
    #[serde(rename = "subUnitName")]
    pub patrol_name: Option<String>,
    #[serde(rename = "subUnitGuid")]
    pub patrol_guid: Option<String>,
    #[serde(rename = "isPatrolLeader")]
    pub is_patrol_leader: Option<bool>,
    #[serde(rename = "currentRankName")]
    pub current_rank: Option<String>,
}

impl Youth {
    pub fn full_name(&self) -> String {
        if let Some(ref full) = self.person_full_name {
            full.clone()
        } else {
            format!("{} {}", self.first_name, self.last_name)
        }
    }

    pub fn display_name(&self) -> String {
        let nick = self.nick_name.as_deref().filter(|n| !n.is_empty() && *n != self.first_name);
        match nick {
            Some(n) => format!("{}, {} ({})", self.last_name, self.first_name, n),
            None => format!("{}, {}", self.last_name, self.first_name),
        }
    }

    pub fn short_name(&self) -> String {
        let first = self.nick_name.as_deref()
            .filter(|n| !n.is_empty())
            .unwrap_or(&self.first_name);
        format!("{} {}", first, self.last_name)
    }

    pub fn get_user_id(&self) -> i64 {
        self.user_id.unwrap_or(0)
    }

    pub fn date_of_birth(&self) -> Option<NaiveDate> {
        self.registrar_info.as_ref()
            .and_then(|r| r.date_of_birth.as_ref())
            .and_then(|dob| NaiveDate::parse_from_str(dob, "%Y-%m-%d").ok())
    }

    pub fn age(&self) -> Option<i32> {
        self.date_of_birth().map(|dob| {
            let today = Utc::now().date_naive();
            let mut age = today.year() - dob.year();
            if (today.month(), today.day()) < (dob.month(), dob.day()) {
                age -= 1;
            }
            age
        })
    }

    pub fn age_str(&self) -> String {
        self.age().map(|a| a.to_string()).unwrap_or_else(|| "-".to_string())
    }

    pub fn grade_str(&self) -> String {
        self.grade.map(|g| g.to_string()).unwrap_or_else(|| "-".to_string())
    }

    pub fn phone(&self) -> Option<String> {
        self.primary_phone_info.as_ref()
            .and_then(|p| p.formatted())
            .or_else(|| self.phone_number.clone())
    }

    pub fn email(&self) -> Option<String> {
        self.primary_email_info.as_ref()
            .and_then(|e| e.email_address.clone())
            .filter(|e| !e.is_empty())
            .or_else(|| self.email.clone())
    }

    pub fn address(&self) -> Option<String> {
        self.primary_address_info.as_ref().and_then(|a| a.formatted())
    }

    pub fn city_state(&self) -> Option<String> {
        self.primary_address_info.as_ref().and_then(|a| a.city_state())
    }

    pub fn registration_status(&self) -> Option<String> {
        self.registrar_info.as_ref()
            .and_then(|r| r.registration_status.clone())
    }

    pub fn registration_expires(&self) -> Option<String> {
        self.registrar_info.as_ref()
            .and_then(|r| r.registration_expire_dt.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adult {
    #[serde(rename = "personGuid")]
    pub person_guid: Option<String>,
    #[serde(rename = "memberId")]
    pub member_id: Option<String>,
    #[serde(rename = "personFullName")]
    pub person_full_name: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "middleName")]
    pub middle_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: String,
    #[serde(rename = "nickName")]
    pub nick_name: Option<String>,
    pub position: Option<String>,
    #[serde(rename = "positionId")]
    pub position_id: Option<i64>,
    pub key3: Option<String>,
    #[serde(rename = "positionTrained")]
    pub position_trained: Option<String>,
    #[serde(rename = "yptStatus")]
    pub ypt_status: Option<String>,
    #[serde(rename = "yptCompletedDate")]
    pub ypt_completed_date: Option<String>,
    #[serde(rename = "yptExpiredDate")]
    pub ypt_expired_date: Option<String>,
    #[serde(rename = "registrarInfo")]
    pub registrar_info: Option<RegistrarInfo>,
    #[serde(rename = "primaryEmailInfo")]
    pub primary_email_info: Option<PrimaryEmailInfo>,
    #[serde(rename = "primaryPhoneInfo")]
    pub primary_phone_info: Option<PrimaryPhoneInfo>,
    #[serde(rename = "primaryAddressInfo")]
    pub primary_address_info: Option<PrimaryAddressInfo>,
    // Legacy fields
    #[serde(rename = "userId")]
    pub user_id: Option<i64>,
    pub email: Option<String>,
    #[serde(rename = "phoneNumber")]
    pub phone_number: Option<String>,
}

impl Adult {
    /// Classify position_trained field into a boolean.
    /// "Trained"/"Y"/"Yes"/"true" → Some(true), "Not Trained"/"N"/"No"/"false" → Some(false), else None
    pub fn is_position_trained(&self) -> Option<bool> {
        match self.position_trained.as_deref() {
            Some("Trained") | Some("Y") | Some("Yes") | Some("true") => Some(true),
            Some("Not Trained") | Some("N") | Some("No") | Some("false") => Some(false),
            _ => None,
        }
    }

    /// Display string for position training status.
    pub fn position_trained_display(&self) -> &str {
        match self.is_position_trained() {
            Some(true) => "Trained",
            Some(false) => DISPLAY_NOT_TRAINED,
            None => "-",
        }
    }

    /// Check if this adult matches a search query (case-insensitive).
    /// Query should already be lowercased.
    pub fn matches_search(&self, query_lowercase: &str) -> bool {
        self.first_name.to_lowercase().contains(query_lowercase)
            || self.last_name.to_lowercase().contains(query_lowercase)
            || self.position.as_ref().map(|s| s.to_lowercase().contains(query_lowercase)).unwrap_or(false)
            || self.email().as_ref().map(|s| s.to_lowercase().contains(query_lowercase)).unwrap_or(false)
    }

    pub fn full_name(&self) -> String {
        if let Some(ref full) = self.person_full_name {
            full.clone()
        } else {
            format!("{} {}", self.first_name, self.last_name)
        }
    }

    pub fn display_name(&self) -> String {
        format!("{}, {}", self.last_name, self.first_name)
    }

    pub fn display_name_full(&self) -> String {
        match &self.middle_name {
            Some(middle) if !middle.is_empty() => {
                format!("{}, {} {}", self.last_name, self.first_name, middle)
            }
            _ => format!("{}, {}", self.last_name, self.first_name)
        }
    }

    pub fn role(&self) -> String {
        self.position
            .clone()
            .unwrap_or_else(|| DEFAULT_ADULT_ROLE.to_string())
    }

    pub fn get_user_id(&self) -> i64 {
        self.user_id.unwrap_or(0)
    }

    pub fn phone(&self) -> Option<String> {
        self.primary_phone_info.as_ref()
            .and_then(|p| p.formatted())
            .or_else(|| self.phone_number.clone())
    }

    pub fn email(&self) -> Option<String> {
        self.primary_email_info.as_ref()
            .and_then(|e| e.email_address.clone())
            .filter(|e| !e.is_empty())
            .or_else(|| self.email.clone())
    }

    /// Deduplicate adults by person_guid, combining multiple positions.
    ///
    /// The BSA API returns duplicate entries for adults who hold multiple
    /// positions (e.g., both "Assistant Scoutmaster" and "Committee Member").
    /// This merges duplicates into single entries with combined position strings
    /// (e.g., "Assistant Scoutmaster, Committee Member").
    ///
    /// Adults without a person_guid are kept as separate entries.
    /// Result is sorted by last_name, first_name.
    pub fn deduplicate(adults: Vec<Adult>) -> Vec<Adult> {
        let mut by_guid: HashMap<String, Adult> = HashMap::new();
        let mut no_guid_counter: usize = 0;

        for adult in adults {
            let guid = adult.person_guid.clone().unwrap_or_default();
            if guid.is_empty() {
                by_guid.insert(format!("_no_guid_{}", no_guid_counter), adult);
                no_guid_counter += 1;
                continue;
            }

            if let Some(existing) = by_guid.get_mut(&guid) {
                if let Some(new_pos) = &adult.position {
                    if let Some(existing_pos) = &existing.position {
                        let existing_positions: Vec<&str> = existing_pos
                            .split(", ")
                            .map(|s| s.trim())
                            .collect();
                        if !existing_positions.contains(&new_pos.as_str()) {
                            existing.position = Some(format!("{}, {}", existing_pos, new_pos));
                        }
                    } else {
                        existing.position = Some(new_pos.clone());
                    }
                }
            } else {
                by_guid.insert(guid, adult);
            }
        }

        let mut result: Vec<Adult> = by_guid.into_values().collect();
        result.sort_by(|a, b| a.last_name.cmp(&b.last_name).then(a.first_name.cmp(&b.first_name)));
        result
    }
}

// API response format for parents endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentResponse {
    #[serde(rename = "youthUserId")]
    pub youth_user_id: i64,
    #[serde(rename = "parentUserId")]
    pub parent_user_id: i64,
    #[serde(rename = "parentInformation")]
    pub parent_information: ParentInformation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentInformation {
    #[serde(rename = "memberId")]
    pub member_id: Option<i64>,
    #[serde(rename = "personGuid")]
    pub person_guid: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "middleName")]
    pub middle_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: String,
    #[serde(rename = "nickName")]
    pub nick_name: Option<String>,
    #[serde(rename = "personFullName")]
    pub person_full_name: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "homePhone")]
    pub home_phone: Option<String>,
    #[serde(rename = "mobilePhone")]
    pub mobile_phone: Option<String>,
    #[serde(rename = "workPhone")]
    pub work_phone: Option<String>,
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
}

impl ParentResponse {
    pub fn to_parent(&self) -> Parent {
        let info = &self.parent_information;
        Parent {
            user_id: Some(self.parent_user_id),
            person_guid: info.person_guid.clone(),
            first_name: info.first_name.clone(),
            last_name: info.last_name.clone(),
            email: info.email.clone(),
            mobile_phone: info.mobile_phone.clone(),
            home_phone: info.home_phone.clone(),
            address1: info.address1.clone(),
            address2: info.address2.clone(),
            city: info.city.clone(),
            state: info.state.clone(),
            zip: info.zip.clone(),
            youth_user_id: Some(self.youth_user_id),
            youth_first_name: None,
            youth_last_name: None,
            relationship: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parent {
    #[serde(rename = "userId")]
    pub user_id: Option<i64>,
    #[serde(rename = "personGuid")]
    pub person_guid: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    pub email: Option<String>,
    #[serde(rename = "mobilePhone", alias = "phoneNumber")]
    pub mobile_phone: Option<String>,
    #[serde(rename = "homePhone", default)]
    pub home_phone: Option<String>,
    #[serde(default)]
    pub address1: Option<String>,
    #[serde(default)]
    pub address2: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub zip: Option<String>,
    #[serde(rename = "youthUserId")]
    pub youth_user_id: Option<i64>,
    #[serde(rename = "youthFirstName")]
    pub youth_first_name: Option<String>,
    #[serde(rename = "youthLastName")]
    pub youth_last_name: Option<String>,
    pub relationship: Option<String>,
}

impl Parent {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn display_name(&self) -> String {
        format!("{}, {}", self.last_name, self.first_name)
    }

    pub fn phone(&self) -> Option<String> {
        self.mobile_phone.as_ref()
            .or(self.home_phone.as_ref())
            .map(|p| crate::utils::format_phone(p))
    }

    pub fn address_line(&self) -> Option<String> {
        let addr = self.address1.as_deref().filter(|a| !a.trim().is_empty())?;
        let city = self.city.as_deref().unwrap_or("");
        let state = self.state.as_deref().unwrap_or("");
        let zip = self.zip.as_deref().unwrap_or("");
        Some(format!("{}, {}, {} {}", addr, city, state, zip))
    }

    /// Format city, state, and zip as a single line.
    pub fn city_state_zip(&self) -> Option<String> {
        let city = self.city.as_deref().unwrap_or("");
        let state = self.state.as_deref().unwrap_or("");
        let zip = self.zip.as_deref().unwrap_or("");
        if city.is_empty() && state.is_empty() {
            return None;
        }
        let cs = if !city.is_empty() {
            format!("{}, {}", city, state)
        } else {
            state.to_string()
        };
        let line = format!("{} {}", cs, zip).trim().to_string();
        if line == "," || line.is_empty() { None } else { Some(line) }
    }

    pub fn youth_name(&self) -> Option<String> {
        match (&self.youth_first_name, &self.youth_last_name) {
            (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
            _ => None,
        }
    }
}

/// Priority order for youth positions of responsibility.
pub const YOUTH_POSITION_PRIORITY: &[&str] = &[
    "Senior Patrol Leader",
    "Assistant Senior Patrol Leader",
    "Troop Guide",
    "Patrol Leader",
    "Assistant Patrol Leader",
    "Quartermaster",
    "Scribe",
    "Historian",
    "Librarian",
    "Chaplain Aide",
    "Outdoor Ethics Guide",
    "Den Chief",
    "Instructor",
    "Junior Assistant Scoutmaster",
    "Bugler",
];

// Sorting options for scouts table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoutSortColumn {
    Name,
    Patrol,
    Rank,
    Grade,
    Age,
}

impl ScoutSortColumn {
    pub fn next(&self) -> Self {
        match self {
            ScoutSortColumn::Name => ScoutSortColumn::Patrol,
            ScoutSortColumn::Patrol => ScoutSortColumn::Rank,
            ScoutSortColumn::Rank => ScoutSortColumn::Grade,
            ScoutSortColumn::Grade => ScoutSortColumn::Age,
            ScoutSortColumn::Age => ScoutSortColumn::Name,
        }
    }
}

impl Youth {
    pub fn patrol(&self) -> String {
        self.patrol_name.clone().unwrap_or_else(|| "-".to_string())
    }

    pub fn rank(&self) -> String {
        self.current_rank.clone().unwrap_or_else(|| "Crossover".to_string())
    }

    pub fn rank_short(&self) -> String {
        use super::advancement::ScoutRank;
        ScoutRank::parse(self.current_rank.as_deref()).abbreviation().to_string()
    }

    pub fn position_display(&self) -> Option<String> {
        self.position.clone().filter(|p| !p.is_empty() && p != POSITION_SCOUT)
    }

    /// Sort key based on YOUTH_POSITION_PRIORITY. Returns 999 for unknown/none.
    pub fn position_sort_key(&self) -> usize {
        match self.position.as_deref() {
            Some(pos) if !pos.is_empty() && pos != POSITION_SCOUT && pos != PROGRAM_SCOUTS_BSA => {
                YOUTH_POSITION_PRIORITY.iter().position(|&p| p == pos).unwrap_or(999)
            }
            _ => 999,
        }
    }

    /// Display position with patrol name for PL/APL.
    /// Returns None for "Scout", "Scouts BSA", or empty positions.
    pub fn position_display_with_patrol(&self) -> Option<String> {
        let pos = self.position.as_deref()?;
        if pos.is_empty() || pos == POSITION_SCOUT || pos == PROGRAM_SCOUTS_BSA {
            return None;
        }
        if (pos == POSITION_PATROL_LEADER || pos == POSITION_ASST_PATROL_LEADER)
            && self.patrol_name.as_ref().map(|p| !p.is_empty()).unwrap_or(false)
        {
            Some(format!("{} ({})", pos, self.patrol_name.as_ref().unwrap()))
        } else {
            Some(pos.to_string())
        }
    }

    /// Check if this youth matches a search query (case-insensitive).
    /// Query should already be lowercased.
    pub fn matches_search(&self, query_lowercase: &str) -> bool {
        self.first_name.to_lowercase().contains(query_lowercase)
            || self.last_name.to_lowercase().contains(query_lowercase)
            || self.patrol_name.as_ref().map(|s| s.to_lowercase().contains(query_lowercase)).unwrap_or(false)
            || self.current_rank.as_ref().map(|s| s.to_lowercase().contains(query_lowercase)).unwrap_or(false)
            || self.email().as_ref().map(|s| s.to_lowercase().contains(query_lowercase)).unwrap_or(false)
    }

    /// Compare two youth by the given column, with name as tiebreaker.
    pub fn cmp_by_column(a: &Youth, b: &Youth, column: ScoutSortColumn) -> Ordering {
        use crate::utils::cmp_ignore_case;
        use super::advancement::ScoutRank;

        let name_cmp = || {
            cmp_ignore_case(&a.last_name, &b.last_name)
                .then_with(|| cmp_ignore_case(&a.first_name, &b.first_name))
        };

        match column {
            ScoutSortColumn::Name => name_cmp(),
            ScoutSortColumn::Rank => {
                let rank_a = ScoutRank::parse(a.current_rank.as_deref());
                let rank_b = ScoutRank::parse(b.current_rank.as_deref());
                // Reversed so ascending shows highest rank first
                rank_b.cmp(&rank_a).then_with(name_cmp)
            }
            ScoutSortColumn::Grade => a.grade.cmp(&b.grade).then_with(name_cmp),
            ScoutSortColumn::Age => a.age().cmp(&b.age()).then_with(name_cmp),
            ScoutSortColumn::Patrol => {
                cmp_ignore_case(
                    a.patrol_name.as_deref().unwrap_or(""),
                    b.patrol_name.as_deref().unwrap_or(""),
                )
                .then_with(name_cmp)
            }
        }
    }
}

/// Build a sorted list of youth positions of responsibility.
/// Filters out youth without positions, sorts by priority order.
/// Returns (position_display, holder_display_name) pairs.
pub fn youth_position_list(youth: &[Youth]) -> Vec<(String, String)> {
    let mut positions: Vec<(usize, String, String)> = Vec::new();
    for y in youth {
        if let Some(display_pos) = y.position_display_with_patrol() {
            positions.push((y.position_sort_key(), display_pos, y.display_name()));
        }
    }
    positions.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    positions.into_iter().map(|(_, display, name)| (display, name)).collect()
}

/// Sorting options for adult table columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdultSortColumn {
    Name,
    Role,
    Email,
}

impl Adult {
    /// Compare two adults by the given column, with name as tiebreaker.
    pub fn cmp_by_column(a: &Adult, b: &Adult, column: AdultSortColumn) -> Ordering {
        use crate::utils::cmp_ignore_case;
        let name_cmp = || {
            cmp_ignore_case(&a.last_name, &b.last_name)
                .then_with(|| cmp_ignore_case(&a.first_name, &b.first_name))
        };
        match column {
            AdultSortColumn::Name => name_cmp(),
            AdultSortColumn::Role => {
                cmp_ignore_case(&a.role(), &b.role()).then_with(name_cmp)
            }
            AdultSortColumn::Email => {
                let ea = a.email().unwrap_or_default();
                let eb = b.email().unwrap_or_default();
                cmp_ignore_case(&ea, &eb).then_with(name_cmp)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn make_adult(position_trained: Option<&str>) -> Adult {
        Adult {
            person_guid: None, member_id: None, person_full_name: None,
            first_name: "Jane".to_string(), middle_name: None, last_name: "Doe".to_string(),
            nick_name: None, position: Some("Scoutmaster".to_string()), position_id: None,
            key3: None, position_trained: position_trained.map(|s| s.to_string()),
            ypt_status: None, ypt_completed_date: None, ypt_expired_date: None,
            registrar_info: None, primary_email_info: None, primary_phone_info: None,
            primary_address_info: None, user_id: None, email: None, phone_number: None,
        }
    }

    fn make_youth(position: Option<&str>, patrol: Option<&str>) -> Youth {
        Youth {
            person_guid: None, member_id: None, person_full_name: None,
            first_name: "John".to_string(), middle_name: None, last_name: "Smith".to_string(),
            nick_name: None, gender: None, name_suffix: None, ethnicity: None,
            grade: None, grade_id: None, position: position.map(|s| s.to_string()),
            position_id: None, program_id: None, program: None, registrar_info: None,
            primary_email_info: None, primary_phone_info: None, primary_address_info: None,
            user_id: None, email: None, phone_number: None,
            patrol_name: patrol.map(|s| s.to_string()), patrol_guid: None,
            is_patrol_leader: None, current_rank: Some("First Class".to_string()),
        }
    }

    #[test]
    fn test_is_position_trained() {
        assert_eq!(make_adult(Some("Trained")).is_position_trained(), Some(true));
        assert_eq!(make_adult(Some("Y")).is_position_trained(), Some(true));
        assert_eq!(make_adult(Some("Yes")).is_position_trained(), Some(true));
        assert_eq!(make_adult(Some("Not Trained")).is_position_trained(), Some(false));
        assert_eq!(make_adult(Some("N")).is_position_trained(), Some(false));
        assert_eq!(make_adult(None).is_position_trained(), None);
        assert_eq!(make_adult(Some("unknown")).is_position_trained(), None);
    }

    #[test]
    fn test_position_trained_display() {
        assert_eq!(make_adult(Some("Trained")).position_trained_display(), "Trained");
        assert_eq!(make_adult(Some("Y")).position_trained_display(), "Trained");
        assert_eq!(make_adult(Some("Not Trained")).position_trained_display(), "Not Trained");
        assert_eq!(make_adult(None).position_trained_display(), "-");
    }

    #[test]
    fn test_position_sort_key() {
        let spl = make_youth(Some("Senior Patrol Leader"), None);
        let pl = make_youth(Some("Patrol Leader"), None);
        let scout = make_youth(Some("Scout"), None);
        let none = make_youth(None, None);

        assert_eq!(spl.position_sort_key(), 0);
        assert_eq!(pl.position_sort_key(), 3);
        assert_eq!(scout.position_sort_key(), 999);
        assert_eq!(none.position_sort_key(), 999);
    }

    #[test]
    fn test_position_display_with_patrol() {
        let pl = make_youth(Some("Patrol Leader"), Some("Eagle"));
        assert_eq!(pl.position_display_with_patrol(), Some("Patrol Leader (Eagle)".to_string()));

        let spl = make_youth(Some("Senior Patrol Leader"), Some("Eagle"));
        assert_eq!(spl.position_display_with_patrol(), Some("Senior Patrol Leader".to_string()));

        let scout = make_youth(Some("Scout"), None);
        assert_eq!(scout.position_display_with_patrol(), None);

        let none = make_youth(None, None);
        assert_eq!(none.position_display_with_patrol(), None);
    }

    #[test]
    fn test_youth_matches_search() {
        let youth = make_youth(Some("Patrol Leader"), Some("Eagle"));
        assert!(youth.matches_search("john"));
        assert!(youth.matches_search("smith"));
        assert!(youth.matches_search("eagle"));
        assert!(youth.matches_search("first class"));
        assert!(!youth.matches_search("bob"));
    }

    #[test]
    fn test_adult_matches_search() {
        let adult = make_adult(Some("Trained"));
        assert!(adult.matches_search("jane"));
        assert!(adult.matches_search("doe"));
        assert!(adult.matches_search("scoutmaster"));
        assert!(!adult.matches_search("bob"));
    }
}
