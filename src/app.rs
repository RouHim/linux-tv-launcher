use iced::alignment::Horizontal;
use iced::keyboard::{self, key::Named, Key};
use iced::widget::scrollable::{AbsoluteOffset, Viewport};
use iced::widget::{operation, Id};
use iced::window;
use iced::{
    widget::{Column, Container, Grid, Image, Row, Scrollable, Stack, Svg, Text},
    Color, ContentFit, Element, Event, Length, Subscription, Task,
};
use rayon::prelude::*;
use std::path::PathBuf;
use urlencoding::decode;
use uuid::Uuid;

use crate::assets::get_default_icon;
use crate::desktop_apps::{scan_desktop_apps, DesktopApp};
use crate::focus_manager::{monitor_app_process, MonitorTarget};
use crate::game_sources::scan_games;
use crate::gamepad::gamepad_subscription;
use crate::icons;
use crate::image_cache::ImageCache;
use crate::input::Action;
use crate::launcher::launch_app;
use crate::model::{AppEntry, Category, LauncherAction, LauncherItem, SystemIcon};
use crate::searxng::SearxngClient;
use crate::steamgriddb::SteamGridDbClient;
use crate::storage::{config_path, load_config, save_config};
use crate::system_update::run_update;
use tracing::{error, info, warn};

struct AppPickerState {
    selected_index: usize,
    cols: usize,
    scrollable_id: Id,
    scroll_offset: f32,
    viewport_height: f32,
}

impl AppPickerState {
    fn new() -> Self {
        Self {
            selected_index: 0,
            cols: 6,
            scrollable_id: Id::unique(),
            scroll_offset: 0.0,
            viewport_height: 0.0,
        }
    }
}

enum ModalState {
    None,
    ContextMenu { index: usize },
    AppPicker(AppPickerState),
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

const GAME_POSTER_WIDTH: f32 = 200.0;
const GAME_POSTER_HEIGHT: f32 = 300.0;

// App/System icon dimensions
const ICON_SIZE: f32 = 128.0;

// Custom font
const SANSATION: iced::Font = iced::Font::with_name("Sansation");
const ICON_ITEM_WIDTH: f32 = 150.0; // Increased to allow padding
const ICON_ITEM_HEIGHT: f32 = 280.0; // Increased to allow text wrapping
const COLOR_BACKGROUND: Color = Color::from_rgb(0.05, 0.05, 0.05);
const COLOR_PANEL: Color = Color::from_rgb(0.1, 0.1, 0.1);
const COLOR_MENU_BACKGROUND: Color = Color::from_rgb(0.15, 0.15, 0.15);
const COLOR_STATUS_BACKGROUND: Color = Color::from_rgb(0.12, 0.12, 0.12);
const COLOR_TEXT_MUTED: Color = Color::from_rgb(0.7, 0.7, 0.7);
const COLOR_TEXT_HINT: Color = Color::from_rgb(0.6, 0.6, 0.6);
const COLOR_TEXT_DIM: Color = Color::from_rgb(0.5, 0.5, 0.5);
const COLOR_TEXT_SOFT: Color = Color::from_rgb(0.8, 0.8, 0.8);
const COLOR_TEXT_BRIGHT: Color = Color::from_rgb(0.9, 0.9, 0.9);
const COLOR_ACCENT: Color = Color::from_rgb(0.2, 0.4, 0.8);
const COLOR_ACCENT_OVERLAY: Color = Color::from_rgba(0.2, 0.4, 0.8, 0.3);
const COLOR_OVERLAY: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.7);
const COLOR_OVERLAY_STRONG: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.8);
const COLOR_STATUS_TEXT: Color = Color::from_rgb(0.9, 0.8, 0.4);

enum IconPath<'a> {
    Str(&'a str),
    Path(&'a std::path::Path),
}

impl IconPath<'_> {
    fn is_svg(&self) -> bool {
        match self {
            IconPath::Str(path) => path.ends_with(".svg"),
            IconPath::Path(path) => path.extension() == Some(std::ffi::OsStr::new("svg")),
        }
    }
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
    AppPickerScrolled(Viewport),
    GameExited,
    WindowOpened(window::Id),
    WindowFocused(window::Id),
    None,
}

