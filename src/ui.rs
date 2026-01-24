use iced::keyboard::{self, key::Named, Key};
use iced::widget::operation;

use crate::ui_app_update_modal::{handle_app_update_navigation, render_app_update_modal};
use crate::ui_modals::{render_app_not_found_modal, render_context_menu, render_help_modal};
use crate::ui_system_update_modal::render_system_update_modal;
use crate::ui_theme::{
    BASE_FONT_TITLE, BASE_PADDING_SMALL, BATTERY_CHECK_INTERVAL_SECS, CATEGORY_ROW_SPACING,
    GAME_POSTER_HEIGHT, GAME_POSTER_WIDTH, ITEM_SPACING, MAIN_CONTENT_VERTICAL_PADDING,
    MAX_UI_SCALE, MIN_UI_SCALE, REFERENCE_WINDOW_HEIGHT, RESTART_DELAY_SECS,
};
use crate::updater::{apply_update, check_update_available, ReleaseInfo};
use iced::window;
use iced::{
    widget::{Column, Container, Scrollable, Stack},
    Color, Element, Event, Length, Subscription, Task,
};
use tracing::{error, info};

use chrono::{DateTime, Local};
use rayon::prelude::*;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

use crate::assets::get_default_icon;
use crate::category_list::CategoryList;
use crate::desktop_apps::{scan_desktop_apps, DesktopApp};
use crate::focus_manager::{monitor_app_process, MonitorTarget};
use crate::game_image_fetcher::GameImageFetcher;
use crate::game_sources::scan_games;
use crate::gamepad::{gamepad_subscription, GamepadEvent, GamepadInfo};
use crate::image_cache::ImageCache;
use crate::input::Action;
use crate::launcher::{launch_app, resolve_monitor_target, LaunchError};
use crate::messages::Message;
use crate::model::{AppEntry, Category, LauncherAction, LauncherItem};
use crate::osk::OskManager;
use crate::searxng::SearxngClient;
use crate::sleep_inhibit::SleepInhibitor;
use crate::steamgriddb::SteamGridDbClient;
use crate::storage::{load_config, save_config, AppConfig};
use crate::sys_utils::restart_process;
use crate::system_battery::read_system_battery;
use crate::system_info::{fetch_system_info, GamingSystemInfo};
use crate::system_update::{is_update_supported, system_update_stream};
use crate::system_update_state::{SystemUpdateProgress, SystemUpdateState, UpdateStatus};
use crate::ui_app_picker::{render_app_picker, AppPickerState};
use crate::ui_background::WhaleSharkBackground;
use crate::ui_components::{get_battery_visuals, render_clock, render_gamepad_infos};
use crate::ui_main_view::{
    get_category_dimensions, render_controls_hint, render_section_row, render_status,
};
use crate::ui_state::{AppUpdatePhase, AppUpdateState, ModalState};
use crate::ui_system_info_modal::render_system_info_modal;

pub struct Launcher {
    apps: CategoryList,
    games: CategoryList,
    system_items: CategoryList,

    category: Category,
    default_icon_handle: Option<iced::widget::svg::Handle>,
    status_message: Option<String>,

    apps_loaded: bool,
    games_loaded: bool,
    sgdb_client: SteamGridDbClient,
    searxng_client: SearxngClient,
    image_cache: Option<ImageCache>,
    scale_factor: f64,
    window_width: f32,
    window_height: f32, // Track window height for scaling
    ui_scale: f32,      // Calculated UI scale factor
    modal: ModalState,
    // App picker data
    available_apps: Vec<DesktopApp>,
    window_id: Option<window::Id>,
    /// Flag to indicate we are recreating the window (e.g. after game exit)
    /// and should skip initial checks like updates.
    recreating_window: bool,
    // Game running state - disables input subscriptions
    game_running: bool,
    osk_manager: OskManager,
    sleep_inhibitor: SleepInhibitor,
    current_exe: Option<PathBuf>,
    api_key: Option<String>,
    current_time: DateTime<Local>,
    gamepad_infos: Vec<GamepadInfo>,
    /// Stores launch timestamps for games (keyed by game identifier)
    game_launch_history: std::collections::HashMap<String, i64>,
    background: WhaleSharkBackground,
    system_battery: Option<gilrs::PowerInfo>,
    last_battery_check: std::time::Instant,
    pending_update: Option<ReleaseInfo>,
    /// Main vertical scrollable Id for programmatic scroll control
    main_scroll_id: iced::widget::Id,
}

impl Launcher {
    pub fn new() -> (Self, Task<Message>) {
        let default_icon = get_default_icon().map(iced::widget::svg::Handle::from_memory);

        // Resolve API Key:
        // 1. Compile-time env (CI/Production)
        // 2. Runtime env (Local Dev)
        let env_key = option_env!("STEAMGRIDDB_API_KEY")
            .map(|s| s.to_string())
            .or_else(|| std::env::var("STEAMGRIDDB_API_KEY").ok());

        let sgdb_client = SteamGridDbClient::new(env_key.clone().unwrap_or_default());
        let searxng_client = SearxngClient::new();
        let image_cache = ImageCache::new().ok();
        let current_exe = env::current_exe().ok();

        let mut system_items_vec = vec![LauncherItem::shutdown(), LauncherItem::suspend()];

        if is_update_supported() {
            system_items_vec.push(LauncherItem::system_update());
        }

        system_items_vec.push(LauncherItem::system_info());
        system_items_vec.push(LauncherItem::exit());

        // Default 1080p assumption until resize event
        let default_height = 720.0; // Assume 720p minimum safe start
        let initial_scale = default_height / REFERENCE_WINDOW_HEIGHT;

        let launcher = Self {
            apps: CategoryList::new(Vec::new()),
            games: CategoryList::new(Vec::new()),
            system_items: CategoryList::new(system_items_vec),
            category: Category::Games,
            default_icon_handle: default_icon,
            status_message: None,

            apps_loaded: false,
            games_loaded: false,
            sgdb_client,
            searxng_client,
            image_cache,
            scale_factor: 1.0,
            window_width: 1280.0,
            window_height: default_height,
            ui_scale: initial_scale,
            available_apps: Vec::new(),
            modal: ModalState::None,
            window_id: None,
            recreating_window: false,
            game_running: false,
            osk_manager: OskManager::new(),
            sleep_inhibitor: SleepInhibitor::new(),
            current_exe,
            api_key: env_key,
            current_time: Local::now(),
            gamepad_infos: Vec::new(),
            game_launch_history: std::collections::HashMap::new(),
            background: WhaleSharkBackground::new(),
            system_battery: None,
            last_battery_check: std::time::Instant::now(),
            pending_update: None,
            main_scroll_id: iced::widget::Id::unique(),
        };

        // Chain startup: Load config first to potentially get API key, then scan games
        // Also perform initial battery check
        let tasks = Task::batch(vec![
            Task::perform(
                async { load_config().map_err(|err| err.to_string()) },
                Message::AppsLoaded,
            ),
            Task::perform(
                async {
                    tokio::task::spawn_blocking(read_system_battery)
                        .await
                        .ok()
                        .flatten()
                },
                Message::SystemBatteryUpdated,
            ),
        ]);

        (launcher, tasks)
    }

