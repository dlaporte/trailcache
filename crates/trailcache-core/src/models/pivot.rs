//! Pivot aggregation logic shared across all interfaces.
//!
//! Groups youth by their highest rank or by badges they're working on,
//! producing intermediate types that each interface converts for display.

use std::collections::HashMap;
use super::advancement::{RankProgress, MeritBadgeProgress, ScoutRank};
use super::person::Youth;

// ============================================================================
// Rank Pivot
// ============================================================================

/// A scout grouped under their highest completed/awarded rank.
#[derive(Debug, Clone)]
pub struct RankGroupEntry {
    pub user_id: i64,
    pub display_name: String,
    /// The rank that placed them in this group. None for crossover scouts.
    pub rank: Option<RankProgress>,
}

/// A rank with its grouped scouts.
#[derive(Debug, Clone)]
pub struct RankGroup {
    pub rank_name: String,
    pub rank_order: usize,
    pub scouts: Vec<RankGroupEntry>,
}

/// Group youth by their highest completed/awarded rank.
///
/// Each youth appears in exactly one group — the group for their highest
/// completed or awarded rank. Youth with no completed ranks are placed
/// in a "Crossover" group.
///
/// Scouts within each group are sorted: completed first (newest to oldest),
/// then awarded (newest to oldest), then by name.
pub fn group_youth_by_rank(
    youth: &[Youth],
    all_ranks: &HashMap<i64, Vec<RankProgress>>,
) -> Vec<RankGroup> {
    let mut by_rank: HashMap<String, Vec<RankGroupEntry>> = HashMap::new();
    let mut crossover: Vec<RankGroupEntry> = Vec::new();

    for y in youth {
        let uid = match y.user_id {
            Some(id) => id,
            None => {
                crossover.push(RankGroupEntry {
                    user_id: 0,
                    display_name: y.display_name(),
                    rank: None,
                });
                continue;
            }
        };

        let ranks = match all_ranks.get(&uid) {
            Some(r) => r,
            None => {
                crossover.push(RankGroupEntry {
                    user_id: uid,
                    display_name: y.display_name(),
                    rank: None,
                });
                continue;
            }
        };

        // Find the highest completed or awarded rank
        let current_rank = ranks
            .iter()
            .filter(|r| r.is_completed() || r.is_awarded())
            .max_by_key(|r| r.level);

        if let Some(rank) = current_rank {
            by_rank
                .entry(rank.rank_name.clone())
                .or_default()
                .push(RankGroupEntry {
                    user_id: uid,
                    display_name: y.display_name(),
                    rank: Some(rank.clone()),
                });
        } else {
            // No completed/awarded rank — Crossover scout.
            // Store their lowest in-progress rank so drill-down can
            // show requirement completion status.
            let lowest_rank = ranks
                .iter()
                .min_by_key(|r| r.sort_order())
                .cloned();
            crossover.push(RankGroupEntry {
                user_id: uid,
                display_name: y.display_name(),
                rank: lowest_rank,
            });
        }
    }

    if !crossover.is_empty() {
        by_rank.insert("Crossover".to_string(), crossover);
    }

    // Sort scouts within each rank: completed first, then awarded, newest to oldest
    for scouts in by_rank.values_mut() {
        scouts.sort_by(|a, b| {
            match (&a.rank, &b.rank) {
                (None, None) => a.display_name.cmp(&b.display_name),
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(ar), Some(br)) => {
                    let aa = ar.is_awarded();
                    let ba = br.is_awarded();
                    if aa != ba {
                        return aa.cmp(&ba); // completed (not awarded) first
                    }
                    let da = if aa {
                        ar.date_awarded.as_deref().unwrap_or("")
                    } else {
                        ar.date_completed.as_deref().unwrap_or("")
                    };
                    let db = if ba {
                        br.date_awarded.as_deref().unwrap_or("")
                    } else {
                        br.date_completed.as_deref().unwrap_or("")
                    };
                    db.cmp(da)
                }
            }
        });
    }

    // Sort groups by rank order (Scout -> Eagle)
    let mut groups: Vec<RankGroup> = by_rank
        .into_iter()
        .map(|(name, scouts)| {
            let order = ScoutRank::parse(Some(&name)).order();
            RankGroup {
                rank_name: name,
                rank_order: order,
                scouts,
            }
        })
        .collect();
    groups.sort_by_key(|g| g.rank_order);
    groups
}

// ============================================================================
// Badge Pivot
// ============================================================================

/// A scout grouped under a badge they're working on or completed.
#[derive(Debug, Clone)]
pub struct BadgeGroupEntry {
    pub user_id: i64,
    pub display_name: String,
    pub badge: MeritBadgeProgress,
}

/// A badge with its grouped scouts.
#[derive(Debug, Clone)]
pub struct BadgeGroup {
    pub badge_name: String,
    pub is_eagle_required: bool,
    pub scouts: Vec<BadgeGroupEntry>,
}

/// Group youth badges into badge-centric groups.
///
/// Each badge gets a group containing all scouts who are working on or
/// have completed that badge. Scouts within each group are sorted:
/// in-progress first (by percent desc), then completed (by date desc).
pub fn group_youth_by_badge(
    youth: &[Youth],
    all_badges: &HashMap<i64, Vec<MeritBadgeProgress>>,
) -> Vec<BadgeGroup> {
    let mut by_badge: HashMap<String, (bool, Vec<BadgeGroupEntry>)> = HashMap::new();

    for y in youth {
        let uid = match y.user_id {
            Some(id) => id,
            None => continue,
        };

        let badges = match all_badges.get(&uid) {
            Some(b) => b,
            None => continue,
        };

        for badge in badges {
            if badge.name.is_empty() {
                continue;
            }
            let entry = by_badge
                .entry(badge.name.clone())
                .or_insert_with(|| (badge.is_eagle_required.unwrap_or(false), Vec::new()));
            if badge.is_eagle_required.unwrap_or(false) {
                entry.0 = true;
            }
            entry.1.push(BadgeGroupEntry {
                user_id: uid,
                display_name: y.display_name(),
                badge: badge.clone(),
            });
        }
    }

    // Sort scouts within each badge
    for (_, scouts) in by_badge.values_mut() {
        scouts.sort_by(|a, b| {
            let a_done = a.badge.is_completed() || a.badge.is_awarded();
            let b_done = b.badge.is_completed() || b.badge.is_awarded();
            if a_done != b_done {
                return a_done.cmp(&b_done); // in-progress first
            }
            if !a_done {
                let pa = a.badge.percent_completed.unwrap_or(0.0);
                let pb = b.badge.percent_completed.unwrap_or(0.0);
                pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                // Completed/awarded: sort by date desc
                let da = a.badge.awarded_date.as_deref()
                    .or(a.badge.date_completed.as_deref())
                    .unwrap_or("");
                let db = b.badge.awarded_date.as_deref()
                    .or(b.badge.date_completed.as_deref())
                    .unwrap_or("");
                db.cmp(da)
            }
        });
    }

    let mut groups: Vec<BadgeGroup> = by_badge
        .into_iter()
        .map(|(name, (eagle, scouts))| BadgeGroup {
            badge_name: name,
            is_eagle_required: eagle,
            scouts,
        })
        .collect();
    groups.sort_by(|a, b| a.badge_name.to_lowercase().cmp(&b.badge_name.to_lowercase()));
    groups
}
