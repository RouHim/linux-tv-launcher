use iced::alignment::Horizontal;
use iced::keyboard::{self, key::Named};
use iced::{
    widget::{Column, Container, Image, Row, Scrollable, Svg, Text},
    Color, Element, Event, Length, Subscription, Task,
};

use crate::assets::get_default_icon;
use crate::game_sources::scan_games;
use crate::gamepad::gamepad_subscription;
use crate::input::Action;
use crate::launcher::launch_app;
use crate::model::{AppEntry, Category, LauncherAction, LauncherItem};
use crate::storage::{config_path, load_config};
use crate::system_update::run_update;
use tracing::warn;

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
}

#[derive(Debug, Clone)]
pub enum Message {
    AppsLoaded(Result<Vec<AppEntry>, String>),
    GamesLoaded(Vec<AppEntry>),
    Input(Action),
    None,
}

impl Launcher {
    pub fn new() -> (Self, Task<Message>) {
        let default_icon = get_default_icon().map(iced::widget::svg::Handle::from_memory);
        let config_path = config_path().ok().map(|path| path.display().to_string());

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
                Task::none()
            }
            Message::Input(action) => self.handle_navigation(action),
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

        Subscription::batch(vec![gamepad, keyboard])
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
        let mut grid = Column::new().spacing(20);

        for (i, chunk) in items.chunks(self.cols).enumerate() {
            let mut row = Row::new().spacing(20);
            for (j, item) in chunk.iter().enumerate() {
                let index = i * self.cols + j;
                let is_selected = index == self.selected_index;

                let icon_widget: Element<Message> = if let Some(icon_path) = &item.icon {
                    if icon_path.ends_with(".svg") {
                        Svg::from_path(icon_path)
                            .width(Length::Fixed(64.0))
                            .height(Length::Fixed(64.0))
                            .into()
                    } else {
                        Image::new(icon_path)
                            .width(Length::Fixed(64.0))
                            .height(Length::Fixed(64.0))
                            .into()
                    }
                } else if let Some(handle) = self.default_icon_handle.clone() {
                    Svg::new(handle)
                        .width(Length::Fixed(64.0))
                        .height(Length::Fixed(64.0))
                        .into()
                } else {
                    Text::new("ICON").color(Color::WHITE).into()
                };

                let content = Column::new()
                    .push(icon_widget)
                    .push(
                        Text::new(item.name.clone())
                            .align_x(Horizontal::Center)
                            .color(Color::WHITE),
                    )
                    .align_x(iced::Alignment::Center)
                    .spacing(10);

                let container = Container::new(content)
                    .width(Length::Fixed(150.0))
                    .height(Length::Fixed(150.0))
                    .center_x(Length::Fixed(150.0))
                    .center_y(Length::Fixed(150.0))
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

                row = row.push(container);
            }
            grid = grid.push(row);
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
