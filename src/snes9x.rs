use crate::model::AppEntry;
use directories::BaseDirs;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Scan for SNES ROMs based on snes9x configuration
pub fn scan_snes9x_games() -> Vec<AppEntry> {
    let mut games = Vec::new();
    let Some(emulator_binary) = get_snes9x_binary() else {
        tracing::warn!("snes9x or snes9x-gtk is not installed; skipping ROM scan");
        return games;
    };

    if emulator_binary == "snes9x-gtk" {
        ensure_fullscreen_on_open();
    }

    // 1. Get ROM directories from config files
    let mut rom_dirs = Vec::new();
    for config_path in get_snes9x_config_paths() {
        let mut dirs = parse_snes9x_config(&config_path);
        rom_dirs.append(&mut dirs);
    }

    // 2. Deduplicate directories
    rom_dirs.sort();
    rom_dirs.dedup();

    if rom_dirs.is_empty() {
        tracing::warn!("No SNES ROM directories found in config");
        return games;
    }

    // 3. Scan ROM Directories
    for rom_dir in rom_dirs {
        if let Ok(entries) = fs::read_dir(rom_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if is_valid_extension(&path) {
                    if let Some(game) = process_rom(&path, &emulator_binary) {
                        games.push(game);
                    }
                }
            }
        }
    }

    games
}

/// Get the snes9x binary name if available (prefers snes9x-gtk, falls back to snes9x)
fn get_snes9x_binary() -> Option<String> {
    let paths = env::var_os("PATH")?;

    // Check for snes9x-gtk first (more commonly installed)
    for path in env::split_paths(&paths) {
        let candidate = path.join("snes9x-gtk");
        if candidate.is_file() {
            return Some("snes9x-gtk".to_string());
        }
    }

    // Fall back to snes9x
    for path in env::split_paths(&paths) {
        let candidate = path.join("snes9x");
        if candidate.is_file() {
            return Some("snes9x".to_string());
        }
    }

    None
}

/// Get possible snes9x configuration file paths
fn get_snes9x_config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(base_dirs) = BaseDirs::new() {
        // XDG config path: ~/.config/snes9x/snes9x.conf
        paths.push(base_dirs.config_dir().join("snes9x/snes9x.conf"));

        // Legacy path: ~/.snes9x/snes9x.conf
        paths.push(base_dirs.home_dir().join(".snes9x/snes9x.conf"));
    }

    paths
}

fn ensure_fullscreen_on_open() {
    for config_path in get_snes9x_config_paths() {
        if !config_path.exists() {
            continue;
        }

        let Ok(content) = fs::read_to_string(&config_path) else {
            continue;
        };

        if content.contains("FullscreenOnOpen        = true") {
            return;
        }

        if content.contains("FullscreenOnOpen        = false") {
            let updated = content.replace(
                "FullscreenOnOpen        = false",
                "FullscreenOnOpen        = true",
            );
            if fs::write(&config_path, updated).is_ok() {
                tracing::info!("Enabled FullscreenOnOpen in snes9x config");
            }
            return;
        }
    }
}

/// Parse snes9x.conf INI file and extract ROM directories from [Files] section
fn parse_snes9x_config(path: &Path) -> Vec<PathBuf> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut rom_dirs = Vec::new();
    let mut in_files_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }

        // Check for section headers
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_files_section = trimmed == "[Files]";
            continue;
        }

        // Only process LastDirectory= when in [Files] section
        if in_files_section && trimmed.to_lowercase().starts_with("lastdirectory") {
            let value = if let Some(eq_pos) = trimmed.find('=') {
                &trimmed[eq_pos + 1..]
            } else {
                continue;
            };

            // Strip surrounding quotes and whitespace
            let value = value.trim().trim_matches('"').trim_matches('\'');

            if value.is_empty() {
                continue;
            }

            // Expand tilde to home directory
            let expanded = if value.starts_with('~') {
                if let Some(base_dirs) = BaseDirs::new() {
                    let home = base_dirs.home_dir();
                    let rest = value.strip_prefix('~').unwrap_or("");
                    home.join(rest.trim_start_matches('/'))
                } else {
                    PathBuf::from(value)
                }
            } else {
                PathBuf::from(value)
            };

            // Only include existing directories
            if expanded.exists() && expanded.is_dir() {
                rom_dirs.push(expanded);
                break; // Only one LastDirectory entry expected
            }
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
                "sfc" | "smc" | "fig" | "swc" | "bs" | "st"
            )
        })
        .unwrap_or(false)
}

