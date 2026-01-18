use crate::model::AppEntry;
use directories::BaseDirs;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct GopherConfig {
    rom_dir: Option<PathBuf>,
}

/// Scan for gopher64 games based on configuration
pub fn scan_gopher64_games() -> Vec<AppEntry> {
    let mut games = Vec::new();
    if !is_gopher64_available() {
        tracing::warn!("gopher64 is not installed; skipping ROM scan");
        return games;
    }

    let Some(config_path) =
        BaseDirs::new().map(|dirs| dirs.config_dir().join("gopher64/config.json"))
    else {
        tracing::warn!("Could not determine config directory for gopher64");
        return games;
    };

    // 1. Parse Config
    let Some(rom_dir) = parse_config(&config_path) else {
        return games;
    };

    // 2. Scan ROM Directory
    if let Ok(entries) = fs::read_dir(rom_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_valid_extension(&path) {
                if let Some(game) = process_rom(&path) {
                    games.push(game);
                }
            }
        }
    }

    games
}

fn is_gopher64_available() -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    for path in env::split_paths(&paths) {
        let candidate = path.join("gopher64");
        if candidate.is_file() {
            return true;
        }
    }

    false
}

fn parse_config(path: &Path) -> Option<PathBuf> {
    let content = fs::read_to_string(path).ok()?;
    let config: GopherConfig = serde_json::from_str(&content).ok()?;
    config.rom_dir.filter(|p| p.exists())
}

fn is_valid_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_ascii_lowercase().as_str(),
                "z64" | "n64" | "v64" | "zip" | "7z"
            )
        })
        .unwrap_or(false)
}

fn process_rom(path: &Path) -> Option<AppEntry> {
    // Prefer filename-based title extraction as it yields better results for scene releases
    // than internal ROM headers (which are often shortened/uppercase).
    let title = extract_title_from_filename(path);

    let cover = find_cover(path);

    // Construct Executable Command
    // We strictly use the absolute path in quotes to handle spaces
    let exec = format!("gopher64 --fullscreen \"{}\"", path.to_string_lossy());

    // Launch Key for history tracking
    let launch_key = format!(
        "gopher64:{}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );

    tracing::info!("Discovered N64 ROM: '{}'", title);

    Some(AppEntry::new(title, exec, cover).with_launch_key(launch_key))
}

fn find_cover(rom_path: &Path) -> Option<String> {
    ["png", "jpg", "jpeg", "webp"].iter().find_map(|ext| {
        let image_path = rom_path.with_extension(ext);
        if image_path.exists() {
            Some(image_path.to_string_lossy().to_string())
        } else {
            None
        }
    })
}

/// Extract clean title from filename.
/// Removes text in () and [] and extension.
fn extract_title_from_filename(path: &Path) -> String {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();

    let mut title = String::with_capacity(stem.len());
    let mut depth_round = 0i32;
    let mut depth_square = 0i32;

    for c in stem.chars() {
        match c {
            '(' => depth_round += 1,
            ')' => depth_round = depth_round.saturating_sub(1),
            '[' => depth_square += 1,
            ']' => depth_square = depth_square.saturating_sub(1),
            c if depth_round == 0 && depth_square == 0 => title.push(c),
            _ => {}
        }
    }

    title.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn temp_dir() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("launcher_test_gopher_{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_extract_title_from_filename() {
        assert_eq!(
            extract_title_from_filename(Path::new("Mario Kart 64 (E) (V1.1) [!].z64")),
            "Mario Kart 64"
        );
        assert_eq!(
            extract_title_from_filename(Path::new("Super Mario 64.z64")),
            "Super Mario 64"
        );
    }

    #[test]
    fn test_parse_config_with_existing_rom_dir() {
        let dir = temp_dir();
        let config_path = dir.join("config.json");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = serde_json::json!({
            "rom_dir": rom_dir
        });

        fs::write(&config_path, content.to_string()).unwrap();

        let parsed_rom_dir = parse_config(&config_path);
        assert_eq!(parsed_rom_dir, Some(rom_dir));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_with_nonexistent_rom_dir() {
        let dir = temp_dir();
        let config_path = dir.join("config.json");
        let rom_dir = "/nonexistent/path/that/does/not/exist";

        let content = serde_json::json!({
            "rom_dir": rom_dir
        });

        fs::write(&config_path, content.to_string()).unwrap();

        let parsed_rom_dir = parse_config(&config_path);
        assert_eq!(parsed_rom_dir, None);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_with_malformed_json() {
        let dir = temp_dir();
        let config_path = dir.join("config.json");

        fs::write(&config_path, "{ invalid json }").unwrap();

        let parsed_rom_dir = parse_config(&config_path);
        assert_eq!(parsed_rom_dir, None);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_with_missing_rom_dir_field() {
        let dir = temp_dir();
        let config_path = dir.join("config.json");

        let content = serde_json::json!({
            "some_other_field": "value"
        });

        fs::write(&config_path, content.to_string()).unwrap();

        let parsed_rom_dir = parse_config(&config_path);
        assert_eq!(parsed_rom_dir, None);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_with_null_rom_dir() {
        let dir = temp_dir();
        let config_path = dir.join("config.json");

        let content = serde_json::json!({
            "rom_dir": null
        });

        fs::write(&config_path, content.to_string()).unwrap();

        let parsed_rom_dir = parse_config(&config_path);
        assert_eq!(parsed_rom_dir, None);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_nonexistent_file() {
        let dir = temp_dir();
        let config_path = dir.join("does_not_exist.json");

        let parsed_rom_dir = parse_config(&config_path);
        assert_eq!(parsed_rom_dir, None);

        let _ = fs::remove_dir_all(dir);
    }
}
