//! API client for communicating with the Scouting.org REST API.
//!
//! This module provides the `ApiClient` struct for making authenticated
//! API requests to fetch scout, event, and advancement data.

use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::{header, Client};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::{debug, warn};

use crate::auth::SessionData;
use crate::models::{
    Adult, AdvancementDashboard, Event, EventGuest, MeritBadgeProgress, MeritBadgeRequirement,
    MeritBadgeWithRequirements, OrgAdultsResponse, OrgYouthsResponse, Parent, ParentResponse,
    Patrol, RankProgress, RankRequirement, RankWithRequirements, RanksResponse, ReadyToAward,
    UnitYouthsResponse, Youth,
    // Domain types for unit info
    Commissioner, Key3Leaders, Leader, MeetingLocation, OrgProfile, UnitContact, UnitInfo,
};

use super::ApiError;

// ============================================================================
// Constants
// ============================================================================

/// Base URL for authentication endpoints (my.scouting.org handles login)
const AUTH_BASE_URL: &str = "https://my.scouting.org/api";

/// Base URL for main API endpoints (api.scouting.org handles data)
const API_BASE_URL: &str = "https://api.scouting.org";

/// HTTP request timeout in seconds.
/// 30s allows for slow API responses while failing fast enough for good UX.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Number of days to look back for events.
/// 30 days captures recent events without overwhelming the list.
const EVENT_LOOKBACK_DAYS: i64 = 30;

/// Number of days to look ahead for events.
/// 6 months captures upcoming events including summer camp planning.
const EVENT_LOOKAHEAD_DAYS: i64 = 180;

/// Maximum number of retries for rate-limited (429) requests.
/// 3 retries with exponential backoff usually succeeds without excessive delay.
const MAX_RATE_LIMIT_RETRIES: u32 = 3;

/// Initial backoff delay in milliseconds for rate limiting.
/// 1 second is polite to the server while not making users wait too long.
const INITIAL_BACKOFF_MS: u64 = 1000;

#[derive(Debug, Deserialize)]
struct AuthResponse {
    token: String,
    #[serde(rename = "personGuid")]
    person_guid: String,
    account: AuthAccount,
}

#[derive(Debug, Deserialize)]
struct AuthAccount {
    #[serde(rename = "userId")]
    user_id: i64,
}

#[derive(Debug, Deserialize)]
struct RenewalRelationship {
    #[serde(rename = "organizationGuid")]
    organization_guid: Option<String>,
    #[serde(rename = "relationshipTypeId")]
    relationship_type_id: Option<i64>,
}

/// API client for Scouting.org.
/// Clone is cheap - reqwest::Client uses Arc internally for connection pooling.
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    token: Option<String>,
}

impl ApiClient {
    /// Create a new API client
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()?;

