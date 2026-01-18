use crate::model::AppEntry;
use directories::BaseDirs;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Scan for simple64 games based on configuration
pub fn scan_simple64_games() -> Vec<AppEntry> {
    let mut games = Vec::new();
    if !is_simple64_available() {
        tracing::warn!("simple64-gui is not installed; skipping ROM scan");
        return games;
    }

    let Some(config_path) = BaseDirs::new().map(|dirs| dirs.config_dir().join("simple64/gui.conf"))
    else {
        tracing::warn!("Could not determine config directory for simple64");
        return games;
    };

    // 1. Parse Config
    let (rom_dir, recent_roms) = parse_config(&config_path);

    // 2. Ensure Fullscreen
    if let Some(config_dir) = config_path.parent() {
        ensure_fullscreen_config(config_dir);
    }

    let mut seen_paths = HashSet::new();

    // 2. Scan ROM Directory
    if let Some(dir) = rom_dir {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if is_valid_extension(&path) && seen_paths.insert(path.clone()) {
                    if let Some(game) = process_rom(&path) {
                        games.push(game);
                    }
                }
            }
        }
    }

    // 3. Scan Recent ROMs
    for path in recent_roms {
        if path.exists() && is_valid_extension(&path) && seen_paths.insert(path.clone()) {
            if let Some(game) = process_rom(&path) {
                games.push(game);
            }
        }
    }

    games
}

fn is_simple64_available() -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    for path in env::split_paths(&paths) {
        let candidate = path.join("simple64-gui");
        if candidate.is_file() {
            return true;
        }
    }

    false
}

fn parse_config(path: &Path) -> (Option<PathBuf>, Vec<PathBuf>) {
    let mut rom_dir = None;
    let mut recent_roms = Vec::new();

    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "ROMdir" => {
                        let p = PathBuf::from(value);
                        if p.exists() {
                            rom_dir = Some(p);
                        }
                    }
                    "RecentROMs2" => {
                        for part in value.split(", ") {
                            let p = PathBuf::from(part);
                            recent_roms.push(p);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    (rom_dir, recent_roms)
}

fn is_valid_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_lowercase();
            matches!(e.as_str(), "z64" | "n64" | "v64" | "zip" | "7z")
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
    let exec = format!("simple64-gui \"{}\"", path.to_string_lossy());

    // Launch Key for history tracking
    let launch_key = format!(
        "simple64:{}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );

    tracing::info!("Discovered N64 ROM: '{}'", title);

    Some(AppEntry::new(title, exec, cover).with_launch_key(launch_key))
}

fn find_cover(rom_path: &Path) -> Option<String> {
    // Look for image with same name in same directory
    let extensions = ["png", "jpg", "jpeg", "webp"];
    for ext in extensions {
        let image_path = rom_path.with_extension(ext);
        if image_path.exists() {
            return Some(image_path.to_string_lossy().to_string());
        }
    }
    None
}

/// Extract clean title from filename.
/// Removes text in () and [] and extension.
fn extract_title_from_filename(path: &Path) -> String {
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Naive cleaning: remove anything in parentheses or brackets
    let mut title = String::with_capacity(stem.len());
    let mut depth_round = 0;
    let mut depth_square = 0;

    for c in stem.chars() {
        match c {
            '(' => depth_round += 1,
            ')' => {
                if depth_round > 0 {
                    depth_round -= 1;
                }
            }
            '[' => depth_square += 1,
            ']' => {
                if depth_square > 0 {
                    depth_square -= 1;
                }
            }
            _ => {
                if depth_round == 0 && depth_square == 0 {
                    title.push(c);
                }
            }
        }
    }

    title.trim().to_string()
}

fn ensure_fullscreen_config(config_dir: &Path) {
    let cfg_path = config_dir.join("mupen64plus.cfg");
    if !cfg_path.exists() {
        return;
    }

    if let Ok(content) = fs::read_to_string(&cfg_path) {
        let mut new_lines = Vec::new();
        let mut inside_video_general = false;
        let mut fullscreen_found = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                if inside_video_general && !fullscreen_found {
                    new_lines.push("Fullscreen = True".to_string());
                }
                inside_video_general = trimmed == "[Video-General]";
                fullscreen_found = false;
            }

            if inside_video_general && trimmed.starts_with("Fullscreen") {
                new_lines.push("Fullscreen = True".to_string());
                fullscreen_found = true;
                continue;
            }

            new_lines.push(line.to_string());
        }

        if inside_video_general && !fullscreen_found {
            new_lines.push("Fullscreen = True".to_string());
        }

        let new_content = new_lines.join("\n");
        if new_content != content {
            let _ = fs::write(&cfg_path, new_content);
            tracing::info!("Enforced Fullscreen=True in mupen64plus.cfg");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn temp_dir() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("launcher_test_{}", Uuid::new_v4()));
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
        assert_eq!(
            extract_title_from_filename(Path::new(
                "Legend of Zelda, The - Ocarina of Time (U) (V1.0) [!].z64"
            )),
            "Legend of Zelda, The - Ocarina of Time"
        );
    }

    #[test]
    fn test_parse_config() {
        let dir = temp_dir();
        let config_path = dir.join("gui.conf");
        let rom_dir = dir.join("Games");
        let rom1 = rom_dir.join("Game1.z64");
        let rom2 = dir.join("Other").join("Game2.z64");

        // Create the directories so they "exist" for the parser
        // This is strictly file system manipulation, but creating an empty directory is minimal
        // compared to writing binary files. The user requirement was "not need rom files".
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!(
            "[General]\nROMdir={}\nRecentROMs2={}, {}\n",
            rom_dir.to_str().unwrap(),
            rom1.to_str().unwrap(),
            rom2.to_str().unwrap()
        );
        fs::write(&config_path, content).unwrap();

        let (parsed_rom_dir, recent_roms) = parse_config(&config_path);

        assert_eq!(parsed_rom_dir, Some(rom_dir));
        assert_eq!(recent_roms.len(), 2);
        assert_eq!(recent_roms[0], rom1);
        assert_eq!(recent_roms[1], rom2);

        // Cleanup
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_ensure_fullscreen_config() {
        let dir = temp_dir();
        let cfg_path = dir.join("mupen64plus.cfg");

        // Case 1: Existing False -> True
        let content_false = "[Video-General]\nFullscreen = False\nOther = 1";
        fs::write(&cfg_path, content_false).unwrap();
        ensure_fullscreen_config(&dir);
        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("Fullscreen = True"));
        assert!(!content.contains("Fullscreen = False"));

        // Case 2: Missing -> Added
        let content_missing = "[Video-General]\nOther = 1";
        fs::write(&cfg_path, content_missing).unwrap();
        ensure_fullscreen_config(&dir);
        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("Fullscreen = True"));

        // Case 3: Already True -> Unchanged (functionally)
        let content_true = "[Video-General]\nFullscreen = True\nOther = 1";
        fs::write(&cfg_path, content_true).unwrap();
        ensure_fullscreen_config(&dir);
        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("Fullscreen = True"));

        // Cleanup
        let _ = fs::remove_dir_all(dir);
    }
}