impl Launcher {
    // ... existing new, title, update methods ...
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
                    let sgdb_client_template = self.sgdb_client.clone();
                    let searxng_client_template = self.searxng_client.clone();
                    let cache_dir_template = cache.cache_dir.clone();

                    tasks = self
                        .games
                        .par_iter()
                        .map(|game| {
                            let game_id = game.id;
                            let game_name = game.name.clone();
                            let source_image_url = game.source_image_url.clone();
                            let sgdb_client = sgdb_client_template.clone();
                            let searxng_client = searxng_client_template.clone();
                            let cache_dir = cache_dir_template.clone();

                            Task::perform(
                                async move {
                                    tokio::task::spawn_blocking(move || {
                                        fetch_game_image(
                                            sgdb_client,
                                            searxng_client,
                                            cache_dir,
                                            game_id,
                                            game_name,
                                            source_image_url,
                                            target_width,
                                            target_height,
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

                    // Save config
                    let apps_to_save: Vec<AppEntry> = self
                        .apps
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

                    match save_config(&apps_to_save) {
                        Ok(_) => info!("Added app: {}", selected_app.name),
                        Err(err) => warn!("Failed to save config after adding app: {}", err),
                    }

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

        match &self.modal {
            ModalState::ContextMenu { index } => Stack::new()
                .push(main_content)
                .push(self.render_context_menu(*index))
                .into(),
            ModalState::AppPicker(state) => Stack::new()
                .push(main_content)
                .push(self.render_app_picker(state))
                .into(),
            ModalState::Help => Stack::new()
                .push(main_content)
                .push(self.render_help_modal())
                .into(),
            ModalState::None => main_content,
        }
    }

    fn render_context_menu(&self, selected_index: usize) -> Element<'_, Message> {
        let menu_items: Vec<&str> = match self.category {
            Category::Apps => vec!["Launch", "Remove Entry", "Quit Launcher", "Close"],
            Category::Games | Category::System => vec!["Launch", "Quit Launcher", "Close"],
        };
        let mut column = Column::new().spacing(10).padding(20);

        for (i, item) in menu_items.iter().enumerate() {
            let is_selected = i == selected_index;
            let text = Text::new(*item)
                .font(SANSATION)
                .size(20)
                .color(if is_selected {
                    Color::WHITE
                } else {
                    COLOR_TEXT_MUTED
                })
                .align_x(Horizontal::Center);

            let container = Container::new(text)
                .padding(10)
                .width(Length::Fill)
                .style(move |_| {
                    if is_selected {
                        iced::widget::container::Style {
                            background: Some(COLOR_ACCENT.into()),
                            text_color: Some(Color::WHITE),
                            ..Default::default()
                        }
                    } else {
                        iced::widget::container::Style {
                            text_color: Some(COLOR_TEXT_MUTED),
                            ..Default::default()
                        }
                    }
                });

            column = column.push(container);
        }

        let menu_box = Container::new(column)
            .width(Length::Fixed(300.0))
            .style(|_| iced::widget::container::Style {
                background: Some(COLOR_MENU_BACKGROUND.into()),
                border: iced::Border {
                    color: Color::WHITE,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            });

        Container::new(menu_box)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(COLOR_OVERLAY.into()),
                ..Default::default()
            })
            .into()
    }

    fn render_app_picker(&self, state: &AppPickerState) -> Element<'_, Message> {
        let title = Text::new("Add Application")
            .font(SANSATION)
            .size(28)
            .color(Color::WHITE);

        let title_container = Container::new(title)
            .padding(20)
            .width(Length::Fill)
            .center_x(Length::Fill);

        let content: Element<'_, Message> = if self.available_apps.is_empty() {
            Container::new(
                Text::new("No applications found")
                    .font(SANSATION)
                    .size(18)
                    .color(COLOR_TEXT_MUTED),
            )
            .padding(40)
            .center_x(Length::Fill)
            .into()
        } else {
            let mut grid = Grid::new()
                .columns(state.cols)
                .spacing(10)
                .height(Length::Shrink);

            for (i, app) in self.available_apps.iter().enumerate() {
                let is_selected = i == state.selected_index;
                grid = grid.push(self.render_picker_item(app, is_selected));
            }

            Scrollable::new(grid)
                .width(Length::Fill)
                .height(Length::Fill)
                .id(state.scrollable_id.clone())
                .on_scroll(Message::AppPickerScrolled)
                .into()
        };

        let hint = Text::new("Enter: Add | Escape: Close")
            .font(SANSATION)
            .size(14)
            .color(COLOR_TEXT_HINT);

        let hint_container = Container::new(hint)
            .padding(10)
            .width(Length::Fill)
            .center_x(Length::Fill);

        let picker_column = Column::new()
            .push(title_container)
            .push(content)
            .push(hint_container)
            .spacing(10);

        let picker_box = Container::new(picker_column)
            .width(Length::FillPortion(80))
            .height(Length::FillPortion(80))
            .padding(20)
            .style(|_| iced::widget::container::Style {
                background: Some(COLOR_PANEL.into()),
                border: iced::Border {
                    color: Color::WHITE,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            });

        Container::new(picker_box)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(COLOR_OVERLAY_STRONG.into()),
                ..Default::default()
            })
            .into()
    }

    fn render_icon(
        &self,
        icon_path: Option<IconPath<'_>>,
        width: f32,
        height: f32,
        fallback_text: &'static str,
        fallback_size: Option<u32>,
    ) -> Element<'_, Message> {
        if let Some(icon_path) = icon_path {
            let is_svg = icon_path.is_svg();
            match icon_path {
                IconPath::Str(path) => {
                    if is_svg {
                        return Svg::from_path(path)
                            .width(Length::Fixed(width))
                            .height(Length::Fixed(height))
                            .into();
                    }

                    return Image::new(path)
                        .width(Length::Fixed(width))
                        .height(Length::Fixed(height))
                        .content_fit(ContentFit::Contain)
                        .into();
                }
                IconPath::Path(path) => {
                    if is_svg {
                        return Svg::from_path(path.to_path_buf())
                            .width(Length::Fixed(width))
                            .height(Length::Fixed(height))
                            .into();
                    }

                    return Image::new(path.to_path_buf())
                        .width(Length::Fixed(width))
                        .height(Length::Fixed(height))
                        .content_fit(ContentFit::Contain)
                        .into();
                }
            }
        }

        if let Some(handle) = self.default_icon_handle.clone() {
            Svg::new(handle)
                .width(Length::Fixed(width))
                .height(Length::Fixed(height))
                .into()
        } else {
            let text = match fallback_size {
                Some(size) => Text::new(fallback_text)
                    .font(SANSATION)
                    .size(size)
                    .color(Color::WHITE),
                None => Text::new(fallback_text).font(SANSATION).color(Color::WHITE),
            };

            Container::new(text)
                .width(Length::Fixed(width))
                .height(Length::Fixed(height))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        }
    }