    pub fn title(&self) -> String {
        String::from("RhincoTV")
    }

    fn current_category_list(&self) -> &CategoryList {
        match self.category {
            Category::Apps => &self.apps,
            Category::Games => &self.games,
            Category::System => &self.system_items,
        }
    }

    fn current_category_list_mut(&mut self) -> &mut CategoryList {
        match self.category {
            Category::Apps => &mut self.apps,
            Category::Games => &mut self.games,
            Category::System => &mut self.system_items,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // Initialization & Data Loading
            Message::AppsLoaded(res) => self.handle_apps_loaded(res),
            Message::GamesLoaded(games) => self.handle_games_loaded(games),
            Message::ImageFetched(id, path) => self.handle_image_fetched(id, path),

            // Input & Navigation
            Message::Input(action) => self.handle_navigation(action),

            // Window & System Events
            Message::ScaleFactorChanged(s) => {
                self.scale_factor = s;
                Task::none()
            }
            Message::Tick(t) => {
                self.current_time = t;
                self.maybe_refresh_battery()
            }
            Message::AppUpdateSpinnerTick => {
                if let ModalState::AppUpdate(state) = &mut self.modal {
                    state.spinner_tick = state.spinner_tick.wrapping_add(1);
                }
                Task::none()
            }
            Message::AppUpdateCheckCompleted(res) => self.handle_app_update_check(res),
            Message::StartAppUpdate => self.start_app_update(),
            Message::AppUpdateApplied(res) => self.handle_app_update_applied(res),
            Message::CloseAppUpdateModal => self.close_app_update_modal(),
            Message::RestartApp => self.restart_app(),
            Message::WindowResized(w, h) => {
                self.window_width = w;
                self.window_height = h;
                // Calculate UI scale based on reference height
                // Clamp to reasonable limits to avoid UI disappearing or becoming massive
                self.ui_scale = (h / REFERENCE_WINDOW_HEIGHT).clamp(MIN_UI_SCALE, MAX_UI_SCALE);
                Task::none()
            }
            Message::WindowFocused(id) => {
                if self.window_id.is_none() {
                    self.window_id = Some(id);
                }
                Task::none()
            }
            Message::WindowOpened(id) => self.handle_window_opened(id),

            // App Picker Modal
            Message::OpenAppPicker => self.open_app_picker(),
            Message::AvailableAppsLoaded(apps) => self.handle_available_apps_loaded(apps),
            Message::AddSelectedApp => self.add_selected_app(),
            Message::CloseAppPicker => self.close_modal_none(),
            Message::AppPickerScrolled(vp) => self.handle_app_picker_scrolled(vp),

            // System Update Modal
            Message::StartSystemUpdate => self.start_system_update(),
            Message::SystemUpdateProgress(p) => self.handle_system_update_progress(p),
            Message::CloseSystemUpdateModal => self.close_modal_none(),
            Message::CancelSystemUpdate => self.cancel_system_update(),
            Message::RequestReboot => self.request_reboot(),

            // System Info Modal
            Message::OpenSystemInfo => self.open_system_info(),
            Message::SystemInfoLoaded(info) => self.handle_system_info_loaded(info),
            Message::CloseSystemInfoModal => self.close_modal_none(),

            // Game Execution Monitoring
            Message::GameExited => self.handle_game_exited(),
            Message::GamepadBatteryUpdate(infos) => {
                self.gamepad_infos = infos;
                Task::none()
            }
            Message::SystemBatteryUpdated(info) => {
                self.system_battery = info;
                Task::none()
            }

            Message::None => Task::none(),
        }
    }

    // --- Message Handlers ---

    /// Checks if enough time has passed since the last battery check and spawns a refresh task if needed.
    fn maybe_refresh_battery(&mut self) -> Task<Message> {
        if self.last_battery_check.elapsed().as_secs() < BATTERY_CHECK_INTERVAL_SECS {
            return Task::none();
        }

        self.last_battery_check = std::time::Instant::now();
        Task::perform(
            async {
                tokio::task::spawn_blocking(read_system_battery)
                    .await
                    .ok()
                    .flatten()
            },
            Message::SystemBatteryUpdated,
        )
    }

    fn handle_apps_loaded(&mut self, result: Result<AppConfig, String>) -> Task<Message> {
        self.apps_loaded = true;
        match result {
            Ok(config) => self.process_loaded_apps(config),
            Err(err) => {
                self.apps.clear();
                self.status_message = Some(err);
            }
        }

        // Continue startup chain: Scan games now that we have config (and potential API key)
        Task::perform(
            async {
                tokio::task::spawn_blocking(scan_games)
                    .await
                    .unwrap_or_else(|_| Vec::new())
            },
            Message::GamesLoaded,
        )
    }

