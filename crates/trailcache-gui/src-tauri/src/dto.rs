//! Display Transfer Objects (DTOs) that wrap core types with pre-computed display fields.
//!
//! These DTOs are what gets serialized to the frontend, so the GUI only needs
//! to render values — no display logic in JavaScript.

use std::collections::HashMap;

use serde::Serialize;
use ts_rs::TS;

use trailcache_core::models::{
    Adult, Award, BadgeSummary, DEFAULT_AWARD_STATUS, DEFAULT_BADGE_STATUS, Event, EventGuest,
    LeadershipPosition, MeritBadgeProgress, MeritBadgeRequirement, Parent, RankProgress,
    RankRequirement, ScoutRank, StatusCategory, Youth,
};
use trailcache_core::models::event::InvitedUser;
use trailcache_core::models::advancement::format_date;
use trailcache_core::utils::format::{check_expiration, strip_html, ExpirationStatus};

// ============================================================================
// Youth & Adult DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct YouthDisplay {
    // Pre-computed display fields
    pub display_name: String,
    pub short_name: String,
    pub rank: String,
    pub rank_short: String,
    pub rank_order: usize,
    pub patrol: String,
    pub age: String,
    pub grade: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub position: Option<String>,
    pub position_sort_key: usize,
    pub position_display: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub membership_status: Option<String>,
    pub membership_style: Option<String>,
    pub membership_sort_date: Option<String>,
    pub has_membership_issue: bool,
    pub is_membership_expired: bool,
    // Raw fields needed for identification / client-side operations
    #[ts(type = "number | null")]
    pub user_id: Option<i64>,
    pub first_name: String,
    pub last_name: String,
    pub nick_name: Option<String>,
    pub member_id: Option<String>,
    pub person_guid: Option<String>,
}

