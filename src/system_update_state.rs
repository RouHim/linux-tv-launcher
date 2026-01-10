#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    Starting,
    SyncingDatabases,
    CheckingUpdates,
    Downloading {
        package: Option<String>,
    },
    Building {
        package: String,
    },
    Installing {
        current: usize,
        total: usize,
        package: String,
    },
    Completed {
        restart_required: bool,
    },
    Failed(String),
    NoUpdates,
}

impl UpdateStatus {
    pub fn is_running(&self) -> bool {
        matches!(
            self,
            Self::Starting
                | Self::SyncingDatabases
                | Self::CheckingUpdates
                | Self::Downloading { .. }
                | Self::Building { .. }
                | Self::Installing { .. }
        )
    }

    pub fn is_finished(&self) -> bool {
        !self.is_running()
    }
}

#[derive(Debug, Clone)]
pub struct SystemUpdateState {
    pub status: UpdateStatus,
    pub spinner_tick: usize,
    pub output_log: Vec<String>,
}

impl SystemUpdateState {
    pub fn new() -> Self {
        Self {
            status: UpdateStatus::Starting,
            spinner_tick: 0,
            output_log: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SystemUpdateProgress {
    StatusChange(UpdateStatus),
    LogLine(String),
    SpinnerTick,
}
