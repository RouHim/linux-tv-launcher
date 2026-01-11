use iced::keyboard::{self, key::Named, Key};
use iced::widget::operation;

use crate::ui_modals::{render_context_menu, render_help_modal};
use crate::ui_system_update_modal::render_system_update_modal;
use crate::ui_theme::*;
use iced::window;
use iced::{
    widget::{Column, Container, Stack},
    Color, Element, Event, Length, Subscription, Task,
};
use tracing::{info, warn};

use chrono::{DateTime, Local};
use rayon::prelude::*;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

use crate::assets::get_default_icon;
use crate::category_list::CategoryList;
use crate::desktop_apps::{scan_desktop_apps, DesktopApp};
use crate::focus_manager::{monitor_app_process, MonitorTarget};
use crate::game_image_fetcher::GameImageFetcher;
use crate::game_sources::scan_games;
use crate::gamepad::gamepad_subscription;
use crate::image_cache::ImageCache;
use crate::input::Action;
use crate::launcher::{launch_app, resolve_monitor_target};
use crate::messages::Message;
use crate::model::{AppEntry, Category, LauncherAction, LauncherItem};
use crate::osk::OskManager;
use crate::searxng::SearxngClient;
use crate::steamgriddb::SteamGridDbClient;
use crate::storage::{config_path, load_config, save_config, AppConfig};
use crate::sys_utils::restart_process;
use crate::system_info::{fetch_system_info, GamingSystemInfo};
use crate::system_update::system_update_stream;
use crate::system_update_state::{SystemUpdateProgress, SystemUpdateState, UpdateStatus};
use crate::ui_app_picker::{render_app_picker, AppPickerState};
use crate::ui_components::render_clock;
use crate::ui_main_view::{
    get_category_dimensions, render_controls_hint, render_section_row, render_status,
};
use crate::ui_system_info_modal::render_system_info_modal;

enum ModalState {
    None,
    ContextMenu { index: usize },
    AppPicker(AppPickerState),
    SystemUpdate(SystemUpdateState),
    SystemInfo(Option<GamingSystemInfo>),
    Help,
}

pub struct Launcher {
    apps: CategoryList,
    games: CategoryList,
    system_items: CategoryList,

    category: Category,
    default_icon_handle: Option<iced::widget::svg::Handle>,
    status_message: Option<String>,

    config_path: Option<String>,
    apps_loaded: bool,
    games_loaded: bool,
    // API Key should be loaded from config ideally, but hardcoding for this user session as requested
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
}

impl Launcher {
    pub fn new() -> (Self, Task<Message>) {
        let _init_span = tracing::info_span!("startup_launcher_init").entered();

        let default_icon = get_default_icon().map(iced::widget::svg::Handle::from_memory);
        let config_path = config_path().ok().map(|path| path.display().to_string());

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

        let launcher = Self {
            apps: CategoryList::new(Vec::new()),
            games: CategoryList::new(Vec::new()),
            system_items: CategoryList::new(vec![
                LauncherItem::shutdown(),
                LauncherItem::suspend(),
                LauncherItem::system_update(),
                LauncherItem::system_info(),
                LauncherItem::exit(),
            ]),
            category: Category::Apps,
            default_icon_handle: default_icon,
            status_message: None,

            config_path,
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
        };

        // Chain startup: Load config first to potentially get API key, then scan games
        let tasks = Task::perform(
            async { load_config().map_err(|err| err.to_string()) },
            Message::AppsLoaded,
        );

        drop(_init_span);

        (launcher, tasks)
    }

