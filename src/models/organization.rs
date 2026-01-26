// Allow dead code: API response structs have fields for completeness
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patrol {
    #[serde(rename = "subUnitGuid")]
    pub guid: String,
    #[serde(rename = "subUnitName")]
    pub name: String,
    #[serde(rename = "memberCount")]
    pub member_count: Option<i32>,
    #[serde(rename = "patrolLeaderUserId")]
    pub patrol_leader_user_id: Option<i64>,
    #[serde(rename = "patrolLeaderName")]
    pub patrol_leader_name: Option<String>,
}

impl Patrol {
    pub fn display_member_count(&self) -> String {
        match self.member_count {
            Some(count) => format!("{} members", count),
            None => "Unknown".to_string(),
        }
    }
}
