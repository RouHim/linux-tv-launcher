use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Apps,
    Games,
    System,
}

impl Category {
    pub const ALL: [Category; 3] = [Category::Apps, Category::Games, Category::System];

    pub fn title(self) -> &'static str {
        match self {
            Category::Apps => "Apps",
            Category::Games => "Games",
            Category::System => "System",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Category::Apps => Category::Games,
            Category::Games => Category::System,
            Category::System => Category::Apps,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Category::Apps => Category::System,
            Category::Games => Category::Apps,
            Category::System => Category::Games,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherAction {
    Launch { exec: String },
    SystemUpdate,
    Shutdown,
    Suspend,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherItem {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub action: LauncherAction,
    /// Source image URL (e.g., from Heroic) to use for fetching cover art
    pub source_image_url: Option<String>,
    pub game_executable: Option<String>,
}

impl LauncherItem {
    pub fn from_app_entry(entry: AppEntry) -> Self {
        // If icon is a URL (starts with http), treat it as source_image_url
        // Otherwise treat it as a local file path
        let (icon, source_image_url) = if let Some(ref icon_str) = entry.icon {
            if icon_str.starts_with("http://") || icon_str.starts_with("https://") {
                (None, Some(icon_str.clone()))
            } else {
                (entry.icon.clone(), None)
            }
        } else {
            (None, None)
        };

        Self {
            id: entry.id,
            name: entry.name,
            icon,
            action: LauncherAction::Launch { exec: entry.exec },
            source_image_url,
            game_executable: entry.game_executable,
        }
    }

    pub fn system_update() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Update System".to_string(),
            icon: None,
            action: LauncherAction::SystemUpdate,
            source_image_url: None,
            game_executable: None,
        }
    }

    pub fn shutdown() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Shutdown".to_string(),
            icon: Some("assets/shutdown.svg".to_string()),
            action: LauncherAction::Shutdown,
            source_image_url: None,
            game_executable: None,
        }
    }

    pub fn suspend() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Suspend".to_string(),
            icon: Some("assets/suspend.svg".to_string()),
            action: LauncherAction::Suspend,
            source_image_url: None,
            game_executable: None,
        }
    }

    pub fn exit() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Exit Launcher".to_string(),
            icon: Some("assets/exit.svg".to_string()),
            action: LauncherAction::Exit,
            source_image_url: None,
            game_executable: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppEntry {
    pub id: Uuid,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub game_executable: Option<String>,
}

impl AppEntry {
    pub fn new(name: String, exec: String, icon: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            exec,
            icon,
            game_executable: None,
        }
    }
    
    pub fn with_executable(mut self, executable: Option<String>) -> Self {
        self.game_executable = executable;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_entry_creation() {
        let entry = AppEntry::new(
            "Terminal".to_string(),
            "gnome-terminal".to_string(),
            Some("utilities-terminal".to_string()),
        );

        assert_eq!(entry.name, "Terminal");
        assert_eq!(entry.exec, "gnome-terminal");
        assert!(entry.icon.is_some());
    }

    #[test]
    fn test_launcher_item_from_app_entry() {
        let entry = AppEntry::new("Game".to_string(), "steam -applaunch 570".to_string(), None);
        let item = LauncherItem::from_app_entry(entry);

        assert_eq!(item.name, "Game");
        match item.action {
            LauncherAction::Launch { .. } => {}
            _ => panic!("expected launch action"),
        }
    }
}
