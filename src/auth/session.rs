// Allow dead code: Infrastructure methods for future use
#![allow(dead_code)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Session file name in cache directory
const SESSION_FILE: &str = "session.json";

/// Token expiry time in minutes.
/// Scouting.org tokens expire after ~30 minutes of inactivity.
const TOKEN_EXPIRY_MINUTES: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub token: String,
    pub user_id: i64,
    pub person_guid: String,
    pub organization_guid: String,
    pub username: String,
    pub created_at: DateTime<Utc>,
}

/// Buffer time before expiry to trigger refresh (5 minutes)
const TOKEN_REFRESH_BUFFER_MINUTES: i64 = 5;

impl SessionData {
    pub fn is_expired(&self) -> bool {
        let expiry = self.created_at + Duration::minutes(TOKEN_EXPIRY_MINUTES);
        Utc::now() > expiry
    }

    /// Check if the session will expire soon and should be refreshed
    pub fn needs_refresh(&self) -> bool {
        let refresh_at = self.created_at + Duration::minutes(TOKEN_EXPIRY_MINUTES - TOKEN_REFRESH_BUFFER_MINUTES);
        Utc::now() > refresh_at
    }

    pub fn time_until_expiry(&self) -> Duration {
        let expiry = self.created_at + Duration::minutes(TOKEN_EXPIRY_MINUTES);
        expiry - Utc::now()
    }

    /// Get minutes remaining until expiry (for display)
    pub fn minutes_until_expiry(&self) -> i64 {
        self.time_until_expiry().num_minutes().max(0)
    }
}

pub struct Session {
    cache_dir: PathBuf,
    pub data: Option<SessionData>,
}

impl Session {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            data: None,
        }
    }

    /// Load session from disk
    pub fn load(&mut self) -> Result<bool> {
        let path = self.session_path();
        if path.exists() {
            let contents = std::fs::read_to_string(&path)
                .context("Failed to read session file")?;
            let data: SessionData = serde_json::from_str(&contents)
                .context("Failed to parse session file")?;

            if !data.is_expired() {
                self.data = Some(data);
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Save session to disk
    pub fn save(&self) -> Result<()> {
        if let Some(ref data) = self.data {
            let path = self.session_path();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let contents = serde_json::to_string_pretty(data)?;
            std::fs::write(path, contents)?;
        }
        Ok(())
    }

    /// Clear session data
    pub fn clear(&mut self) -> Result<()> {
        self.data = None;
        let path = self.session_path();
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Update session with new data
    pub fn update(&mut self, data: SessionData) {
        self.data = Some(data);
    }

    /// Get the bearer token if session is valid
    pub fn token(&self) -> Option<&str> {
        self.data.as_ref().map(|d| d.token.as_str())
    }

    /// Get the user ID if session exists
    pub fn user_id(&self) -> Option<i64> {
        self.data.as_ref().map(|d| d.user_id)
    }

    /// Check if session is valid (exists and not expired)
    pub fn is_valid(&self) -> bool {
        self.data.as_ref().map(|d| !d.is_expired()).unwrap_or(false)
    }

    fn session_path(&self) -> PathBuf {
        self.cache_dir.join(SESSION_FILE)
    }
}
