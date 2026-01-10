use iced::alignment::Horizontal;
use iced::keyboard::{self, key::Named, Key};
use iced::widget::operation;
use iced::window;
use iced::{
    widget::{Column, Container, Grid, Row, Scrollable, Stack, Text},
    Color, Element, Event, Length, Subscription, Task,
};
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

use crate::assets::get_default_icon;
use crate::desktop_apps::{scan_desktop_apps, DesktopApp};
use crate::focus_manager::{monitor_app_process, MonitorTarget};
use crate::game_image_fetcher::GameImageFetcher;
use crate::game_sources::scan_games;
use crate::gamepad::gamepad_subscription;
use crate::icons;
use crate::image_cache::ImageCache;
use crate::input::Action;
use crate::launcher::{launch_app, resolve_monitor_target};
use crate::model::{AppEntry, Category, LauncherAction, LauncherItem, SystemIcon};
use crate::searxng::SearxngClient;
use crate::steamgriddb::SteamGridDbClient;
use crate::storage::{config_path, load_config, save_config};
use crate::system_update::system_update_stream;
use crate::system_update_state::{SystemUpdateProgress, SystemUpdateState, UpdateStatus};
use crate::ui_app_picker::{render_app_picker, AppPickerState};
use crate::ui_components::{render_icon, IconPath};
use crate::ui_modals::{render_context_menu, render_help_modal};
use crate::ui_system_update_modal::render_system_update_modal;
use crate::ui_theme::*;
use tracing::{info, warn};

enum ModalState {
    None,
    ContextMenu { index: usize },
    AppPicker(AppPickerState),
    SystemUpdate(SystemUpdateState),
    Help,
}

pub struct Launcher {
    apps: Vec<LauncherItem>,
    games: Vec<LauncherItem>,
    system_items: Vec<LauncherItem>,
    selected_index: usize,
    category: Category,
    cols: usize,
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
}

#[derive(Debug, Clone)]
pub enum Message {
    AppsLoaded(Result<Vec<AppEntry>, String>),
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
    None,
}

impl Launcher {
    pub fn new() -> (Self, Task<Message>) {
        let default_icon = get_default_icon().map(iced::widget::svg::Handle::from_memory);
        let config_path = config_path().ok().map(|path| path.display().to_string());

        let sgdb_client = SteamGridDbClient::new("276bca336e815a4e2dd2250ea674eb31".to_string());
        let searxng_client = SearxngClient::new();
        let image_cache = ImageCache::new().ok();

        let mut launcher = Self {
            apps: Vec::new(),
            games: Vec::new(),
            system_items: vec![
                LauncherItem::shutdown(),
                LauncherItem::suspend(),
                LauncherItem::system_update(),
                LauncherItem::exit(),
            ],
            selected_index: 0,
            category: Category::Apps,
            cols: 6,
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
        };
        launcher.update_columns();

        (
            launcher,
            Task::batch(vec![
                Task::perform(
                    async { load_config().map_err(|err| err.to_string()) },
                    Message::AppsLoaded,
                ),
                Task::perform(async { scan_games() }, Message::GamesLoaded),
            ]),
        )
    }

