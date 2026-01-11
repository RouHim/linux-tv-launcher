use crate::model::AppEntry;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub apps: Vec<AppEntry>,
    pub steamgriddb_api_key: Option<String>,
}

/// Returns the project directories for this application.
/// Centralized to ensure consistent paths across all modules.
pub fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("com", "linux-tv-launcher", "linux-tv-launcher")
        .context("Could not determine project directories")
}

pub fn config_path() -> Result<PathBuf> {
    let proj_dirs = project_dirs()?;
    let config_dir = proj_dirs.config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(config_dir).context("Failed to create config directory")?;
    }
    Ok(config_dir.join("config.json"))
}

/// Load application configuration from disk
pub fn load_config() -> Result<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&path).context("Failed to read config file")?;

    // Try parsing as new config structure
    if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
        return Ok(config);
    }

    // Fallback: Try parsing as legacy array
    let apps: Vec<AppEntry> = serde_json::from_str(&content)
        .context("Failed to parse config file (tried both v2 and legacy format)")?;

    Ok(AppConfig {
        apps,
        steamgriddb_api_key: None,
    })
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    let path = config_path()?;
    let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    fs::write(&path, content).context("Failed to write config file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AppEntry;

    #[test]
    fn test_serialization_v2() {
        let config = AppConfig {
            apps: vec![
                AppEntry::new("A".into(), "e1".into(), None),
                AppEntry::new("B".into(), "e2".into(), None),
            ],
            steamgriddb_api_key: Some("test-key".into()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.apps, loaded.apps);
        assert_eq!(config.steamgriddb_api_key, loaded.steamgriddb_api_key);
    }

    #[test]
    fn test_legacy_deserialization() {
        let apps = vec![
            AppEntry::new("A".into(), "e1".into(), None),
            AppEntry::new("B".into(), "e2".into(), None),
        ];

        // Simulate old config format (JSON array)
        let json = serde_json::to_string(&apps).unwrap();

        // This logic mimics load_config's fallback
        let loaded_config: AppConfig = if let Ok(config) = serde_json::from_str::<AppConfig>(&json)
        {
            config
        } else {
            let apps: Vec<AppEntry> = serde_json::from_str(&json).unwrap();
            AppConfig {
                apps,
                steamgriddb_api_key: None,
            }
        };

        assert_eq!(apps, loaded_config.apps);
        assert_eq!(loaded_config.steamgriddb_api_key, None);
    }
}
