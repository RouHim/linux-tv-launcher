use uuid::Uuid;

use crate::model::Category;
use crate::system_info::GamingSystemInfo;
use crate::system_update_state::SystemUpdateState;
use crate::ui_app_picker::AppPickerState;

pub enum ModalState {
    None,
    ContextMenu {
        index: usize,
    },
    AppPicker(AppPickerState),
    SystemUpdate(SystemUpdateState),
    SystemInfo(Box<Option<GamingSystemInfo>>),
    AppNotFound {
        item_id: Uuid,
        item_name: String,
        category: Category,
        selected_index: usize,
    },
    Help,
}
