use iced::window;
use std::path::PathBuf;
use uuid::Uuid;

use crate::desktop_apps::DesktopApp;
use crate::input::Action;
use crate::model::AppEntry;
use crate::storage::AppConfig;
use crate::system_update_state::SystemUpdateProgress;

#[derive(Debug, Clone)]
pub enum Message {
    AppsLoaded(Result<AppConfig, String>),
    GamesLoaded(Vec<AppEntry>),
    ImageFetched(Uuid, PathBuf),
    Input(Action),
    ScaleFactorChanged(f64),
    WindowResized(f32, f32),
    // App picker messages
    OpenAppPicker,
    AvailableAppsLoaded(Vec<DesktopApp>),
    AddSelectedApp,
    CloseAppPicker,
    AppPickerScrolled(iced::widget::scrollable::Viewport),
    // System Update messages
    StartSystemUpdate,
    SystemUpdateProgress(SystemUpdateProgress),
    CloseSystemUpdateModal,
    CancelSystemUpdate,
    RequestReboot,
    GameExited,
    WindowOpened(window::Id),
    WindowFocused(window::Id),
    AppUpdateResult(Result<bool, String>),
    RestartApp,
    None,
}