    fn process_loaded_apps(&mut self, config: AppConfig) {
        let items: Vec<LauncherItem> = config
            .apps
            .into_iter()
            .map(|entry| {
                let mut item = LauncherItem::from_app_entry(entry);
                if item.launch_key.is_none() {
                    if let LauncherAction::Launch { exec } = &item.action {
                        item.launch_key = Some(format!("desktop:{}", exec));
                    }
                }
                item
            })
            .collect();
        self.apps.set_items(items);
        self.apps.sort_inplace();
        self.status_message = None;

        // Store game launch history for later use when games are loaded
        self.game_launch_history = config.game_launch_history;

        // If no env key was found, try using the one from config
        if self.api_key.is_none() {
            if let Some(key) = config.steamgriddb_api_key {
                self.api_key = Some(key.clone());
                self.sgdb_client = SteamGridDbClient::new(key);
            }
        }
    }

    fn handle_games_loaded(&mut self, games: Vec<AppEntry>) -> Task<Message> {
        let items: Vec<LauncherItem> = games
            .into_iter()
            .map(|entry| {
                let mut item = LauncherItem::from_app_entry(entry);
                // Lookup launch history using game identifier
                if let Some(launch_key) = item.launch_key.as_ref() {
                    if let Some(&timestamp) = self.game_launch_history.get(launch_key) {
                        item.last_started = Some(timestamp);
                    }
                }
                item
            })
            .collect();
        self.games.set_items(items);
        self.games.sort_inplace();
        self.games_loaded = true;
        self.status_message = None;

        self.create_image_fetch_tasks()
    }

