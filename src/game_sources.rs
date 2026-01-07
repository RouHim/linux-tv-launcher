use crate::model::AppEntry;
use directories::BaseDirs;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

pub fn scan_games() -> Vec<AppEntry> {
    let (steam_games, heroic_games) = rayon::join(scan_steam_games, scan_heroic_games);

    let mut games = Vec::new();
    games.extend(steam_games);
    games.extend(heroic_games);

    games.sort_by(|a, b| a.name.cmp(&b.name));
    games.dedup_by(|a, b| a.name == b.name && a.exec == b.exec);

    games
}

fn scan_steam_games() -> Vec<AppEntry> {
    let base_dirs = match BaseDirs::new() {
        Some(dirs) => dirs,
        None => return Vec::new(),
    };

    let home = base_dirs.home_dir();
    let mut roots = Vec::new();
    for candidate in [
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        home.join(".steam/root"),
    ] {
        if candidate.exists() {
            roots.push(candidate);
        }
    }

    let mut library_paths = Vec::new();
    for root in &roots {
        let library_file = root.join("steamapps/libraryfolders.vdf");
        if let Ok(contents) = fs::read_to_string(&library_file) {
            library_paths.extend(parse_library_folders(&contents));
        }
        if root.join("steamapps").exists() {
            library_paths.push(root.clone());
        }
    }

    let mut seen = HashSet::new();
    let mut unique_paths = Vec::new();
    for path in library_paths {
        if seen.insert(path.clone()) {
            unique_paths.push(path);
        }
    }

    let mut games = Vec::new();
    for library in unique_paths {
        let steamapps = library.join("steamapps");
        let entries = match fs::read_dir(&steamapps) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !is_manifest_file(&path) {
                continue;
            }

            let appid_from_name = appid_from_manifest_path(&path);
            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(_) => continue,
            };

            let Some(mut manifest) = parse_steam_manifest(&contents) else {
                continue;
            };

            if manifest.appid.is_empty() {
                if let Some(appid) = appid_from_name {
                    manifest.appid = appid;
                }
            }

            if manifest.appid.is_empty() {
                continue;
            }

            if is_ignored_app(&manifest.name, &manifest.appid) {
                continue;
            }

            let exec = format!("steam -applaunch {}", manifest.appid);
            games.push(AppEntry::new(manifest.name, exec, None));
        }
    }

    games
}

fn is_ignored_app(name: &str, id: &str) -> bool {
    let name_lower = name.to_lowercase();

    // Exact ID matches for common Steam runtimes/tools
    match id {
        "228980" => return true,  // Steamworks Common Redist
        "1391110" => return true, // Steam Linux Runtime - Soldier
        "1628350" => return true, // Steam Linux Runtime - Sniper
        "1070560" => return true, // Steam Linux Runtime
        "1493710" => return true, // Proton Experimental
        "1887720" => return true, // Proton EasyAntiCheat Runtime
        _ => {}
    }

    // Keyword matching
    if name_lower.contains("proton")
        || name_lower.contains("steam linux runtime")
        || name_lower.contains("steamworks common redist")
        || name_lower.contains("galaxy common redist")
        || name_lower == "dxvk"
        || name_lower == "vkd3d"
    {
        return true;
    }

    false
}

