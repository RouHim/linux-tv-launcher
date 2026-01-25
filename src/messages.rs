use chrono::{DateTime, Local};
use iced::window;
use std::path::PathBuf;
use uuid::Uuid;

use crate::desktop_apps::DesktopApp;
use crate::gamepad::GamepadInfo;
use crate::input::Action;
use crate::model::AppEntry;
use crate::storage::AppConfig;
use crate::sudo_askpass::AskpassEvent;
use crate::system_info::GamingSystemInfo;
use crate::system_update_state::SystemUpdateProgress;
use crate::updater::ReleaseInfo;
use crate::virtual_keyboard::KeyboardMessage;

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
    // App Update messages
    AppUpdateCheckCompleted(Result<Option<ReleaseInfo>, String>),
    StartAppUpdate,
    AppUpdateApplied(Result<(), String>),
    CloseAppUpdateModal,
    // System Info messages
    OpenSystemInfo,
    SystemInfoLoaded(Box<GamingSystemInfo>),
    CloseSystemInfoModal,
    // Game/App lifecycle
    GameExited,
    WindowOpened(window::Id),
    WindowFocused(window::Id),
    RestartApp,
    GamepadBatteryUpdate(Vec<GamepadInfo>),
    SystemBatteryUpdated(Option<gilrs::PowerInfo>),
    Tick(DateTime<Local>),
    AppUpdateSpinnerTick,
    AskpassEvent(AskpassEvent),
    AuthKeyboard(KeyboardMessage),
    AuthSubmit,
    AuthCancel,
    None,
}
