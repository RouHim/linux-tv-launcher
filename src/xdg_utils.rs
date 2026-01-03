use crate::model::AppEntry;
use freedesktop_desktop_entry::{default_paths, DesktopEntry, Iter};
use freedesktop_icons::{list_themes, lookup};
use rayon::prelude::*;
use tracing::debug;

pub fn scan_system_apps() -> Vec<AppEntry> {
    // Collect all paths first
    let paths: Vec<_> = Iter::new(default_paths()).collect();

    // Process in parallel
    let mut apps: Vec<AppEntry> = paths
        .par_iter()
        .filter_map(|path| {
            if let Ok(entry) = DesktopEntry::from_path(path, Some(&[] as &[&str])) {
                convert_entry(entry)
            } else {
                None
            }
        })
        .collect();

    // Dedup by name + exec to avoid duplicates from different paths (e.g. local vs system)
    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps.dedup_by(|a, b| a.name == b.name && a.exec == b.exec);

    apps
}

fn convert_entry(entry: DesktopEntry) -> Option<AppEntry> {
    if entry.no_display() {
        return None;
    }

    // We only care about Applications
    if entry.type_() != Some("Application") {
        return None;
    }

    let name = entry.name(&[] as &[&str]).map(|s| s.to_string())?;
    let exec_raw = entry.exec()?;

    // Clean up Exec command (remove %f, %u, etc.)
    let exec = clean_exec_code(exec_raw);

    let raw_icon_name = entry.icon().map(|s| s.to_string());
    let mut icon = None;

    // Try to resolve icon path
    if let Some(icon_name) = raw_icon_name {
        debug!("Resolving icon: {}", icon_name);
        if icon_name.starts_with('/') {
            icon = Some(icon_name);
        } else {
            // Strip extension if present (XDG lookup usually wants the name only)
            let search_name = if let Some(last_dot) = icon_name.rfind('.') {
                let (name, ext) = icon_name.split_at(last_dot);
                let ext = &ext[1..];
                if ["png", "svg", "xpm", "jpg", "jpeg"].contains(&ext.to_lowercase().as_str()) {
                    name.to_string()
                } else {
                    icon_name.clone()
                }
            } else {
                icon_name.clone()
            };

            icon = resolve_icon_path(&search_name);

            if icon.is_none() {
                // Fallback: /usr/share/pixmaps
                let fallback_extensions = ["png", "svg", "xpm", "jpg"];
                for ext in fallback_extensions {
                    let fallback_path = format!("/usr/share/pixmaps/{}.{}", search_name, ext);
                    if std::path::Path::new(&fallback_path).exists() {
                        icon = Some(fallback_path);
                        break;
                    }
                }
            }

            if icon.is_none() {
                // println!("No icon found for {}", icon_name);
            }
        }
    }

    Some(AppEntry::new(name, exec, icon))
}

fn clean_exec_code(exec: &str) -> String {
    exec.split_whitespace()
        .filter(|part| !part.starts_with('%'))
        .collect::<Vec<&str>>()
        .join(" ")
}

fn resolve_icon_path(search_name: &str) -> Option<String> {
    let sizes = [512, 256, 128, 96, 64, 48, 32, 24, 22, 16];
    let themes = ordered_themes();

    for size in sizes {
        for theme in &themes {
            if let Some(path) = lookup(search_name)
                .with_size(size)
                .with_theme(theme)
                .with_cache()
                .find()
            {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }

    None
}

fn ordered_themes() -> Vec<String> {
    let mut ordered = Vec::new();
    ordered.push("hicolor".to_string());

    for theme in list_themes() {
        if !theme.eq_ignore_ascii_case("hicolor") {
            ordered.push(theme);
        }
    }

    ordered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_exec() {
        assert_eq!(clean_exec_code("vlc %U"), "vlc");
        assert_eq!(clean_exec_code("eog %f"), "eog");
        assert_eq!(clean_exec_code("my-app --arg %i"), "my-app --arg");
    }
}