fn scan_heroic_games() -> Vec<AppEntry> {
    let base_dirs = match BaseDirs::new() {
        Some(dirs) => dirs,
        None => return Vec::new(),
    };

    let config_dir = base_dirs.config_dir().to_path_buf();
    let home = base_dirs.home_dir();

    let mut games = Vec::new();
    let mut seen = HashSet::new();
    let mut seen_app_names = HashSet::new();

    let heroic_roots = [
        config_dir.join("heroic"),
        home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
    ];

    for root in heroic_roots {
        if !root.exists() {
            continue;
        }

        let store_cache = root.join("store_cache");
        let library_files = [
            ("legendary_library.json", "legendary"),
            ("gog_library.json", "gog"),
            ("nile_library.json", "nile"),
        ];
        let install_info_files = [
            ("legendary_install_info.json", "legendary"),
            ("gog_install_info.json", "gog"),
            ("nile_install_info.json", "nile"),
        ];

        for (file, store_hint) in library_files {
            let path = store_cache.join(file);
            if !path.exists() {
                continue;
            }

            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(err) => {
                    warn!("Failed to read Heroic file {}: {}", path.display(), err);
                    continue;
                }
            };

            let games_from_file = parse_heroic_library_json(&contents, store_hint);
            for game in games_from_file {
                if is_ignored_app(&game.title, &game.app_name) {
                    continue;
                }
                if seen_app_names.contains(&game.app_name) {
                    continue;
                }
                seen_app_names.insert(game.app_name.clone());

                let exec = heroic_exec(&game.store, &game.app_name);
                let entry = AppEntry::new(game.title, exec, None);
                let key = format!("{}:{}", entry.name, entry.exec);
                if seen.insert(key) {
                    games.push(entry);
                }
            }
        }

        for (file, store_hint) in install_info_files {
            let path = store_cache.join(file);
            if !path.exists() {
                continue;
            }

            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(err) => {
                    warn!("Failed to read Heroic file {}: {}", path.display(), err);
                    continue;
                }
            };

            // install_info files imply installation, so we don't strictly require "installed": true
            let games_from_file = parse_heroic_install_info_json(&contents, store_hint);
            for game in games_from_file {
                if is_ignored_app(&game.title, &game.app_name) {
                    continue;
                }
                if seen_app_names.contains(&game.app_name) {
                    continue;
                }
                seen_app_names.insert(game.app_name.clone());

                let exec = heroic_exec(&game.store, &game.app_name);
                let entry = AppEntry::new(game.title, exec, None);
                let key = format!("{}:{}", entry.name, entry.exec);
                if seen.insert(key) {
                    games.push(entry);
                }
            }
        }

        // Scan for sideloaded games or other config files
        // Check root and store_cache for potential game lists
        let generic_files = [
            root.join("sideload_cache.json"),
            root.join("store_cache").join("sideload_cache.json"),
            root.join("config.json"),
            root.join("store").join("config.json"),
            root.join("library.json"),
        ];

        for path in generic_files {
            if !path.exists() {
                continue;
            }

            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(err) => {
                    warn!("Failed to read Heroic file {}: {}", path.display(), err);
                    continue;
                }
            };

            // Parse as install info (flexible) to find any embedded game objects
            let games_from_file = parse_heroic_install_info_json(&contents, "heroic");
            for game in games_from_file {
                if is_ignored_app(&game.title, &game.app_name) {
                    continue;
                }
                if seen_app_names.contains(&game.app_name) {
                    continue;
                }
                seen_app_names.insert(game.app_name.clone());

                let exec = heroic_exec(&game.store, &game.app_name);
                let entry = AppEntry::new(game.title, exec, None);
                let key = format!("{}:{}", entry.name, entry.exec);
                if seen.insert(key) {
                    games.push(entry);
                }
            }
        }

        // Scan GameConfig folder for individual game configs (sideloaded or store games)
        let game_config_dir = root.join("GameConfig");
        if game_config_dir.exists() {
            if let Ok(entries) = fs::read_dir(&game_config_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("json") {
                        continue;
                    }

                    let file_stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default();
                    if file_stem.is_empty() {
                        continue;
                    }

                    let contents = match fs::read_to_string(&path) {
                        Ok(contents) => contents,
                        Err(_) => continue,
                    };

                    // Assume "heroic" as store for generic GameConfig files unless specified otherwise
                    // Pass file_stem as fallback app_name
                    if let Some(game) =
                        parse_heroic_game_config(&contents, "heroic", Some(file_stem))
                    {
                        if is_ignored_app(&game.title, &game.app_name) {
                            continue;
                        }
                        if seen_app_names.contains(&game.app_name) {
                            continue;
                        }
                        seen_app_names.insert(game.app_name.clone());

                        let exec = heroic_exec(&game.store, &game.app_name);
                        let entry = AppEntry::new(game.title, exec, None);
                        let key = format!("{}:{}", entry.name, entry.exec);
                        if seen.insert(key) {
                            games.push(entry);
                        }
                    }
                }
            }
        }
    }

    let installed_roots = [
        config_dir.join("heroic"),
        config_dir.join("legendary"),
        home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
        home.join(".var/app/com.heroicgameslauncher.hgl/config/legendary"),
    ];

    for root in installed_roots {
        if !root.exists() {
            continue;
        }

        let mut installed_files = Vec::new();
        collect_installed_json_files(&root, 4, &mut installed_files);

        for path in installed_files {
            let store = store_from_path(&path);
            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(err) => {
                    warn!("Failed to read Heroic file {}: {}", path.display(), err);
                    continue;
                }
            };

            let games_from_file = parse_heroic_installed_json(&contents, &store);
            for game in games_from_file {
                if seen_app_names.contains(&game.app_name) {
                    continue;
                }
                seen_app_names.insert(game.app_name.clone());

                let exec = heroic_exec(&game.store, &game.app_name);
                let entry = AppEntry::new(game.title, exec, None);
                let key = format!("{}:{}", entry.name, entry.exec);
                if seen.insert(key) {
                    games.push(entry);
                }
            }
        }
    }

    games
}

