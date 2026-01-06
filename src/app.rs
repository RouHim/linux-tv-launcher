use iced::alignment::Horizontal;
use iced::keyboard::{self, key::Named};
use iced::{
    widget::{Column, Container, Grid, Image, Row, Scrollable, Svg, Text},
    Color, ContentFit, Element, Event, Length, Subscription, Task,
};
use std::path::PathBuf;
use uuid::Uuid;

use crate::assets::get_default_icon;
use crate::game_sources::scan_games;
use crate::gamepad::gamepad_subscription;
use crate::image_cache::ImageCache;
use crate::input::Action;
use crate::launcher::launch_app;
use crate::model::{AppEntry, Category, LauncherAction, LauncherItem};
use crate::steamgriddb::SteamGridDbClient;
use crate::storage::{config_path, load_config};
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
    image_cache: Option<ImageCache>,
    scale_factor: f64,
}

const GAME_POSTER_WIDTH: f32 = 200.0;
const GAME_POSTER_HEIGHT: f32 = 300.0;

#[derive(Debug, Clone)]
pub enum Message {
    AppsLoaded(Result<Vec<AppEntry>, String>),
    GamesLoaded(Vec<AppEntry>),
    ImageFetched(Uuid, PathBuf),
    Input(Action),
    ScaleFactorChanged(f64),
    None,
}