    fn render_picker_item(&self, app: &DesktopApp, is_selected: bool) -> Element<'_, Message> {
        let icon_widget = self.render_icon(
            app.icon_path.as_deref().map(IconPath::Path),
            ICON_SIZE,
            ICON_SIZE,
            "?",
            Some(48),
        );

        let icon_container = Container::new(icon_widget).padding(6);

        let label = Text::new(app.name.clone())
            .font(SANSATION)
            .width(Length::Fixed(ICON_ITEM_WIDTH))
            .align_x(Horizontal::Center)
            .color(Color::WHITE)
            .size(12);

        let content = Column::new()
            .push(icon_container)
            .push(label)
            .align_x(iced::Alignment::Center)
            .spacing(5);

        Container::new(content)
            .width(Length::Fixed(ICON_ITEM_WIDTH))
            .height(Length::Fixed(ICON_ITEM_HEIGHT))
            .padding(6)
            .align_x(Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(move |_theme| {
                if is_selected {
                    iced::widget::container::Style {
                        border: iced::Border {
                            color: COLOR_ACCENT,
                            width: 2.0,
                            radius: 4.0.into(),
                        },
                        background: Some(COLOR_ACCENT_OVERLAY.into()),
                        ..Default::default()
                    }
                } else {
                    iced::widget::container::Style::default()
                }
            })
            .into()
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

        Subscription::batch(vec![gamepad, keyboard, window_events])
    }

