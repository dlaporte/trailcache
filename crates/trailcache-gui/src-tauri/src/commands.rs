use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{Emitter, State};

use trailcache_core::auth::CredentialStore;
use trailcache_core::cache::{fetch_with_cache, CacheAges};
use trailcache_core::models::{
    sort_requirements, Adult, AdvancementDashboard, Commissioner, Key3Leaders, LeadershipPosition,
    MeritBadgeProgress, OrgProfile, Patrol, RankProgress, UnitInfo, Youth,
};

use crate::dto::{
    AdultDisplay, AwardDisplay, BadgePivotEntry, BadgeRequirementsResponseDisplay,
    EventDisplay, EventGuestDisplay, LeadershipDisplay, MeritBadgeDisplay,
    MeritBadgeRequirementDisplay, ParentDisplay, RankPivotEntry, RankProgressDisplay,
    RankRequirementDisplay, YouthBadgesResponse, YouthDisplay,
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

/// Extract a user-friendly message from a login error.
fn format_login_error(err: &anyhow::Error) -> String {
    let raw = format!("{:#}", err);

    // Try to extract the human-readable message from JSON error bodies
    // e.g. {"message":"Invalid request","errors":[{"message":"\"password\" length must be at least 8 characters long",...}]}
    if let Some(json_start) = raw.find('{') {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw[json_start..]) {
            // Try errors[].message first (most specific)
            if let Some(errors) = parsed.get("errors").and_then(|e| e.as_array()) {
                let messages: Vec<&str> = errors.iter()
                    .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                    .collect();
                if !messages.is_empty() {
                    return format!("Login failed: {}", messages.join("; "));
                }
            }
            // Fall back to top-level message
            if let Some(msg) = parsed.get("message").and_then(|m| m.as_str()) {
                return format!("Login failed: {}", msg);
            }
        }
    }

    // Known error patterns without JSON
    if raw.contains("Unauthorized") {
        return "Login failed: Invalid username or password".to_string();
    }

    format!("Login failed: {}", raw)
}

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
        message: format_login_error(&e),
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
pub async fn quit_app(app: tauri::AppHandle) -> CommandResult<()> {
    app.exit(0);
    Ok(())
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

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_youth(),
        |d| cache.save_youth(d),
        api.fetch_youth(&org_guid),
    ).await?;

    Ok(data.map(|d| d.iter().map(YouthDisplay::from).collect()).unwrap_or_default())
}

#[tauri::command]
pub async fn get_adults(state: State<'_, GuiAppState>) -> CommandResult<Vec<AdultDisplay>> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_adults(),
        |d| cache.save_adults(d),
        async { api.fetch_adults(&org_guid).await.map(Adult::deduplicate) },
    ).await?;

    Ok(data.map(|d| d.iter().map(AdultDisplay::from).collect()).unwrap_or_default())
}

#[tauri::command]
pub async fn get_parents(state: State<'_, GuiAppState>) -> CommandResult<Vec<ParentDisplay>> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_parents(),
        |d| cache.save_parents(d),
        api.fetch_parents(&org_guid),
    ).await?;

    Ok(data.map(|d| d.iter().map(ParentDisplay::from).collect()).unwrap_or_default())
}

#[tauri::command]
pub async fn get_events(state: State<'_, GuiAppState>) -> CommandResult<Vec<EventDisplay>> {
    let offline = is_offline(&state).await;

    let session = state.session.lock().await;
    let user_id = session.user_id().unwrap_or(0);
    drop(session);

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_events(),
        |d| cache.save_events(d),
        api.fetch_events(user_id),
    ).await?;

    Ok(data.map(|d| d.iter().map(EventDisplay::from).collect()).unwrap_or_default())
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

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_patrols(),
        |d| cache.save_patrols(d),
        api.fetch_patrols(&org_guid),
    ).await?;

    Ok(data.unwrap_or_default())
}

#[tauri::command]
pub async fn get_unit_info(state: State<'_, GuiAppState>) -> CommandResult<UnitInfo> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_unit_info(),
        |d| cache.save_unit_info(d),
        api.fetch_unit_pin(&org_guid),
    ).await?;

    data.map(|d| d.with_computed_fields())
        .ok_or_else(|| CommandError { message: "No cached unit info available".into() })
}

#[tauri::command]
pub async fn get_key3(state: State<'_, GuiAppState>) -> CommandResult<Key3Leaders> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_key3(),
        |d| cache.save_key3(d),
        api.fetch_key3(&org_guid),
    ).await?;

    data.ok_or_else(|| CommandError { message: "No cached Key 3 data available".into() })
}

#[tauri::command]
pub async fn get_org_profile(state: State<'_, GuiAppState>) -> CommandResult<OrgProfile> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_org_profile(),
        |d| cache.save_org_profile(d),
        api.fetch_org_profile(&org_guid),
    ).await?;

    data.ok_or_else(|| CommandError { message: "No cached org profile available".into() })
}

