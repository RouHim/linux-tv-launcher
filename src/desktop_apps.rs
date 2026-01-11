use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a parsed XDG .desktop application
#[derive(Debug, Clone)]
pub struct DesktopApp {
    pub name: String,
    pub exec: String,
    pub icon_path: Option<PathBuf>,
    pub _desktop_file: PathBuf,
}

/// Scan all XDG application directories for .desktop files
pub fn scan_desktop_apps() -> Vec<DesktopApp> {
    let mut apps = Vec::new();
    let home = directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .unwrap_or_default();

    // XDG application directories (in priority order)
    let app_dirs = [
        // User directories (higher priority)
        home.join(".local/share/applications"),
        // Flatpak user apps
        home.join(".local/share/flatpak/exports/share/applications"),
        // System directories
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/usr/share/applications"),
        // Snap apps
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ];

    for dir in &app_dirs {
        if dir.exists() {
            scan_directory(dir, &mut apps);
        }
    }

    // Sort by name
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Deduplicate by name (keep first occurrence, which is user-level)
    apps.dedup_by(|a, b| a.name == b.name);

    apps
}

fn scan_directory(dir: &Path, apps: &mut Vec<DesktopApp>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_err) => {
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "desktop") {
            if let Some(app) = parse_desktop_file(&path) {
                apps.push(app);
            }
        }
    }
}

fn parse_desktop_file(path: &Path) -> Option<DesktopApp> {
    let content = fs::read_to_string(path).ok()?;

    // Parse INI-like format
    let mut in_desktop_entry = false;
    let mut fields: HashMap<&str, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Section header
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        // Key=Value pairs
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.trim(), value.trim().to_string());
        }
    }

    // Filter criteria
    // Skip if Type is not Application
    if fields.get("Type").is_some_and(|t| t != "Application") {
        return None;
    }

    // Skip if NoDisplay=true
    if fields.get("NoDisplay").is_some_and(|v| v == "true") {
        return None;
    }

    // Skip if Hidden=true
    if fields.get("Hidden").is_some_and(|v| v == "true") {
        return None;
    }

    // Get required fields
    let name = fields.get("Name")?.clone();
    let exec_raw = fields.get("Exec")?.clone();

    // Clean up exec command: remove field codes like %f, %F, %u, %U, etc.
    let exec = clean_exec_command(&exec_raw);

    // Resolve icon
    let icon_path = fields
        .get("Icon")
        .and_then(|icon_name| resolve_icon(icon_name));

    Some(DesktopApp {
        name,
        exec,
        icon_path,
        _desktop_file: path.to_path_buf(),
    })
}

/// Remove .desktop field codes from exec command
fn clean_exec_command(exec: &str) -> String {
    let mut result = String::new();
    let mut chars = exec.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            // Skip the field code character
            chars.next();
        } else {
            result.push(c);
        }
    }

    result.trim().to_string()
}

/// Resolve icon name to file path
fn resolve_icon(icon_name: &str) -> Option<PathBuf> {
    // If it's already an absolute path, check if it exists
    if icon_name.starts_with('/') {
        let path = PathBuf::from(icon_name);
        if path.exists() {
            return Some(path);
        }
        return None;
    }

    let home = directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .unwrap_or_default();

    // Icon theme directories to search (in priority order)
    let icon_dirs = [
        // User icons
        home.join(".icons"),
        home.join(".local/share/icons"),
        // System icons - hicolor is the fallback theme
        PathBuf::from("/usr/share/icons/hicolor"),
        PathBuf::from("/usr/share/icons/Adwaita"),
        PathBuf::from("/usr/share/icons"),
        // Pixmaps (legacy)
        PathBuf::from("/usr/share/pixmaps"),
    ];

    // Sizes to try (prefer larger)
    let sizes = [
        "256x256", "scalable", "128x128", "96x96", "64x64", "48x48", "32x32", "24x24", "22x22",
        "16x16",
    ];

    // Categories to try
    let categories = ["apps", "applications", "mimetypes", "categories", "devices"];

    // Extensions to try
    let extensions = ["svg", "png", "xpm"];

    for icon_dir in &icon_dirs {
        if !icon_dir.exists() {
            continue;
        }

        // For pixmaps, files are directly in the directory
        if icon_dir.ends_with("pixmaps") {
            for ext in &extensions {
                let path = icon_dir.join(format!("{}.{}", icon_name, ext));
                if path.exists() {
                    return Some(path);
                }
            }
            // Also try exact match (some icons have full filename)
            let exact_path = icon_dir.join(icon_name);
            if exact_path.exists() {
                return Some(exact_path);
            }
            continue;
        }

        // For theme directories, search size/category subdirectories
        for size in &sizes {
            for category in &categories {
                for ext in &extensions {
                    let path = icon_dir
                        .join(size)
                        .join(category)
                        .join(format!("{}.{}", icon_name, ext));
                    if path.exists() {
                        return Some(path);
                    }
                }
            }
        }

        // Also try direct in theme dir (some themes structure differently)
        for ext in &extensions {
            let path = icon_dir.join(format!("{}.{}", icon_name, ext));
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_exec_command() {
        assert_eq!(clean_exec_command("firefox %u"), "firefox");
        assert_eq!(clean_exec_command("code %F"), "code");
        assert_eq!(clean_exec_command("gimp-2.10 %U"), "gimp-2.10");
        assert_eq!(
            clean_exec_command("/usr/bin/app --flag %f --other"),
            "/usr/bin/app --flag  --other"
        );
    }

    #[test]
    fn test_scan_finds_apps() {
        let apps = scan_desktop_apps();
        // Should find at least some apps on a typical Linux system
        // This test may need adjustment based on the test environment
        println!("Found {} apps", apps.len());
        for app in apps.iter().take(5) {
            println!("  - {} ({:?})", app.name, app.icon_path);
        }
    }
}