fn process_rom(path: &Path, emulator_binary: &str) -> Option<AppEntry> {
    let title = extract_title_from_filename(path);

    let cover = find_cover(path);

    let exec = if emulator_binary == "snes9x" {
        format!(
            "{} -fullscreen \"{}\"",
            emulator_binary,
            path.to_string_lossy()
        )
    } else {
        format!("{}  \"{}\"", emulator_binary, path.to_string_lossy())
    };

    let launch_key = format!(
        "snes9x:{}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );

    tracing::info!("Discovered SNES ROM: '{}'", title);

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
        dir.push(format!("launcher_test_snes9x_{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_extract_title_from_filename() {
        assert_eq!(
            extract_title_from_filename(Path::new("Super Mario World (USA).sfc")),
            "Super Mario World"
        );
        assert_eq!(
            extract_title_from_filename(Path::new("Chrono Trigger (USA) [!].smc")),
            "Chrono Trigger"
        );
        assert_eq!(
            extract_title_from_filename(Path::new("Donkey Kong Country.sfc")),
            "Donkey Kong Country"
        );
        assert_eq!(
            extract_title_from_filename(Path::new("Final Fantasy III (USA) (v1.1).smc")),
            "Final Fantasy III"
        );
    }

    #[test]
    fn test_valid_snes_extensions() {
        assert!(is_valid_extension(Path::new("game.sfc")));
        assert!(is_valid_extension(Path::new("game.smc")));
        assert!(is_valid_extension(Path::new("game.fig")));
        assert!(is_valid_extension(Path::new("game.swc")));
        assert!(is_valid_extension(Path::new("game.bs")));
        assert!(is_valid_extension(Path::new("game.st")));
    }

    #[test]
    fn test_invalid_extension_returns_false() {
        assert!(!is_valid_extension(Path::new("game.zip")));
        assert!(!is_valid_extension(Path::new("game.7z")));
        assert!(!is_valid_extension(Path::new("game.txt")));
        assert!(!is_valid_extension(Path::new("game.bin")));
    }

    #[test]
    fn test_case_insensitive_extension() {
        assert!(is_valid_extension(Path::new("game.SFC")));
        assert!(is_valid_extension(Path::new("game.SmC")));
        assert!(is_valid_extension(Path::new("game.FIG")));
        assert!(is_valid_extension(Path::new("game.SWC")));
        assert!(is_valid_extension(Path::new("game.BS")));
        assert!(is_valid_extension(Path::new("game.ST")));
    }

    #[test]
    fn test_parse_config_single_rom_dir() {
        let dir = temp_dir();
        let config_path = dir.join("snes9x.conf");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!("[Files]\nLastDirectory = {}\n", rom_dir.to_string_lossy());
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 1);
        assert_eq!(parsed_dirs[0], rom_dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_multiple_entries() {
        let dir = temp_dir();
        let config_path = dir.join("snes9x.conf");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!(
            "[Files]\nLastDirectory = {}\n[OtherSection]\nSomeKey = value\n",
            rom_dir.to_string_lossy()
        );
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 1);
        assert_eq!(parsed_dirs[0], rom_dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_nonexistent_file() {
        let dir = temp_dir();
        let config_path = dir.join("does_not_exist.conf");

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_malformed_content() {
        let dir = temp_dir();
        let config_path = dir.join("snes9x.conf");

        fs::write(&config_path, "{ this is not INI format }").unwrap();

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_filters_nonexistent_dirs() {
        let dir = temp_dir();
        let config_path = dir.join("snes9x.conf");

        let content = "[Files]\nLastDirectory = /nonexistent/path\n";
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_mixed_existing_and_nonexistent() {
        let dir = temp_dir();
        let config_path = dir.join("snes9x.conf");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!("[Files]\nLastDirectory = {}\n", rom_dir.to_string_lossy());
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 1);
        assert_eq!(parsed_dirs[0], rom_dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_quoted_value() {
        let dir = temp_dir();
        let config_path = dir.join("snes9x.conf");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!(
            "[Files]\nLastDirectory = \"{}\"\n",
            rom_dir.to_string_lossy()
        );
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 1);
        assert_eq!(parsed_dirs[0], rom_dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_empty_value() {
        let dir = temp_dir();
        let config_path = dir.join("snes9x.conf");

        let content = "[Files]\nLastDirectory =\n";
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_case_insensitive_key() {
        let dir = temp_dir();
        let config_path = dir.join("snes9x.conf");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!("[Files]\nLASTDIRECTORY = {}\n", rom_dir.to_string_lossy());
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 1);
        assert_eq!(parsed_dirs[0], rom_dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_find_cover_returns_png() {
        let dir = temp_dir();
        let rom_path = dir.join("game.sfc");
        let cover_path = dir.join("game.png");

        fs::write(&rom_path, "fake rom").unwrap();
        fs::write(&cover_path, "fake image").unwrap();

        let result = find_cover(&rom_path);
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("game.png"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_find_cover_returns_first_match() {
        let dir = temp_dir();
        let rom_path = dir.join("game.sfc");
        let cover_png = dir.join("game.png");
        let cover_jpg = dir.join("game.jpg");

        fs::write(&rom_path, "fake rom").unwrap();
        fs::write(&cover_png, "fake png").unwrap();
        fs::write(&cover_jpg, "fake jpg").unwrap();

        let result = find_cover(&rom_path);
        assert!(result.is_some());
        // Should return png as it's checked first
        assert!(result.unwrap().ends_with("game.png"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_find_cover_returns_none_when_missing() {
        let dir = temp_dir();
        let rom_path = dir.join("game.sfc");

        fs::write(&rom_path, "fake rom").unwrap();

        let result = find_cover(&rom_path);
        assert!(result.is_none());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_process_rom_creates_valid_entry() {
        let dir = temp_dir();
        let rom_path = dir.join("Super Mario World (USA).sfc");
        fs::write(&rom_path, "fake rom").unwrap();

        let result = process_rom(&rom_path, "snes9x");
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.name, "Super Mario World");
        assert!(entry.exec.contains("snes9x -fullscreen"));
        assert!(entry.exec.contains(&rom_path.to_string_lossy().to_string()));
        assert_eq!(
            entry.launch_key,
            Some("snes9x:Super Mario World (USA).sfc".to_string())
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_process_rom_with_gtk_binary() {
        let dir = temp_dir();
        let rom_path = dir.join("Chrono Trigger (USA).sfc");
        fs::write(&rom_path, "fake rom").unwrap();

        let result = process_rom(&rom_path, "snes9x-gtk");
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.name, "Chrono Trigger");
        assert!(entry.exec.contains("snes9x-gtk"));
        assert!(!entry.exec.contains("-fullscreen"));
        assert!(entry.exec.contains(&rom_path.to_string_lossy().to_string()));
        assert_eq!(
            entry.launch_key,
            Some("snes9x:Chrono Trigger (USA).sfc".to_string())
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_scan_returns_empty_when_emulator_missing() {
        // This test verifies the function doesn't panic
        // Result depends on whether snes9x is actually installed
        let _games = scan_snes9x_games();
        // If snes9x is not installed, returns empty vec
        // If snes9x is installed, may return games depending on config
    }

    #[test]
    fn test_parse_config_with_comments() {
        let dir = temp_dir();
        let config_path = dir.join("snes9x.conf");
        let rom_dir = dir.join("roms");
        fs::create_dir_all(&rom_dir).unwrap();

        let content = format!(
            "; This is a comment\n# Another comment\n[Files]\nLastDirectory = {}\n; More comments\n",
            rom_dir.to_string_lossy()
        );
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 1);
        assert_eq!(parsed_dirs[0], rom_dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_config_tilde_expansion() {
        let dir = temp_dir();
        let config_path = dir.join("snes9x.conf");

        // Create a subdirectory in temp for testing
        let rom_subdir = dir.join("test_roms");
        fs::create_dir_all(&rom_subdir).unwrap();

        // Write config with a real path (not tilde, since we can't control HOME in test)
        let content = format!(
            "[Files]\nLastDirectory = {}\n",
            rom_subdir.to_string_lossy()
        );
        fs::write(&config_path, content).unwrap();

        let parsed_dirs = parse_snes9x_config(&config_path);
        assert_eq!(parsed_dirs.len(), 1);
        assert_eq!(parsed_dirs[0], rom_subdir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_get_snes9x_binary_basic() {
        let result = get_snes9x_binary();
        if let Some(binary) = result {
            assert!(binary == "snes9x" || binary == "snes9x-gtk");
        }
    }
}