    fn create_image_fetch_tasks(&self) -> Task<Message> {
        let Some(cache) = &self.image_cache else {
            return Task::none();
        };

        let target_width = (GAME_POSTER_WIDTH as f64 * self.scale_factor) as u32;
        let target_height = (GAME_POSTER_HEIGHT as f64 * self.scale_factor) as u32;
        let pipeline_template = GameImageFetcher::new(
            cache.cache_dir.clone(),
            self.sgdb_client.clone(),
            self.searxng_client.clone(),
            target_width,
            target_height,
        );

        let tasks: Vec<_> = self
            .games
            .items
            .par_iter()
            .map(|game| {
                let game_id = game.id;
                let game_name = game.name.clone();
                let source_image_url = game.source_image_url.clone();
                let steam_appid = game.steam_appid.clone();
                let pipeline = pipeline_template.clone();

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            pipeline.fetch(
                                game_id,
                                &game_name,
                                source_image_url.as_deref(),
                                steam_appid.as_deref(),
                            )
                        })
                        .await
                        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
                    },
                    |res| match res {
                        Ok(Some((id, path))) => Message::ImageFetched(id, path),
                        _ => Message::None,
                    },
                )
            })
            .collect();

        Task::batch(tasks)
    }

    fn handle_image_fetched(&mut self, id: uuid::Uuid, path: PathBuf) -> Task<Message> {
        self.games.update_item_by_id(id, |item| {
            item.icon = Some(path.to_string_lossy().to_string());
        });
        Task::none()
    }

    fn handle_window_opened(&mut self, id: window::Id) -> Task<Message> {
        self.window_id = Some(id);

        // If we are recreating the window (returning from game), skip the update check
        if self.recreating_window {
            self.recreating_window = false;
            return Task::none();
        }

        // Acquire sleep inhibition now that window is open
        self.sleep_inhibitor.acquire();

        if cfg!(debug_assertions) {
            info!("Debug mode detected: Skipping app update check");
            return Task::none();
        }

        // Defer update check until window is ready
        Task::perform(
            async {
                tokio::task::spawn_blocking(check_update_available)
                    .await
                    .map_err(|e| format!("Task join error: {}", e))
                    .and_then(|r| r)
            },
            Message::AppUpdateCheckCompleted,
        )
    }

    fn open_app_picker(&mut self) -> Task<Message> {
        self.modal = ModalState::AppPicker(AppPickerState::new());
        self.available_apps.clear();
        // Scan for desktop apps asynchronously
        Task::perform(async { scan_desktop_apps() }, Message::AvailableAppsLoaded)
    }

    fn handle_available_apps_loaded(&mut self, apps: Vec<DesktopApp>) -> Task<Message> {
        self.available_apps = self.filter_available_apps(apps);

        if let Some(state) = self.app_picker_state_mut() {
            state.selected_index = 0;
        }
        self.update_app_picker_cols();
        self.snap_to_picker_selection()
    }

    fn filter_available_apps(&self, apps: Vec<DesktopApp>) -> Vec<DesktopApp> {
        // Filter out apps already added (by exec command)
        let existing_execs: std::collections::HashSet<_> = self
            .apps
            .items
            .iter()
            .filter_map(|item| match &item.action {
                LauncherAction::Launch { exec } => Some(exec.clone()),
                _ => None,
            })
            .collect();

        apps.into_iter()
            .filter(|app| !existing_execs.contains(&app.exec))
            .collect()
    }

    fn add_selected_app(&mut self) -> Task<Message> {
        let selected_index = match self.app_picker_state() {
            Some(state) => state.selected_index,
            None => return Task::none(),
        };
        if let Some(selected_app) = self.available_apps.get(selected_index).cloned() {
            let icon_path = selected_app
                .icon_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());

            let new_entry = AppEntry::new(
                selected_app.name.clone(),
                selected_app.exec.clone(),
                icon_path,
            )
            .with_launch_key(format!("desktop:{}", selected_app.exec));

            let new_item = LauncherItem::from_app_entry(new_entry);

            self.apps.add_item(new_item);

            self.save_apps_config("Added", "adding", &selected_app.name);

            // Remove from available apps and close picker
            self.available_apps.remove(selected_index);
            self.close_modal();
        }
        Task::none()
    }

    fn handle_app_picker_scrolled(
        &mut self,
        viewport: iced::widget::scrollable::Viewport,
    ) -> Task<Message> {
        if let Some(state) = self.app_picker_state_mut() {
            state.scroll_offset = viewport.absolute_offset().y;
            state.viewport_height = viewport.bounds().height;
        }
        Task::none()
    }

    fn start_system_update(&mut self) -> Task<Message> {
        self.osk_manager.show();
        self.modal = ModalState::SystemUpdate(SystemUpdateState::new());
        Task::none()
    }

    fn handle_system_update_progress(&mut self, progress: SystemUpdateProgress) -> Task<Message> {
        if let ModalState::SystemUpdate(state) = &mut self.modal {
            // Prevent updates if the process is already finished (e.g. cancelled/failed)
            // This avoids race conditions where pending stream messages overwrite the cancellation state
            if !state.status.is_finished() {
                match progress {
                    SystemUpdateProgress::StatusChange(new_status) => {
                        state.status = new_status;
                    }
                    SystemUpdateProgress::LogLine(line) => {
                        state.output_log.push(line);
                    }
                    SystemUpdateProgress::SpinnerTick => {
                        state.spinner_tick = state.spinner_tick.wrapping_add(1);
                    }
                }
            }
        }
        Task::none()
    }

    fn cancel_system_update(&mut self) -> Task<Message> {
        if let ModalState::SystemUpdate(state) = &mut self.modal {
            // Only allow cancelling if not installing
            if !matches!(state.status, UpdateStatus::Installing { .. }) {
                state.status = UpdateStatus::Failed("Update cancelled by user".to_string());
            }
        }
        Task::none()
    }

    fn request_reboot(&mut self) -> Task<Message> {
        if let Err(e) = std::process::Command::new("systemctl")
            .arg("reboot")
            .spawn()
        {
            self.status_message = Some(format!("Failed to reboot: {}", e));
        }
        Task::none()
    }

    fn open_system_info(&mut self) -> Task<Message> {
        self.modal = ModalState::SystemInfo(Box::new(None));
        Task::perform(
            async { tokio::task::spawn_blocking(fetch_system_info).await.ok() },
            |info| {
                if let Some(info) = info {
                    Message::SystemInfoLoaded(Box::new(info))
                } else {
                    Message::None
                }
            },
        )
    }

    fn handle_system_info_loaded(&mut self, info_box: Box<GamingSystemInfo>) -> Task<Message> {
        if let ModalState::SystemInfo(state) = &mut self.modal {
            **state = Some(*info_box);
        }
        Task::none()
    }

    fn handle_game_exited(&mut self) -> Task<Message> {
        self.game_running = false;
        self.try_show_pending_update();
        if let Some(old_id) = self.window_id {
            let settings = window::Settings {
                decorations: false,
                fullscreen: true,
                level: window::Level::AlwaysOnTop,
                ..Default::default()
            };
            let (new_id, open_task) = window::open(settings);
            self.window_id = Some(new_id);
            self.recreating_window = true;

            // Open the new window. We use the recreating_window flag to ensure
            // the subsequent WindowOpened event doesn't trigger another update check.
            Task::batch(vec![
                open_task.map(|_| Message::None),
                window::close(old_id),
            ])
        } else {
            Task::none()
        }
    }

    fn handle_app_update_check(
        &mut self,
        result: Result<Option<crate::updater::ReleaseInfo>, String>,
    ) -> Task<Message> {
        match result {
            Ok(Some(release)) => {
                self.pending_update = Some(release);
                self.try_show_pending_update();
                Task::none()
            }
            Ok(None) => Task::none(),
            Err(err) => {
                error!("App update check failed: {}", err);
                Task::none()
            }
        }
    }

    fn try_show_pending_update(&mut self) {
        if self.game_running {
            return;
        }
        if matches!(self.modal, ModalState::None) {
            if let Some(release) = self.pending_update.take() {
                self.modal = ModalState::AppUpdate(AppUpdateState::new(release));
            }
        }
    }

    /// Close the current modal and attempt to show any pending update.
    /// Use this helper instead of manually setting `self.modal = ModalState::None`
    /// followed by `self.try_show_pending_update()`.
    fn close_modal(&mut self) {
        self.modal = ModalState::None;
        self.try_show_pending_update();
    }

    /// Convenience method that closes the modal and returns `Task::none()`.
    /// Use this to reduce boilerplate in navigation handlers.
    fn close_modal_none(&mut self) -> Task<Message> {
        self.close_modal();
        Task::none()
    }

    fn start_app_update(&mut self) -> Task<Message> {
        if let ModalState::AppUpdate(state) = &mut self.modal {
            state.phase = AppUpdatePhase::Updating;
            state.status_message = None;
        }

        Task::perform(
            async {
                tokio::task::spawn_blocking(apply_update)
                    .await
                    .map_err(|e| format!("Task join error: {}", e))
                    .and_then(|r| r)
            },
            Message::AppUpdateApplied,
        )
    }

    fn handle_app_update_applied(&mut self, result: Result<(), String>) -> Task<Message> {
        match result {
            Ok(()) => {
                if let ModalState::AppUpdate(state) = &mut self.modal {
                    state.phase = AppUpdatePhase::Completed;
                    state.status_message = None;

                    info!("App update complete. Restarting.");
                    return Task::perform(
                        async {
                            tokio::time::sleep(Duration::from_secs(RESTART_DELAY_SECS)).await;
                        },
                        |_| Message::RestartApp,
                    );
                }
                Task::none()
            }
            Err(err) => {
                if let ModalState::AppUpdate(state) = &mut self.modal {
                    state.phase = AppUpdatePhase::Failed;
                    state.status_message = Some(err);
                }
                Task::none()
            }
        }
    }

    fn close_app_update_modal(&mut self) -> Task<Message> {
        if matches!(self.modal, ModalState::AppUpdate(_)) {
            self.modal = ModalState::None;
        }
        Task::none()
    }

    fn restart_app(&mut self) -> Task<Message> {
        if let Some(exe) = &self.current_exe {
            restart_process(exe.clone());
        }
        Task::none()
    }

    fn update_app_picker_cols(&mut self) {
        let width = self.window_width;
        let scale = self.ui_scale;
        if let Some(state) = self.app_picker_state_mut() {
            state.update_cols(width, scale);
        }
    }

    fn app_picker_state(&self) -> Option<&AppPickerState> {
        match &self.modal {
            ModalState::AppPicker(state) => Some(state),
            _ => None,
        }
    }

    fn app_picker_state_mut(&mut self) -> Option<&mut AppPickerState> {
        match &mut self.modal {
            ModalState::AppPicker(state) => Some(state),
            _ => None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = self.render_category();

        let mut column = Column::new().push(content);
        if let Some(status) = render_status(&self.status_message, self.ui_scale) {
            column = column.push(status);
        }

        let scrollable_content = Scrollable::new(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .id(self.main_scroll_id.clone())
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4.0 * self.ui_scale)
                    .scroller_width(3.0 * self.ui_scale),
            ))
            .style(|_theme, _status| {
                use crate::ui_theme::{COLOR_PANEL, COLOR_TEXT_MUTED};
                let scroller = iced::widget::scrollable::Scroller {
                    background: iced::Background::Color(COLOR_TEXT_MUTED),
                    border: iced::Border {
                        radius: 2.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                };
                let rail = iced::widget::scrollable::Rail {
                    background: Some(iced::Background::Color(COLOR_PANEL)),
                    border: iced::Border {
                        radius: 2.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                    scroller,
                };
                iced::widget::scrollable::Style {
                    container: iced::widget::container::Style::default(),
                    vertical_rail: rail,
                    horizontal_rail: rail,
                    gap: None,
                    auto_scroll: iced::widget::scrollable::AutoScroll {
                        background: iced::Background::Color(COLOR_PANEL),
                        border: iced::Border::default(),
                        shadow: iced::Shadow::default(),
                        icon: Color::WHITE,
                    },
                }
            });

        let main_content = Container::new(scrollable_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(iced::Padding {
                top: MAIN_CONTENT_VERTICAL_PADDING * self.ui_scale,
                bottom: MAIN_CONTENT_VERTICAL_PADDING * self.ui_scale,
                ..Default::default()
            })
            .style(|_theme| iced::widget::container::Style {
                background: Some(Color::TRANSPARENT.into()),
                text_color: Some(Color::WHITE),
                ..Default::default()
            });

        let mut status_bar_row = iced::widget::Row::new()
            .align_y(iced::Alignment::Center)
            .push(render_gamepad_infos(&self.gamepad_infos, self.ui_scale))
            .push(iced::widget::Space::new().width(Length::Fill));

        if let Some(battery_info) = self.system_battery {
            if let Some((icon, _color)) = get_battery_visuals(battery_info, self.ui_scale) {
                status_bar_row = status_bar_row
                    .push(icon)
                    .push(iced::widget::Space::new().width(16.0 * self.ui_scale));
            }
        }

        let status_bar_row = status_bar_row.push(render_clock(&self.current_time, self.ui_scale));

        let status_bar = Container::new(status_bar_row)
            .padding([10.0 * self.ui_scale, 20.0 * self.ui_scale])
            .width(Length::Fill);

        let background = self.background.view();

        let mut base_stack = Stack::new()
            .push(background)
            .push(main_content)
            .push(status_bar);

        // Add controls hint when no modal is open
        if matches!(&self.modal, ModalState::None) {
            let hint_layer = Column::new()
                .push(iced::widget::Space::new().height(Length::Fill))
                .push(render_controls_hint(self.ui_scale));
            base_stack = base_stack.push(hint_layer);
        }

        let base_view = base_stack.into();

        self.render_with_modal(base_view)
    }

    fn render_with_modal<'a>(&'a self, main_content: Element<'a, Message>) -> Element<'a, Message> {
        if let Some(overlay) = self.render_modal_layer() {
            Stack::new().push(main_content).push(overlay).into()
        } else {
            main_content
        }
    }

    fn render_modal_layer(&self) -> Option<Element<'_, Message>> {
        let scale = self.ui_scale;
        match &self.modal {
            ModalState::ContextMenu { index } => {
                Some(render_context_menu(*index, self.category, scale))
            }
            ModalState::AppPicker(state) => {
                Some(render_app_picker(state, &self.available_apps, scale))
            }
            ModalState::SystemUpdate(state) => Some(render_system_update_modal(state, scale)),
            ModalState::AppUpdate(state) => Some(render_app_update_modal(state, scale)),
            ModalState::SystemInfo(info) => Some(render_system_info_modal(info, scale)),
            ModalState::AppNotFound {
                item_name,
                selected_index,
                ..
            } => Some(render_app_not_found_modal(
                item_name,
                *selected_index,
                scale,
            )),
            ModalState::Help => Some(render_help_modal(scale)),
            ModalState::None => None,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // Disable all input subscriptions while a game is running
        if self.game_running {
            return Subscription::none();
        }

        let gamepad = gamepad_subscription().map(|event| match event {
            GamepadEvent::Input(action) => Message::Input(action),
            GamepadEvent::Battery(batteries) => Message::GamepadBatteryUpdate(batteries),
        });

        let window_events = iced::event::listen_with(|event, _status, window_id| match event {
            Event::Window(iced::window::Event::Opened { .. }) => {
                Some(Message::WindowOpened(window_id))
            }
            Event::Window(iced::window::Event::Rescaled(scale_factor)) => {
                Some(Message::ScaleFactorChanged(scale_factor as f64))
            }
            Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::WindowResized(size.width, size.height))
            }
            Event::Window(iced::window::Event::Focused) => Some(Message::WindowFocused(window_id)),
            _ => None,
        });

        let keyboard = self.build_keyboard_subscription();

        let mut subscriptions = vec![gamepad, keyboard, window_events];

        // Clock subscription (every 1 second)
        subscriptions
            .push(iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick(Local::now())));

        // System update subscriptions (stream + spinner)
        if let ModalState::SystemUpdate(state) = &self.modal {
            if state.status.is_running() {
                subscriptions.push(
                    Subscription::run(system_update_stream).map(Message::SystemUpdateProgress),
                );
                subscriptions.push(
                    iced::time::every(Duration::from_millis(150))
                        .map(|_| Message::SystemUpdateProgress(SystemUpdateProgress::SpinnerTick)),
                );
            }
        }

        if let ModalState::AppUpdate(state) = &self.modal {
            if state.phase == AppUpdatePhase::Updating {
                subscriptions.push(
                    iced::time::every(Duration::from_millis(150))
                        .map(|_| Message::AppUpdateSpinnerTick),
                );
            }
        }

        Subscription::batch(subscriptions)
    }

    fn build_keyboard_subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, status, _window| {
            if let iced::event::Status::Captured = status {
                return None;
            }

            match event {
                Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => match key.as_ref() {
                    Key::Named(Named::ArrowUp) => Some(Message::Input(Action::Up)),
                    Key::Named(Named::ArrowDown) => Some(Message::Input(Action::Down)),
                    Key::Named(Named::ArrowLeft) => Some(Message::Input(Action::Left)),
                    Key::Named(Named::ArrowRight) => Some(Message::Input(Action::Right)),
                    Key::Named(Named::Enter) => Some(Message::Input(Action::Select)),
                    Key::Named(Named::Escape) => Some(Message::Input(Action::Back)),
                    Key::Named(Named::Tab) => Some(Message::Input(Action::NextCategory)),
                    Key::Named(Named::F4) => Some(Message::Input(Action::Quit)),
                    Key::Character("c") => Some(Message::Input(Action::ContextMenu)),
                    Key::Character("+") | Key::Character("a") => {
                        Some(Message::Input(Action::AddApp))
                    }
                    Key::Character("-") => Some(Message::Input(Action::ShowHelp)),
                    _ => None,
                },
                _ => None,
            }
        })
    }

    fn handle_modal_navigation(&mut self, action: Action) -> Option<Task<Message>> {
        match &self.modal {
            ModalState::Help => Some(self.handle_help_modal_navigation(action)),
            ModalState::ContextMenu { .. } => Some(self.handle_context_menu_navigation(action)),
            ModalState::AppPicker(_) => Some(self.handle_app_picker_navigation(action)),
            ModalState::SystemUpdate(_) => Some(self.handle_system_update_navigation(action)),
            ModalState::AppUpdate(state) => {
                handle_app_update_navigation(state, action).map(|message| self.update(message))
            }
            ModalState::SystemInfo(_) => Some(self.handle_system_info_navigation(action)),
            ModalState::AppNotFound { .. } => Some(self.handle_app_not_found_navigation(action)),
            ModalState::None => None,
        }
    }

    fn exit_app(&mut self) -> ! {
        self.osk_manager.restore();
        self.sleep_inhibitor.release();
        std::process::exit(0);
    }

    fn handle_navigation(&mut self, action: Action) -> Task<Message> {
        if action == Action::Quit {
            self.exit_app();
        }

        // Modal navigation takes priority
        if let Some(task) = self.handle_modal_navigation(action) {
            return task;
        }

        // Handle global actions first
        match action {
            Action::ShowHelp => {
                self.modal = ModalState::Help;
                return Task::none();
            }
            Action::AddApp if self.category == Category::Apps => {
                return self.update(Message::OpenAppPicker);
            }
            Action::ContextMenu if !self.current_category_list().is_empty() => {
                self.modal = ModalState::ContextMenu { index: 0 };
                return Task::none();
            }
            Action::Back => {
                self.status_message = None;
                return Task::none();
            }
            _ => {}
        }

        // Handle directional navigation
        self.handle_directional_navigation(action)
    }

    /// Handles Up/Down/Left/Right and category cycling navigation.
    fn handle_directional_navigation(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::Up => {
                let prev_cat = self.category.prev();
                if prev_cat != self.category {
                    self.category = prev_cat;
                    return self.snap_to_main_selection();
                }
            }
            Action::Down => {
                let next_cat = self.category.next();
                if next_cat != self.category {
                    self.category = next_cat;
                    return self.snap_to_main_selection();
                }
            }
            Action::Left if self.current_category_list_mut().move_left() => {
                return self.snap_to_main_selection();
            }
            Action::Right if self.current_category_list_mut().move_right() => {
                return self.snap_to_main_selection();
            }
            Action::Select if !self.current_category_list().is_empty() => {
                return self.activate_selected();
            }
            Action::NextCategory => {
                self.cycle_category();
                return self.snap_to_main_selection();
            }
            Action::PrevCategory => {
                self.cycle_category_back();
                return self.snap_to_main_selection();
            }
            _ => {}
        }

        Task::none()
    }

    fn snap_to_main_selection(&self) -> Task<Message> {
        let list = self.current_category_list();
        let scroll_id = list.scroll_id.clone();

        let (item_width, _item_height, _image_width, _image_height) =
            get_category_dimensions(self.category, self.ui_scale);

        let item_width_with_spacing = item_width + (ITEM_SPACING * self.ui_scale);

        let target_x = list.selected_index as f32 * item_width_with_spacing;
        let center_offset = target_x - (self.window_width / 2.0) + (item_width / 2.0);

        operation::scroll_to(
            scroll_id,
            iced::widget::scrollable::AbsoluteOffset {
                x: center_offset.max(0.0),
                y: 0.0,
            },
        )
        .chain(self.scroll_main_to_category())
    }

    fn scroll_main_to_category(&self) -> Task<Message> {
        let category_index = match self.category {
            Category::Games => 0,
            Category::Apps => 1,
            Category::System => 2,
        };

        let title_height = BASE_FONT_TITLE * self.ui_scale;
        let padding = BASE_PADDING_SMALL * self.ui_scale;
        let spacing = CATEGORY_ROW_SPACING * self.ui_scale;

        let mut target_y = 0.0;

        for i in 0..category_index {
            let cat = match i {
                0 => Category::Games,
                1 => Category::Apps,
                _ => Category::System,
            };

            let (_item_width, item_height, _image_width, _image_height) =
                get_category_dimensions(cat, self.ui_scale);

            let row_height = item_height;

            target_y += title_height + padding + row_height + padding + spacing;
        }

        operation::scroll_to(
            self.main_scroll_id.clone(),
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: target_y.max(0.0),
            },
        )
    }

    fn handle_context_menu_navigation(&mut self, action: Action) -> Task<Message> {
        let mut index = match &self.modal {
            ModalState::ContextMenu { index } => *index,
            _ => return Task::none(),
        };

        // Context menu options vary by category:
        // Apps: [Launch, Remove, Quit, Close] (indices 0-3)
        // Games/System: [Launch, Quit, Close] (indices 0-2)
        let max_index = if self.category == Category::Apps {
            3
        } else {
            2
        };

        match action {
            Action::Up => index = index.saturating_sub(1),
            Action::Down => index = (index + 1).min(max_index),
            Action::Back | Action::ContextMenu => return self.close_modal_none(),
            Action::Select => return self.execute_context_menu_action(index),
            _ => {}
        }

        self.modal = ModalState::ContextMenu { index };
        Task::none()
    }

    /// Executes the selected context menu action based on category and index.
    fn execute_context_menu_action(&mut self, index: usize) -> Task<Message> {
        // Index 0 is always "Launch" for all categories
        if index == 0 {
            self.modal = ModalState::None;
            return self.activate_selected();
        }

        // For Apps category: index 1 = Remove, index 2 = Quit, index 3 = Close
        // For Games/System: index 1 = Quit, index 2 = Close
        let (remove_index, quit_index, close_index) = if self.category == Category::Apps {
            (Some(1), 2, 3)
        } else {
            (None, 1, 2)
        };

        if remove_index == Some(index) {
            self.close_modal();
            if let Some(removed) = self.apps.remove_selected() {
                self.save_apps_config("Removed", "removing", &removed.name);
            }
            return Task::none();
        }

        if index == quit_index {
            self.exit_app();
        }

        // close_index or any unhandled index -> close modal
        if index == close_index {
            return self.close_modal_none();
        }

        Task::none()
    }

    fn handle_help_modal_navigation(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::Back | Action::ShowHelp => self.close_modal_none(),
            _ => Task::none(), // Ignore other inputs while modal is open
        }
    }

    fn handle_app_not_found_navigation(&mut self, action: Action) -> Task<Message> {
        let (item_id, item_name, category, mut selected_index) = match &self.modal {
            ModalState::AppNotFound {
                item_id,
                item_name,
                category,
                selected_index,
            } => (*item_id, item_name.clone(), *category, *selected_index),
            _ => return Task::none(),
        };

        match action {
            Action::Left | Action::Right | Action::Up | Action::Down => {
                // Toggle between the two options (Remove / Cancel)
                selected_index = 1 - selected_index;
            }
            Action::Select => {
                if selected_index == 0 {
                    self.remove_missing_item(item_id, &item_name, category);
                }
                return self.close_modal_none();
            }
            Action::Back | Action::ContextMenu | Action::ShowHelp => {
                return self.close_modal_none();
            }
            _ => {}
        }

        self.modal = ModalState::AppNotFound {
            item_id,
            item_name,
            category,
            selected_index,
        };
        Task::none()
    }

    fn handle_system_update_navigation(&mut self, action: Action) -> Task<Message> {
        if let ModalState::SystemUpdate(state) = &self.modal {
            match &state.status {
                UpdateStatus::Completed { restart_required } if *restart_required => match action {
                    Action::Select => return self.update(Message::RequestReboot),
                    Action::Back | Action::ShowHelp => {
                        return self.update(Message::CloseSystemUpdateModal)
                    }
                    _ => {}
                },
                // Finished states -> Close
                status if status.is_finished() => match action {
                    Action::Back | Action::Select | Action::ShowHelp => {
                        return self.update(Message::CloseSystemUpdateModal);
                    }
                    _ => {}
                },
                // Running states -> Cancel if allowed
                status if status.is_running() => {
                    if !matches!(status, UpdateStatus::Installing { .. }) && action == Action::Back
                    {
                        return self.update(Message::CancelSystemUpdate);
                    }
                }
                _ => {}
            }
        }
        Task::none()
    }

    fn handle_system_info_navigation(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::Back | Action::Select | Action::ShowHelp => {
                return self.update(Message::CloseSystemInfoModal);
            }
            _ => {}
        }
        Task::none()
    }

    fn snap_to_picker_selection(&self) -> Task<Message> {
        let scale = self.ui_scale;
        self.app_picker_state()
            .map(|state| state.snap_to_selection(scale))
            .unwrap_or(Task::none())
    }

    fn handle_app_picker_navigation(&mut self, action: Action) -> Task<Message> {
        let list_len = self.available_apps.len();

        // Handle close actions regardless of app count
        if matches!(action, Action::Back | Action::AddApp) {
            return self.update(Message::CloseAppPicker);
        }

        if list_len == 0 {
            return Task::none();
        }

        match action {
            Action::Select => return self.update(Message::AddSelectedApp),
            _ => {
                if let Some(state) = self.app_picker_state_mut() {
                    state.navigate(action, list_len);
                }
            }
        }

        self.snap_to_picker_selection()
    }

    fn activate_selected(&mut self) -> Task<Message> {
        if self.current_category_list().get_selected().is_none() {
            return Task::none();
        }

        self.status_message = None;

        let item = self.current_category_list().get_selected().unwrap().clone();

        match &item.action {
            LauncherAction::Launch { exec } => {
                self.launch_app(exec, &item, item.game_executable.as_ref())
            }
            LauncherAction::SystemUpdate => self.update(Message::StartSystemUpdate),
            LauncherAction::SystemInfo => self.update(Message::OpenSystemInfo),
            LauncherAction::Shutdown => self.system_command("systemctl", &["poweroff"], "shutdown"),
            LauncherAction::Suspend => self.system_command("systemctl", &["suspend"], "suspend"),
            LauncherAction::Exit => self.exit_app(),
        }
    }

    /// Records the current timestamp for the launched item, updates the list, re-sorts, and persists
    fn record_launch_timestamp(&mut self, item: &LauncherItem) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let item_id = item.id;
        let item_name = item.name.clone();

        match self.category {
            Category::Apps => {
                self.apps.update_item_by_id(item_id, |i| {
                    i.last_started = Some(now);
                });
                self.apps.sort_inplace();
                // Reset selection to 0 so the just-launched item stays selected at top
                self.apps.selected_index = 0;
                self.save_apps_config("Launched", "launching", &item_name);
            }
            Category::Games => {
                self.games.update_item_by_id(item_id, |i| {
                    i.last_started = Some(now);
                });
                self.games.sort_inplace();
                self.games.selected_index = 0;
                // Update game launch history and persist
                if let Some(launch_key) = item.launch_key.as_ref() {
                    self.game_launch_history.insert(launch_key.clone(), now);
                }
                self.save_apps_config("Launched", "launching", &item_name);
            }
            Category::System => {
                // System items don't need launch tracking
            }
        }
    }

    fn remove_missing_item(&mut self, item_id: Uuid, item_name: &str, category: Category) {
        let removed = match category {
            Category::Apps => self.apps.remove_item_by_id(item_id).is_some(),
            Category::Games => {
                if let Some(removed_item) = self.games.remove_item_by_id(item_id) {
                    if let Some(launch_key) = removed_item.launch_key.as_ref() {
                        self.game_launch_history.remove(launch_key);
                    }
                    true
                } else {
                    false
                }
            }
            Category::System => false,
        };

        if removed {
            self.save_apps_config("Removed", "removing", item_name);
        }
    }

    /// Launch an application with proper process monitoring
    fn launch_app(
        &mut self,
        exec: &str,
        item: &LauncherItem,
        game_executable: Option<&String>,
    ) -> Task<Message> {
        let monitor_target = resolve_monitor_target(exec, &item.name, game_executable);

        match launch_app(exec) {
            Ok(pid) => {
                self.game_running = true;
                self.record_launch_timestamp(item);

                // Optimization: Always check the main PID first.
                // If the direct PID is running, we avoid the expensive full-system scan
                // required for resolving monitor targets (names, env vars, etc.).
                let target = match monitor_target {
                    Some(t) => MonitorTarget::Any(vec![MonitorTarget::Pid(pid), t]),
                    None => MonitorTarget::Pid(pid),
                };

                let monitor_task =
                    Task::perform(async move { monitor_app_process(target).await }, |_| {
                        Message::GameExited
                    });

                if let Some(id) = self.window_id {
                    Task::batch(vec![window::minimize(id, true), monitor_task])
                } else {
                    monitor_task
                }
            }
            Err(LaunchError::CommandNotFound { .. }) => {
                self.modal = ModalState::AppNotFound {
                    item_id: item.id,
                    item_name: item.name.clone(),
                    category: self.category,
                    selected_index: 0,
                };
                Task::none()
            }
            Err(err) => {
                self.status_message = Some(err.to_string());
                Task::none()
            }
        }
    }

    /// Execute a system command and handle errors
    fn system_command(&mut self, command: &str, args: &[&str], action: &str) -> Task<Message> {
        if let Err(e) = std::process::Command::new(command).args(args).spawn() {
            self.status_message = Some(format!("Failed to {}: {}", action, e));
        }
        Task::none()
    }

    fn cycle_category(&mut self) {
        self.category = self.category.next();
        self.status_message = None;
    }

    fn cycle_category_back(&mut self) {
        self.category = self.category.prev();
        self.status_message = None;
    }

    fn render_category(&self) -> Element<'_, Message> {
        let apps_msg = if !self.apps_loaded {
            "Loading apps...".to_string()
        } else {
            self.apps_empty_message()
        };

        let apps_row = render_section_row(
            self.category,
            Category::Apps,
            &self.apps,
            apps_msg,
            self.default_icon_handle.clone(),
            self.ui_scale,
        );

        let games_msg = if !self.games_loaded {
            "Scanning games...".to_string()
        } else {
            "No games found.".to_string()
        };

        let games_row = render_section_row(
            self.category,
            Category::Games,
            &self.games,
            games_msg,
            self.default_icon_handle.clone(),
            self.ui_scale,
        );

        let system_row = render_section_row(
            self.category,
            Category::System,
            &self.system_items,
            "No system actions available.".to_string(),
            self.default_icon_handle.clone(),
            self.ui_scale,
        );

        Column::new()
            .push(games_row)
            .push(apps_row)
            .push(system_row)
            .spacing(40.0 * self.ui_scale) // Adjusted spacing with scale
            .into()
    }

    fn save_apps_config(&self, action_desc: &str, action_gerund: &str, item_name: &str) {
        let mut config = load_config().unwrap_or_default();

        config.apps = self
            .apps
            .items
            .iter()
            .filter(|item| matches!(item.action, LauncherAction::Launch { .. }))
            .map(|item| item.to_app_entry())
            .collect();

        // Also save game launch history
        config.game_launch_history = self.game_launch_history.clone();

        match save_config(&config) {
            Ok(_) => info!("{} '{}' and saved config.", action_desc, item_name),
            Err(e) => error!(
                "Error saving config after {} '{}': {}",
                action_gerund, item_name, e
            ),
        }
    }

    fn apps_empty_message(&self) -> String {
        "No desktop applications found.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_memory() {
        let (mut launcher, _) = Launcher::new();
        // Setup mock data
        launcher.apps.set_items(vec![
            LauncherItem::exit(), // 0
            LauncherItem::exit(), // 1
        ]);
        launcher.games.set_items(vec![
            LauncherItem::exit(), // 0
            LauncherItem::exit(), // 1
            LauncherItem::exit(), // 2
        ]);

        // Start at Apps
        launcher.category = Category::Apps;
        launcher.apps.selected_index = 0;

        // Move Right -> Apps index 1
        let _ = launcher.handle_navigation(Action::Right);
        assert_eq!(launcher.apps.selected_index, 1);

        // Switch to Games (Up)
        let _ = launcher.handle_navigation(Action::Up);
        assert_eq!(launcher.category, Category::Games);
        assert_eq!(launcher.games.selected_index, 0); // Default

        // Move Right -> Games index 1
        let _ = launcher.handle_navigation(Action::Right);
        assert_eq!(launcher.games.selected_index, 1);

        // Switch back to Apps (Down)
        let _ = launcher.handle_navigation(Action::Down);
        assert_eq!(launcher.category, Category::Apps);
        assert_eq!(launcher.apps.selected_index, 1); // REMEMBERED!
    }

    #[test]
    fn test_bounds_checking() {
        let (mut launcher, _) = Launcher::new();
        launcher.apps.set_items(vec![LauncherItem::exit()]); // Len 1
        launcher.apps.selected_index = 0;

        // Right on last item -> should stay
        let _ = launcher.handle_navigation(Action::Right);
        assert_eq!(launcher.apps.selected_index, 0);

        // Left on first item -> should stay
        let _ = launcher.handle_navigation(Action::Left);
        assert_eq!(launcher.apps.selected_index, 0);
    }
}
