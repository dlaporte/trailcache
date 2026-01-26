//! Application configuration management.
//!
//! This module handles loading and saving the application configuration,
//! which includes the organization GUID, unit name, and last used username.
//!
//! Configuration is stored at `~/.config/scoutbook-tui/config.json`.

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Application name used for config/cache directory paths
const APP_NAME: &str = "scoutbook-tui";

/// Config file name
const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub organization_guid: Option<String>,
    pub unit_name: Option<String>,
    pub last_username: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let contents = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&contents)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        Ok(config_dir.join(APP_NAME).join(CONFIG_FILE))
    }

    pub fn cache_dir(&self) -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find cache directory"))?;

        let mut path = cache_dir.join(APP_NAME);
        if let Some(ref org) = self.organization_guid {
            path = path.join(org);
        }
        Ok(path)
    }
}
