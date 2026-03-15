//! Shared refresh orchestration for base data.
//!
//! Fetches all 10 base data sources in parallel, saves to cache,
//! and returns results so callers can update in-memory state.

use tracing::warn;

use crate::api::ApiClient;
use crate::cache::CacheManager;
use crate::cache::offline::CacheProgress;
use crate::models::{
    Adult, AdvancementDashboard, Commissioner, Event, Key3Leaders,
    OrgProfile, Parent, Patrol, UnitInfo, Youth,
};

/// Results from a base data refresh.
pub struct RefreshResult {
    pub youth: Option<Vec<Youth>>,
    pub adults: Option<Vec<Adult>>,
    pub events: Option<Vec<Event>>,
    pub patrols: Option<Vec<Patrol>>,
    pub unit_info: Option<UnitInfo>,
    pub key3: Option<Key3Leaders>,
    pub commissioners: Option<Vec<Commissioner>>,
    pub org_profile: Option<OrgProfile>,
    pub parents: Option<Vec<Parent>>,
    pub advancement: Option<AdvancementDashboard>,
    pub errors: Vec<String>,
    pub successes: u32,
}

/// Fetch all 10 base data sources in parallel, save to cache, deduplicate adults.
///
/// Progress is reported via `on_progress` so each frontend can display it.
/// Returns all fetched data so callers can update in-memory state without re-reading cache.
pub async fn refresh_base_data(
    api: &ApiClient,
    cache: &CacheManager,
    org_guid: &str,
    user_id: i64,
    on_progress: impl Fn(CacheProgress),
) -> RefreshResult {
    let total = 10u32;
    let mut errors: Vec<String> = Vec::new();
    let mut successes = 0u32;

    on_progress(CacheProgress {
        current: 0,
        total,
        description: "Refreshing data...".into(),
    });

    // Fetch all 10 sources in parallel
    let (
        youth_res, adults_res, events_res, patrols_res, unit_info_res,
        key3_res, commissioners_res, org_profile_res, parents_res, advancement_res,
    ) = tokio::join!(
        api.fetch_youth(org_guid),
        api.fetch_adults(org_guid),
        api.fetch_events(user_id),
        api.fetch_patrols(org_guid),
        api.fetch_unit_pin(org_guid),
        api.fetch_key3(org_guid),
        api.fetch_commissioners(org_guid),
        api.fetch_org_profile(org_guid),
        api.fetch_parents(org_guid),
        api.fetch_advancement_dashboard(org_guid),
    );

    // Process each result: save to cache, collect data and errors
    macro_rules! process {
        ($label:expr, $step:expr, $result:expr, $save:expr) => {{
            on_progress(CacheProgress {
                current: $step,
                total,
                description: format!("Processing {}...", $label),
            });
            match $result {
                Ok(data) => {
                    let _ = $save(&data);
                    successes += 1;
                    Some(data)
                }
                Err(e) => {
                    let msg = format!("{}: {}", $label, e);
                    warn!("{}", msg);
                    errors.push(msg);
                    None
                }
            }
        }};
    }

    let youth = process!("scouts", 1, youth_res, |d: &Vec<Youth>| cache.save_youth(d));

    // Deduplicate adults before saving
    let adults = {
        on_progress(CacheProgress {
            current: 2,
            total,
            description: "Processing adults...".into(),
        });
        match adults_res {
            Ok(data) => {
                let deduped = Adult::deduplicate(data);
                if let Err(e) = cache.save_adults(&deduped) {
                    warn!("Failed to save adults to cache: {e}");
                }
                successes += 1;
                Some(deduped)
            }
            Err(e) => {
                let msg = format!("adults: {}", e);
                warn!("{}", msg);
                errors.push(msg);
                None
            }
        }
    };

    let events = process!("events", 3, events_res, |d: &Vec<Event>| cache.save_events(d));
    let patrols = process!("patrols", 4, patrols_res, |d: &Vec<Patrol>| cache.save_patrols(d));
    let unit_info = process!("unit info", 5, unit_info_res, |d: &UnitInfo| cache.save_unit_info(d));
    let key3 = process!("Key 3", 6, key3_res, |d: &Key3Leaders| cache.save_key3(d));
    let commissioners = process!("commissioners", 7, commissioners_res, |d: &Vec<Commissioner>| cache.save_commissioners(d));
    let org_profile = process!("org profile", 8, org_profile_res, |d: &OrgProfile| cache.save_org_profile(d));
    let parents = process!("parents", 9, parents_res, |d: &Vec<Parent>| cache.save_parents(d));
    let advancement = process!("advancement", 10, advancement_res, |d: &AdvancementDashboard| cache.save_advancement_dashboard(d));

    on_progress(CacheProgress {
        current: total,
        total,
        description: "Refresh complete".into(),
    });

    RefreshResult {
        youth,
        adults,
        events,
        patrols,
        unit_info,
        key3,
        commissioners,
        org_profile,
        parents,
        advancement,
        errors,
        successes,
    }
}
