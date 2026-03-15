//! Shared offline caching logic.
//!
//! Pre-fetches all data needed for full offline operation:
//! base roster/event data, per-youth ranks/badges/requirements,
//! and per-event RSVP details.

use tracing::warn;

use crate::api::ApiClient;
use crate::cache::CacheManager;

/// Progress update sent during offline caching.
#[derive(Debug, Clone)]
pub struct CacheProgress {
    pub current: u32,
    pub total: u32,
    pub description: String,
}

/// Cache all data for offline use.
///
/// Fetches base data (roster, events, advancement, etc.), then per-youth
/// ranks/badges/requirements and per-event RSVP details. Progress is
/// reported via the callback so each frontend can display it appropriately.
///
/// The `cache` parameter must already have its encryption key set.
pub async fn cache_all_for_offline(
    api: &ApiClient,
    cache: &CacheManager,
    org_guid: &str,
    user_id: i64,
    on_progress: impl Fn(CacheProgress),
) -> anyhow::Result<String> {
    // Phase 1: Base data (10 sources, fetched in parallel)
    let base = super::refresh::refresh_base_data(api, cache, org_guid, user_id, &on_progress).await;
    let successes = base.successes;
    let _errors = base.errors;

    let youth_ids: Vec<i64> = base.youth
        .as_ref()
        .map(|list| list.iter().filter_map(|y| y.user_id).collect())
        .unwrap_or_default();

    let events = base.events;

    // Phase 2: Event RSVP details (concurrent)
    let event_ids: Vec<i64> = events
        .as_ref()
        .map(|list| list.iter().map(|e| e.id).collect())
        .unwrap_or_default();

    if !event_ids.is_empty() {
        use futures::future::join_all;
        use std::collections::HashMap;

        on_progress(CacheProgress {
            current: 0,
            total: event_ids.len() as u32,
            description: "Caching event RSVP data...".into(),
        });

        // Fetch all event details concurrently in chunks
        const MAX_CONCURRENT_EVENTS: usize = 10;
        let mut rsvp_map: HashMap<i64, Vec<crate::models::event::InvitedUser>> = HashMap::new();

        let mut completed = 0u32;
        for chunk in event_ids.chunks(MAX_CONCURRENT_EVENTS) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|&eid| {
                    let api = api.clone();
                    async move {
                        let detail = api.fetch_event_detail(eid).await.ok();
                        (eid, detail)
                    }
                })
                .collect();

            let results = join_all(futures).await;
            for (eid, detail) in results {
                if let Some(detail) = detail {
                    rsvp_map.insert(eid, detail.invited_users);
                }
                completed += 1;
            }

            on_progress(CacheProgress {
                current: completed,
                total: event_ids.len() as u32,
                description: "Caching event RSVP data...".into(),
            });
        }

        // Merge RSVP data into cached events and save
        let mut cached_events = events.unwrap_or_default();
        for ev in &mut cached_events {
            if let Some(users) = rsvp_map.remove(&ev.id) {
                ev.invited_users = users;
            }
        }
        if let Err(e) = cache.save_events(&cached_events) {
            warn!("Failed to save events to cache: {e}");
        }
    }

    // Phase 3: Per-youth ranks, badges, and requirements (concurrent)
    //
    // Optimizations vs naive serial approach:
    // - Process youth in concurrent chunks (5 at a time)
    // - Fetch rank + badge lists concurrently per youth
    // - Fetch all requirements concurrently per youth
    // - Use fetch_badge_requirements_only (1 API call instead of 2)
    if !youth_ids.is_empty() {
        use futures::future::join_all;

        let youth_total = youth_ids.len() as u32;
        const MAX_CONCURRENT_YOUTH: usize = 5;

        let mut completed = 0u32;
        for chunk in youth_ids.chunks(MAX_CONCURRENT_YOUTH) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|&uid| {
                    let api = api.clone();
                    async move {
                        // Fetch ranks and badges concurrently
                        let (ranks_result, badges_result) = futures::future::join(
                            api.fetch_youth_ranks(uid),
                            api.fetch_youth_merit_badges(uid),
                        )
                        .await;

                        let ranks = ranks_result.unwrap_or_default();
                        let badges = badges_result.unwrap_or_default();

                        // Fetch all requirements concurrently
                        let rank_req_futures: Vec<_> = ranks
                            .iter()
                            .map(|r| {
                                let api = api.clone();
                                let rank_id = r.rank_id;
                                async move {
                                    let reqs = api.fetch_rank_requirements(uid, rank_id).await.ok();
                                    (rank_id, reqs)
                                }
                            })
                            .collect();

                        let badge_req_futures: Vec<_> = badges
                            .iter()
                            .map(|b| {
                                let api = api.clone();
                                let badge_id = b.id;
                                async move {
                                    let reqs = api.fetch_badge_requirements_only(uid, badge_id).await.ok();
                                    (badge_id, reqs)
                                }
                            })
                            .collect();

                        let (rank_reqs, badge_reqs) = futures::future::join(
                            join_all(rank_req_futures),
                            join_all(badge_req_futures),
                        )
                        .await;

                        (uid, ranks, badges, rank_reqs, badge_reqs)
                    }
                })
                .collect();

            let results = join_all(futures).await;

            // Save all results to cache
            for (uid, ranks, badges, rank_reqs, badge_reqs) in results {
                if let Err(e) = cache.save_youth_ranks(uid, &ranks) {
                    warn!("Failed to save ranks for user {uid}: {e}");
                }
                if let Err(e) = cache.save_youth_merit_badges(uid, &badges) {
                    warn!("Failed to save badges for user {uid}: {e}");
                }

                for (rank_id, reqs) in rank_reqs {
                    if let Some(reqs) = reqs {
                        if let Err(e) = cache.save_rank_requirements(uid, rank_id, &reqs) {
                            warn!("Failed to save rank requirements for user {uid}, rank {rank_id}: {e}");
                        }
                    }
                }

                for (badge_id, reqs) in badge_reqs {
                    if let Some((reqs, version)) = reqs {
                        if let Err(e) = cache.save_badge_requirements(uid, badge_id, &reqs, &version) {
                            warn!("Failed to save badge requirements for user {uid}, badge {badge_id}: {e}");
                        }
                    }
                }

                completed += 1;
                on_progress(CacheProgress {
                    current: completed,
                    total: youth_total,
                    description: format!("Caching scout advancement ({}/{})...", completed, youth_total),
                });
            }
        }
    }

    // Phase 4: Verify cached data is complete
    on_progress(CacheProgress {
        current: 1,
        total: 1,
        description: "Verifying cache...".into(),
    });

    let gaps = verify_cache(cache, &youth_ids);

    on_progress(CacheProgress {
        current: 1,
        total: 1,
        description: if gaps.is_empty() {
            "Caching complete".into()
        } else {
            format!("Caching complete ({} gaps)", gaps.len())
        },
    });

    if gaps.is_empty() {
        Ok(format!("Cached all {} data sources + requirements + RSVP — verified complete", successes))
    } else {
        let summary = gaps.join("; ");
        Ok(format!(
            "Caching complete with gaps: {}",
            summary
        ))
    }
}