    pub fn title(&self) -> String {
        String::from("Linux TV Launcher")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppsLoaded(result) => {
                self.apps_loaded = true;
                match result {
                    Ok(apps) => {
                        self.apps = apps.into_iter().map(LauncherItem::from_app_entry).collect();
                        self.status_message = None;
                        Self::sort_items(&mut self.apps);
                        self.clamp_selected_index();
                    }
                    Err(err) => {
                        warn!("Failed to load app config: {}", err);
                        self.apps.clear();
                        self.status_message = Some(err);
                        self.clamp_selected_index();
                    }
                }
                Task::none()
            }
            Message::GamesLoaded(games) => {
                self.games = games
                    .into_iter()
                    .map(LauncherItem::from_app_entry)
                    .collect();
                self.games_loaded = true;
                self.status_message = None;
                Self::sort_items(&mut self.games);
                self.clamp_selected_index();

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
                if let Some(item) = self.games.iter_mut().find(|g| g.id == id) {
                    item.icon = Some(path.to_string_lossy().to_string());
                }
                Task::none()
            }
            Message::Input(action) => self.handle_navigation(action),
            Message::ScaleFactorChanged(scale) => {
                self.scale_factor = scale;
                Task::none()
            }
            Message::WindowResized(width, _height) => {
                self.window_width = width;
                self.update_columns();
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
                    self.apps.push(new_item);
                    Self::sort_items(&mut self.apps);
                    self.clamp_selected_index();

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
                Task::none()
            }
            Message::WindowFocused(id) => {
                if self.window_id.is_none() {
                    info!("Captured window ID from Focus event: {:?}", id);
                    self.window_id = Some(id);
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
        let header = self.render_header();
        let content = self.render_category();

        let mut column = Column::new().push(header).push(content).spacing(20);
        if let Some(status) = self.render_status() {
            column = column.push(status);
        }

        // Add controls hint when no modal is open
        if matches!(&self.modal, ModalState::None) {
            column = column.push(self.render_controls_hint());
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
            })
            .into();

        self.render_with_modal(main_content)
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
            ModalState::None => None,
        }
    }

    fn handle_navigation(&mut self, action: Action) -> Task<Message> {
        if action == Action::Quit {
            std::process::exit(0);
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
                if !self.active_items().is_empty() {
                    self.modal = ModalState::ContextMenu { index: 0 };
                }
                return Task::none();
            }
            Action::NextCategory => {
                self.cycle_category();
                return Task::none();
            }
            Action::PrevCategory => {
                self.cycle_category_back();
                return Task::none();
            }
            Action::Back => {
                self.status_message = None;
                return Task::none();
            }
            _ => {}
        }

        let list_len = self.active_items().len();
        if list_len == 0 {
            return Task::none();
        }

