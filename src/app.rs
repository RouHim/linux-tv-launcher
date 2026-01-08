use iced::alignment::Horizontal;
use iced::keyboard::{self, key::Named, Key};
use iced::{
    widget::{Column, Container, Grid, Image, Row, Scrollable, Stack, Svg, Text},
    Color, ContentFit, Element, Event, Length, Subscription, Task,
};
use rayon::prelude::*;
use std::path::PathBuf;
use uuid::Uuid;

use crate::assets::get_default_icon;
use crate::game_sources::scan_games;
use crate::gamepad::gamepad_subscription;
use crate::image_cache::ImageCache;
use crate::input::Action;
use crate::launcher::launch_app;
use crate::model::{AppEntry, Category, LauncherAction, LauncherItem};
use crate::searxng::SearxngClient;
use crate::steamgriddb::SteamGridDbClient;
use crate::storage::{config_path, load_config, save_config};
use crate::system_update::run_update;
use tracing::{error, info, warn};

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
    context_menu_open: bool,
    context_menu_index: usize,
}

const GAME_POSTER_WIDTH: f32 = 200.0;
const GAME_POSTER_HEIGHT: f32 = 300.0;

// App/System icon dimensions
const ICON_SIZE: f32 = 128.0;

// Custom font
const SANSATION: iced::Font = iced::Font::with_name("Sansation");
const ICON_ITEM_WIDTH: f32 = 150.0; // Increased to allow padding
const ICON_ITEM_HEIGHT: f32 = 280.0; // Increased to allow text wrapping

#[derive(Debug, Clone)]
pub enum Message {
    AppsLoaded(Result<Vec<AppEntry>, String>),
    GamesLoaded(Vec<AppEntry>),
    ImageFetched(Uuid, PathBuf),
    Input(Action),
    ScaleFactorChanged(f64),
    WindowResized(f32, f32),
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
            context_menu_open: false,
            context_menu_index: 0,
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
            Message::None => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = self.render_header();
        let content = self.render_category();

        let mut column = Column::new().push(header).push(content).spacing(20);
        if let Some(status) = self.render_status() {
            column = column.push(status);
        }

        let main_content = Container::new(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| iced::widget::container::Style {
                background: Some(Color::from_rgb(0.05, 0.05, 0.05).into()),
                text_color: Some(Color::WHITE),
                ..Default::default()
            })
            .into();

        if self.context_menu_open {
            Stack::new()
                .push(main_content)
                .push(self.render_context_menu())
                .into()
        } else {
            main_content
        }
    }

    fn render_context_menu(&self) -> Element<'_, Message> {
        let menu_items: Vec<&str> = match self.category {
            Category::Apps => vec!["Launch", "Remove Entry", "Quit Launcher", "Close"],
            Category::Games | Category::System => vec!["Launch", "Quit Launcher", "Close"],
        };
        let mut column = Column::new().spacing(10).padding(20);

        for (i, item) in menu_items.iter().enumerate() {
            let is_selected = i == self.context_menu_index;
            let text = Text::new(*item)
                .font(SANSATION)
                .size(20)
                .color(if is_selected {
                    Color::WHITE
                } else {
                    Color::from_rgb(0.7, 0.7, 0.7)
                })
                .align_x(Horizontal::Center);

            let container = Container::new(text)
                .padding(10)
                .width(Length::Fill)
                .style(move |_| {
                    if is_selected {
                        iced::widget::container::Style {
                            background: Some(Color::from_rgb(0.2, 0.4, 0.8).into()),
                            text_color: Some(Color::WHITE),
                            ..Default::default()
                        }
                    } else {
                        iced::widget::container::Style {
                            text_color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
                            ..Default::default()
                        }
                    }
                });

            column = column.push(container);
        }

        let menu_box = Container::new(column)
            .width(Length::Fixed(300.0))
            .style(|_| iced::widget::container::Style {
                background: Some(Color::from_rgb(0.15, 0.15, 0.15).into()),
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
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.7).into()),
                ..Default::default()
            })
            .into()
    }
