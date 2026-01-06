use crate::model::AppEntry;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "linux-tv-launcher", "linux-tv-launcher")
        .context("Could not determine config directory")?;

    let config_dir = proj_dirs.config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(config_dir).context("Failed to create config directory")?;
    }

    Ok(config_dir.join("config.json"))
}

pub fn config_path() -> Result<PathBuf> {
    get_config_path()
}

pub fn load_config() -> Result<Vec<AppEntry>> {
    let path = get_config_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path).context("Failed to read config file")?;
    let apps: Vec<AppEntry> =
        serde_json::from_str(&content).context("Failed to parse config file")?;

    Ok(apps)
}

#[cfg(test)]
mod tests {
    use crate::model::AppEntry;

    // To test storage properly we might need to mock the project dirs or just test the serialization logic separately.
    // Here we trust serde works and file I/O works.
    // Ideally we would inject the path, but ProjectDirs is hardcoded above.
    // We can refactor get_config_path to accept an optional root or override env vars.
    // For now, simple serialization test.

    #[test]
    fn test_serialization() {
        let apps = vec![
            AppEntry::new("A".into(), "e1".into(), None),
            AppEntry::new("B".into(), "e2".into(), None),
        ];

        let json = serde_json::to_string(&apps).unwrap();
        let loaded: Vec<AppEntry> = serde_json::from_str(&json).unwrap();

        assert_eq!(apps, loaded);
    }
}