fn collect_installed_json_files(base: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }

    let direct = base.join("installed.json");
    if direct.exists() {
        out.push(direct);
    }

    let entries = match fs::read_dir(base) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_installed_json_files(&path, depth - 1, out);
        } else if path.file_name().and_then(|name| name.to_str()) == Some("installed.json") {
            out.push(path);
        }
    }
}

fn store_from_path(path: &Path) -> String {
    let lower = path.to_string_lossy().to_lowercase();
    if lower.contains("legendary") {
        "legendary".to_string()
    } else if lower.contains("gog") {
        "gog".to_string()
    } else if lower.contains("amazon") {
        "amazon".to_string()
    } else {
        "heroic".to_string()
    }
}

fn heroic_exec(store: &str, app_name: &str) -> String {
    let encoded = encode_uri_component(app_name);
    if store.is_empty()
        || store == "heroic"
        || store == "wine"
        || store == "native"
        || store == "proton"
    {
        format!("xdg-open heroic://launch/{}", encoded)
    } else {
        format!("xdg-open heroic://launch/{}/{}", store, encoded)
    }
}

fn encode_uri_component(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }
    encoded
}

struct HeroicGame {
    app_name: String,
    title: String,
    store: String,
}

fn parse_heroic_library_json(contents: &str, store_hint: &str) -> Vec<HeroicGame> {
    let value: Value = match serde_json::from_str(contents) {
        Ok(value) => value,
        Err(err) => {
            debug!("Failed to parse Heroic JSON: {}", err);
            return Vec::new();
        }
    };

    let mut games = Vec::new();
    collect_heroic_games(&value, store_hint, true, &mut games);
    games
}

fn parse_heroic_install_info_json(contents: &str, store_hint: &str) -> Vec<HeroicGame> {
    let value: Value = match serde_json::from_str(contents) {
        Ok(value) => value,
        Err(err) => {
            debug!("Failed to parse Heroic JSON: {}", err);
            return Vec::new();
        }
    };

    let mut games = Vec::new();
    // Relaxed check: don't require explicit "installed": true for install_info files
    collect_heroic_games(&value, store_hint, false, &mut games);
    games
}

fn parse_heroic_game_config(
    contents: &str,
    store_hint: &str,
    app_name_hint: Option<&str>,
) -> Option<HeroicGame> {
    let value: Value = match serde_json::from_str(contents) {
        Ok(value) => value,
        Err(err) => {
            debug!("Failed to parse Heroic GameConfig JSON: {}", err);
            return None;
        }
    };

    if let Value::Object(map) = value {
        // For individual config files, we don't strictly require "installed": true,
        // but if it says "false", we respect it.
        heroic_game_from_object(app_name_hint, &map, store_hint, false)
    } else {
        None
    }
}

fn parse_heroic_installed_json(contents: &str, store_hint: &str) -> Vec<HeroicGame> {
    let value: Value = match serde_json::from_str(contents) {
        Ok(value) => value,
        Err(err) => {
            debug!("Failed to parse Heroic JSON: {}", err);
            return Vec::new();
        }
    };

    let mut games = Vec::new();
    collect_heroic_games(&value, store_hint, false, &mut games);
    games
}