// ... subscription, handle_navigation, handle_context_menu_navigation ...

    pub fn subscription(&self) -> Subscription<Message> {
        let gamepad = gamepad_subscription().map(Message::Input);

        let window_events = iced::event::listen_with(|event, _status, _window| match event {
            Event::Window(iced::window::Event::Rescaled(scale_factor)) => {
                Some(Message::ScaleFactorChanged(scale_factor as f64))
            }
            Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::WindowResized(size.width, size.height))
            }
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

        if self.context_menu_open {
            return self.handle_context_menu_navigation(action);
        }

        match action {
            Action::ContextMenu => {
                if !self.active_items().is_empty() {
                    self.context_menu_open = true;
                    self.context_menu_index = 0;
                }
                return Task::none();
            }
            Action::NextCategory => {
                self.cycle_category();
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
            Action::Up => {
                if self.selected_index >= self.cols {
                    self.selected_index -= self.cols;
                }
            }
            Action::Down => {
                if self.selected_index + self.cols < list_len {
                    self.selected_index += self.cols;
                }
            }
            Action::Left => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            Action::Right => {
                if self.selected_index + 1 < list_len {
                    self.selected_index += 1;
                }
            }
            Action::Select => {
                self.activate_selected();
            }
            _ => {}
        }

        Task::none()
    }

    fn handle_context_menu_navigation(&mut self, action: Action) -> Task<Message> {
        let max_index = match self.category {
            Category::Apps => 3,
            Category::Games | Category::System => 2,
        };

        match action {
            Action::Up => {
                if self.context_menu_index > 0 {
                    self.context_menu_index -= 1;
                }
            }
            Action::Down => {
                if self.context_menu_index < max_index {
                    self.context_menu_index += 1;
                }
            }
            Action::Select => {
                match (self.category, self.context_menu_index) {
                    (Category::Apps, 0) | (Category::Games, 0) | (Category::System, 0) => {
                        // Launch
                        self.context_menu_open = false;
                        self.activate_selected();
                    }
                    (Category::Apps, 1) => {
                        // Remove Entry
                        self.context_menu_open = false;
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
                        if self.context_menu_index == quit_index {
                            // Quit Launcher
                            std::process::exit(0);
                        } else {
                            // Close
                            self.context_menu_open = false;
                        }
                    }
                }
            }
            Action::Back | Action::ContextMenu => {
                self.context_menu_open = false;
            }
            _ => {}
        }
        Task::none()
    }

    fn activate_selected(&mut self) {
        let action = self
            .active_items()
            .get(self.selected_index)
            .map(|item| item.action.clone());

        let Some(action) = action else {
            return;
        };

        self.status_message = None;

        match action {
            LauncherAction::Launch { exec } => {
                if let Err(err) = launch_app(&exec) {
                    self.status_message = Some(err.to_string());
                }
            }
            LauncherAction::SystemUpdate => match run_update() {
                Ok(message) => {
                    self.status_message = Some(message);
                }
                Err(err) => {
                    self.status_message = Some(err.to_string());
                }
            },
            LauncherAction::Shutdown => {
                if let Err(e) = std::process::Command::new("systemctl")
                    .arg("poweroff")
                    .spawn()
                {
                    self.status_message = Some(format!("Failed to shutdown: {}", e));
                }
            }
            LauncherAction::Suspend => {
                if let Err(e) = std::process::Command::new("systemctl")
                    .arg("suspend")
                    .spawn()
                {
                    self.status_message = Some(format!("Failed to suspend: {}", e));
                }
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

    fn update_columns(&mut self) {
        let (item_width, _item_height, _image_width, _image_height) = match self.category {
            Category::Games => (
                GAME_POSTER_WIDTH + 16.0, // Extra width for padding/border
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
            let label = Text::new(category.title())
                .font(SANSATION)
                .size(22)
                .color(if is_selected {
                    Color::WHITE
                } else {
                    Color::from_rgb(0.7, 0.7, 0.7)
                });

            let tab = Container::new(label).padding(8).style(move |_theme| {
                if is_selected {
                    iced::widget::container::Style {
                        background: Some(Color::from_rgb(0.2, 0.4, 0.8).into()),
                        text_color: Some(Color::WHITE),
                        ..Default::default()
                    }
                } else {
                    iced::widget::container::Style {
                        background: Some(Color::from_rgb(0.1, 0.1, 0.1).into()),
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
                        .push(Text::new("No system actions available.").font(SANSATION).color(Color::WHITE))
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
        let icon_widget: Element<Message> = if let Some(icon_path) = &item.icon {
            // Check for embedded assets first
            let embedded_handle = match icon_path.as_str() {
                "assets/shutdown.svg" => {
                    crate::assets::get_shutdown_icon().map(iced::widget::svg::Handle::from_memory)
                }
                "assets/suspend.svg" => {
                    crate::assets::get_suspend_icon().map(iced::widget::svg::Handle::from_memory)
                }
                "assets/exit.svg" => {
                    crate::assets::get_exit_icon().map(iced::widget::svg::Handle::from_memory)
                }
                _ => None,
            };

            if let Some(handle) = embedded_handle {
                Svg::new(handle)
                    .width(Length::Fixed(image_width))
                    .height(Length::Fixed(image_height))
                    .into()
            } else if icon_path.ends_with(".svg") {
                Svg::from_path(icon_path)
                    .width(Length::Fixed(image_width))
                    .height(Length::Fixed(image_height))
                    .into()
            } else {
                Image::new(icon_path)
                    .width(Length::Fixed(image_width))
                    .height(Length::Fixed(image_height))
                    .content_fit(ContentFit::Contain)
                    .into()
            }
        } else if let Some(handle) = self.default_icon_handle.clone() {
            Svg::new(handle)
                .width(Length::Fixed(image_width))
                .height(Length::Fixed(image_height))
                .into()
        } else {
            Container::new(Text::new("ICON").font(SANSATION).color(Color::WHITE))
                .width(Length::Fixed(image_width))
                .height(Length::Fixed(image_height))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        let icon_container = Container::new(icon_widget)
            .padding(6);

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
                            color: Color::from_rgb(0.2, 0.4, 0.8),
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
            Container::new(Text::new(status).font(SANSATION).color(Color::from_rgb(0.9, 0.8, 0.4)))
                .padding(8)
                .style(|_theme| iced::widget::container::Style {
                    background: Some(Color::from_rgb(0.12, 0.12, 0.12).into()),
                    text_color: Some(Color::WHITE),
                    ..Default::default()
                })
                .into(),
        )
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
        info!(
            "Trying Heroic image URL for '{}': {}",
            game_name,
            url
        );
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
                    error!(
                        "Failed to save SearXNG image for '{}': {}",
                        game_name, e
                    );
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
}
