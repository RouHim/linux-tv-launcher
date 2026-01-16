use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemIcon {
    PowerOff,
    Pause,
    ArrowsRotate,
    ExitBracket,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Games,
    Apps,
    System,
}

impl Category {
    pub fn title(self) -> &'static str {
        match self {
            Category::Apps => "Apps",
            Category::Games => "Games",
            Category::System => "System",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Category::Games => Category::Apps,
            Category::Apps => Category::System,
            Category::System => Category::Games,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Category::Games => Category::System,
            Category::Apps => Category::Games,
            Category::System => Category::Apps,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LauncherAction {
    Launch { exec: String },
    SystemUpdate,
    SystemInfo,
    Shutdown,
    Suspend,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LauncherItem {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub system_icon: Option<SystemIcon>,
    pub action: LauncherAction,
    pub source_image_url: Option<String>,
    pub game_executable: Option<String>,
    /// Unique key for launch history tracking
    pub launch_key: Option<String>,
    /// Unix timestamp of when this item was last started via the launcher
    pub last_started: Option<i64>,
}

impl LauncherItem {
    pub fn from_app_entry(entry: AppEntry) -> Self {
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
            system_icon: None,
            action: LauncherAction::Launch { exec: entry.exec },
            source_image_url,
            game_executable: entry.game_executable,
            launch_key: entry.launch_key,
            last_started: entry.last_started,
        }
    }

    pub fn system_update() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Update System".to_string(),
            icon: None,
            system_icon: Some(SystemIcon::ArrowsRotate),
            action: LauncherAction::SystemUpdate,
            source_image_url: None,
            game_executable: None,
            launch_key: None,
            last_started: None,
        }
    }

    pub fn system_info() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "System Info".to_string(),
            icon: None,
            system_icon: Some(SystemIcon::Info),
            action: LauncherAction::SystemInfo,
            source_image_url: None,
            game_executable: None,
            launch_key: None,
            last_started: None,
        }
    }

    pub fn shutdown() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Shutdown".to_string(),
            icon: None,
            system_icon: Some(SystemIcon::PowerOff),
            action: LauncherAction::Shutdown,
            source_image_url: None,
            game_executable: None,
            launch_key: None,
            last_started: None,
        }
    }

    pub fn suspend() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Suspend".to_string(),
            icon: None,
            system_icon: Some(SystemIcon::Pause),
            action: LauncherAction::Suspend,
            source_image_url: None,
            game_executable: None,
            launch_key: None,
            last_started: None,
        }
    }

    pub fn exit() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Exit Launcher".to_string(),
            icon: None,
            system_icon: Some(SystemIcon::ExitBracket),
            action: LauncherAction::Exit,
            source_image_url: None,
            game_executable: None,
            launch_key: None,
            last_started: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AppEntry {
    pub id: Uuid,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    /// Unique key for launch history tracking
    #[serde(default)]
    pub launch_key: Option<String>,
    #[serde(default)]
    pub game_executable: Option<String>,
    /// Unix timestamp of when this app was last started via the launcher
    #[serde(default)]
    pub last_started: Option<i64>,
}

impl AppEntry {
    pub fn new(name: String, exec: String, icon: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            exec,
            icon,
            launch_key: None,
            game_executable: None,
            last_started: None,
        }
    }

    pub fn with_executable(mut self, executable: Option<String>) -> Self {
        self.game_executable = executable;
        self
    }

    pub fn with_launch_key(mut self, launch_key: String) -> Self {
        self.launch_key = Some(launch_key);
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
        let entry = AppEntry::new("Game".to_string(), "steam -applaunch 570".to_string(), None)
            .with_launch_key("steam:570".to_string());
        let item = LauncherItem::from_app_entry(entry);

        assert_eq!(item.name, "Game");
        assert_eq!(item.launch_key.as_deref(), Some("steam:570"));
        match item.action {
            LauncherAction::Launch { .. } => {}
            _ => panic!("expected launch action"),
        }
    }
}
