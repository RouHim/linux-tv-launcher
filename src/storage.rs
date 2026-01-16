use crate::model::AppEntry;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub apps: Vec<AppEntry>,
    pub steamgriddb_api_key: Option<String>,
    /// Stores launch timestamps for games (keyed by game identifier)
    /// Games are scanned fresh each startup, so we persist their launch history separately
    #[serde(default)]
    pub game_launch_history: HashMap<String, i64>,
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
    let config = serde_json::from_str::<AppConfig>(&content).context("Failed to parse config")?;
    Ok(config)
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
        let mut game_history = HashMap::new();
        game_history.insert("game1".to_string(), 1234567890_i64);

        let config = AppConfig {
            apps: vec![
                AppEntry::new("A".into(), "e1".into(), None).with_launch_key("desktop:e1".into()),
                AppEntry::new("B".into(), "e2".into(), None),
            ],
            steamgriddb_api_key: Some("test-key".into()),
            game_launch_history: game_history,
        };

        let json = serde_json::to_string(&config).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.apps, loaded.apps);
        assert_eq!(config.steamgriddb_api_key, loaded.steamgriddb_api_key);
        assert_eq!(config.game_launch_history, loaded.game_launch_history);
    }
}
