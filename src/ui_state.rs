use uuid::Uuid;

use crate::auth_flow::AuthFlow;
use crate::model::Category;
use crate::system_info::GamingSystemInfo;
use crate::system_update_state::SystemUpdateState;
use crate::ui_app_picker::AppPickerState;
use crate::updater::ReleaseInfo;
use crate::virtual_keyboard::VirtualKeyboard;

pub enum ModalState {
    None,
    ContextMenu {
        index: usize,
    },
    AppPicker(AppPickerState),
    SystemUpdate(SystemUpdateState),
    SystemUpdateAuth {
        update: SystemUpdateState,
        auth: AuthState,
    },
    AppUpdate(AppUpdateState),
    SystemInfo(Box<Option<GamingSystemInfo>>),
    Auth(AuthState),
    AppNotFound {
        item_id: Uuid,
        item_name: String,
        category: Category,
        selected_index: usize,
    },
    Help,
}

pub struct AppUpdateState {
    pub release: ReleaseInfo,
    pub phase: AppUpdatePhase,
    pub status_message: Option<String>,
    pub spinner_tick: usize,
}

pub struct AuthState {
    pub flow: AuthFlow,
    pub keyboard: VirtualKeyboard,
}

impl AppUpdateState {
    pub fn new(release: ReleaseInfo) -> Self {
        Self {
            release,
            phase: AppUpdatePhase::Prompt,
            status_message: None,
            spinner_tick: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppUpdatePhase {
    Prompt,
    Updating,
    Completed,
    Failed,
}
