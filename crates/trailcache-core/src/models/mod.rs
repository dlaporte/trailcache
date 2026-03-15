//! Data models for Scout troop entities.
//!
//! This module contains all the data structures used to represent
//! troop data including:
//!
//! - `Youth`, `Adult`, `Parent`: Person models with contact info
//! - `Event`, `EventGuest`: Calendar events and RSVP tracking
//! - `Patrol`: Troop organization structure
//! - Advancement types: `RankProgress`, `MeritBadgeProgress`, etc.
//! - Unit types: `Key3Leaders`, `UnitInfo`, `OrgProfile`, `Commissioner`

pub mod advancement;
pub mod event;
pub mod organization;
pub mod person;
pub mod pivot;
pub mod sorting;
pub mod stats;
pub mod unit;

pub use advancement::{
    format_date, AdvancementDashboard, Award, BadgeSummary, DEFAULT_AWARD_STATUS,
    DEFAULT_BADGE_STATUS, EAGLE_REQUIRED_COUNT, LeadershipPosition, MeritBadgeCatalogEntry,
    MeritBadgeProgress, MeritBadgeRequirement, MeritBadgeWithRequirements, RankProgress,
    RankRequirement, RankWithRequirements, RanksResponse, ReadyToAward, ScoutRank,
    StatusCategory, STATUS_AWARDED, STATUS_COUNSELOR_APPROVED, STATUS_LEADER_APPROVED,
    UNKNOWN_DATE,
};
pub use event::{Event, EventGuest, EventSortColumn, RsvpStatus};
pub use organization::Patrol;
pub use person::{Adult, AdultSortColumn, DEFAULT_ADULT_ROLE, DISPLAY_NOT_TRAINED, OrgAdultsResponse, OrgYouthsResponse, Parent, ParentResponse, PROGRAM_ID_SCOUTS_BSA, PROGRAM_SCOUTS_BSA, ScoutSortColumn, UnitYouthsResponse, Youth, youth_position_list, YOUTH_POSITION_PRIORITY};
pub use sorting::{sort_requirements, HasRequirementNumber};
pub use stats::{patrol_rank_breakdown, PatrolBreakdown, RenewalStats, TrainingStats};
pub use unit::{Commissioner, Key3Leaders, Leader, MeetingLocation, OrgProfile, UnitContact, UnitInfo};