impl Launcher {
    pub fn new() -> (Self, Task<Message>) {
        let default_icon = get_default_icon().map(iced::widget::svg::Handle::from_memory);
        let config_path = config_path().ok().map(|path| path.display().to_string());
        
        let sgdb_client = SteamGridDbClient::new("276bca336e815a4e2dd2250ea674eb31".to_string());
        let image_cache = ImageCache::new().ok();

        (
            Self {
                apps: Vec::new(),
                games: Vec::new(),
                system_items: vec![LauncherItem::system_update()],
                selected_index: 0,
                category: Category::Apps,
                cols: 4,
                default_icon_handle: default_icon,
                status_message: None,
                config_path,
                apps_loaded: false,
                games_loaded: false,
                sgdb_client,
                image_cache,
                scale_factor: 1.0,
            },
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

                    for game in &self.games {
                        let game_id = game.id;
                        let game_name = game.name.clone();
                        let client = self.sgdb_client.clone();
                        let cache_dir = cache.cache_dir.clone();

                        tasks.push(Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || {
                                    fetch_game_image(
                                        client,
                                        cache_dir,
                                        game_id,
                                        game_name,
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
                        ));
                    }
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

        Container::new(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| iced::widget::container::Style {
                background: Some(Color::from_rgb(0.05, 0.05, 0.05).into()),
                text_color: Some(Color::WHITE),
                ..Default::default()
            })
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let gamepad = gamepad_subscription().map(Message::Input);

        let window_events = iced::event::listen_with(|event, _status, _window| match event {
            Event::Window(iced::window::Event::Rescaled(scale_factor)) => {
                Some(Message::ScaleFactorChanged(scale_factor as f64))
            }
            _ => None,
        });

        let keyboard = iced::event::listen_with(|event, status, _window| {
            if let iced::event::Status::Captured = status {
                return None;
            }

            match event {
                Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => match key {
                    keyboard::Key::Named(Named::ArrowUp) => Some(Message::Input(Action::Up)),
                    keyboard::Key::Named(Named::ArrowDown) => Some(Message::Input(Action::Down)),
                    keyboard::Key::Named(Named::ArrowLeft) => Some(Message::Input(Action::Left)),
                    keyboard::Key::Named(Named::ArrowRight) => Some(Message::Input(Action::Right)),
                    keyboard::Key::Named(Named::Enter) => Some(Message::Input(Action::Select)),
                    keyboard::Key::Named(Named::Escape) => Some(Message::Input(Action::Back)),
                    keyboard::Key::Named(Named::Tab) => Some(Message::Input(Action::NextCategory)),
                    _ => None,
                },
                _ => None,
            }
        });

        Subscription::batch(vec![gamepad, keyboard, window_events])
    }

    fn handle_navigation(&mut self, action: Action) -> Task<Message> {
        match action {
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
        }
    }

    fn cycle_category(&mut self) {
        self.category = self.category.next();
        self.selected_index = 0;
        self.status_message = None;
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
            let label = Text::new(category.title()).size(22).color(if is_selected {
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

        Column::new()
            .push(tabs)
            .push(
                Text::new("Tab/Start to switch categories")
                    .size(14)
                    .color(Color::from_rgb(0.6, 0.6, 0.6)),
            )
            .spacing(6)
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
                        .push(Text::new("No system actions available.").color(Color::WHITE))
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
                .push(Text::new(message).color(Color::WHITE))
                .align_x(iced::Alignment::Center)
                .into();
        }

        self.render_grid(items)
    }

    fn render_grid(&self, items: &[LauncherItem]) -> Element<'_, Message> {
        // Determine dimensions based on category
        let (item_width, item_height, image_width, image_height) = match self.category {
            Category::Games => (
                GAME_POSTER_WIDTH + 20.0,
                GAME_POSTER_HEIGHT + 40.0,
                GAME_POSTER_WIDTH,
                GAME_POSTER_HEIGHT,
            ), // Poster style
            _ => (150.0, 150.0, 64.0, 64.0), // Icon style
        };

        let mut grid = Grid::new()
            .columns(self.cols)
            .spacing(20);

        for (i, item) in items.iter().enumerate() {
            let is_selected = i == self.selected_index;

            let icon_widget: Element<Message> = if let Some(icon_path) = &item.icon {
                if icon_path.ends_with(".svg") {
                    Svg::from_path(icon_path)
                        .width(Length::Fixed(image_width))
                        .height(Length::Fixed(image_height))
                        .into()
                } else {
                    Image::new(icon_path)
                        .width(Length::Fixed(image_width))
                        .height(Length::Fixed(image_height))
                        .content_fit(ContentFit::Cover)
                        .into()
                }
            } else if let Some(handle) = self.default_icon_handle.clone() {
                Svg::new(handle)
                    .width(Length::Fixed(image_width))
                    .height(Length::Fixed(image_height))
                    .into()
            } else {
                Text::new("ICON").color(Color::WHITE).into()
            };

            let content = if matches!(self.category, Category::Games) {
                // For games, just show the poster image (maybe text overlay if selected?)
                // Or standard layout: Image + Text below
                Column::new()
                    .push(icon_widget)
                    .push(
                        Text::new(item.name.clone())
                            .width(Length::Fixed(image_width))
                            .align_x(Horizontal::Center)
                            .color(Color::WHITE)
                            .size(14),
                    )
                    .align_x(iced::Alignment::Center)
                    .spacing(10)
            } else {
                    Column::new()
                .push(icon_widget)
                .push(
                    Text::new(item.name.clone())
                        .align_x(Horizontal::Center)
                        .color(Color::WHITE),
                )
                .align_x(iced::Alignment::Center)
                .spacing(10)
            };

            let container = Container::new(content)
                .width(Length::Fixed(item_width))
                .height(Length::Fixed(item_height))
                .center_x(Length::Fixed(item_width))
                .center_y(Length::Fixed(item_height))
                .style(move |_theme| {
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

            grid = grid.push(container);
        }

        Scrollable::new(grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn render_status(&self) -> Option<Element<'_, Message>> {
        let status = self.status_message.as_ref()?;
        Some(
            Container::new(Text::new(status).color(Color::from_rgb(0.9, 0.8, 0.4)))
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
    client: SteamGridDbClient,
    cache_dir: PathBuf,
    game_id: Uuid,
    game_name: String,
    width: u32,
    height: u32,
) -> anyhow::Result<Option<(Uuid, PathBuf)>> {
    let cache = ImageCache {
        cache_dir: cache_dir.clone(),
    };

    let path = if let Some(path) = cache.find_existing_image(&game_name) {
        info!("Cache hit for '{}': {:?}", game_name, path);
        path
    } else {
        info!("Fetching image for '{}' from SteamGridDB...", game_name);
        match client.search_game(&game_name) {
            Ok(Some(sgdb_id)) => {
                info!("Found SteamGridDB ID for '{}': {}", game_name, sgdb_id);
                match client.get_images_for_game(sgdb_id) {
                    Ok(images) => {
                        if let Some(first_image) = images.first() {
                            info!("Downloading image for '{}': {}", game_name, first_image.url);
                            match cache.save_image(&game_name, &first_image.url, width, height) {
                                Ok(path) => {
                                    info!(
                                        "Successfully saved image for '{}' to {:?}",
                                        game_name, path
                                    );
                                    path
                                }
                                Err(e) => {
                                    error!("Failed to save image for '{}': {}", game_name, e);
                                    return Ok(None);
                                }
                            }
                        } else {
                            warn!("No images found for '{}' (ID: {})", game_name, sgdb_id);
                            return Ok(None);
                        }
                    }
                    Err(e) => {
                        error!("Failed to get images for '{}': {}", game_name, e);
                        return Ok(None);
                    }
                }
            }
            Ok(None) => {
                warn!("Game not found on SteamGridDB: '{}'", game_name);
                return Ok(None);
            }
            Err(e) => {
                error!("Failed to search for game '{}': {}", game_name, e);
                return Ok(None);
            }
        }
    };

    Ok(Some((game_id, path)))
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