        match action {
            Action::Select => {
                return self.activate_selected();
            }
            _ => {
                self.selected_index =
                    Self::next_grid_index(self.selected_index, action, self.cols, list_len);
            }
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
                        if self.selected_index < self.apps.len() {
                            let removed = self.apps.remove(self.selected_index);
                            self.clamp_selected_index();

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
                            std::process::exit(0);
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
        let selection = self.active_items().get(self.selected_index).map(|item| {
            (
                item.action.clone(),
                item.name.clone(),
                item.game_executable.clone(),
            )
        });

        let Some((action, item_name, game_executable)) = selection else {
            return Task::none();
        };

        self.status_message = None;

        match action {
            LauncherAction::Launch { exec } => {
                let monitor_target =
                    resolve_monitor_target(&exec, &item_name, game_executable.as_ref());

                match launch_app(&exec) {
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
            LauncherAction::SystemUpdate => {
                // Trigger the modal start
                self.update(Message::StartSystemUpdate)
            }
            LauncherAction::Shutdown => {
                if let Err(e) = std::process::Command::new("systemctl")
                    .arg("poweroff")
                    .spawn()
                {
                    self.status_message = Some(format!("Failed to shutdown: {}", e));
                }
                Task::none()
            }
            LauncherAction::Suspend => {
                if let Err(e) = std::process::Command::new("systemctl")
                    .arg("suspend")
                    .spawn()
                {
                    self.status_message = Some(format!("Failed to suspend: {}", e));
                }
                Task::none()
            }
            LauncherAction::Exit => {
                std::process::exit(0);
            }
        }
    }

    fn cycle_category(&mut self) {
        self.category = self.category.next();
        self.selected_index = 0;
        self.status_message = None;
        self.update_columns();
    }

    fn cycle_category_back(&mut self) {
        self.category = self.category.prev();
        self.selected_index = 0;
        self.status_message = None;
        self.update_columns();
    }

    fn update_columns(&mut self) {
        let (item_width, _item_height, _image_width, _image_height) = match self.category {
            Category::Games => (
                GAME_POSTER_WIDTH + 16.0,   // Extra width for padding/border
                GAME_POSTER_HEIGHT + 140.0, // Increased for text wrapping
                GAME_POSTER_WIDTH,
                GAME_POSTER_HEIGHT,
            ),
            _ => (ICON_ITEM_WIDTH, ICON_ITEM_HEIGHT, ICON_SIZE, ICON_SIZE),
        };

        // Estimate available width: Window Width - (Spacing * 2 for outer margins + spacing)
        // Grid spacing is 10.
        // Assuming typical margin/padding around the grid.
        let available_width = self.window_width - 40.0;
        let item_space = item_width + 10.0; // Item width + grid spacing

        let cols = (available_width / item_space).floor() as usize;
        self.cols = cols.max(1);
    }

    fn clamp_selected_index(&mut self) {
        let list_len = self.active_items().len();
        if list_len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= list_len {
            self.selected_index = list_len.saturating_sub(1);
        }
    }

    fn active_items(&self) -> &[LauncherItem] {
        match self.category {
            Category::Apps => &self.apps,
            Category::Games => &self.games,
            Category::System => &self.system_items,
        }
    }

    fn render_header(&self) -> Element<'_, Message> {
        let mut tabs = Row::new().spacing(12);
        for category in Category::ALL {
            let is_selected = category == self.category;
            let label =
                Text::new(category.title())
                    .font(SANSATION)
                    .size(22)
                    .color(if is_selected {
                        Color::WHITE
                    } else {
                        COLOR_TEXT_MUTED
                    });

            let tab = Container::new(label).padding(8).style(move |_theme| {
                if is_selected {
                    iced::widget::container::Style {
                        background: Some(COLOR_ACCENT.into()),
                        text_color: Some(Color::WHITE),
                        ..Default::default()
                    }
                } else {
                    iced::widget::container::Style {
                        background: Some(COLOR_PANEL.into()),
                        text_color: Some(Color::WHITE),
                        ..Default::default()
                    }
                }
            });

            tabs = tabs.push(tab);
        }

        Container::new(tabs)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
    }

    fn render_category(&self) -> Element<'_, Message> {
        match self.category {
            Category::Apps => self.render_list(
                &self.apps,
                self.apps_loaded,
                self.apps_empty_message(),
                "Loading apps...",
            ),
            Category::Games => self.render_list(
                &self.games,
                self.games_loaded,
                "No games found.".to_string(),
                "Scanning games...",
            ),
            Category::System => {
                if self.system_items.is_empty() {
                    Column::new()
                        .push(
                            Text::new("No system actions available.")
                                .font(SANSATION)
                                .color(Color::WHITE),
                        )
                        .align_x(iced::Alignment::Center)
                        .into()
                } else {
                    self.render_grid(&self.system_items)
                }
            }
        }
    }

    fn render_list(
        &self,
        items: &[LauncherItem],
        loaded: bool,
        empty_message: String,
        loading_message: &str,
    ) -> Element<'_, Message> {
        if items.is_empty() {
            let message = if loaded {
                empty_message
            } else {
                loading_message.to_string()
            };
            return Column::new()
                .push(Text::new(message).font(SANSATION).color(Color::WHITE))
                .align_x(iced::Alignment::Center)
                .into();
        }

        self.render_grid(items)
    }

    fn render_grid(&self, items: &[LauncherItem]) -> Element<'_, Message> {
        // Determine dimensions based on category.
        // For Games: tight fit around poster.
        // For Apps: tight fit around icon.
        let (item_width, item_height, image_width, image_height) = match self.category {
            Category::Games => (
                GAME_POSTER_WIDTH + 16.0,
                GAME_POSTER_HEIGHT + 140.0,
                GAME_POSTER_WIDTH,
                GAME_POSTER_HEIGHT,
            ),
            _ => (ICON_ITEM_WIDTH, ICON_ITEM_HEIGHT, ICON_SIZE, ICON_SIZE),
        };

        let mut grid = Grid::new()
            .columns(self.cols)
            .spacing(10)
            .height(Length::Shrink);

        for (i, item) in items.iter().enumerate() {
            let is_selected = i == self.selected_index;
            grid = grid.push(self.render_item(
                item,
                is_selected,
                image_width,
                image_height,
                item_width,
                item_height,
            ));
        }

