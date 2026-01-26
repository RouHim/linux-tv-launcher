use crate::model::AppEntry;
use directories::BaseDirs;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Scan for mupen64plus games based on configuration
pub fn scan_mupen64plus_games() -> Vec<AppEntry> {
    let mut games = Vec::new();
    if !is_mupen64plus_available() {
        tracing::warn!("mupen64plus is not installed; skipping ROM scan");
        return games;
    }

    let Some(config_path) =
        BaseDirs::new().map(|dirs| dirs.config_dir().join("mupen64plus/mupen64plus-qt.conf"))
    else {
        tracing::warn!("Could not determine config directory for mupen64plus");
        return games;
    };

    // 1. Parse Config
    let rom_dirs = parse_mupen64plus_qt_config(&config_path);
    if rom_dirs.is_empty() {
        return games;
    }

    // 2. Scan ROM Directories
    for rom_dir in rom_dirs {
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
    }

    games
}

fn is_mupen64plus_available() -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    for path in env::split_paths(&paths) {
        let candidate = path.join("mupen64plus");
        if candidate.is_file() {
            return true;
        }
    }

    false
}

/// Parse mupen64plus-qt.conf INI file and extract ROM directories from [Paths] section
fn parse_mupen64plus_qt_config(path: &Path) -> Vec<PathBuf> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut in_paths_section = false;
    let mut rom_dirs = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }

        // Check for section headers
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_paths_section = trimmed == "[Paths]";
            continue;
        }

        // Only process roms= line when in [Paths] section
        if in_paths_section && trimmed.starts_with("roms=") {
            let value = trimmed.strip_prefix("roms=").unwrap_or("");

            // Strip surrounding quotes if present
            let value = value.trim_matches('"');

            // Handle @Invalid() marker (Qt writes this for unset values)
            if value.contains("@Invalid()") {
                return Vec::new();
            }

            // Split by pipe separator
            for segment in value.split('|') {
                let segment = segment.trim();
                if segment.is_empty() {
                    continue;
                }

                // Expand tilde to home directory
                let expanded = if segment.starts_with('~') {
                    if let Some(base_dirs) = BaseDirs::new() {
                        let home = base_dirs.home_dir();
                        let rest = segment.strip_prefix('~').unwrap_or("");
                        home.join(rest.trim_start_matches('/'))
                    } else {
                        PathBuf::from(segment)
                    }
                } else {
                    PathBuf::from(segment)
                };

                // Only include existing directories
                if expanded.exists() && expanded.is_dir() {
                    rom_dirs.push(expanded);
                }
            }

            break; // Found roms= in [Paths], no need to continue
        }
    }

    rom_dirs
}

fn is_valid_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_ascii_lowercase().as_str(),
                "z64" | "n64" | "v64" | "zip"
            )
        })
        .unwrap_or(false)
}

fn process_rom(path: &Path) -> Option<AppEntry> {
    let title = extract_title_from_filename(path);

    let cover = find_cover(path);

    let exec = format!("mupen64plus --fullscreen \"{}\"", path.to_string_lossy());

    let launch_key = format!(
        "mupen64plus:{}",
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
        dir.push(format!("launcher_test_mupen64plus_{}", Uuid::new_v4()));
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
    fn test_parse_config_single_rom_dir() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!("[Paths]\nroms={}\n", rom_dir.to_string_lossy());
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 1);
        assert_eq!(parsed_dirs[0], rom_dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_multiple_rom_dirs() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");
        let rom_dir1 = dir.join("roms1");
        let rom_dir2 = dir.join("roms2");
        let rom_dir3 = dir.join("roms3");
        fs::create_dir_all(&rom_dir1).unwrap();
        fs::create_dir_all(&rom_dir2).unwrap();
        fs::create_dir_all(&rom_dir3).unwrap();

        let content = format!(
            "[Paths]\nroms={}|{}|{}\n",
            rom_dir1.to_string_lossy(),
            rom_dir2.to_string_lossy(),
            rom_dir3.to_string_lossy()
        );
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 3);
        assert!(parsed_dirs.contains(&rom_dir1));
        assert!(parsed_dirs.contains(&rom_dir2));
        assert!(parsed_dirs.contains(&rom_dir3));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_with_nonexistent_dirs() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");

        let content = "[Paths]\nroms=/nonexistent/path1|/nonexistent/path2\n";
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_mixed_existing_and_nonexistent() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!(
            "[Paths]\nroms=/nonexistent/path|{}|/another/nonexistent\n",
            rom_dir.to_string_lossy()
        );
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 1);
        assert_eq!(parsed_dirs[0], rom_dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_nonexistent_file() {
        let dir = temp_dir();
        let config_path = dir.join("does_not_exist.conf");

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_malformed_ini() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");

        fs::write(&config_path, "{ this is not INI format }").unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_empty_roms_value() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");

        let content = "[Paths]\nroms=\n";
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_roms_not_in_paths_section() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!("[OtherSection]\nroms={}\n", rom_dir.to_string_lossy());
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_quoted_value() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!("[Paths]\nroms=\"{}\"\n", rom_dir.to_string_lossy());
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 1);
        assert_eq!(parsed_dirs[0], rom_dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_at_invalid_marker() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");

        let content = "[Paths]\nroms=@Invalid()\n";
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_empty_segments() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");
        let rom_dir1 = dir.join("roms1");
        let rom_dir3 = dir.join("roms3");
        fs::create_dir_all(&rom_dir1).unwrap();
        fs::create_dir_all(&rom_dir3).unwrap();

        let content = format!(
            "[Paths]\nroms={}||{}\n",
            rom_dir1.to_string_lossy(),
            rom_dir3.to_string_lossy()
        );
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 2);
        assert!(parsed_dirs.contains(&rom_dir1));
        assert!(parsed_dirs.contains(&rom_dir3));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_with_null_rom_dir() {
        let dir = temp_dir();
        let config_path = dir.join("mupen64plus-qt.conf");

        let content = "[Paths]\nroms=\n";
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_mupen64plus_qt_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_7z_extension_not_matched() {
        let path = Path::new("test.7z");
        assert!(!is_valid_extension(path));
    }
}