    pub fn title(&self) -> String {
        String::from("Linux TV Launcher")
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
            Message::AppsLoaded(result) => {
                self.apps_loaded = true;
                match result {
                    Ok(config) => {
                        let items: Vec<LauncherItem> = config
                            .apps
                            .into_iter()
                            .map(LauncherItem::from_app_entry)
                            .collect();
                        self.apps.set_items(items);
                        self.apps.sort_inplace();
                        self.status_message = None;

                        // If no env key was found, try using the one from config
                        if self.api_key.is_none() {
                            if let Some(key) = config.steamgriddb_api_key {
                                info!("Using SteamGridDB API key from config");
                                self.api_key = Some(key.clone());
                                self.sgdb_client = SteamGridDbClient::new(key);
                            }
                        }
                    }
                    Err(err) => {
                        warn!("Failed to load app config: {}", err);
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
            Message::GamesLoaded(games) => {
                let items: Vec<LauncherItem> = games
                    .into_iter()
                    .map(LauncherItem::from_app_entry)
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
                            let pipeline = pipeline_template.clone();

                            Task::perform(
                                async move {
                                    tokio::task::spawn_blocking(move || {
                                        pipeline.fetch(
                                            game_id,
                                            &game_name,
                                            source_image_url.as_deref(),
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
            Message::ImageFetched(id, path) => {
                self.games.update_item_by_id(id, |item| {
                    item.icon = Some(path.to_string_lossy().to_string());
                });
                Task::none()
            }
            Message::Input(action) => self.handle_navigation(action),
            Message::ScaleFactorChanged(scale) => {
                self.scale_factor = scale;
                Task::none()
            }
            Message::Tick(time) => {
                self.current_time = time;
                Task::none()
            }
            Message::WindowResized(width, _height) => {
                self.window_width = width;
                Task::none()
            }

            Message::OpenAppPicker => {
                self.modal = ModalState::AppPicker(AppPickerState::new());
                self.available_apps.clear();
                // Scan for desktop apps asynchronously
                Task::perform(async { scan_desktop_apps() }, Message::AvailableAppsLoaded)
            }
            Message::AvailableAppsLoaded(apps) => {
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
            Message::AddSelectedApp => {
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
                    );

                    let new_item = LauncherItem::from_app_entry(new_entry);

                    self.apps.add_item(new_item);

                    self.save_apps_config("Added", "adding", &selected_app.name);

                    // Remove from available apps and close picker
                    self.available_apps.remove(selected_index);
                    self.modal = ModalState::None;
                }
                Task::none()
            }
            Message::CloseAppPicker => {
                self.modal = ModalState::None;
                Task::none()
            }
            Message::AppPickerScrolled(viewport) => {
                if let Some(state) = self.app_picker_state_mut() {
                    state.scroll_offset = viewport.absolute_offset().y;
                    state.viewport_height = viewport.bounds().height;
                }
                Task::none()
            }
            Message::StartSystemUpdate => {
                self.osk_manager.show();
                self.modal = ModalState::SystemUpdate(SystemUpdateState::new());
                Task::none()
            }
            Message::SystemUpdateProgress(progress) => {
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
            Message::CloseSystemUpdateModal => {
                self.modal = ModalState::None;
                Task::none()
            }
            Message::CancelSystemUpdate => {
                if let ModalState::SystemUpdate(state) = &mut self.modal {
                    // Only allow cancelling if not installing
                    if !matches!(state.status, UpdateStatus::Installing { .. }) {
                        state.status = UpdateStatus::Failed("Update cancelled by user".to_string());
                    }
                }
                Task::none()
            }
            Message::RequestReboot => {
                if let Err(e) = std::process::Command::new("systemctl")
                    .arg("reboot")
                    .spawn()
                {
                    self.status_message = Some(format!("Failed to reboot: {}", e));
                }
                Task::none()
            }
            Message::OpenSystemInfo => {
                self.modal = ModalState::SystemInfo(None);
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
            Message::SystemInfoLoaded(info_box) => {
                if let ModalState::SystemInfo(state) = &mut self.modal {
                    *state = Some(*info_box);
                }
                Task::none()
            }
            Message::CloseSystemInfoModal => {
                self.modal = ModalState::None;
                Task::none()
            }
            Message::GameExited => {
                self.game_running = false;
                info!("Game/App process exited. Recreating window to regain focus.");
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
            Message::WindowOpened(id) => {
                info!("Main window opened with ID: {:?}", id);
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
            Message::WindowFocused(id) => {
                if self.window_id.is_none() {
                    info!("Captured window ID from Focus event: {:?}", id);
                    self.window_id = Some(id);
                }
                Task::none()
            }
            Message::AppUpdateResult(result) => match result {
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
                        info!("App is up to date");
                        Task::none()
                    }
                }
                Err(e) => {
                    // Log error but don't show user facing message for background check failure
                    warn!("Auto-update check failed: {}", e);
                    Task::none()
                }
            },
            Message::RestartApp => {
                if let Some(exe) = &self.current_exe {
                    restart_process(exe.clone());
                }
                Task::none()
            }
            Message::None => Task::none(),
        }
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

        let mut column = Column::new().push(content).spacing(20);
        if let Some(status) = render_status(&self.status_message) {
            column = column.push(status);
        }

        // Add controls hint when no modal is open
        if matches!(&self.modal, ModalState::None) {
            column = column.push(render_controls_hint());
        }

        let main_content = Container::new(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| iced::widget::container::Style {
                background: Some(COLOR_BACKGROUND.into()),
                text_color: Some(Color::WHITE),
                ..Default::default()
            });

        let clock = render_clock(&self.current_time);

        let stack = Stack::new().push(main_content).push(clock).into();

        self.render_with_modal(stack)
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
            ModalState::Help => Some(render_help_modal()),
            ModalState::None => None,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // Disable all input subscriptions while a game is running
        if self.game_running {
            return Subscription::none();
        }

        let gamepad = gamepad_subscription().map(Message::Input);

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
                self.launch_app(exec, &item.name, item.game_executable.as_ref())
            }
            LauncherAction::SystemUpdate => self.update(Message::StartSystemUpdate),
            LauncherAction::SystemInfo => self.update(Message::OpenSystemInfo),
            LauncherAction::Shutdown => self.system_command("systemctl", &["poweroff"], "shutdown"),
            LauncherAction::Suspend => self.system_command("systemctl", &["suspend"], "suspend"),
            LauncherAction::Exit => self.exit_app(),
        }
    }

    /// Launch an application with proper process monitoring
    fn launch_app(
        &mut self,
        exec: &str,
        item_name: &str,
        game_executable: Option<&String>,
    ) -> Task<Message> {
        let monitor_target = resolve_monitor_target(exec, item_name, game_executable);

        match launch_app(exec) {
            Ok(pid) => {
                self.game_running = true;
                let target = monitor_target.unwrap_or(MonitorTarget::Pid(pid));
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
            .push(apps_row)
            .push(games_row)
            .push(system_row)
            .spacing(30)
            .into()
    }

    fn save_apps_config(&self, success_action: &str, failure_action: &str, app_name: &str) {
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
                    game_executable: item.game_executable.clone(),
                }),
                _ => None,
            })
            .collect();

        let config = AppConfig {
            apps: apps_to_save,
            steamgriddb_api_key: self.api_key.clone(),
        };

        match save_config(&config) {
            Ok(_) => info!("{} app: {}", success_action, app_name),
            Err(err) => warn!(
                "Failed to save config after {} app {}: {}",
                failure_action, app_name, err
            ),
        }
    }

    fn apps_empty_message(&self) -> String {
        if let Some(path) = &self.config_path {
            format!("No apps configured. Edit {}.", path)
        } else {
            "No apps configured.".to_string()
        }
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

        // Switch to Games (Down)
        let _ = launcher.handle_navigation(Action::Down);
        assert_eq!(launcher.category, Category::Games);
        assert_eq!(launcher.games.selected_index, 0); // Default

        // Move Right -> Games index 1
        let _ = launcher.handle_navigation(Action::Right);
        assert_eq!(launcher.games.selected_index, 1);

        // Switch back to Apps (Up)
        let _ = launcher.handle_navigation(Action::Up);
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