        Ok(Self {
            client,
            token: None,
        })
    }

    /// Set the bearer token for authenticated requests
    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    /// Create a new ApiClient with the given token, sharing the connection pool.
    /// This is more efficient than creating a new client for each request.
    pub fn with_token(&self, token: String) -> Self {
        Self {
            client: self.client.clone(), // Cheap clone, shares connection pool
            token: Some(token),
        }
    }

    /// Authenticate with Scoutbook and return session data
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<SessionData> {
        let url = format!("{}/users/{}/authenticate", AUTH_BASE_URL, username);

        let response = self
            .client
            .post(&url)
            .header(header::ACCEPT, "application/json; version=2")
            .form(&[("password", password)])
            .send()
            .await
            .context("Failed to send authentication request")?;

        let response = Self::check_response(response).await?;

        let auth: AuthResponse = response.json().await.context("Failed to parse auth response")?;

        // Fetch organization GUID from renewal relationships
        let org_guid = self
            .fetch_organization_guid(&auth.token, &auth.person_guid)
            .await?;

        Ok(SessionData {
            token: auth.token,
            user_id: auth.account.user_id,
            person_guid: auth.person_guid,
            organization_guid: org_guid,
            username: username.to_string(),
            created_at: Utc::now(),
        })
    }

    /// Validate that a string looks like a valid GUID (UUID format).
    /// GUIDs should be 36 characters with dashes: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    fn is_valid_guid(s: &str) -> bool {
        if s.len() != 36 {
            return false;
        }
        s.chars().enumerate().all(|(i, c)| {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
    }

    async fn fetch_organization_guid(&self, token: &str, person_guid: &str) -> Result<String> {
        let url = format!(
            "{}/persons/{}/renewalRelationships",
            API_BASE_URL, person_guid
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to fetch renewal relationships")?;

        let response = Self::check_response(response).await?;

        let relationships: Vec<RenewalRelationship> = response
            .json()
            .await
            .context("Failed to parse renewal relationships")?;

        // Find the entry where relationshipTypeId is null and validate GUID format
        for rel in relationships {
            if rel.relationship_type_id.is_none() {
                if let Some(org_guid) = rel.organization_guid {
                    if Self::is_valid_guid(&org_guid) {
                        return Ok(org_guid);
                    }
                    warn!(guid = %org_guid, "Invalid organization GUID format");
                }
            }
        }

        Err(anyhow::anyhow!(
            "Could not find valid organization GUID in renewal relationships"
        ))
    }

    fn auth_headers(&self) -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();
        if let Some(ref token) = self.token {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {}", token))?,
            );
        }
        Ok(headers)
    }

    /// Check if response is successful, returning an error with body if not.
    /// Returns Ok(Some(response)) for success, Ok(None) for rate limit (should retry),
    /// or Err for other errors.
    async fn check_response_for_retry(response: reqwest::Response) -> Result<Option<reqwest::Response>> {
        if response.status().is_success() {
            Ok(Some(response))
        } else if response.status().as_u16() == 429 {
            // Rate limited - signal to retry
            Ok(None)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ApiError::from_status(status, &body).into())
        }
    }

    /// Check if response is successful, returning an error with body if not.
    async fn check_response(response: reqwest::Response) -> Result<reqwest::Response> {
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ApiError::from_status(status, &body).into())
        }
    }

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let mut retries = 0;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        loop {
            let response = self
                .client
                .get(url)
                .headers(self.auth_headers()?)
                .send()
                .await
                .with_context(|| format!("Failed to send GET request to {}", url))?;

            match Self::check_response_for_retry(response).await? {
                Some(response) => {
                    return response.json().await
                        .with_context(|| format!("Failed to parse JSON response from {}", url));
                }
                None => {
                    // Rate limited
                    retries += 1;
                    if retries > MAX_RATE_LIMIT_RETRIES {
                        return Err(ApiError::RateLimited.into());
                    }
                    warn!(url = url, retry = retries, backoff_ms = backoff_ms, "Rate limited, backing off");
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms *= 2; // Exponential backoff
                }
            }
        }
    }

    async fn post<T: DeserializeOwned, B: Serialize>(&self, url: &str, body: &B) -> Result<T> {
        let mut retries = 0;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        loop {
            let response = self
                .client
                .post(url)
                .headers(self.auth_headers()?)
                .json(body)
                .send()
                .await
                .with_context(|| format!("Failed to send POST request to {}", url))?;

            match Self::check_response_for_retry(response).await? {
                Some(response) => {
                    return response.json().await
                        .with_context(|| format!("Failed to parse JSON response from {}", url));
                }
                None => {
                    // Rate limited
                    retries += 1;
                    if retries > MAX_RATE_LIMIT_RETRIES {
                        return Err(ApiError::RateLimited.into());
                    }
                    warn!(url = url, retry = retries, backoff_ms = backoff_ms, "Rate limited, backing off");
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms *= 2; // Exponential backoff
                }
            }
        }
    }

    // ===== Data Fetching Methods =====

    /// Fetch all youth members for the organization
    pub async fn fetch_youth(&self, org_guid: &str) -> Result<Vec<Youth>> {
        // Fetch from GET endpoint for patrol/rank data
        let url1 = format!("{}/organizations/v2/units/{}/youths", API_BASE_URL, org_guid);
        let response1 = self
            .client
            .get(&url1)
            .headers(self.auth_headers()?)
            .send()
            .await
            .context("Failed to fetch youth list")?;

        let response1 = Self::check_response(response1).await?;

        let text1 = response1.text().await.context("Failed to read youth response body")?;
        let parsed1: UnitYouthsResponse = serde_json::from_str(&text1)
            .context("Failed to parse units youth response")?;

        let mut youth_list: Vec<Youth> = parsed1.users.iter().map(|u| u.to_youth()).collect();

        // Fetch from POST endpoint for registration details
        let url2 = format!("{}/organizations/v2/{}/orgYouths", API_BASE_URL, org_guid);
        let body = serde_json::json!({
            "includeRegistrationDetails": true,
            "includeAddressPhoneEmail": false,
            "includeExpired": false
        });

        let response2 = self
            .client
            .post(&url2)
            .headers(self.auth_headers()?)
            .json(&body)
            .send()
            .await?;

        if response2.status().is_success() {
            let text2 = response2.text().await?;
            debug!("Youth POST response received");

            if let Ok(parsed2) = serde_json::from_str::<OrgYouthsResponse>(&text2) {
                debug!("Parsed {} members from orgYouths", parsed2.members.len());
                // Merge registration data and grade by personGuid
                for youth in &mut youth_list {
                    if let Some(ref person_guid) = youth.person_guid {
                        if let Some(detailed) = parsed2.members.iter().find(|m| m.person_guid.as_ref() == Some(person_guid)) {
                            // Copy over registrarInfo and grade from orgYouths (more accurate)
                            youth.registrar_info = detailed.registrar_info.clone();
                            let old_grade = youth.grade;
                            youth.grade = detailed.grade;
                            youth.grade_id = detailed.grade_id;
                            if old_grade != youth.grade {
                                debug!("Updated grade for {}: {:?} -> {:?}", youth.last_name, old_grade, youth.grade);
                            }
                        }
                    }
                }
            } else {
                warn!("Failed to parse orgYouths response");
            }
        }

        Ok(youth_list)
    }

    /// Fetch all adult leaders for the organization
    pub async fn fetch_adults(&self, org_guid: &str) -> Result<Vec<Adult>> {
        let url = format!("{}/organizations/v2/{}/orgAdults", API_BASE_URL, org_guid);
        let body = serde_json::json!({
            "includeRegistrationDetails": true,
            "includeAddressPhoneEmail": true,
        });
        let response: OrgAdultsResponse = self.post(&url, &body).await?;
        Ok(response.members)
    }

    /// Fetch all parents of youth members in the organization
    pub async fn fetch_parents(&self, org_guid: &str) -> Result<Vec<Parent>> {
        let url = format!(
            "{}/organizations/v2/units/{}/parents",
            API_BASE_URL, org_guid
        );

        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let response = Self::check_response(response).await?;

        let text = response.text().await?;
        debug!("Parents response received");

        let parsed: Vec<ParentResponse> = serde_json::from_str(&text)
            .context("Failed to parse parents response")?;

        // Convert to Parent structs
        Ok(parsed.iter().map(|p| p.to_parent()).collect())
    }

    /// Fetch all patrols (sub-units) in the organization
    pub async fn fetch_patrols(&self, org_guid: &str) -> Result<Vec<Patrol>> {
        let url = format!(
            "{}/organizations/v2/units/{}/subUnits",
            API_BASE_URL, org_guid
        );
        self.get(&url).await
    }

    /// Fetch advancement dashboard summary for the organization
    pub async fn fetch_advancement_dashboard(&self, org_guid: &str) -> Result<AdvancementDashboard> {
        let url = format!(
            "{}/organizations/v2/{}/advancementDashboard",
            API_BASE_URL, org_guid
        );
        self.get(&url).await
    }

    /// Fetch list of advancements ready to be awarded
    pub async fn fetch_ready_to_award(&self, org_guid: &str) -> Result<Vec<ReadyToAward>> {
        let url = format!(
            "{}/organizations/v2/{}/advancementsReadyToBeAwarded",
            API_BASE_URL, org_guid
        );
        self.post(&url, &serde_json::json!({})).await
    }

    /// Fetch rank progress for a specific youth member
    pub async fn fetch_youth_ranks(&self, user_id: i64) -> Result<Vec<RankProgress>> {
        let url = format!("{}/advancements/v2/youth/{}/ranks", API_BASE_URL, user_id);
        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let response = Self::check_response(response).await?;

        let text = response.text().await?;
        debug!("Ranks response received");

        // Parse the nested response and extract Scouts BSA ranks (programId 2)
        let parsed: RanksResponse = serde_json::from_str(&text)
            .context("Failed to parse ranks response")?;

        let mut ranks: Vec<RankProgress> = Vec::new();
        for program in &parsed.program {
            if program.program_id == 2 {
                // Scouts BSA
                for rank in &program.ranks {
                    ranks.push(RankProgress::from_api(rank));
                }
            }
        }

        // Sort by level ascending (Scout first, Eagle last)
        ranks.sort_by(|a, b| a.level.cmp(&b.level));

        Ok(ranks)
    }

    /// Fetch merit badge progress for a specific youth member
    pub async fn fetch_youth_merit_badges(&self, user_id: i64) -> Result<Vec<MeritBadgeProgress>> {
        let url = format!(
            "{}/advancements/v2/youth/{}/meritBadges",
            API_BASE_URL, user_id
        );
        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let response = Self::check_response(response).await?;

        let text = response.text().await?;
        debug!("Merit badges response received");
        Ok(serde_json::from_str(&text)?)
    }

    /// Fetch requirements for a specific rank for a youth member
    pub async fn fetch_rank_requirements(&self, user_id: i64, rank_id: i64) -> Result<Vec<RankRequirement>> {
        // Try the requirements endpoint first
        let url = format!(
            "{}/advancements/v2/youth/{}/ranks/{}/requirements",
            API_BASE_URL, user_id, rank_id
        );
        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let response = Self::check_response(response).await?;

        let text = response.text().await?;
        debug!("Rank requirements response received");

        // Try parsing as direct array first, then as rank wrapper
        if let Ok(requirements) = serde_json::from_str::<Vec<RankRequirement>>(&text) {
            return Ok(requirements);
        }

        // Fall back to parsing as rank object with embedded requirements
        let rank: RankWithRequirements = serde_json::from_str(&text)
            .context("Failed to parse rank requirements")?;
        Ok(rank.requirements)
    }

    /// Fetch badge requirements, returns (requirements, version)
    pub async fn fetch_badge_requirements(&self, user_id: i64, badge_id: i64) -> Result<(Vec<MeritBadgeRequirement>, Option<String>)> {
        // Try the requirements endpoint first (like ranks)
        let url = format!(
            "{}/advancements/v2/youth/{}/meritBadges/{}/requirements",
            API_BASE_URL, user_id, badge_id
        );
        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        if response.status().is_success() {
            let text = response.text().await?;
            debug!("Badge requirements response received");

            // Try parsing as direct array first (no version info available)
            if let Ok(requirements) = serde_json::from_str::<Vec<MeritBadgeRequirement>>(&text) {
                debug!(count = requirements.len(), "Parsed badge requirements as array");
                return Ok((requirements, None));
            }

            // Fall back to parsing as badge object with embedded requirements
            match serde_json::from_str::<MeritBadgeWithRequirements>(&text) {
                Ok(badge) => {
                    debug!(count = badge.requirements.len(), version = ?badge.version, "Parsed badge with requirements");
                    return Ok((badge.requirements, badge.version));
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse badge requirements");
                }
            }
        }

        // Try fetching badge with embedded requirements
        let url2 = format!(
            "{}/advancements/v2/youth/{}/meritBadges/{}",
            API_BASE_URL, user_id, badge_id
        );
        let response2 = self
            .client
            .get(&url2)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let response2 = Self::check_response(response2).await?;

        let text = response2.text().await?;
        debug!("Badge requirements fallback response received");

        // Try parsing as badge object with embedded requirements
        if let Ok(badge) = serde_json::from_str::<MeritBadgeWithRequirements>(&text) {
            return Ok((badge.requirements, badge.version));
        }

        // If still no requirements, return empty
        Ok((vec![], None))
    }

    /// Fetch all merit badges from the catalog (not youth-specific)
    pub async fn fetch_merit_badge_catalog(&self) -> Result<Vec<crate::models::MeritBadgeCatalogEntry>> {
        let url = format!("{}/advancements/meritBadges", API_BASE_URL);

        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let response = Self::check_response(response).await?;
        let text = response.text().await?;

        // Try parsing as direct array first
        if let Ok(badges) = serde_json::from_str::<Vec<crate::models::MeritBadgeCatalogEntry>>(&text) {
            return Ok(badges);
        }

        // Try as wrapped object with common field names
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(default, alias = "meritBadges", alias = "data", alias = "badges")]
            merit_badges: Vec<crate::models::MeritBadgeCatalogEntry>,
        }

        if let Ok(wrapper) = serde_json::from_str::<Wrapper>(&text) {
            return Ok(wrapper.merit_badges);
        }

        // Debug: print first 500 chars to help diagnose
        debug!("Merit badge catalog response (first 500 chars): {}", &text[..text.len().min(500)]);

        Err(anyhow::anyhow!("Failed to parse merit badge catalog. Response starts with: {}", &text[..text.len().min(200)]))
    }

    /// Fetch events for a date range around the current date
    pub async fn fetch_events(&self, user_id: i64) -> Result<Vec<Event>> {
        let url = format!("{}/advancements/events", API_BASE_URL);

        // Calculate date range
        let now = chrono::Utc::now();
        let from_date = (now - chrono::Duration::days(EVENT_LOOKBACK_DAYS)).format("%Y-%m-%d").to_string();
        let to_date = (now + chrono::Duration::days(EVENT_LOOKAHEAD_DAYS)).format("%Y-%m-%d").to_string();

        debug!(from = %from_date, to = %to_date, "Fetching events");

        let body = serde_json::json!({
            "fromDate": from_date,
            "toDate": to_date,
            "invitedUserId": user_id
        });

        let response = self
            .client
            .post(&url)
            .headers(self.auth_headers()?)
            .json(&body)
            .send()
            .await?;

        let response = Self::check_response(response).await?;

        let text = response.text().await?;
        debug!("Events response received");

        // Try to parse as array directly first, then as wrapped object
        if let Ok(events) = serde_json::from_str::<Vec<Event>>(&text) {
            return Ok(events);
        }

        // Try common wrapper formats
        #[derive(Deserialize)]
        struct EventsWrapper {
            #[serde(default)]
            events: Vec<Event>,
            #[serde(default)]
            data: Vec<Event>,
        }

        if let Ok(wrapper) = serde_json::from_str::<EventsWrapper>(&text) {
            if !wrapper.events.is_empty() {
                return Ok(wrapper.events);
            }
            if !wrapper.data.is_empty() {
                return Ok(wrapper.data);
            }
        }

        // Return empty if we can't parse
        Ok(vec![])
    }

    /// Fetch detailed event info including full invited_users with RSVP data
    pub async fn fetch_event_detail(&self, event_id: i64) -> Result<Event> {
        let url = format!("{}/advancements/events/{}", API_BASE_URL, event_id);

        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!(event_id, status = %status, "Event detail response received");

        if !status.is_success() {
            return Err(ApiError::from_status(status, &text).into());
        }

        let mut event: Event = serde_json::from_str(&text)
            .context("Failed to parse event detail")?;
        // GET response doesn't include id, so set it from the URL
        event.id = event_id;
        Ok(event)
    }

    /// Fetch guest list for a specific event
    pub async fn fetch_event_guests(&self, event_id: i64) -> Result<Vec<EventGuest>> {
        let url = format!(
            "{}/advancements/v2/events/{}/guests",
            API_BASE_URL, event_id
        );

        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!(event_id, status = %status, "Event guests response received");

        // Try to parse as array first
        if let Ok(guests) = serde_json::from_str::<Vec<EventGuest>>(&text) {
            return Ok(guests);
        }

        // Try as wrapper object
        #[derive(serde::Deserialize)]
        struct GuestsWrapper {
            #[serde(default)]
            guests: Vec<EventGuest>,
            #[serde(default)]
            data: Vec<EventGuest>,
        }

        if let Ok(wrapper) = serde_json::from_str::<GuestsWrapper>(&text) {
            if !wrapper.guests.is_empty() {
                return Ok(wrapper.guests);
            }
            if !wrapper.data.is_empty() {
                return Ok(wrapper.data);
            }
        }

        Ok(vec![])
    }

    /// Fetch Key 3 leaders for the organization
    pub async fn fetch_key3(&self, org_guid: &str) -> Result<Key3Leaders> {
        let url = format!("{}/organizations/v2/{}/key3", API_BASE_URL, org_guid);

        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!(status = %status, "Key3 response received");

        if !status.is_success() {
            return Err(ApiError::from_status(status, &text).into());
        }

        // Parse array of Key3 items
        let items: Vec<Key3ApiItem> = serde_json::from_str(&text)
            .context("Failed to parse key3 response")?;

        let mut result = Key3Leaders::default();

        for item in items {
            let k3 = &item.organization_key3;
            let position = k3.position_long.as_deref().unwrap_or("");

            let person = Leader {
                first_name: k3.first_name.clone().unwrap_or_default(),
                last_name: k3.last_name.clone().unwrap_or_default(),
            };

            if position.contains("Scoutmaster") {
                result.scoutmaster = Some(person);
            } else if position.contains("Committee Chair") {
                result.committee_chair = Some(person);
            } else if position.contains("Chartered Organization Rep") || position.contains("Charter Org") {
                result.charter_org_rep = Some(person);
            }
        }

        Ok(result)
    }

    /// Fetch unit registration PIN info (includes website and charter info)
    pub async fn fetch_unit_pin(&self, org_guid: &str) -> Result<UnitInfo> {
        let url = format!("{}/organizations/{}/pin", API_BASE_URL, org_guid);

        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!(status = %status, "Unit PIN response received");

        if !status.is_success() {
            return Err(ApiError::from_status(status, &text).into());
        }

        let api_response: PinApiResponse = serde_json::from_str(&text)
            .context("Failed to parse PIN response")?;

        // Convert API response to domain type
        let pin = &api_response.pin_information;
        let unit = &api_response.unit_information;
        let council = &api_response.council_information;

        Ok(UnitInfo {
            name: unit.name.clone(),
            website: pin.unit_website.clone(),
            registration_url: unit.tiny_url.clone(),
            district_name: unit.district_name.clone(),
            council_name: council.name.clone(),
            charter_org_name: unit.charter_information.community_organization_name.clone(),
            charter_expiry: unit.charter_information.expiry_dt.clone(),
            meeting_location: Some(MeetingLocation {
                address_line1: pin.meeting_address_line1.clone(),
                address_line2: pin.meeting_address_line2.clone(),
                city: pin.meeting_city.clone(),
                state: pin.meeting_state.clone(),
                zip: pin.meeting_zip.clone(),
            }),
            contacts: pin.contact_persons.iter().map(|c| UnitContact {
                first_name: c.first_name.clone(),
                last_name: c.last_name.clone(),
                email: c.email.clone(),
                phone: c.phone.clone(),
            }).collect(),
        })
    }

    /// Fetch organization profile
    pub async fn fetch_org_profile(&self, org_guid: &str) -> Result<OrgProfile> {
        let url = format!("{}/organizations/v2/{}/profile", API_BASE_URL, org_guid);

        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!(status = %status, "Org profile response received");

        if !status.is_success() {
            return Err(ApiError::from_status(status, &text).into());
        }

        let api_profile: OrgProfileApiResponse = serde_json::from_str(&text)
            .context("Failed to parse org profile response")?;

        // Convert to domain type
        Ok(OrgProfile {
            name: api_profile.organization_name,
            full_name: api_profile.organization_full_name,
            charter_org_name: api_profile.chartered_org_name,
            charter_exp_date: api_profile.charter_exp_date,
            charter_status: api_profile.charter_status,
        })
    }

    /// Fetch assigned commissioners for a unit
    pub async fn fetch_commissioners(&self, org_guid: &str) -> Result<Vec<Commissioner>> {
        let url = format!(
            "{}/commissioners/v2/organizations/{}/units/assignedCommissioners",
            API_BASE_URL, org_guid
        );

        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!(status = %status, "Commissioners response received");

        if !status.is_success() {
            return Err(ApiError::from_status(status, &text).into());
        }

        // Log the raw response for debugging
        debug!("Commissioners raw response: {}", &text[..text.len().min(500)]);

        // Helper to convert API item to domain type
        let convert = |api: CommissionerApiItem| Commissioner {
            first_name: api.first_name,
            last_name: api.last_name,
            position: api.position,
        };

        // Try parsing as CommissionersResponse first, then as direct array
        if let Ok(resp) = serde_json::from_str::<CommissionersResponse>(&text) {
            debug!("Parsed as CommissionersResponse with {} commissioners", resp.commissioners.len());
            return Ok(resp.commissioners.into_iter().map(convert).collect());
        }

        // Try as direct array
        if let Ok(items) = serde_json::from_str::<Vec<CommissionerApiItem>>(&text) {
            debug!("Parsed as direct array with {} commissioners", items.len());
            return Ok(items.into_iter().map(convert).collect());
        }

        warn!("Failed to parse commissioners response");
        Ok(vec![])
    }
}

