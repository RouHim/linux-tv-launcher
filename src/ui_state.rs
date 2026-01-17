use crate::system_info::GamingSystemInfo;
use crate::system_update_state::SystemUpdateState;
use crate::ui_app_picker::AppPickerState;

pub enum ModalState {
    None,
    ContextMenu { index: usize },
    AppPicker(AppPickerState),
    SystemUpdate(SystemUpdateState),
    SystemInfo(Box<Option<GamingSystemInfo>>),
    Help,
}