    fn handle_navigation(&mut self, action: Action) -> Task<Message> {
        if action == Action::Quit {
            std::process::exit(0);
        }

        match &self.modal {
            ModalState::Help => return self.handle_help_modal_navigation(action),
            ModalState::ContextMenu { .. } => return self.handle_context_menu_navigation(action),
            ModalState::AppPicker(_) => return self.handle_app_picker_navigation(action),
            ModalState::None => {}
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
                Self::navigate_grid(&mut self.selected_index, action, self.cols, list_len);
            }
        }

        Task::none()
    }

    fn navigate_grid(index: &mut usize, action: Action, cols: usize, len: usize) -> bool {
        match action {
            Action::Up if *index >= cols => {
                *index -= cols;
                true
            }
            Action::Down if *index + cols < len => {
                *index += cols;
                true
            }
            Action::Left if *index > 0 => {
                *index -= 1;
                true
            }
            Action::Right if *index + 1 < len => {
                *index += 1;
                true
            }
            _ => false,
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

                            match save_config(
                                &self
                                    .apps
                                    .iter()
                                    .map(|item| AppEntry {
                                        id: item.id,
                                        name: item.name.clone(),
                                        icon: item.icon.clone(),
                                        exec: match &item.action {
                                            LauncherAction::Launch { exec } => exec.clone(),
                                            LauncherAction::SystemUpdate
                                            | LauncherAction::Shutdown
                                            | LauncherAction::Suspend
                                            | LauncherAction::Exit => unreachable!(),
                                        },
                                        game_executable: item.game_executable.clone(),
                                    })
                                    .collect::<Vec<_>>(),
                            ) {
                                Ok(_) => info!("Removed app: {}", removed.name),
                                Err(err) => warn!("Failed to save config after removal: {}", err),
                            }
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
                AbsoluteOffset {
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
                Self::navigate_grid(&mut selected_index, action, cols, list_len);
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
                // Check if it's a Steam game launch
                let steam_launch_prefix = "steam -applaunch ";
                let heroic_launch_prefix = "xdg-open heroic://launch/";

                let monitor_target = if exec.starts_with(steam_launch_prefix) {
                    let appid = exec
                        .trim_start_matches(steam_launch_prefix)
                        .trim()
                        .to_string();
                    // We still launch the steam command, but we monitor the AppId
                    Some(MonitorTarget::SteamAppId(appid))
                } else if exec.starts_with(heroic_launch_prefix) {
                    let url_part = exec.trim_start_matches(heroic_launch_prefix).trim();
                    let parts: Vec<&str> = url_part.split('/').collect();

                    let mut app_name = None;

                    if parts.len() >= 2 {
                        // store/app_name
                        if let Ok(decoded) = decode(parts[1]) {
                            app_name = Some(decoded.to_string());
                        }
                    } else if parts.len() == 1 {
                        // app_name
                        if let Ok(decoded) = decode(parts[0]) {
                            app_name = Some(decoded.to_string());
                        }
                    }

                    if let Some(name) = app_name {
                        info!("Detected Heroic launch for app: {}", name);

                        let mut targets = vec![
                            MonitorTarget::EnvVarEq("LEGENDARY_GAME_ID".to_string(), name.clone()),
                            MonitorTarget::EnvVarEq("HeroicAppName".to_string(), name.clone()),
                            MonitorTarget::CmdLineContains(item_name.clone()),
                        ];

                        // Add exact executable match if available
                        if let Some(exe) = game_executable {
                            info!("Monitoring executable for {}: {}", name, exe);
                            targets.push(MonitorTarget::CmdLineContains(exe));
                        }

                        let sanitized_name = item_name.replace(":", "");
                        if sanitized_name != item_name {
                            targets.push(MonitorTarget::CmdLineContains(sanitized_name));
                        }

                        Some(MonitorTarget::Any(targets))
                    } else {
                        None
                    }
                } else {
                    None
                };

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
            LauncherAction::SystemUpdate => match run_update() {
                Ok(message) => {
                    self.status_message = Some(message);
                    Task::none()
                }
                Err(err) => {
                    self.status_message = Some(err.to_string());
                    Task::none()
                }
            },
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
            self.render_icon(
                item.icon.as_deref().map(IconPath::Str),
                image_width,
                image_height,
                "ICON",
                None,
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

    fn render_help_modal(&self) -> Element<'_, Message> {
        let title = Text::new("Controller Bindings")
            .font(SANSATION)
            .size(28)
            .color(Color::WHITE);

        let title_container = Container::new(title)
            .padding(20)
            .width(Length::Fill)
            .center_x(Length::Fill);

        // Gamepad bindings
        let gamepad_bindings = vec![
            ("A / South", "Select / Confirm"),
            ("B / East", "Back / Cancel"),
            ("X / West", "Context Menu"),
            ("D-Pad / Left Stick", "Navigate"),
            ("LB / LT", "Previous Category"),
            ("RB / RT", "Next Category"),
            ("− / Select", "Show/Hide Controls"),
        ];

        // Keyboard bindings
        let keyboard_bindings = vec![
            ("Arrow Keys", "Navigate"),
            ("Enter", "Select / Confirm"),
            ("Escape", "Back / Cancel"),
            ("Tab", "Next Category"),
            ("C", "Context Menu"),
            ("+ / A", "Add App (in Apps)"),
            ("−", "Show/Hide Controls"),
            ("F4", "Quit Launcher"),
        ];

        let mut content_column = Column::new().spacing(8);

        // Gamepad section header
        content_column = content_column.push(
            Text::new("Gamepad")
                .font(SANSATION)
                .size(18)
                .color(COLOR_TEXT_SOFT),
        );

        // Gamepad bindings
        for (button, action) in gamepad_bindings {
            let row = Row::new()
                .push(
                    Container::new(
                        Text::new(button)
                            .font(SANSATION)
                            .size(16)
                            .color(COLOR_TEXT_BRIGHT),
                    )
                    .width(Length::Fixed(200.0)),
                )
                .push(
                    Text::new(action)
                        .font(SANSATION)
                        .size(16)
                        .color(COLOR_TEXT_MUTED),
                )
                .spacing(20);
            content_column = content_column.push(row);
        }

        // Spacer
        content_column =
            content_column.push(Container::new(Text::new("")).height(Length::Fixed(16.0)));

        // Keyboard section header
        content_column = content_column.push(
            Text::new("Keyboard")
                .font(SANSATION)
                .size(18)
                .color(COLOR_TEXT_SOFT),
        );

        // Keyboard bindings
        for (key, action) in keyboard_bindings {
            let row = Row::new()
                .push(
                    Container::new(
                        Text::new(key)
                            .font(SANSATION)
                            .size(16)
                            .color(COLOR_TEXT_BRIGHT),
                    )
                    .width(Length::Fixed(200.0)),
                )
                .push(
                    Text::new(action)
                        .font(SANSATION)
                        .size(16)
                        .color(COLOR_TEXT_MUTED),
                )
                .spacing(20);
            content_column = content_column.push(row);
        }

        let scrollable_content = Scrollable::new(content_column)
            .width(Length::Fill)
            .height(Length::Fill);

        // Hint at bottom
        let hint = Text::new("Press B or − to close")
            .font(SANSATION)
            .size(14)
            .color(COLOR_TEXT_HINT);

        let hint_container = Container::new(hint)
            .padding(10)
            .width(Length::Fill)
            .center_x(Length::Fill);

        let modal_column = Column::new()
            .push(title_container)
            .push(scrollable_content)
            .push(hint_container)
            .spacing(10);

        // Modal box
        let modal_box = Container::new(modal_column)
            .width(Length::Fixed(500.0))
            .height(Length::FillPortion(70))
            .padding(20)
            .style(|_| iced::widget::container::Style {
                background: Some(COLOR_PANEL.into()),
                border: iced::Border {
                    color: Color::WHITE,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            });

        // Overlay container with semi-transparent background
        Container::new(modal_box)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(COLOR_OVERLAY_STRONG.into()),
                ..Default::default()
            })
            .into()
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

#[allow(clippy::too_many_arguments)]
fn fetch_game_image(
    sgdb_client: SteamGridDbClient,
    searxng_client: SearxngClient,
    cache_dir: PathBuf,
    game_id: Uuid,
    game_name: String,
    source_image_url: Option<String>,
    width: u32,
    height: u32,
) -> anyhow::Result<Option<(Uuid, PathBuf)>> {
    let cache = ImageCache {
        cache_dir: cache_dir.clone(),
    };

    // 1. Check cache first
    if let Some(path) = cache.find_existing_image(&game_name) {
        info!("Cache hit for '{}': {:?}", game_name, path);
        return Ok(Some((game_id, path)));
    }

    // 2. Try source image URL (from Heroic) if available
    if let Some(url) = &source_image_url {
        info!("Trying Heroic image URL for '{}': {}", game_name, url);
        match cache.save_image(&game_name, url, width, height) {
            Ok(path) => {
                info!(
                    "Successfully saved Heroic image for '{}' to {:?}",
                    game_name, path
                );
                return Ok(Some((game_id, path)));
            }
            Err(e) => {
                warn!(
                    "Failed to download Heroic image for '{}': {}, trying SteamGridDB...",
                    game_name, e
                );
            }
        }
    }

    // 3. Try SteamGridDB (primary API source)
    info!("Fetching image for '{}' from SteamGridDB...", game_name);
    match sgdb_client.search_game(&game_name) {
        Ok(Some(sgdb_id)) => {
            info!("Found SteamGridDB ID for '{}': {}", game_name, sgdb_id);
            match sgdb_client.get_images_for_game(sgdb_id) {
                Ok(images) => {
                    if let Some(first_image) = images.first() {
                        info!("Downloading image for '{}': {}", game_name, first_image.url);
                        match cache.save_image(&game_name, &first_image.url, width, height) {
                            Ok(path) => {
                                info!(
                                    "Successfully saved SteamGridDB image for '{}' to {:?}",
                                    game_name, path
                                );
                                return Ok(Some((game_id, path)));
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to save SteamGridDB image for '{}': {}, trying SearXNG...",
                                    game_name, e
                                );
                            }
                        }
                    } else {
                        warn!(
                            "No images found on SteamGridDB for '{}' (ID: {}), trying SearXNG...",
                            game_name, sgdb_id
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to get SteamGridDB images for '{}': {}, trying SearXNG...",
                        game_name, e
                    );
                }
            }
        }
        Ok(None) => {
            warn!(
                "Game not found on SteamGridDB: '{}', trying SearXNG...",
                game_name
            );
        }
        Err(e) => {
            warn!(
                "Failed to search SteamGridDB for '{}': {}, trying SearXNG...",
                game_name, e
            );
        }
    }

    // 4. Fall back to SearXNG image search
    let search_query = format!("{} game cover", game_name);
    info!("Searching SearXNG for '{}' cover art...", game_name);
    match searxng_client.search_image(&search_query) {
        Ok(Some(url)) => {
            info!("Found SearXNG image for '{}': {}", game_name, url);
            match cache.save_image(&game_name, &url, width, height) {
                Ok(path) => {
                    info!(
                        "Successfully saved SearXNG image for '{}' to {:?}",
                        game_name, path
                    );
                    return Ok(Some((game_id, path)));
                }
                Err(e) => {
                    error!("Failed to save SearXNG image for '{}': {}", game_name, e);
                }
            }
        }
        Ok(None) => {
            warn!("No images found on SearXNG for '{}'", game_name);
        }
        Err(e) => {
            error!("Failed to search SearXNG for '{}': {}", game_name, e);
        }
    }

    // No image found from any source
    warn!("Could not find any cover art for '{}'", game_name);
    Ok(None)
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
        let mut index = 5;
        let moved = Launcher::navigate_grid(&mut index, Action::Up, 3, 10);

        assert!(moved);
        assert_eq!(index, 2);
    }

    #[test]
    fn test_navigate_grid_blocks_out_of_bounds() {
        let mut index = 1;
        let moved = Launcher::navigate_grid(&mut index, Action::Up, 3, 10);

        assert!(!moved);
        assert_eq!(index, 1);
    }
}