fn collect_heroic_games(
    value: &Value,
    store_hint: &str,
    require_installed: bool,
    games: &mut Vec<HeroicGame>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_heroic_games(item, store_hint, require_installed, games);
            }
        }
        Value::Object(map) => {
            if let Some(game) = heroic_game_from_object(None, map, store_hint, require_installed) {
                games.push(game);
                return;
            }

            if let Some(installed) = map.get("installed") {
                collect_heroic_games(installed, store_hint, require_installed, games);
            }
            if let Some(installed) = map.get("games") {
                collect_heroic_games(installed, store_hint, require_installed, games);
            }

            for (key, value) in map {
                if key == "installed" || key == "games" {
                    continue;
                }

                match value {
                    Value::Object(obj) => {
                        if let Some(game) =
                            heroic_game_from_object(Some(key), obj, store_hint, require_installed)
                        {
                            games.push(game);
                        } else {
                            collect_heroic_games(value, store_hint, require_installed, games);
                        }
                    }
                    Value::Array(_) => {
                        collect_heroic_games(value, store_hint, require_installed, games)
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn heroic_game_from_object(
    key: Option<&str>,
    obj: &serde_json::Map<String, Value>,
    store_hint: &str,
    require_installed: bool,
) -> Option<HeroicGame> {
    let installed = obj
        .get("installed")
        .and_then(parse_json_bool)
        .or_else(|| obj.get("is_installed").and_then(parse_json_bool))
        .or_else(|| obj.get("isInstalled").and_then(parse_json_bool))
        .or_else(|| {
            obj.get("install")
                .and_then(|value| value.get("is_installed"))
                .and_then(parse_json_bool)
        });

    if require_installed {
        if installed != Some(true) {
            return None;
        }
    } else if matches!(installed, Some(false)) {
        return None;
    }

    let app_name = obj
        .get("app_name")
        .and_then(|value| value.as_str())
        .or_else(|| obj.get("appName").and_then(|value| value.as_str()))
        .or(key);
    let title = obj
        .get("title")
        .and_then(|value| value.as_str())
        .or_else(|| obj.get("name").and_then(|value| value.as_str()))
        .or_else(|| obj.get("display_name").and_then(|value| value.as_str()));

    let store = obj
        .get("runner")
        .and_then(|value| value.as_str())
        .or_else(|| obj.get("store").and_then(|value| value.as_str()))
        .or_else(|| obj.get("provider").and_then(|value| value.as_str()))
        .or_else(|| obj.get("backend").and_then(|value| value.as_str()))
        .unwrap_or(store_hint);

    let app_name = app_name?.trim();
    let title = title?.trim();

    if app_name.is_empty() || title.is_empty() {
        return None;
    }

    Some(HeroicGame {
        app_name: app_name.to_string(),
        title: title.to_string(),
        store: store.to_string(),
    })
}

fn parse_json_bool(value: &Value) -> Option<bool> {
    if let Some(bool_value) = value.as_bool() {
        return Some(bool_value);
    }

    let str_value = value.as_str()?;
    match str_value.to_ascii_lowercase().as_str() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

struct SteamManifest {
    appid: String,
    name: String,
}

fn parse_steam_manifest(contents: &str) -> Option<SteamManifest> {
    let mut appid = None;
    let mut name = None;

    for line in contents.lines() {
        let parts = extract_quoted_strings(line);
        if parts.len() < 2 {
            continue;
        }

        match parts[0].as_str() {
            "appid" => appid = Some(parts[1].clone()),
            "name" => name = Some(parts[1].clone()),
            _ => {}
        }
    }

    let name = name?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    Some(SteamManifest {
        appid: appid.unwrap_or_default(),
        name,
    })
}

fn is_manifest_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    file_name.starts_with("appmanifest_") && file_name.ends_with(".acf")
}

fn appid_from_manifest_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy();
    let appid = stem.strip_prefix("appmanifest_")?;
    if appid.chars().all(|c| c.is_ascii_digit()) {
        Some(appid.to_string())
    } else {
        None
    }
}

fn parse_library_folders(contents: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for line in contents.lines() {
        let parts = extract_quoted_strings(line);
        if parts.len() < 2 {
            continue;
        }

        if parts[0].eq_ignore_ascii_case("path") || parts[0].chars().all(|c| c.is_ascii_digit()) {
            paths.push(normalize_vdf_path(&parts[1]));
        }
    }

    paths
}

fn normalize_vdf_path(value: &str) -> PathBuf {
    PathBuf::from(value.replace("\\\\", "\\"))
}

fn extract_quoted_strings(line: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape = false;

    for ch in line.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }

        if in_quotes && ch == '\\' {
            escape = true;
            continue;
        }

        if ch == '"' {
            if in_quotes {
                items.push(current.clone());
                current.clear();
                in_quotes = false;
            } else {
                in_quotes = true;
            }
            continue;
        }

        if in_quotes {
            current.push(ch);
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_library_folders_extracts_paths() {
        let contents = r#"
        "libraryfolders"
        {
            "1"
            {
                "path" "/mnt/games"
            }
            "2" "/home/test/Steam"
        }
        "#;

        let paths = parse_library_folders(contents);
        assert!(paths.contains(&PathBuf::from("/mnt/games")));
        assert!(paths.contains(&PathBuf::from("/home/test/Steam")));
    }

    #[test]
    fn test_parse_steam_manifest_extracts_name_and_appid() {
        let contents = r#"
        "AppState"
        {
            "appid" "570"
            "name" "Dota 2"
        }
        "#;

        let manifest = parse_steam_manifest(contents).expect("manifest parsed");
        assert_eq!(manifest.appid, "570");
        assert_eq!(manifest.name, "Dota 2");
    }

    #[test]
    fn test_parse_heroic_installed_json_extracts_installed_games() {
        let contents = r#"
        {
            "installed": [
                {"app_name": "fortnite", "title": "Fortnite", "installed": true},
                {"app_name": "demo", "title": "Demo", "installed": false}
            ]
        }
        "#;

        let games = parse_heroic_installed_json(contents, "legendary");
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].app_name, "fortnite");
        assert_eq!(games[0].title, "Fortnite");
    }

    #[test]
    fn test_parse_heroic_install_info_json_requires_installed() {
        let contents = r#"
        {
            "games": [
                {"app_name": "nile-1", "title": "Nile One", "is_installed": true, "runner": "nile"},
                {"app_name": "nile-2", "title": "Nile Two", "is_installed": false, "runner": "nile"}
            ]
        }
        "#;

        let games = parse_heroic_install_info_json(contents, "nile");
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].app_name, "nile-1");
        assert_eq!(games[0].title, "Nile One");
        assert_eq!(games[0].store, "nile");
    }

    #[test]
    fn test_parse_heroic_library_json_requires_installed() {
        let contents = r#"
        {
            "games": [
                {"app_name": "gog-1", "title": "GOG One", "is_installed": true, "runner": "gog"},
                {"app_name": "gog-2", "title": "GOG Two", "is_installed": false, "runner": "gog"}
            ]
        }
        "#;

        let games = parse_heroic_library_json(contents, "gog");
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].app_name, "gog-1");
        assert_eq!(games[0].title, "GOG One");
        assert_eq!(games[0].store, "gog");
    }

    #[test]
    fn test_parse_heroic_game_config_parses_single_game() {
        let contents = r#"
        {
            "app_name": "MyGame",
            "title": "My Custom Game",
            "runner": "heroic",
            "is_installed": true
        }
        "#;

        let game = parse_heroic_game_config(contents, "heroic", None).expect("Parsed game config");
        assert_eq!(game.app_name, "MyGame");
        assert_eq!(game.title, "My Custom Game");
        assert_eq!(game.store, "heroic");
    }

    #[test]
    fn test_parse_heroic_game_config_uses_hint() {
        let contents = r#"
        {
            "title": "Sideloaded",
            "runner": "heroic"
        }
        "#;

        let game = parse_heroic_game_config(contents, "heroic", Some("sideload-123"))
            .expect("Parsed game config with hint");
        assert_eq!(game.app_name, "sideload-123");
        assert_eq!(game.title, "Sideloaded");
    }

    #[test]
    fn test_is_ignored_app() {
        assert!(is_ignored_app("Proton Experimental", "1493710"));
        assert!(is_ignored_app("Steam Linux Runtime - Sniper", "1628350"));
        assert!(!is_ignored_app("My Game", "123456"));
    }

    #[test]
    fn test_parse_sideload_cache_json() {
        let contents = r#"
        [
            {
                "app_name": "Sideload1",
                "title": "My Sideloaded Game",
                "runner": "wine",
                "is_installed": true
            }
        ]
        "#;

        let games = parse_heroic_install_info_json(contents, "heroic");
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].app_name, "Sideload1");
        assert_eq!(games[0].title, "My Sideloaded Game");
        assert_eq!(games[0].store, "wine");
    }

    #[test]
    fn test_heroic_exec_handles_sideload_runners() {
        assert_eq!(heroic_exec("wine", "App1"), "xdg-open heroic://launch/App1");
        assert_eq!(
            heroic_exec("native", "App2"),
            "xdg-open heroic://launch/App2"
        );
        assert_eq!(
            heroic_exec("legendary", "App3"),
            "xdg-open heroic://launch/legendary/App3"
        );
    }
}
