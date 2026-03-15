//! Application configuration management.
//!
//! This module handles loading and saving the application configuration,
//! which includes the organization GUID, unit name, and last used username.
//!
//! Configuration is stored at `~/.config/trailcache/config.json`.

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Application name used for config/cache directory paths
const APP_NAME: &str = "trailcache";

/// Config file name
const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub organization_guid: Option<String>,
    pub unit_name: Option<String>,
    pub last_username: Option<String>,
    #[serde(default)]
    pub offline_mode: bool,
    /// Explicit config directory override (for mobile platforms where `dirs` doesn't work).
    #[serde(skip)]
    pub config_dir_override: Option<PathBuf>,
    /// Explicit cache directory override (for mobile platforms).
    #[serde(skip)]
    pub cache_dir_override: Option<PathBuf>,
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

    /// Load config from an explicit directory (for mobile where `dirs` doesn't work).
    pub fn load_from(config_dir: PathBuf) -> Result<Self> {
        let path = config_dir.join(APP_NAME).join(CONFIG_FILE);
        if path.exists() {
            let contents = std::fs::read_to_string(&path)?;
            let mut config: Config = serde_json::from_str(&contents)?;
            config.config_dir_override = Some(config_dir);
            Ok(config)
        } else {
            Ok(Self {
                config_dir_override: Some(config_dir),
                ..Self::default()
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = self.resolved_config_path()?;
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

    fn resolved_config_path(&self) -> Result<PathBuf> {
        if let Some(ref dir) = self.config_dir_override {
            Ok(dir.join(APP_NAME).join(CONFIG_FILE))
        } else {
            Self::config_path()
        }
    }

    pub fn cache_dir(&self) -> Result<PathBuf> {
        let base = if let Some(ref dir) = self.cache_dir_override {
            dir.clone()
        } else {
            dirs::cache_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not find cache directory"))?
        };

        let mut path = base.join(APP_NAME);
        if let Some(ref org) = self.organization_guid {
            path = path.join(org);
        }
        Ok(path)
    }

    /// Set an explicit cache directory (for mobile).
    pub fn set_cache_dir(&mut self, dir: PathBuf) {
        self.cache_dir_override = Some(dir);
    }
}
