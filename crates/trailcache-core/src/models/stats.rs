//! Aggregate statistics computed over collections of model objects.
//!
//! These functions consolidate business logic that was previously
//! duplicated between the TUI and GUI interfaces.

use std::collections::HashMap;

use crate::models::person::{Adult, Youth};
use crate::models::advancement::ScoutRank;
use crate::utils::format::{check_expiration, ExpirationStatus};

// ============================================================================
// Training Statistics
// ============================================================================

/// Aggregated training status across all adults in a unit.
#[derive(Debug, Clone, Default)]
pub struct TrainingStats {
    pub ypt_current: usize,
    pub ypt_expiring: usize,
    pub ypt_expired: usize,
    /// (display_name, status_text) for adults with YPT issues, sorted by name.
    pub ypt_issues: Vec<(String, String)>,
    pub position_trained: usize,
    pub position_not_trained: usize,
    /// Display names of adults not position-trained, sorted.
    pub position_not_trained_list: Vec<String>,
}

impl TrainingStats {
    /// Compute training statistics from a slice of adults.
    pub fn from_adults(adults: &[Adult]) -> Self {
        let mut stats = TrainingStats::default();

        for adult in adults {
            // YPT status
            if let Some(ref exp_str) = adult.ypt_expired_date {
                if let Some((status, formatted)) = check_expiration(exp_str) {
                    match status {
                        ExpirationStatus::Expired => {
                            stats.ypt_expired += 1;
                            stats.ypt_issues.push((adult.display_name(), format!("Expired {}", formatted)));
                        }
                        ExpirationStatus::ExpiringSoon => {
                            stats.ypt_expiring += 1;
                            stats.ypt_issues.push((adult.display_name(), format!("Expires {}", formatted)));
                        }
                        ExpirationStatus::Active => {
                            stats.ypt_current += 1;
                        }
                    }
                }
            }

            // Position training
            match adult.is_position_trained() {
                Some(true) => stats.position_trained += 1,
                Some(false) => {
                    stats.position_not_trained += 1;
                    stats.position_not_trained_list.push(adult.display_name());
                }
                None => {}
            }
        }

        stats.ypt_issues.sort_by(|a, b| a.0.cmp(&b.0));
        stats.position_not_trained_list.sort();
        stats
    }
}

// ============================================================================
// Membership Renewal Statistics
// ============================================================================

/// Aggregated membership renewal status across youth and adults.
#[derive(Debug, Clone, Default)]
pub struct RenewalStats {
    pub scouts_current: usize,
    pub scouts_expiring: usize,
    pub scouts_expired: usize,
    /// (display_name, status_text) for scouts with renewal issues, sorted by name.
    pub scout_issues: Vec<(String, String)>,
    pub adults_current: usize,
    pub adults_expiring: usize,
    pub adults_expired: usize,
    /// (display_name, status_text) for adults with renewal issues, sorted by name.
    pub adult_issues: Vec<(String, String)>,
}

impl RenewalStats {
    /// Compute renewal statistics from youth and adult slices.
    pub fn compute(youth: &[Youth], adults: &[Adult]) -> Self {
        let mut stats = RenewalStats::default();

        // Scout renewals
        for y in youth {
            let exp_str = y.registrar_info.as_ref()
                .and_then(|r| r.registration_expire_dt.as_ref());

            match exp_str.and_then(|s| check_expiration(s)) {
                Some((ExpirationStatus::Expired, formatted)) => {
                    stats.scouts_expired += 1;
                    stats.scout_issues.push((y.display_name(), format!("Expired {}", formatted)));
                }
                Some((ExpirationStatus::ExpiringSoon, formatted)) => {
                    stats.scouts_expiring += 1;
                    stats.scout_issues.push((y.display_name(), format!("Expires {}", formatted)));
                }
                Some((ExpirationStatus::Active, _)) | None => {
                    stats.scouts_current += 1;
                }
            }
        }

        // Adult renewals
        for a in adults {
            let exp_str = a.registrar_info.as_ref()
                .and_then(|r| r.registration_expire_dt.as_ref());

            match exp_str.and_then(|s| check_expiration(s)) {
                Some((ExpirationStatus::Expired, formatted)) => {
                    stats.adults_expired += 1;
                    stats.adult_issues.push((a.display_name(), format!("Expired {}", formatted)));
                }
                Some((ExpirationStatus::ExpiringSoon, formatted)) => {
                    stats.adults_expiring += 1;
                    stats.adult_issues.push((a.display_name(), format!("Expires {}", formatted)));
                }
                Some((ExpirationStatus::Active, _)) | None => {
                    stats.adults_current += 1;
                }
            }
        }

        stats.scout_issues.sort_by(|a, b| a.0.cmp(&b.0));
        stats.adult_issues.sort_by(|a, b| a.0.cmp(&b.0));
        stats
    }
}

// ============================================================================
// Patrol Rank Breakdown
// ============================================================================

/// Rank breakdown within a single patrol.
#[derive(Debug, Clone)]
pub struct PatrolBreakdown {
    pub member_count: usize,
    /// Rank name → count of members at that rank.
    pub rank_counts: HashMap<String, usize>,
}

