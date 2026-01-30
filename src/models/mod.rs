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
pub mod unit;

pub use advancement::{
    format_date, AdvancementDashboard, Award, LeadershipPosition, MeritBadgeCatalogEntry,
    MeritBadgeProgress, MeritBadgeRequirement, MeritBadgeWithRequirements, RankProgress,
    RankRequirement, RankWithRequirements, RanksResponse, ReadyToAward,
};
pub use event::{Event, EventGuest, EventSortColumn, RsvpStatus};
pub use organization::Patrol;
pub use person::{Adult, OrgAdultsResponse, OrgYouthsResponse, Parent, ParentResponse, ScoutSortColumn, UnitYouthsResponse, Youth};
pub use unit::{Commissioner, Key3Leaders, Leader, MeetingLocation, OrgProfile, UnitContact, UnitInfo};