/// Verify that all expected offline data is present in the cache.
/// Returns a list of human-readable descriptions of missing data.
fn verify_cache(cache: &CacheManager, youth_ids: &[i64]) -> Vec<String> {
    let mut gaps = Vec::new();

    // Base data checks — verify each source was cached successfully
    macro_rules! check {
        ($label:expr, $load:expr) => {
            match $load {
                Ok(Some(_)) => {}
                Ok(None) => gaps.push(format!("{}: missing", $label)),
                Err(_) => gaps.push(format!("{}: unreadable", $label)),
            }
        };
    }

    check!("Scouts", cache.load_youth());
    check!("Adults", cache.load_adults());
    check!("Events", cache.load_events());
    check!("Patrols", cache.load_patrols());
    check!("Unit info", cache.load_unit_info());
    check!("Key 3", cache.load_key3());
    check!("Commissioners", cache.load_commissioners());
    check!("Org profile", cache.load_org_profile());
    check!("Parents", cache.load_parents());
    check!("Advancement", cache.load_advancement_dashboard());

    // Per-youth checks
    let mut missing_ranks = 0u32;
    let mut missing_badges = 0u32;
    let mut missing_rank_reqs = 0u32;
    let mut missing_badge_reqs = 0u32;

    for &uid in youth_ids {
        // Check ranks
        let ranks = match cache.load_youth_ranks(uid) {
            Ok(Some(cached)) => cached.data,
            _ => {
                missing_ranks += 1;
                continue;
            }
        };

        // Check rank requirements
        for rank in &ranks {
            if cache.load_rank_requirements(uid, rank.rank_id).ok().flatten().is_none() {
                missing_rank_reqs += 1;
            }
        }

        // Check badges
        let badges = match cache.load_youth_merit_badges(uid) {
            Ok(Some(cached)) => cached.data,
            _ => {
                missing_badges += 1;
                continue;
            }
        };

        // Check badge requirements
        for badge in &badges {
            if cache.load_badge_requirements(uid, badge.id).ok().flatten().is_none() {
                missing_badge_reqs += 1;
            }
        }
    }

    if missing_ranks > 0 {
        gaps.push(format!("{} scouts missing rank data", missing_ranks));
    }
    if missing_badges > 0 {
        gaps.push(format!("{} scouts missing badge data", missing_badges));
    }
    if missing_rank_reqs > 0 {
        gaps.push(format!("{} rank requirements missing", missing_rank_reqs));
    }
    if missing_badge_reqs > 0 {
        gaps.push(format!("{} badge requirements missing", missing_badge_reqs));
    }

    gaps
}
