use iced::alignment::Horizontal;
use iced::keyboard::{self, key::Named};
use iced::{
    widget::{Column, Container, Image, Row, Scrollable, Svg, Text},
    Color, Element, Event, Length, Subscription, Task,
};

use crate::assets::get_default_icon;
use crate::gamepad::gamepad_subscription;
use crate::input::Action;
use crate::launcher::launch_app;
use crate::model::AppEntry;
use crate::storage::{load_config, save_config};
use crate::xdg_utils::scan_system_apps;

pub struct Launcher {
    entries: Vec<AppEntry>,
    available_apps: Vec<AppEntry>,
    selected_index: usize,
    mode: Mode,
    cols: usize,
    default_icon_handle: Option<iced::widget::svg::Handle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Main,
    Add,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<AppEntry>),
    Input(Action),
    ToggleMode,
    AppSelected(usize),
    AddApp(usize),
    RemoveApp(usize),
    ScannedApps(Vec<AppEntry>),
    None,
}

impl Launcher {
    pub fn new() -> (Self, Task<Message>) {
        let default_icon = get_default_icon().map(iced::widget::svg::Handle::from_memory);

        (
            Self {
                entries: Vec::new(),
                available_apps: Vec::new(),
                selected_index: 0,
                mode: Mode::Main,
                cols: 4,
                default_icon_handle: default_icon,
            },
            Task::perform(async { load_config().unwrap_or_default() }, Message::Loaded),
        )
    }

    pub fn title(&self) -> String {
        String::from("Linux TV Launcher")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(apps) => {
                self.entries = apps;
                self.selected_index = 0;
                Task::none()
            }
            Message::Input(action) => self.handle_navigation(action),
            Message::ToggleMode => match self.mode {
                Mode::Main => {
                    self.mode = Mode::Add;
                    self.selected_index = 0;
                    Task::perform(async { scan_system_apps() }, Message::ScannedApps)
                }
                Mode::Add => {
                    self.mode = Mode::Main;
                    self.selected_index = 0;
                    Task::none()
                }
            },
            Message::ScannedApps(apps) => {
                self.available_apps = apps;
                Task::none()
            }
            Message::AppSelected(idx) => {
                if let Some(app) = self.entries.get(idx) {
                    launch_app(&app.exec);
                }
                Task::none()
            }
            Message::AddApp(idx) => {
                if let Some(app) = self.available_apps.get(idx) {
                    if !self.entries.iter().any(|e| e.name == app.name) {
                        self.entries.push(app.clone());
                        let _ = save_config(&self.entries);
                    }
                }
                self.mode = Mode::Main;
                self.selected_index = self.entries.len().saturating_sub(1);
                Task::none()
            }
            Message::RemoveApp(idx) => {
                if idx < self.entries.len() {
                    self.entries.remove(idx);
                    let _ = save_config(&self.entries);
                    if self.selected_index >= self.entries.len() {
                        self.selected_index = self.entries.len().saturating_sub(1);
                    }
                }
                Task::none()
            }
            Message::None => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = match self.mode {
            Mode::Main => self.view_main(),
            Mode::Add => self.view_add(),
        };

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            // Use simple theme color or check Appearance
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
                    keyboard::Key::Named(Named::Tab) => Some(Message::ToggleMode),
                    _ => None,
                },
                _ => None,
            }
        });

        Subscription::batch(vec![gamepad, keyboard])
    }

    fn handle_navigation(&mut self, action: Action) -> Task<Message> {
        let list_len = match self.mode {
            Mode::Main => self.entries.len(),
            Mode::Add => self.available_apps.len(),
        };

        if list_len == 0 {
            if action == Action::Select && self.mode == Mode::Main {
                return Task::perform(async {}, |_| Message::ToggleMode);
            }
            if action == Action::Back && self.mode == Mode::Add {
                return Task::perform(async {}, |_| Message::ToggleMode);
            }
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
                let idx = self.selected_index;
                match self.mode {
                    Mode::Main => return Task::perform(async move { idx }, Message::AppSelected),
                    Mode::Add => return Task::perform(async move { idx }, Message::AddApp),
                }
            }
            Action::Back => {
                return Task::perform(async {}, |_| Message::ToggleMode);
            }
        }
        Task::none()
    }

    fn view_main(&self) -> Element<'_, Message> {
        if self.entries.is_empty() {
            return Column::new()
                .push(
                    Text::new("No apps added. Press 'Select' or 'Tab' to add apps.")
                        .color(Color::WHITE),
                )
                .align_x(iced::Alignment::Center)
                .into();
        }

        self.render_grid(&self.entries)
    }

    fn view_add(&self) -> Element<'_, Message> {
        if self.available_apps.is_empty() {
            return Column::new()
                .push(Text::new("Scanning apps...").color(Color::WHITE))
                .align_x(iced::Alignment::Center)
                .into();
        }

        Column::new()
            .push(
                Text::new("Select an app to add")
                    .size(24)
                    .color(Color::WHITE),
            )
            .push(self.render_grid(&self.available_apps))
            .spacing(20)
            .into()
    }

    fn render_grid(&self, apps: &[AppEntry]) -> Element<'_, Message> {
        let mut grid = Column::new().spacing(20);

        for (i, chunk) in apps.chunks(self.cols).enumerate() {
            let mut row = Row::new().spacing(20);
            for (j, app) in chunk.iter().enumerate() {
                let index = i * self.cols + j;
                let is_selected = index == self.selected_index;

                let icon_widget: Element<Message> = if let Some(icon_path) = &app.icon {
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
                        Text::new(app.name.clone())
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

        Scrollable::new(grid).height(Length::Fill).into()
    }
}