        Scrollable::new(grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn render_item(
        &self,
        item: &LauncherItem,
        is_selected: bool,
        image_width: f32,
        image_height: f32,
        item_width: f32,
        _item_height: f32,
    ) -> Element<'_, Message> {
        let icon_widget: Element<'_, Message> = if let Some(sys_icon) = &item.system_icon {
            match sys_icon {
                SystemIcon::PowerOff => icons::power_off_icon(image_width),
                SystemIcon::Pause => icons::pause_icon(image_width),
                SystemIcon::ArrowsRotate => icons::arrows_rotate_icon(image_width),
                SystemIcon::ExitBracket => icons::exit_icon(image_width),
            }
        } else {
            render_icon(
                item.icon.as_deref().map(IconPath::Str),
                image_width,
                image_height,
                "ICON",
                None,
                self.default_icon_handle.clone(),
            )
        };

        let icon_container = Container::new(icon_widget).padding(6);

        let text = Text::new(item.name.clone());

        let label = text
            .font(SANSATION)
            .width(Length::Fixed(item_width)) // Use full item width for text centering
            .align_x(Horizontal::Center)
            .color(Color::WHITE)
            .size(14);

        let content = Column::new()
            .push(icon_container)
            .push(label)
            .align_x(iced::Alignment::Center)
            .spacing(5);

        Container::new(content)
            .width(Length::Fixed(item_width))
            .height(Length::Shrink)
            .padding(6)
            .align_x(Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(move |_theme| {
                if is_selected {
                    iced::widget::container::Style {
                        border: iced::Border {
                            color: COLOR_ACCENT,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                } else {
                    iced::widget::container::Style::default()
                }
            })
            .into()
    }

    fn render_status(&self) -> Option<Element<'_, Message>> {
        let status = self.status_message.as_ref()?;
        Some(
            Container::new(Text::new(status).font(SANSATION).color(COLOR_STATUS_TEXT))
                .padding(8)
                .style(|_theme| iced::widget::container::Style {
                    background: Some(COLOR_STATUS_BACKGROUND.into()),
                    text_color: Some(Color::WHITE),
                    ..Default::default()
                })
                .into(),
        )
    }

    fn render_controls_hint(&self) -> Element<'_, Message> {
        let hint = Text::new("Press  −  for controls")
            .font(SANSATION)
            .size(14)
            .color(COLOR_TEXT_DIM);

        Container::new(hint)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(10)
            .into()
    }

    fn save_apps_config(&self, success_action: &str, failure_action: &str, app_name: &str) {
        let apps_to_save = Self::app_entries_from_items(&self.apps);
        match save_config(&apps_to_save) {
            Ok(_) => info!("{} app: {}", success_action, app_name),
            Err(err) => warn!(
                "Failed to save config after {} app {}: {}",
                failure_action, app_name, err
            ),
        }
    }

    fn app_entries_from_items(items: &[LauncherItem]) -> Vec<AppEntry> {
        items
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
            .collect()
    }

    fn sort_items(items: &mut [LauncherItem]) {
        items.sort_by(|a, b| a.name.cmp(&b.name));
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
    fn test_category_cycles() {
        let (mut launcher, _) = Launcher::new();

        assert_eq!(launcher.category, Category::Apps);
        let _ = launcher.handle_navigation(Action::NextCategory);
        assert_eq!(launcher.category, Category::Games);
        let _ = launcher.handle_navigation(Action::NextCategory);
        assert_eq!(launcher.category, Category::System);
        let _ = launcher.handle_navigation(Action::NextCategory);
        assert_eq!(launcher.category, Category::Apps);
    }

    #[test]
    fn test_navigate_grid_moves_within_bounds() {
        let index = 5;
        let new_index = Launcher::next_grid_index(index, Action::Up, 3, 10);

        assert_eq!(new_index, 2);
    }

    #[test]
    fn test_navigate_grid_blocks_out_of_bounds() {
        let index = 1;
        let new_index = Launcher::next_grid_index(index, Action::Up, 3, 10);

        assert_eq!(new_index, 1);
    }
}
