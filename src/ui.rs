use iced::keyboard::{self, key::Named, Key};
use iced::widget::operation;

use crate::ui_modals::{render_app_not_found_modal, render_context_menu, render_help_modal};
use crate::ui_system_update_modal::render_system_update_modal;
use crate::ui_theme::*;
use iced::window;
use iced::{
    widget::{Column, Container, Stack},
    Color, Element, Event, Length, Subscription, Task,
};
use tracing::error;

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
use crate::ui_state::ModalState;
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
    modal: ModalState,
    // App picker data
    available_apps: Vec<DesktopApp>,
    window_id: Option<window::Id>,
    // Game running state - disables input subscriptions
    game_running: bool,
    osk_manager: OskManager,
    current_exe: Option<PathBuf>,
    api_key: Option<String>,
    current_time: DateTime<Local>,
    gamepad_infos: Vec<GamepadInfo>,
    /// Stores launch timestamps for games (keyed by game identifier)
    game_launch_history: std::collections::HashMap<String, i64>,
    background: WhaleSharkBackground,
    system_battery: Option<gilrs::PowerInfo>,
    last_battery_check: std::time::Instant,
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
            available_apps: Vec::new(),
            modal: ModalState::None,
            window_id: None,
            game_running: false,
            osk_manager: OskManager::new(),
            current_exe,
            api_key: env_key,
            current_time: Local::now(),
            gamepad_infos: Vec::new(),
            game_launch_history: std::collections::HashMap::new(),
            background: WhaleSharkBackground::new(),
            system_battery: None,
            last_battery_check: std::time::Instant::now(),
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
                let mut tasks = Vec::new();

                // Check battery once a minute
                if self.last_battery_check.elapsed().as_secs() >= 60 {
                    self.last_battery_check = std::time::Instant::now();
                    tasks.push(Task::perform(
                        async {
                            tokio::task::spawn_blocking(read_system_battery)
                                .await
                                .ok()
                                .flatten()
                        },
                        Message::SystemBatteryUpdated,
                    ));
                }
                Task::batch(tasks)
            }
            Message::WindowResized(w) => {
                self.window_width = w;
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
            Message::CloseAppPicker => {
                self.modal = ModalState::None;
                Task::none()
            }
            Message::AppPickerScrolled(vp) => self.handle_app_picker_scrolled(vp),

            // System Update Modal
            Message::StartSystemUpdate => self.start_system_update(),
            Message::SystemUpdateProgress(p) => self.handle_system_update_progress(p),
            Message::CloseSystemUpdateModal => {
                self.modal = ModalState::None;
                Task::none()
            }
            Message::CancelSystemUpdate => self.cancel_system_update(),
            Message::RequestReboot => self.request_reboot(),

            // System Info Modal
            Message::OpenSystemInfo => self.open_system_info(),
            Message::SystemInfoLoaded(info) => self.handle_system_info_loaded(info),
            Message::CloseSystemInfoModal => {
                self.modal = ModalState::None;
                Task::none()
            }

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

            // App Updates (Self-updater)
            Message::AppUpdateResult(res) => self.handle_app_update_result(res),
            Message::RestartApp => self.restart_app(),

            Message::None => Task::none(),
        }
    }

    // --- Message Handlers ---

    fn handle_apps_loaded(&mut self, result: Result<AppConfig, String>) -> Task<Message> {
        self.apps_loaded = true;
        match result {
            Ok(config) => {
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

        // Spawn tasks to fetch images
        let mut tasks = Vec::new();
        if let Some(cache) = &self.image_cache {
            let target_width = (GAME_POSTER_WIDTH as f64 * self.scale_factor) as u32;
            let target_height = (GAME_POSTER_HEIGHT as f64 * self.scale_factor) as u32;
            let pipeline_template = GameImageFetcher::new(
                cache.cache_dir.clone(),
                self.sgdb_client.clone(),
                self.searxng_client.clone(),
                target_width,
                target_height,
            );

            tasks = self
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
        }
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
        // Defer update check until window is ready
        Task::perform(
            async {
                tokio::task::spawn_blocking(crate::updater::check_for_updates)
                    .await
                    .map_err(|e| format!("Task join error: {}", e))
                    .and_then(|r| r)
            },
            Message::AppUpdateResult,
        )
    }

    fn open_app_picker(&mut self) -> Task<Message> {
        self.modal = ModalState::AppPicker(AppPickerState::new());
        self.available_apps.clear();
        // Scan for desktop apps asynchronously
        Task::perform(async { scan_desktop_apps() }, Message::AvailableAppsLoaded)
    }

    fn handle_available_apps_loaded(&mut self, apps: Vec<DesktopApp>) -> Task<Message> {
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

        self.available_apps = apps
            .into_iter()
            .filter(|app| !existing_execs.contains(&app.exec))
            .collect();
        if let Some(state) = self.app_picker_state_mut() {
            state.selected_index = 0;
        }
        self.update_app_picker_cols();
        self.snap_to_picker_selection()
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
            self.modal = ModalState::None;
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
        if let Some(old_id) = self.window_id {
            let settings = window::Settings {
                decorations: false,
                fullscreen: true,
                level: window::Level::AlwaysOnTop,
                ..Default::default()
            };
            let (new_id, open_task) = window::open(settings);
            self.window_id = Some(new_id);

            Task::batch(vec![
                open_task.map(Message::WindowOpened),
                window::close(old_id),
            ])
        } else {
            Task::none()
        }
    }

    fn handle_app_update_result(&mut self, result: Result<bool, String>) -> Task<Message> {
        match result {
            Ok(updated) => {
                if updated {
                    self.status_message = Some("App updated, restarting...".to_string());
                    Task::perform(
                        async {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                        },
                        |_| Message::RestartApp,
                    )
                } else {
                    // Silent update check when no updates found
                    Task::none()
                }
            }
            Err(_e) => {
                // Log error but don't show user facing message for background check failure
                Task::none()
            }
        }
    }

    fn restart_app(&mut self) -> Task<Message> {
        if let Some(exe) = &self.current_exe {
            restart_process(exe.clone());
        }
        Task::none()
    }

    fn update_app_picker_cols(&mut self) {
        let available_width = self.window_width * 0.8 - 80.0; // 80% width minus padding
        let item_space = ICON_ITEM_WIDTH + 10.0;
        let cols = (available_width / item_space).floor() as usize;
        if let Some(state) = self.app_picker_state_mut() {
            state.cols = cols.max(1);
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
        if let Some(status) = render_status(&self.status_message) {
            column = column.push(status);
        }

        let main_content = Container::new(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(iced::Padding {
                top: 80.0,
                bottom: 80.0,
                ..Default::default()
            })
            .style(|_theme| iced::widget::container::Style {
                background: Some(Color::TRANSPARENT.into()),
                text_color: Some(Color::WHITE),
                ..Default::default()
            });

        let mut status_bar_row = iced::widget::Row::new()
            .align_y(iced::Alignment::Center)
            .push(render_gamepad_infos(&self.gamepad_infos))
            .push(iced::widget::Space::new().width(Length::Fill));

        if let Some(battery_info) = self.system_battery {
            if let Some((icon, _color)) = get_battery_visuals(battery_info) {
                status_bar_row = status_bar_row
                    .push(icon)
                    .push(iced::widget::Space::new().width(16)); // Spacing between battery and clock
            }
        }

        let status_bar_row = status_bar_row.push(render_clock(&self.current_time));

        let status_bar = Container::new(status_bar_row)
            .padding([10, 20])
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
                .push(render_controls_hint());
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
        match &self.modal {
            ModalState::ContextMenu { index } => Some(render_context_menu(*index, self.category)),
            ModalState::AppPicker(state) => Some(render_app_picker(state, &self.available_apps)),
            ModalState::SystemUpdate(state) => Some(render_system_update_modal(state)),
            ModalState::SystemInfo(info) => Some(render_system_info_modal(info)),
            ModalState::AppNotFound {
                item_name,
                selected_index,
                ..
            } => Some(render_app_not_found_modal(item_name, *selected_index)),
            ModalState::Help => Some(render_help_modal()),
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
                Some(Message::WindowResized(size.width))
            }
            Event::Window(iced::window::Event::Focused) => Some(Message::WindowFocused(window_id)),
            _ => None,
        });

        let keyboard = iced::event::listen_with(|event, status, _window| {
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
        });

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

        Subscription::batch(subscriptions)
    }

    fn handle_modal_navigation(&mut self, action: Action) -> Option<Task<Message>> {
        match &self.modal {
            ModalState::Help => Some(self.handle_help_modal_navigation(action)),
            ModalState::ContextMenu { .. } => Some(self.handle_context_menu_navigation(action)),
            ModalState::AppPicker(_) => Some(self.handle_app_picker_navigation(action)),
            ModalState::SystemUpdate(_) => Some(self.handle_system_update_navigation(action)),
            ModalState::SystemInfo(_) => Some(self.handle_system_info_navigation(action)),
            ModalState::AppNotFound { .. } => Some(self.handle_app_not_found_navigation(action)),
            ModalState::None => None,
        }
    }

    fn exit_app(&mut self) -> ! {
        self.osk_manager.restore();
        std::process::exit(0);
    }

    fn handle_navigation(&mut self, action: Action) -> Task<Message> {
        if action == Action::Quit {
            self.exit_app();
        }

        if let Some(task) = self.handle_modal_navigation(action) {
            return task;
        }

        match action {
            Action::ShowHelp => {
                self.modal = ModalState::Help;
                return Task::none();
            }
            Action::AddApp => {
                if self.category == Category::Apps {
                    return self.update(Message::OpenAppPicker);
                }
                return Task::none();
            }
            Action::ContextMenu => {
                if !self.current_category_list().is_empty() {
                    self.modal = ModalState::ContextMenu { index: 0 };
                }
                return Task::none();
            }
            Action::Back => {
                self.status_message = None;
                return Task::none();
            }
            _ => {}
        }

        // Handle navigation:
        // Up/Down: Change Category (Row)
        // Left/Right: Change Index in current Row
        match action {
            Action::Up => {
                let prev_cat = self.category.prev();
                if prev_cat != self.category {
                    self.category = prev_cat;
                    // self.clamp_selected_index(); // Already guaranteed by CategoryList state
                    return self.snap_to_main_selection();
                }
            }
            Action::Down => {
                let next_cat = self.category.next();
                if next_cat != self.category {
                    self.category = next_cat;
                    // self.clamp_selected_index();
                    return self.snap_to_main_selection();
                }
            }
            Action::Left => {
                if self.current_category_list_mut().move_left() {
                    return self.snap_to_main_selection();
                }
            }
            Action::Right => {
                if self.current_category_list_mut().move_right() {
                    return self.snap_to_main_selection();
                }
            }
            Action::Select => {
                if !self.current_category_list().is_empty() {
                    return self.activate_selected();
                }
            }
            // Keep tab cycling as fallback
            Action::NextCategory => {
                self.cycle_category();
                // self.clamp_selected_index(); // Handled by CategoryList logic implicitly or we might need a method if we wanted to reset
                return self.snap_to_main_selection();
            }
            Action::PrevCategory => {
                self.cycle_category_back();
                // self.clamp_selected_index();
                return self.snap_to_main_selection();
            }
            _ => {}
        }

        Task::none()
    }

    fn next_grid_index(current: usize, action: Action, cols: usize, len: usize) -> usize {
        match action {
            Action::Up if current >= cols => current - cols,
            Action::Down if current + cols < len => current + cols,
            Action::Left if current > 0 => current - 1,
            Action::Right if current + 1 < len => current + 1,
            _ => current,
        }
    }

    fn snap_to_main_selection(&self) -> Task<Message> {
        let list = self.current_category_list();
        let scroll_id = list.scroll_id.clone();

        let (item_width, _item_height, _image_width, _image_height) =
            get_category_dimensions(self.category);

        let spacing = 10.0;
        let item_width_with_spacing = item_width + spacing;

        let target_x = list.selected_index as f32 * item_width_with_spacing;
        // Center the item roughly or just scroll to it?
        // Let's just scroll to it for now.
        // Iced scrollable offset is absolute.

        // We probably want to center the selected item if possible, but
        // simple "ensure visible" logic is easier.
        // Assuming "ensure visible" is what we want.
        // But `scroll_to` takes an absolute position.

        // Let's try to center it: target_x - (window_width / 2) + (item_width / 2)
        let center_offset = target_x - (self.window_width / 2.0) + (item_width / 2.0);

        operation::scroll_to(
            scroll_id,
            iced::widget::scrollable::AbsoluteOffset {
                x: center_offset.max(0.0),
                y: 0.0,
            },
        )
    }

    fn handle_context_menu_navigation(&mut self, action: Action) -> Task<Message> {
        let max_index = match self.category {
            Category::Apps => 3,
            Category::Games | Category::System => 2,
        };

        let mut index = match &self.modal {
            ModalState::ContextMenu { index } => *index,
            _ => return Task::none(),
        };

        match action {
            Action::Up => {
                index = index.saturating_sub(1);
            }
            Action::Down => {
                if index < max_index {
                    index += 1;
                }
            }
            Action::Select => {
                match (self.category, index) {
                    (Category::Apps, 0) | (Category::Games, 0) | (Category::System, 0) => {
                        // Launch
                        self.modal = ModalState::None;
                        return self.activate_selected();
                    }
                    (Category::Apps, 1) => {
                        // Remove Entry
                        self.modal = ModalState::None;
                        if let Some(removed) = self.apps.remove_selected() {
                            self.save_apps_config("Removed", "removing", &removed.name);
                        }
                    }
                    (_, _) => {
                        let quit_index = if self.category == Category::Apps {
                            2
                        } else {
                            1
                        };
                        if index == quit_index {
                            // Quit Launcher
                            self.exit_app();
                        } else {
                            // Close
                            self.modal = ModalState::None;
                            return Task::none();
                        }
                    }
                }
            }
            Action::Back | Action::ContextMenu => {
                self.modal = ModalState::None;
                return Task::none();
            }
            _ => {}
        }
        self.modal = ModalState::ContextMenu { index };
        Task::none()
    }

    fn handle_help_modal_navigation(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::Back | Action::ShowHelp => {
                self.modal = ModalState::None;
            }
            _ => {} // Ignore other inputs while modal is open
        }
        Task::none()
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
                selected_index = if selected_index == 0 { 1 } else { 0 };
            }
            Action::Select => {
                if selected_index == 0 {
                    self.remove_missing_item(item_id, &item_name, category);
                }
                self.modal = ModalState::None;
                return Task::none();
            }
            Action::Back | Action::ContextMenu | Action::ShowHelp => {
                self.modal = ModalState::None;
                return Task::none();
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
        let Some(state) = self.app_picker_state() else {
            return Task::none();
        };

        let row = state.selected_index / state.cols;
        let item_height_with_spacing = ICON_ITEM_HEIGHT + 10.0;

        let item_top = row as f32 * item_height_with_spacing;
        let item_bottom = item_top + ICON_ITEM_HEIGHT;

        let viewport_top = state.scroll_offset;
        let viewport_height = if state.viewport_height > 0.0 {
            state.viewport_height
        } else {
            // Fallback estimate if viewport not yet reported (e.g. initial render)
            600.0
        };
        let viewport_bottom = viewport_top + viewport_height;

        let target_y = if item_top < viewport_top {
            // Scroll Up
            Some(item_top)
        } else if item_bottom > viewport_bottom {
            // Scroll Down
            Some(item_bottom - viewport_height + 10.0) // +10 for padding
        } else {
            // Already visible
            None
        };

        if let Some(y) = target_y {
            operation::scroll_to(
                state.scrollable_id.clone(),
                iced::widget::scrollable::AbsoluteOffset {
                    x: 0.0,
                    y: y.max(0.0),
                },
            )
        } else {
            Task::none()
        }
    }

    fn handle_app_picker_navigation(&mut self, action: Action) -> Task<Message> {
        let list_len = self.available_apps.len();
        if list_len == 0 {
            // No apps available, just handle close
            match action {
                Action::Back | Action::AddApp => {
                    return self.update(Message::CloseAppPicker);
                }
                _ => {}
            }
            return Task::none();
        }

        let (mut selected_index, cols) = match self.app_picker_state() {
            Some(state) => (state.selected_index, state.cols),
            None => return Task::none(),
        };

        match action {
            Action::Select => {
                return self.update(Message::AddSelectedApp);
            }
            Action::Back | Action::AddApp => {
                return self.update(Message::CloseAppPicker);
            }
            _ => {
                selected_index = Self::next_grid_index(selected_index, action, cols, list_len);
            }
        }

        if let Some(state) = self.app_picker_state_mut() {
            state.selected_index = selected_index;
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
        );

        let system_row = render_section_row(
            self.category,
            Category::System,
            &self.system_items,
            "No system actions available.".to_string(),
            self.default_icon_handle.clone(),
        );

        Column::new()
            .push(games_row)
            .push(apps_row)
            .push(system_row)
            .spacing(30)
            .into()
    }

    fn save_apps_config(&self, _success_action: &str, failure_action: &str, app_name: &str) {
        let apps_to_save: Vec<AppEntry> = self
            .apps
            .items
            .iter()
            .filter_map(|item| match &item.action {
                LauncherAction::Launch { exec } => Some(AppEntry {
                    id: item.id,
                    name: item.name.clone(),
                    exec: exec.clone(),
                    icon: item.icon.clone(),
                    launch_key: item.launch_key.clone(),
                    game_executable: item.game_executable.clone(),
                    last_started: item.last_started,
                    steam_appid: item.steam_appid.clone(),
                }),

                _ => None,
            })
            .collect();

        let config = AppConfig {
            apps: apps_to_save,
            steamgriddb_api_key: self.api_key.clone(),
            game_launch_history: self.game_launch_history.clone(),
        };

        if let Err(err) = save_config(&config) {
            error!(
                "Failed to save config after {} app {}: {}",
                failure_action, app_name, err
            );
        }
    }

    fn apps_empty_message(&self) -> String {
        "No apps configured. Press Y or + to add an app.".to_string()
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
