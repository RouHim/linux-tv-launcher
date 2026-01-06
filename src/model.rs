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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherAction {
    Launch { exec: String },
    SystemUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherItem {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub action: LauncherAction,
}

impl LauncherItem {
    pub fn from_app_entry(entry: AppEntry) -> Self {
        Self {
            id: entry.id,
            name: entry.name,
            icon: entry.icon,
            action: LauncherAction::Launch { exec: entry.exec },
        }
    }

    pub fn system_update() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Update System".to_string(),
            icon: None,
            action: LauncherAction::SystemUpdate,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppEntry {
    pub id: Uuid,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
}

impl AppEntry {
    pub fn new(name: String, exec: String, icon: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            exec,
            icon,
        }
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
            LauncherAction::SystemUpdate => panic!("expected launch action"),
        }
    }
}