impl From<&Youth> for YouthDisplay {
    fn from(y: &Youth) -> Self {
        let dob = y.date_of_birth().map(|d| d.format("%b %d, %Y").to_string());

        let addr_line1 = y.primary_address_info.as_ref()
            .and_then(|a| a.address1.clone())
            .filter(|a| !a.trim().is_empty());

        let addr_line2 = y.primary_address_info.as_ref()
            .and_then(|a| a.city_state_zip());

        let (membership_status, membership_style, has_membership_issue, is_membership_expired) = y.registration_expires()
            .and_then(|exp| check_expiration(&exp))
            .map(|(status, formatted)| {
                let issue = matches!(status, ExpirationStatus::Expired | ExpirationStatus::ExpiringSoon);
                let expired = matches!(status, ExpirationStatus::Expired);
                (
                    Some(status.format_expiry(&formatted)),
                    Some(status.style_class().to_string()),
                    issue,
                    expired,
                )
            })
            .unwrap_or((None, None, false, false));

        Self {
            display_name: y.display_name(),
            short_name: y.short_name(),
            rank: y.rank(),
            rank_short: y.rank_short(),
            rank_order: ScoutRank::parse(y.current_rank.as_deref()).order(),
            patrol: y.patrol(),
            age: y.age_str(),
            grade: y.grade_str(),
            email: y.email(),
            phone: y.phone(),
            position: y.position_display(),
            position_sort_key: y.position_sort_key(),
            position_display: y.position_display_with_patrol(),
            date_of_birth: dob,
            gender: y.gender.clone(),
            address_line1: addr_line1,
            address_line2: addr_line2,
            membership_status,
            membership_style,
            membership_sort_date: y.registration_expires()
                .map(|d| d[..10.min(d.len())].to_string()),
            has_membership_issue,
            is_membership_expired,
            user_id: y.user_id,
            first_name: y.first_name.clone(),
            last_name: y.last_name.clone(),
            nick_name: y.nick_name.clone(),
            member_id: y.member_id.clone(),
            person_guid: y.person_guid.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ParentDisplay {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    #[ts(type = "number | null")]
    pub youth_user_id: Option<i64>,
}

impl From<&Parent> for ParentDisplay {
    fn from(p: &Parent) -> Self {
        let phone = p.phone();
        let addr1 = p.address1.clone().filter(|a| !a.trim().is_empty());
        let addr2 = p.city_state_zip();

        Self {
            name: p.full_name(),
            email: p.email.clone(),
            phone,
            address_line1: addr1,
            address_line2: addr2,
            youth_user_id: p.youth_user_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AdultDisplay {
    pub display_name: String,
    pub role: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub ypt_status: Option<String>,
    pub ypt_style: Option<String>,
    pub ypt_sort_date: Option<String>,
    pub position_trained: Option<String>,
    pub membership_status: Option<String>,
    pub membership_style: Option<String>,
    pub membership_sort_date: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub has_membership_issue: bool,
    pub has_ypt_issue: bool,
    pub is_membership_expired: bool,
    pub is_ypt_expired: bool,
    pub needs_training: bool,
    // Raw fields
    #[ts(type = "number | null")]
    pub user_id: Option<i64>,
    pub first_name: String,
    pub last_name: String,
    pub position: Option<String>,
    pub member_id: Option<String>,
}

impl From<&Adult> for AdultDisplay {
    fn from(a: &Adult) -> Self {
        let (ypt_status, ypt_style, has_ypt_issue, is_ypt_expired) = a.ypt_expired_date.as_ref()
            .and_then(|exp| check_expiration(exp))
            .map(|(status, formatted)| {
                let issue = matches!(status, ExpirationStatus::Expired | ExpirationStatus::ExpiringSoon);
                let expired = matches!(status, ExpirationStatus::Expired);
                (
                    Some(status.format_ypt(&formatted)),
                    Some(status.membership_style_class().to_string()),
                    issue,
                    expired,
                )
            })
            .unwrap_or((None, None, false, false));

        let (membership_status, membership_style, has_membership_issue, is_membership_expired) = a.registrar_info.as_ref()
            .and_then(|r| r.registration_expire_dt.as_ref())
            .and_then(|exp| check_expiration(exp))
            .map(|(status, formatted)| {
                let issue = matches!(status, ExpirationStatus::Expired | ExpirationStatus::ExpiringSoon);
                let expired = matches!(status, ExpirationStatus::Expired);
                (
                    Some(status.format_expiry(&formatted)),
                    Some(status.membership_style_class().to_string()),
                    issue,
                    expired,
                )
            })
            .unwrap_or((None, None, false, false));

        let position_trained = Some(a.position_trained_display().to_string())
            .filter(|s| s != "-");

        let ypt_sort_date = a.ypt_expired_date.clone();
        let membership_sort_date = a.registrar_info.as_ref()
            .and_then(|r| r.registration_expire_dt.clone())
            .map(|d| d[..10.min(d.len())].to_string());

        let addr_line1 = a.primary_address_info.as_ref()
            .and_then(|addr| addr.address1.clone())
            .filter(|s| !s.trim().is_empty());

        let addr_line2 = a.primary_address_info.as_ref()
            .and_then(|addr| addr.city_state_zip());

        let needs_training = a.is_position_trained() == Some(false);

        Self {
            display_name: a.display_name(),
            role: a.role(),
            email: a.email(),
            phone: a.phone(),
            ypt_status,
            ypt_style,
            ypt_sort_date,
            position_trained,
            membership_status,
            membership_style,
            membership_sort_date,
            address_line1: addr_line1,
            address_line2: addr_line2,
            has_membership_issue,
            has_ypt_issue,
            is_membership_expired,
            is_ypt_expired,
            needs_training,
            user_id: a.user_id,
            first_name: a.first_name.clone(),
            last_name: a.last_name.clone(),
            position: a.position.clone(),
            member_id: a.member_id.clone(),
        }
    }
}

// ============================================================================
// Event DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct EventDisplay {
    // Pre-computed display fields
    pub formatted_date: String,
    pub formatted_start: String,
    pub formatted_end: String,
    pub derived_type: String,
    pub description_text: String,
    pub going_count: i32,
    pub not_going_count: i32,
    // Raw fields for sorting / identification
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub location: Option<String>,
    pub event_type: Option<String>,
    pub rsvp: bool,
    #[ts(type = "number | null")]
    pub unit_id: Option<i64>,
}

impl From<&Event> for EventDisplay {
    fn from(e: &Event) -> Self {
        Self {
            formatted_date: e.formatted_date(),
            formatted_start: e.formatted_start_datetime(),
            formatted_end: e.formatted_end_datetime(),
            derived_type: e.derived_type().to_string(),
            description_text: e
                .description
                .as_deref()
                .map(strip_html)
                .unwrap_or_default(),
            going_count: e.going_count(),
            not_going_count: e.not_going_count(),
            id: e.id,
            name: e.name.clone(),
            start_date: e.start_date.clone(),
            end_date: e.end_date.clone(),
            location: e.location.clone(),
            event_type: e.event_type.clone(),
            rsvp: e.rsvp,
            unit_id: e.unit_id(),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct EventGuestDisplay {
    pub display_name: String,
    pub status: String,
    pub is_youth: bool,
    pub is_going: bool,
    pub is_not_going: bool,
    #[ts(type = "number")]
    pub user_id: i64,
    pub first_name: String,
    pub last_name: String,
}

impl From<&EventGuest> for EventGuestDisplay {
    fn from(g: &EventGuest) -> Self {
        use trailcache_core::models::event::RsvpStatus;
        let rsvp = g.status();
        Self {
            display_name: g.display_name(),
            status: rsvp.to_string(),
            is_youth: g.is_youth.unwrap_or(false),
            is_going: matches!(rsvp, RsvpStatus::Going),
            is_not_going: matches!(rsvp, RsvpStatus::NotGoing),
            user_id: g.user_id,
            first_name: g.first_name.clone(),
            last_name: g.last_name.clone(),
        }
    }
}

impl EventGuestDisplay {
    pub fn from_invited_user(u: &InvitedUser) -> Self {
        use trailcache_core::models::event::RsvpStatus;
        let rsvp = u.status();
        Self {
            display_name: u.display_name(),
            status: rsvp.to_string(),
            is_youth: !u.is_adult,
            is_going: matches!(rsvp, RsvpStatus::Going),
            is_not_going: matches!(rsvp, RsvpStatus::NotGoing),
            user_id: u.user_id,
            first_name: u.first_name.clone(),
            last_name: u.last_name.clone(),
        }
    }
}

// ============================================================================
// Advancement DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RankProgressDisplay {
    pub rank_name: String,
    #[ts(type = "number")]
    pub rank_id: i64,
    pub sort_order: i32,
    pub is_completed: bool,
    pub is_awarded: bool,
    pub formatted_date_completed: String,
    pub formatted_date_awarded: String,
    pub display_date: String,
    pub status_label: String,
    pub status_category: String,
    pub progress_percent: Option<i32>,
    // Raw fields for drill-down
    pub date_completed: Option<String>,
    pub date_awarded: Option<String>,
    pub percent_completed: Option<f32>,
    pub requirements_completed: Option<i32>,
    pub requirements_total: Option<i32>,
}

impl From<&RankProgress> for RankProgressDisplay {
    fn from(r: &RankProgress) -> Self {
        let (cat, label) = r.status_display();
        Self {
            rank_name: r.rank_name.clone(),
            rank_id: r.rank_id,
            sort_order: r.sort_order(),
            is_completed: r.is_completed(),
            is_awarded: r.is_awarded(),
            formatted_date_completed: format_date(r.date_completed.as_deref()),
            formatted_date_awarded: format_date(r.date_awarded.as_deref()),
            display_date: r.display_date(),
            status_label: label,
            status_category: cat.as_str().to_string(),
            progress_percent: r.progress_percent(),
            date_completed: r.date_completed.clone(),
            date_awarded: r.date_awarded.clone(),
            percent_completed: r.percent_completed,
            requirements_completed: r.requirements_completed,
            requirements_total: r.requirements_total,
        }
    }
}


#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct MeritBadgeDisplay {
    pub name: String,
    #[ts(type = "number")]
    pub id: i64,
    pub is_completed: bool,
    pub is_awarded: bool,
    pub is_eagle_required: bool,
    pub progress_percent: Option<i32>,
    pub formatted_date_completed: String,
    pub status: String,
    pub status_label: String,
    pub status_category: String,
    pub counselor_name: String,
    pub counselor_phone: String,
    pub sort_date: String,
    // Raw fields
    pub date_completed: Option<String>,
    pub percent_completed: Option<f32>,
}

impl From<&MeritBadgeProgress> for MeritBadgeDisplay {
    fn from(b: &MeritBadgeProgress) -> Self {
        let counselor = b.assigned_counselor.as_ref();
        let sort_date = b.sort_date();
        let (cat, label) = b.status_display();
        Self {
            name: b.name.clone(),
            id: b.id,
            is_completed: b.is_completed(),
            is_awarded: b.is_awarded(),
            is_eagle_required: b.is_eagle_required.unwrap_or(false),
            progress_percent: b.progress_percent(),
            formatted_date_completed: format_date(b.date_completed.as_deref()),
            status: b
                .status
                .clone()
                .unwrap_or_else(|| DEFAULT_BADGE_STATUS.to_string()),
            status_label: label,
            status_category: cat.as_str().to_string(),
            counselor_name: counselor.map(|c| c.full_name()).unwrap_or_default(),
            counselor_phone: counselor
                .and_then(|c| c.phone())
                .unwrap_or("")
                .to_string(),
            sort_date,
            date_completed: b.date_completed.clone(),
            percent_completed: b.percent_completed,
        }
    }
}

/// Wrapper DTO that bundles badges with a computed summary.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct YouthBadgesResponse {
    pub badges: Vec<MeritBadgeDisplay>,
    pub summary: BadgeSummary,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RankRequirementDisplay {
    pub number: String,
    pub text: String,
    pub is_completed: bool,
    pub formatted_date_completed: String,
}

impl trailcache_core::models::HasRequirementNumber for RankRequirementDisplay {
    fn requirement_number_str(&self) -> String {
        self.number.clone()
    }
}

impl From<&RankRequirement> for RankRequirementDisplay {
    fn from(r: &RankRequirement) -> Self {
        Self {
            number: r.number(),
            text: strip_html(&r.full_text()),
            is_completed: r.is_completed(),
            formatted_date_completed: format_date(r.date_completed.as_deref()),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct MeritBadgeRequirementDisplay {
    pub number: String,
    pub text: String,
    pub is_completed: bool,
    pub formatted_date_completed: String,
}

impl trailcache_core::models::HasRequirementNumber for MeritBadgeRequirementDisplay {
    fn requirement_number_str(&self) -> String {
        self.number.clone()
    }
}

impl From<&MeritBadgeRequirement> for MeritBadgeRequirementDisplay {
    fn from(r: &MeritBadgeRequirement) -> Self {
        Self {
            number: r.number(),
            text: strip_html(&r.full_text()),
            is_completed: r.is_completed(),
            formatted_date_completed: format_date(r.date_completed.as_deref()),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LeadershipDisplay {
    pub name: String,
    pub date_range: String,
    pub days_display: String,
    pub is_current: bool,
    pub patrol: Option<String>,
    pub sort_date: String,
}

impl From<&LeadershipPosition> for LeadershipDisplay {
    fn from(l: &LeadershipPosition) -> Self {
        let sort_date = l.start_date.clone().unwrap_or_default();
        Self {
            name: l.name().to_string(),
            date_range: l.date_range(),
            days_display: l.days_display(),
            is_current: l.is_current(),
            patrol: l.patrol.clone(),
            sort_date,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AwardDisplay {
    pub name: String,
    pub date_display: String,
    pub type_display: String,
    pub is_awarded: bool,
    pub is_completed: bool,
    pub status: String,
    pub progress_percent: Option<i32>,
}

impl From<&Award> for AwardDisplay {
    fn from(a: &Award) -> Self {
        Self {
            name: a.name().to_string(),
            date_display: a.date_display(),
            type_display: a.type_display().to_string(),
            is_awarded: a.is_awarded(),
            is_completed: a.is_completed(),
            status: a
                .status
                .clone()
                .unwrap_or_else(|| DEFAULT_AWARD_STATUS.to_string()),
            progress_percent: a.progress_percent(),
        }
    }
}

// ============================================================================
// Badge Requirements Response DTO
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BadgeRequirementsResponseDisplay {
    pub requirements: Vec<MeritBadgeRequirementDisplay>,
    pub version: Option<String>,
    pub counselor_name: String,
    pub counselor_phone: String,
    pub counselor_email: Option<String>,
}

// ============================================================================
// Pivot Data DTOs (Pre-aggregated)
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RankPivotEntry {
    pub rank_name: String,
    pub rank_order: usize,
    pub count: usize,
    pub scouts: Vec<RankPivotScout>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RankPivotScout {
    #[ts(type = "number")]
    pub user_id: i64,
    #[ts(type = "number")]
    pub rank_id: i64,
    pub display_name: String,
    pub formatted_date_awarded: String,
    pub formatted_date_completed: String,
    pub sort_date: String,
    pub status_label: String,
    pub status_category: String,
    pub percent_completed: Option<f32>,
    pub is_completed: bool,
    pub is_awarded: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BadgePivotEntry {
    pub badge_name: String,
    pub is_eagle_required: bool,
    pub count: usize,
    pub scouts: Vec<BadgePivotScout>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BadgePivotScout {
    #[ts(type = "number")]
    pub user_id: i64,
    #[ts(type = "number")]
    pub badge_id: i64,
    pub display_name: String,
    pub formatted_date_completed: String,
    pub formatted_date_awarded: String,
    pub sort_date: String,
    pub status_label: String,
    pub status_category: String,
    pub percent_completed: Option<f32>,
    pub is_completed: bool,
    pub is_awarded: bool,
    pub status: String,
}

// ============================================================================
// Pivot Aggregation — delegates to trailcache_core::models::pivot
// ============================================================================

pub fn build_rank_pivot(
    all_ranks: &HashMap<i64, Vec<RankProgress>>,
    youth: &[Youth],
) -> Vec<RankPivotEntry> {
    use trailcache_core::models::pivot::group_youth_by_rank;

    group_youth_by_rank(youth, all_ranks)
        .into_iter()
        .map(|g| {
            let scouts = g.scouts.into_iter().map(|s| {
                let r = s.rank.as_ref();
                let (cat, label) = r.map(|r| r.status_display()).unwrap_or((StatusCategory::None, String::new()));
                RankPivotScout {
                    user_id: s.user_id,
                    rank_id: r.map(|r| r.rank_id).unwrap_or(0),
                    display_name: s.display_name,
                    formatted_date_awarded: format_date(r.and_then(|r| r.date_awarded.as_deref())),
                    formatted_date_completed: format_date(r.and_then(|r| r.date_completed.as_deref())),
                    sort_date: r.map(|r| r.sort_date()).unwrap_or_default(),
                    status_label: label,
                    status_category: cat.as_str().to_string(),
                    percent_completed: r.and_then(|r| r.percent_completed),
                    is_completed: r.map(|r| r.is_completed()).unwrap_or(false),
                    is_awarded: r.map(|r| r.is_awarded()).unwrap_or(false),
                }
            }).collect::<Vec<_>>();
            let count = scouts.len();
            RankPivotEntry {
                rank_name: g.rank_name,
                rank_order: g.rank_order,
                count,
                scouts,
            }
        })
        .collect()
}

pub fn build_badge_pivot(
    all_badges: &HashMap<i64, Vec<MeritBadgeProgress>>,
    youth: &[Youth],
) -> Vec<BadgePivotEntry> {
    use trailcache_core::models::pivot::group_youth_by_badge;

    group_youth_by_badge(youth, all_badges)
        .into_iter()
        .map(|g| {
            let scouts = g.scouts.into_iter().map(|s| {
                let sort_date = s.badge.sort_date();
                let (cat, label) = s.badge.status_display();
                BadgePivotScout {
                    user_id: s.user_id,
                    badge_id: s.badge.id,
                    display_name: s.display_name,
                    formatted_date_completed: format_date(s.badge.date_completed.as_deref()),
                    formatted_date_awarded: format_date(s.badge.awarded_date.as_deref()),
                    sort_date,
                    status_label: label,
                    status_category: cat.as_str().to_string(),
                    percent_completed: s.badge.percent_completed,
                    is_completed: s.badge.is_completed(),
                    is_awarded: s.badge.is_awarded(),
                    status: s.badge.status.unwrap_or_else(|| DEFAULT_BADGE_STATUS.to_string()),
                }
            }).collect::<Vec<_>>();
            let count = scouts.len();
            BadgePivotEntry {
                badge_name: g.badge_name,
                is_eagle_required: g.is_eagle_required,
                count,
                scouts,
            }
        })
        .collect()
}
