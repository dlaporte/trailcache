use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{Emitter, State};

use trailcache_core::auth::CredentialStore;
use trailcache_core::cache::CacheAges;
use trailcache_core::models::{
    sort_requirements, AdvancementDashboard, Commissioner, Key3Leaders, MeritBadgeProgress,
    OrgProfile, Patrol, RankProgress, UnitInfo, Youth,
};

use crate::dto::{
    AdultDisplay, AwardDisplay, BadgePivotEntry, BadgeRequirementsResponseDisplay,
    EventDisplay, EventGuestDisplay, LeadershipDisplay, MeritBadgeDisplay,
    MeritBadgeRequirementDisplay, ParentDisplay, RankPivotEntry, RankProgressDisplay,
    RankRequirementDisplay, YouthDisplay,
    build_badge_pivot, build_rank_pivot,
};
use crate::state::GuiAppState;

/// Error type for Tauri commands - must implement Serialize.
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub message: String,
}

impl From<anyhow::Error> for CommandError {
    fn from(err: anyhow::Error) -> Self {
        CommandError {
            message: format!("{:#}", err),
        }
    }
}

type CommandResult<T> = Result<T, CommandError>;

/// Progress payload emitted during background refresh.
#[derive(Clone, Serialize)]
pub struct RefreshProgress {
    pub step: String,
    pub current: u32,
    pub total: u32,
    pub error: Option<String>,
}

// ============================================================================
// Auth Commands
// ============================================================================

#[derive(Serialize)]
pub struct LoginResponse {
    pub user_id: i64,
    pub organization_guid: String,
    pub username: String,
    pub unit_name: Option<String>,
}

#[tauri::command]
pub async fn login(
    username: String,
    password: String,
    state: State<'_, GuiAppState>,
) -> CommandResult<LoginResponse> {
    let api = state.api_client.lock().await;
    let session_data = api.authenticate(&username, &password).await.map_err(|e| CommandError {
        message: format!("Login failed: {:#}", e),
    })?;

    let org_guid = session_data.organization_guid.clone();
    let user_id = session_data.user_id;
    let token = Arc::new(session_data.token.clone());

    // Update API client with token
    drop(api);
    {
        let mut api = state.api_client.lock().await;
        api.set_token(Arc::clone(&token));
    }

    // Update session
    {
        let mut session = state.session.lock().await;
        session.update(session_data);
        let _ = session.save();
    }

    // Update config
    let unit_name;
    {
        let mut config = state.config.lock().await;
        config.organization_guid = Some(org_guid.clone());
        config.last_username = Some(username.clone());
        let _ = config.save();
        unit_name = config.unit_name.clone();
    }

    // Update cache with encryption key and correct directory
    {
        let config = state.config.lock().await;
        let cache_dir = config.cache_dir().unwrap_or_else(|_| PathBuf::from("./cache"));
        drop(config);

        let mut cache = state.cache.lock().await;
        *cache = trailcache_core::cache::CacheManager::new_without_encryption(cache_dir)
            ?;
        cache.set_password(&password, &org_guid);
    }

    // Store credentials
    let _ = CredentialStore::store(&username, &password);

    Ok(LoginResponse {
        user_id,
        organization_guid: org_guid,
        username,
        unit_name,
    })
}

#[tauri::command]
pub async fn get_saved_username(
    state: State<'_, GuiAppState>,
) -> CommandResult<Option<String>> {
    let config = state.config.lock().await;
    Ok(config.last_username.clone())
}