/// Compute patrol rank breakdown from a slice of youth.
///
/// Returns a map of patrol_name → PatrolBreakdown.
/// Youth with empty or missing patrol names are skipped.
/// Rank names are normalized via `ScoutRank::parse()` and stored using `display_name()`
/// (e.g. "Crossover" for unknown rank).
pub fn patrol_rank_breakdown(youth: &[Youth]) -> HashMap<String, PatrolBreakdown> {
    let mut result: HashMap<String, PatrolBreakdown> = HashMap::new();

    for y in youth {
        let patrol = match y.patrol_name.as_deref() {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        let scout_rank = ScoutRank::parse(y.current_rank.as_deref());
        let rank = scout_rank.display_name();

        let entry = result.entry(patrol.to_string()).or_insert_with(|| PatrolBreakdown {
            member_count: 0,
            rank_counts: HashMap::new(),
        });
        entry.member_count += 1;
        *entry.rank_counts.entry(rank.to_string()).or_insert(0) += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::person::RegistrarInfo;

    fn make_adult(ypt_expired: Option<&str>, position_trained: Option<&str>, reg_expire: Option<&str>) -> Adult {
        Adult {
            person_guid: None, member_id: None, person_full_name: None,
            first_name: "Jane".to_string(), middle_name: None, last_name: "Doe".to_string(),
            nick_name: None, position: Some("Scoutmaster".to_string()), position_id: None,
            key3: None,
            position_trained: position_trained.map(|s| s.to_string()),
            ypt_status: None,
            ypt_completed_date: None,
            ypt_expired_date: ypt_expired.map(|s| s.to_string()),
            registrar_info: reg_expire.map(|exp| RegistrarInfo {
                date_of_birth: None, registration_id: None, registration_status_id: None,
                registration_status: None, registration_effective_dt: None,
                registration_expire_dt: Some(exp.to_string()),
                renewal_status: None, is_yearly_membership: None,
                is_manually_ended: None, is_auto_renewal_opted_out: None,
            }),
            primary_email_info: None, primary_phone_info: None,
            primary_address_info: None, user_id: None, email: None, phone_number: None,
        }
    }

    fn make_youth(patrol: Option<&str>, rank: Option<&str>, reg_expire: Option<&str>) -> Youth {
        Youth {
            person_guid: None, member_id: None, person_full_name: None,
            first_name: "John".to_string(), middle_name: None, last_name: "Smith".to_string(),
            nick_name: None, gender: None, name_suffix: None, ethnicity: None,
            grade: None, grade_id: None, position: None, position_id: None,
            program_id: None, program: None,
            registrar_info: reg_expire.map(|exp| RegistrarInfo {
                date_of_birth: None, registration_id: None, registration_status_id: None,
                registration_status: None, registration_effective_dt: None,
                registration_expire_dt: Some(exp.to_string()),
                renewal_status: None, is_yearly_membership: None,
                is_manually_ended: None, is_auto_renewal_opted_out: None,
            }),
            primary_email_info: None, primary_phone_info: None, primary_address_info: None,
            user_id: None, email: None, phone_number: None,
            patrol_name: patrol.map(|s| s.to_string()), patrol_guid: None,
            is_patrol_leader: None, current_rank: rank.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_training_stats() {
        let adults = vec![
            make_adult(Some("2020-01-01"), Some("Trained"), None),    // expired YPT, trained
            make_adult(Some("2099-01-01"), Some("Not Trained"), None), // active YPT, not trained
            make_adult(None, None, None),                              // no YPT, unknown training
        ];
        let stats = TrainingStats::from_adults(&adults);
        assert_eq!(stats.ypt_expired, 1);
        assert_eq!(stats.ypt_current, 1);
        assert_eq!(stats.position_trained, 1);
        assert_eq!(stats.position_not_trained, 1);
        assert_eq!(stats.ypt_issues.len(), 1);
        assert_eq!(stats.position_not_trained_list.len(), 1);
    }

    #[test]
    fn test_renewal_stats() {
        let youth = vec![
            make_youth(None, None, Some("2020-01-01")), // expired
            make_youth(None, None, Some("2099-01-01")), // current
            make_youth(None, None, None),                // no reg info, assume current
        ];
        let adults = vec![
            make_adult(None, None, Some("2020-06-01")), // expired
        ];
        let stats = RenewalStats::compute(&youth, &adults);
        assert_eq!(stats.scouts_expired, 1);
        assert_eq!(stats.scouts_current, 2);
        assert_eq!(stats.adults_expired, 1);
        assert_eq!(stats.scout_issues.len(), 1);
        assert_eq!(stats.adult_issues.len(), 1);
    }

    #[test]
    fn test_patrol_rank_breakdown() {
        let youth = vec![
            make_youth(Some("Eagle"), Some("Eagle"), None),
            make_youth(Some("Eagle"), Some("First Class"), None),
            make_youth(Some("Hawk"), Some("Scout"), None),
            make_youth(None, Some("Star"), None), // no patrol, skipped
        ];
        let breakdown = patrol_rank_breakdown(&youth);
        assert_eq!(breakdown.len(), 2); // Eagle and Hawk patrols
        let eagle = breakdown.get("Eagle").unwrap();
        assert_eq!(eagle.member_count, 2);
        assert_eq!(*eagle.rank_counts.get("Eagle").unwrap(), 1);
        assert_eq!(*eagle.rank_counts.get("First Class").unwrap(), 1);
        let hawk = breakdown.get("Hawk").unwrap();
        assert_eq!(hawk.member_count, 1);
    }
}