// Internal API response types for parsing

#[derive(Debug, Clone, Deserialize)]
struct Key3ApiItem {
    #[serde(rename = "organizationKey3")]
    organization_key3: Key3PersonRaw,
}

#[derive(Debug, Clone, Deserialize)]
struct Key3PersonRaw {
    #[serde(rename = "positionLong")]
    position_long: Option<String>,
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PinApiResponse {
    #[serde(rename = "pinInformation")]
    pin_information: PinInformation,
    #[serde(rename = "unitInformation")]
    unit_information: UnitInformationApi,
    #[serde(rename = "councilInformation")]
    council_information: CouncilInformation,
}

#[derive(Debug, Clone, Deserialize)]
struct PinInformation {
    #[serde(rename = "unitWebsite")]
    unit_website: Option<String>,
    #[serde(rename = "meetingAddressLine1")]
    meeting_address_line1: Option<String>,
    #[serde(rename = "meetingAddressLine2")]
    meeting_address_line2: Option<String>,
    #[serde(rename = "meetingCity")]
    meeting_city: Option<String>,
    #[serde(rename = "meetingState")]
    meeting_state: Option<String>,
    #[serde(rename = "meetingZip")]
    meeting_zip: Option<String>,
    #[serde(rename = "contactPersons", default)]
    contact_persons: Vec<ContactPersonApi>,
}

#[derive(Debug, Clone, Deserialize)]
struct ContactPersonApi {
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct UnitInformationApi {
    name: Option<String>,
    #[serde(rename = "districtName")]
    district_name: Option<String>,
    #[serde(rename = "tinyUrl")]
    tiny_url: Option<String>,
    #[serde(rename = "charterInformation")]
    charter_information: CharterInformation,
}

#[derive(Debug, Clone, Deserialize)]
struct CharterInformation {
    #[serde(rename = "communityOrganizationName")]
    community_organization_name: Option<String>,
    #[serde(rename = "expiryDt")]
    expiry_dt: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CouncilInformation {
    name: Option<String>,
}

// Commissioner API response types - internal only
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CommissionersResponse {
    #[serde(rename = "assignedCommissioners", default)]
    commissioners: Vec<CommissionerApiItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommissionerApiItem {
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
    #[serde(default)]
    position: Option<String>,
}

/// Internal API response type - use OrgProfile from models for domain code
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct OrgProfileApiResponse {
    #[serde(rename = "organizationName")]
    organization_name: Option<String>,
    #[serde(rename = "organizationFullName")]
    organization_full_name: Option<String>,
    #[serde(rename = "charteredOrgName")]
    chartered_org_name: Option<String>,
    #[serde(rename = "charterExpDate")]
    charter_exp_date: Option<String>,
    #[serde(rename = "charterStatus")]
    charter_status: Option<String>,
    #[serde(rename = "unitNumber")]
    unit_number: Option<String>,
    #[serde(rename = "unitType")]
    unit_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_guid() {
        // Valid GUIDs
        assert!(ApiClient::is_valid_guid("0E65066C-AB20-4DA0-B3BF-79DFD0668049"));
        assert!(ApiClient::is_valid_guid("22b210e3-d325-41be-b761-31e18bfe2c73")); // lowercase
        assert!(ApiClient::is_valid_guid("00000000-0000-0000-0000-000000000000"));

        // Invalid GUIDs
        assert!(!ApiClient::is_valid_guid("")); // empty
        assert!(!ApiClient::is_valid_guid("not-a-guid")); // too short
        assert!(!ApiClient::is_valid_guid("0E65066CAB204DA0B3BF79DFD0668049")); // no dashes
        assert!(!ApiClient::is_valid_guid("0E65066C-AB20-4DA0-B3BF-79DFD066804")); // too short
        assert!(!ApiClient::is_valid_guid("0E65066C-AB20-4DA0-B3BF-79DFD06680490")); // too long
        assert!(!ApiClient::is_valid_guid("ZZZZZZZZ-ZZZZ-ZZZZ-ZZZZ-ZZZZZZZZZZZZ")); // invalid chars
    }

    #[test]
    fn test_parse_commissioners_response() {
        let json = r#"{"organizationGuid": "0E65066C-AB20-4DA0-B3BF-79DFD0668049","organizationType": "Troop","organizationNumber": "0053","organizationCharterName": "St John Evangelist Catholic Church","assignedCommissioners": [{"personGuid": "22B210E3-D325-41BE-B761-31E18BFE2C73","memberId": 8563888,"personId": 5816532,"personfullName": "Robert Diran Sarkisian Jr","firstName": "Robert","middleName": "Diran","lastName": "Sarkisian","nameSuffix": "Jr","positionId": 422,"position": "District Commissioner"}]}"#;

        let resp: CommissionersResponse = serde_json::from_str(json)
            .expect("Failed to parse commissioners test JSON");
        assert_eq!(resp.commissioners.len(), 1);

        // Verify API response parses correctly
        let c = &resp.commissioners[0];
        assert_eq!(c.first_name.as_deref(), Some("Robert"));
        assert_eq!(c.last_name.as_deref(), Some("Sarkisian"));
        assert_eq!(c.position.as_deref(), Some("District Commissioner"));

        // Test conversion to domain type
        let domain_commissioner = Commissioner {
            first_name: c.first_name.clone(),
            last_name: c.last_name.clone(),
            position: c.position.clone(),
        };
        assert_eq!(domain_commissioner.full_name(), "Robert Sarkisian");
        assert_eq!(domain_commissioner.position_display(), "District Commissioner");
    }
}