#[tauri::command]
pub async fn get_commissioners(state: State<'_, GuiAppState>) -> CommandResult<Vec<Commissioner>> {
    let config = state.config.lock().await;
    let offline = config.offline_mode;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_commissioners(),
        |d| cache.save_commissioners(d),
        api.fetch_commissioners(&org_guid),
    ).await?;

    Ok(data.unwrap_or_default())
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

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_advancement_dashboard(),
        |d| cache.save_advancement_dashboard(d),
        api.fetch_advancement_dashboard(&org_guid),
    ).await?;

    data.ok_or_else(|| CommandError { message: "No cached advancement data available".into() })
}

#[tauri::command]
pub async fn get_youth_ranks(
    user_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<RankProgressDisplay>> {
    let offline = is_offline(&state).await;
    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_youth_ranks(user_id),
        |d| cache.save_youth_ranks(user_id, d),
        api.fetch_youth_ranks(user_id),
    ).await?;

    Ok(data.map(|d| d.iter().map(RankProgressDisplay::from).collect()).unwrap_or_default())
}

#[tauri::command]
pub async fn get_youth_merit_badges(
    user_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<YouthBadgesResponse> {
    let offline = is_offline(&state).await;
    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_youth_merit_badges(user_id),
        |d| cache.save_youth_merit_badges(user_id, d),
        api.fetch_youth_merit_badges(user_id),
    ).await?;

    Ok(data.map(|mut d| {
        let summary = MeritBadgeProgress::summarize(&d);
        d.sort_by(MeritBadgeProgress::cmp_by_progress);
        let badges = d.iter().map(MeritBadgeDisplay::from).collect();
        YouthBadgesResponse { badges, summary }
    }).unwrap_or_else(|| YouthBadgesResponse {
        badges: vec![],
        summary: Default::default(),
    }))
}

#[tauri::command]
pub async fn get_youth_leadership(
    user_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<LeadershipDisplay>> {
    let offline = is_offline(&state).await;
    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_youth_leadership(user_id),
        |d| cache.save_youth_leadership(user_id, d),
        api.fetch_youth_leadership(user_id),
    ).await?;

    Ok(data.map(|mut d| {
        LeadershipPosition::sort_for_display(&mut d);
        d.iter().map(LeadershipDisplay::from).collect()
    }).unwrap_or_default())
}

#[tauri::command]
pub async fn get_youth_awards(
    user_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<AwardDisplay>> {
    let offline = is_offline(&state).await;
    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_youth_awards(user_id),
        |d| cache.save_youth_awards(user_id, d),
        api.fetch_youth_awards(user_id),
    ).await?;

    Ok(data.map(|d| d.iter().map(AwardDisplay::from).collect()).unwrap_or_default())
}

#[tauri::command]
pub async fn get_rank_requirements(
    user_id: i64,
    rank_id: i64,
    state: State<'_, GuiAppState>,
) -> CommandResult<Vec<RankRequirementDisplay>> {
    let offline = is_offline(&state).await;
    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;
    let data = fetch_with_cache(
        offline,
        || cache.load_rank_requirements(user_id, rank_id),
        |d| cache.save_rank_requirements(user_id, rank_id, d),
        api.fetch_rank_requirements(user_id, rank_id),
    ).await?;

    let mut reqs: Vec<RankRequirementDisplay> = data
        .map(|d| d.iter().map(RankRequirementDisplay::from).collect())
        .unwrap_or_default();
    sort_requirements(&mut reqs);
    Ok(reqs)
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
// Offline Caching Command
// ============================================================================

/// Cache all data for offline use: base data + per-youth ranks/badges/requirements + event RSVP.
#[tauri::command]
pub async fn cache_for_offline(
    app: tauri::AppHandle,
    state: State<'_, GuiAppState>,
) -> CommandResult<String> {
    let config = state.config.lock().await;
    let org_guid = config.organization_guid.clone().unwrap_or_default();
    drop(config);

    let session = state.session.lock().await;
    let user_id = session.user_id().unwrap_or(0);
    drop(session);

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;

    let app_handle = app.clone();
    let result = trailcache_core::cache::cache_all_for_offline(
        &api,
        &cache,
        &org_guid,
        user_id,
        |progress| {
            let _ = app_handle.emit("refresh-progress", RefreshProgress {
                step: progress.description,
                current: progress.current,
                total: progress.total,
                error: None,
            });
        },
    )
    .await
    .map_err(|e| CommandError { message: e.to_string() })?;

    Ok(result)
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

    let api = state.api_client.lock().await;
    let cache = state.cache.lock().await;

    let app_handle = app.clone();
    let result = trailcache_core::cache::refresh_base_data(
        &api,
        &cache,
        &org_guid,
        user_id,
        |progress| {
            let _ = app_handle.emit("refresh-progress", RefreshProgress {
                step: progress.description,
                current: progress.current,
                total: progress.total,
                error: None,
            });
        },
    ).await;

    let successes = result.successes;
    let errors = result.errors;
    let total = 10u32;

    // Emit errors for any failed sources
    for err in &errors {
        let _ = app.emit("refresh-progress", RefreshProgress {
            step: err.clone(),
            current: total,
            total,
            error: Some(err.clone()),
        });
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