#[tauri::command]
pub async fn logout(state: State<'_, GuiAppState>) -> CommandResult<()> {
    let mut session = state.session.lock().await;
    session.clear()?;
    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

/// Check if offline mode is enabled.
async fn is_offline(state: &State<'_, GuiAppState>) -> bool {
    state.config.lock().await.offline_mode
}

// ============================================================================
// Data Commands (with stale-cache fallback) — now returning DTOs
// ============================================================================

#[tauri::command]
pub async fn get_youth(state: State<'_, GuiAppState>) -> CommandResult<Vec<YouthDisplay>> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    // In offline mode, return cached data regardless of staleness
    if offline {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_youth() {
            return Ok(cached.data.iter().map(YouthDisplay::from).collect());
        }
        return Ok(vec![]);
    }

    // Try cache first; save stale data for fallback
    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_youth() {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data.iter().map(YouthDisplay::from).collect());
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    // Fetch from API, fall back to stale cache on error
    let api = state.api_client.lock().await;
    match api.fetch_youth(&org_guid).await {
        Ok(youth) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_youth(&youth);
            Ok(youth.iter().map(YouthDisplay::from).collect())
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data.iter().map(YouthDisplay::from).collect()),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_adults(state: State<'_, GuiAppState>) -> CommandResult<Vec<AdultDisplay>> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    if offline {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_adults() {
            return Ok(cached.data.iter().map(AdultDisplay::from).collect());
        }
        return Ok(vec![]);
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_adults() {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data.iter().map(AdultDisplay::from).collect());
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_adults(&org_guid).await {
        Ok(adults) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_adults(&adults);
            Ok(adults.iter().map(AdultDisplay::from).collect())
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data.iter().map(AdultDisplay::from).collect()),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_parents(state: State<'_, GuiAppState>) -> CommandResult<Vec<ParentDisplay>> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    if offline {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_parents() {
            return Ok(cached.data.iter().map(ParentDisplay::from).collect());
        }
        return Ok(vec![]);
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_parents() {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data.iter().map(ParentDisplay::from).collect());
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_parents(&org_guid).await {
        Ok(parents) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_parents(&parents);
            Ok(parents.iter().map(ParentDisplay::from).collect())
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data.iter().map(ParentDisplay::from).collect()),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_events(state: State<'_, GuiAppState>) -> CommandResult<Vec<EventDisplay>> {
    if is_offline(&state).await {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_events() {
            return Ok(cached.data.iter().map(EventDisplay::from).collect());
        }
        return Ok(vec![]);
    }

    let session = state.session.lock().await;
    let user_id = session.user_id().unwrap_or(0);
    drop(session);

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_events() {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data.iter().map(EventDisplay::from).collect());
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_events(user_id).await {
        Ok(events) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_events(&events);
            Ok(events.iter().map(EventDisplay::from).collect())
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data.iter().map(EventDisplay::from).collect()),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_event_guests(
    event_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<EventGuestDisplay>> {
    if is_offline(&state).await {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_events() {
            if let Some(event) = cached.data.iter().find(|e| e.id == event_id) {
                if !event.invited_users.is_empty() {
                    return Ok(event.invited_users.iter()
                        .map(EventGuestDisplay::from_invited_user)
                        .collect());
                }
            }
        }
        return Ok(vec![]);
    }

    // Fetch event detail (GET /events/{id}) — returns full invited_users with RSVP data.
    // The cached events list only has the requesting user, not all invitees.
    let api = state.api_client.lock().await;
    match api.fetch_event_detail(event_id).await {
        Ok(detail) if !detail.invited_users.is_empty() => {
            return Ok(detail.invited_users.iter()
                .map(EventGuestDisplay::from_invited_user)
                .collect());
        }
        _ => {}
    }

    // Fall back to cached events (partial — only has requesting user)
    drop(api);
    {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_events() {
            if let Some(event) = cached.data.iter().find(|e| e.id == event_id) {
                if !event.invited_users.is_empty() {
                    return Ok(event.invited_users.iter()
                        .map(EventGuestDisplay::from_invited_user)
                        .collect());
                }
            }
        }
    }

    // Last resort: guest API endpoint (less reliable)
    let api = state.api_client.lock().await;
    let guests = api.fetch_event_guests(event_id).await?;
    Ok(guests.iter().map(EventGuestDisplay::from).collect())
}

#[tauri::command]
pub async fn get_patrols(state: State<'_, GuiAppState>) -> CommandResult<Vec<Patrol>> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    if offline {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_patrols() {
            return Ok(cached.data);
        }
        return Ok(vec![]);
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_patrols() {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data);
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_patrols(&org_guid).await {
        Ok(patrols) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_patrols(&patrols);
            Ok(patrols)
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_unit_info(state: State<'_, GuiAppState>) -> CommandResult<UnitInfo> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    if offline {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_unit_info() {
            return Ok(cached.data);
        }
        return Err(CommandError { message: "No cached unit info available".into() });
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_unit_info() {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data);
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_unit_pin(&org_guid).await {
        Ok(info) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_unit_info(&info);
            Ok(info)
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_key3(state: State<'_, GuiAppState>) -> CommandResult<Key3Leaders> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    if offline {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_key3() {
            return Ok(cached.data);
        }
        return Err(CommandError { message: "No cached Key 3 data available".into() });
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_key3() {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data);
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_key3(&org_guid).await {
        Ok(key3) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_key3(&key3);
            Ok(key3)
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_org_profile(state: State<'_, GuiAppState>) -> CommandResult<OrgProfile> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    if offline {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_org_profile() {
            return Ok(cached.data);
        }
        return Err(CommandError { message: "No cached org profile available".into() });
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_org_profile() {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data);
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_org_profile(&org_guid).await {
        Ok(profile) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_org_profile(&profile);
            Ok(profile)
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_commissioners(state: State<'_, GuiAppState>) -> CommandResult<Vec<Commissioner>> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    if offline {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_commissioners() {
            return Ok(cached.data);
        }
        return Ok(vec![]);
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_commissioners() {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data);
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_commissioners(&org_guid).await {
        Ok(commissioners) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_commissioners(&commissioners);
            Ok(commissioners)
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data),
            None => Err(e.into()),
        },
    }
}

// ============================================================================
// Advancement Commands (with stale-cache fallback) — now returning DTOs
// ============================================================================

#[tauri::command]
pub async fn get_advancement_dashboard(
    state: State<'_, GuiAppState>,
) -> CommandResult<AdvancementDashboard> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    if offline {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_advancement_dashboard() {
            return Ok(cached.data);
        }
        return Err(CommandError { message: "No cached advancement data available".into() });
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_advancement_dashboard() {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data);
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_advancement_dashboard(&org_guid).await {
        Ok(dashboard) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_advancement_dashboard(&dashboard);
            Ok(dashboard)
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_youth_ranks(
    user_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<RankProgressDisplay>> {
    if is_offline(&state).await {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_youth_ranks(user_id) {
            return Ok(cached.data.iter().map(RankProgressDisplay::from).collect());
        }
        return Ok(vec![]);
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_youth_ranks(user_id) {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data.iter().map(RankProgressDisplay::from).collect());
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_youth_ranks(user_id).await {
        Ok(ranks) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_youth_ranks(user_id, &ranks);
            Ok(ranks.iter().map(RankProgressDisplay::from).collect())
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data.iter().map(RankProgressDisplay::from).collect()),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_youth_merit_badges(
    user_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<MeritBadgeDisplay>> {
    if is_offline(&state).await {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_youth_merit_badges(user_id) {
            return Ok(cached.data.iter().map(MeritBadgeDisplay::from).collect());
        }
        return Ok(vec![]);
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_youth_merit_badges(user_id) {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data.iter().map(MeritBadgeDisplay::from).collect());
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_youth_merit_badges(user_id).await {
        Ok(badges) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_youth_merit_badges(user_id, &badges);
            Ok(badges.iter().map(MeritBadgeDisplay::from).collect())
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data.iter().map(MeritBadgeDisplay::from).collect()),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_youth_leadership(
    user_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<LeadershipDisplay>> {
    if is_offline(&state).await {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_youth_leadership(user_id) {
            return Ok(cached.data.iter().map(LeadershipDisplay::from).collect());
        }
        return Ok(vec![]);
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_youth_leadership(user_id) {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data.iter().map(LeadershipDisplay::from).collect());
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_youth_leadership(user_id).await {
        Ok(positions) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_youth_leadership(user_id, &positions);
            Ok(positions.iter().map(LeadershipDisplay::from).collect())
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data.iter().map(LeadershipDisplay::from).collect()),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_youth_awards(
    user_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<AwardDisplay>> {
    if is_offline(&state).await {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_youth_awards(user_id) {
            return Ok(cached.data.iter().map(AwardDisplay::from).collect());
        }
        return Ok(vec![]);
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_youth_awards(user_id) {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    return Ok(cached.data.iter().map(AwardDisplay::from).collect());
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_youth_awards(user_id).await {
        Ok(awards) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_youth_awards(user_id, &awards);
            Ok(awards.iter().map(AwardDisplay::from).collect())
        }
        Err(e) => match stale_data {
            Some(data) => Ok(data.iter().map(AwardDisplay::from).collect()),
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_rank_requirements(
    user_id: i64,
    rank_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<RankRequirementDisplay>> {
    if is_offline(&state).await {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_rank_requirements(user_id, rank_id) {
            let mut reqs: Vec<_> = cached.data.iter().map(RankRequirementDisplay::from).collect();
            sort_requirements(&mut reqs);
            return Ok(reqs);
        }
        return Ok(vec![]);
    }

    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_rank_requirements(user_id, rank_id) {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    let mut reqs: Vec<_> = cached.data.iter().map(RankRequirementDisplay::from).collect();
                    sort_requirements(&mut reqs);
                    return Ok(reqs);
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_rank_requirements(user_id, rank_id).await {
        Ok(requirements) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_rank_requirements(user_id, rank_id, &requirements);
            let mut reqs: Vec<_> = requirements.iter().map(RankRequirementDisplay::from).collect();
            sort_requirements(&mut reqs);
            Ok(reqs)
        }
        Err(e) => match stale_data {
            Some(data) => {
                let mut reqs: Vec<_> = data.iter().map(RankRequirementDisplay::from).collect();
                sort_requirements(&mut reqs);
                Ok(reqs)
            }
            None => Err(e.into()),
        },
    }
}

#[tauri::command]
pub async fn get_badge_requirements(
    user_id: i64,
    badge_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<BadgeRequirementsResponseDisplay> {
    if is_offline(&state).await {
        let cache = state.cache.lock().await;
        if let Ok(Some(cached)) = cache.load_badge_requirements(user_id, badge_id) {
            let (requirements, version) = cached.data;
            let mut reqs: Vec<_> = requirements.iter().map(MeritBadgeRequirementDisplay::from).collect();
            sort_requirements(&mut reqs);
            return Ok(BadgeRequirementsResponseDisplay {
                requirements: reqs,
                version,
                counselor_name: String::new(),
                counselor_phone: String::new(),
                counselor_email: None,
            });
        }
        return Ok(BadgeRequirementsResponseDisplay {
            requirements: vec![],
            version: None,
            counselor_name: String::new(),
            counselor_phone: String::new(),
            counselor_email: None,
        });
    }

    // Try cache first (counselor is not cached, will be empty on fallback)
    let stale_data = {
        let cache = state.cache.lock().await;
        match cache.load_badge_requirements(user_id, badge_id) {
            Ok(Some(cached)) => {
                if !cached.is_stale() {
                    let (requirements, version) = cached.data;
                    let mut reqs: Vec<_> = requirements.iter().map(MeritBadgeRequirementDisplay::from).collect();
                    sort_requirements(&mut reqs);
                    return Ok(BadgeRequirementsResponseDisplay {
                        requirements: reqs,
                        version,
                        counselor_name: String::new(),
                        counselor_phone: String::new(),
                        counselor_email: None,
                    });
                }
                Some(cached.data)
            }
            _ => None,
        }
    };

    let api = state.api_client.lock().await;
    match api.fetch_badge_requirements(user_id, badge_id).await {
        Ok((requirements, version, counselor)) => {
            drop(api);
            let cache = state.cache.lock().await;
            let _ = cache.save_badge_requirements(user_id, badge_id, &requirements, &version);
            let mut reqs: Vec<_> = requirements.iter().map(MeritBadgeRequirementDisplay::from).collect();
            sort_requirements(&mut reqs);
            Ok(BadgeRequirementsResponseDisplay {
                requirements: reqs,
                version,
                counselor_name: counselor.as_ref().map(|c| c.full_name()).unwrap_or_default(),
                counselor_phone: counselor.as_ref().and_then(|c| c.phone()).unwrap_or("").to_string(),
                counselor_email: counselor.as_ref().and_then(|c| c.email.clone()),
            })
        }
        Err(e) => match stale_data {
            Some((requirements, version)) => {
                let mut reqs: Vec<_> = requirements.iter().map(MeritBadgeRequirementDisplay::from).collect();
                sort_requirements(&mut reqs);
                Ok(BadgeRequirementsResponseDisplay {
                    requirements: reqs,
                    version,
                    counselor_name: String::new(),
                    counselor_phone: String::new(),
                    counselor_email: None,
                })
            }
            None => Err(e.into()),
        },
    }
}

// ============================================================================
// Cache Info Commands
// ============================================================================

#[tauri::command]
pub async fn get_cache_ages(state: State<'_, GuiAppState>) -> CommandResult<CacheAges> {
    let cache = state.cache.lock().await;
    Ok(cache.get_cache_ages())
}

// ============================================================================
// Offline Mode Commands
// ============================================================================

#[tauri::command]
pub async fn get_offline_mode(state: State<'_, GuiAppState>) -> CommandResult<bool> {
    let config = state.config.lock().await;
    Ok(config.offline_mode)
}

#[tauri::command]
pub async fn set_offline_mode(
    offline: bool,
    state: State<'_, GuiAppState>,
) -> CommandResult<bool> {
    let mut config = state.config.lock().await;
    config.offline_mode = offline;
    let _ = config.save();
    Ok(config.offline_mode)
}

// ============================================================================
// Refresh Command (with progress events)
// ============================================================================

#[tauri::command]
pub async fn refresh_data(
    app: tauri::AppHandle,
    state: State<'_, GuiAppState>,
) -> CommandResult<String> {
    if is_offline(&state).await {
        return Ok("Offline mode — refresh skipped".into());
    }

    let config = state.config.lock().await;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    let session = state.session.lock().await;
    let user_id = session.user_id().unwrap_or(0);
    drop(session);

    let total = 10u32;
    let mut successes = 0u32;
    let mut errors: Vec<String> = Vec::new();

    // Helper to emit progress
    let emit = |step: &str, current: u32, error: Option<String>| {
        let _ = app.emit("refresh-progress", RefreshProgress {
            step: step.to_string(),
            current,
            total,
            error,
        });
    };

    // 1. Youth
    emit("Fetching scouts...", 1, None);
    {
        let api = state.api_client.lock().await;
        match api.fetch_youth(&org_guid).await {
            Ok(data) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_youth(&data);
                successes += 1;
            }
            Err(e) => {
                let msg = format!("Scouts: {}", e);
                emit("Fetching scouts...", 1, Some(msg.clone()));
                errors.push(msg);
            }
        }
    }

    // 2. Adults
    emit("Fetching adults...", 2, None);
    {
        let api = state.api_client.lock().await;
        match api.fetch_adults(&org_guid).await {
            Ok(data) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_adults(&data);
                successes += 1;
            }
            Err(e) => {
                let msg = format!("Adults: {}", e);
                emit("Fetching adults...", 2, Some(msg.clone()));
                errors.push(msg);
            }
        }
    }

    // 3. Events
    emit("Fetching events...", 3, None);
    {
        let api = state.api_client.lock().await;
        match api.fetch_events(user_id).await {
            Ok(data) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_events(&data);
                successes += 1;
            }
            Err(e) => {
                let msg = format!("Events: {}", e);
                emit("Fetching events...", 3, Some(msg.clone()));
                errors.push(msg);
            }
        }
    }

    // 4. Patrols
    emit("Fetching patrols...", 4, None);
    {
        let api = state.api_client.lock().await;
        match api.fetch_patrols(&org_guid).await {
            Ok(data) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_patrols(&data);
                successes += 1;
            }
            Err(e) => {
                let msg = format!("Patrols: {}", e);
                emit("Fetching patrols...", 4, Some(msg.clone()));
                errors.push(msg);
            }
        }
    }

    // 5. Unit Info
    emit("Fetching unit info...", 5, None);
    {
        let api = state.api_client.lock().await;
        match api.fetch_unit_pin(&org_guid).await {
            Ok(data) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_unit_info(&data);
                successes += 1;
            }
            Err(e) => {
                let msg = format!("Unit info: {}", e);
                emit("Fetching unit info...", 5, Some(msg.clone()));
                errors.push(msg);
            }
        }
    }

    // 6. Key 3
    emit("Fetching Key 3...", 6, None);
    {
        let api = state.api_client.lock().await;
        match api.fetch_key3(&org_guid).await {
            Ok(data) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_key3(&data);
                successes += 1;
            }
            Err(e) => {
                let msg = format!("Key 3: {}", e);
                emit("Fetching Key 3...", 6, Some(msg.clone()));
                errors.push(msg);
            }
        }
    }

    // 7. Commissioners
    emit("Fetching commissioners...", 7, None);
    {
        let api = state.api_client.lock().await;
        match api.fetch_commissioners(&org_guid).await {
            Ok(data) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_commissioners(&data);
                successes += 1;
            }
            Err(e) => {
                let msg = format!("Commissioners: {}", e);
                emit("Fetching commissioners...", 7, Some(msg.clone()));
                errors.push(msg);
            }
        }
    }

    // 8. Org Profile
    emit("Fetching org profile...", 8, None);
    {
        let api = state.api_client.lock().await;
        match api.fetch_org_profile(&org_guid).await {
            Ok(data) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_org_profile(&data);
                successes += 1;
            }
            Err(e) => {
                let msg = format!("Org profile: {}", e);
                emit("Fetching org profile...", 8, Some(msg.clone()));
                errors.push(msg);
            }
        }
    }

    // 9. Parents
    emit("Fetching parents...", 9, None);
    {
        let api = state.api_client.lock().await;
        match api.fetch_parents(&org_guid).await {
            Ok(data) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_parents(&data);
                successes += 1;
            }
            Err(e) => {
                let msg = format!("Parents: {}", e);
                emit("Fetching parents...", 9, Some(msg.clone()));
                errors.push(msg);
            }
        }
    }

    // 10. Advancement Dashboard
    emit("Fetching advancement...", 10, None);
    {
        let api = state.api_client.lock().await;
        match api.fetch_advancement_dashboard(&org_guid).await {
            Ok(data) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_advancement_dashboard(&data);
                successes += 1;
            }
            Err(e) => {
                let msg = format!("Advancement: {}", e);
                emit("Fetching advancement...", 10, Some(msg.clone()));
                errors.push(msg);
            }
        }
    }

    if errors.is_empty() {
        Ok(format!("Refreshed all {} data sources", successes))
    } else {
        Ok(format!(
            "Refreshed {}/{} ({} errors)",
            successes,
            total,
            errors.len()
        ))
    }
}

// ============================================================================
// Aggregate Commands (for Ranks/Badges pivot tabs) — now pre-aggregated
// ============================================================================

/// Fetch ranks for all youth, pre-aggregate into pivot entries.
#[tauri::command]
pub async fn get_all_youth_ranks(
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<RankPivotEntry>> {
    let offline = is_offline(&state).await;

    // Load youth for display names
    let youth: Vec<Youth> = {
        let cache = state.cache.lock().await;
        match cache.load_youth() {
            Ok(Some(cached)) => cached.data,
            _ => vec![],
        }
    };

    let youth_ids: Vec<i64> = youth.iter().filter_map(|y| y.user_id).collect();

    let mut all_ranks: HashMap<i64, Vec<RankProgress>> = HashMap::new();

    for uid in &youth_ids {
        // Try cache first
        let stale_data = {
            let cache = state.cache.lock().await;
            match cache.load_youth_ranks(*uid) {
                Ok(Some(cached)) => {
                    if offline || !cached.is_stale() {
                        all_ranks.insert(*uid, cached.data);
                        continue;
                    }
                    Some(cached.data)
                }
                _ => {
                    if offline { continue; }
                    None
                }
            }
        };

        // Fetch fresh, fall back to stale
        let api = state.api_client.lock().await;
        match api.fetch_youth_ranks(*uid).await {
            Ok(ranks) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_youth_ranks(*uid, &ranks);
                all_ranks.insert(*uid, ranks);
            }
            Err(_) => {
                if let Some(data) = stale_data {
                    all_ranks.insert(*uid, data);
                }
            }
        }
    }

    Ok(build_rank_pivot(&all_ranks, &youth))
}

/// Fetch merit badges for all youth, pre-aggregate into pivot entries.
#[tauri::command]
pub async fn get_all_youth_badges(
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<BadgePivotEntry>> {
    let offline = is_offline(&state).await;

    // Load youth for display names
    let youth: Vec<Youth> = {
        let cache = state.cache.lock().await;
        match cache.load_youth() {
            Ok(Some(cached)) => cached.data,
            _ => vec![],
        }
    };

    let youth_ids: Vec<i64> = youth.iter().filter_map(|y| y.user_id).collect();

    let mut all_badges: HashMap<i64, Vec<MeritBadgeProgress>> = HashMap::new();

    for uid in &youth_ids {
        let stale_data = {
            let cache = state.cache.lock().await;
            match cache.load_youth_merit_badges(*uid) {
                Ok(Some(cached)) => {
                    if offline || !cached.is_stale() {
                        all_badges.insert(*uid, cached.data);
                        continue;
                    }
                    Some(cached.data)
                }
                _ => {
                    if offline { continue; }
                    None
                }
            }
        };

        let api = state.api_client.lock().await;
        match api.fetch_youth_merit_badges(*uid).await {
            Ok(badges) => {
                drop(api);
                let cache = state.cache.lock().await;
                let _ = cache.save_youth_merit_badges(*uid, &badges);
                all_badges.insert(*uid, badges);
            }
            Err(_) => {
                if let Some(data) = stale_data {
                    all_badges.insert(*uid, data);
                }
            }
        }
    }

    Ok(build_badge_pivot(&all_badges, &youth))
}
