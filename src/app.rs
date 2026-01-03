use iced::alignment::Horizontal;
use iced::keyboard::{self, key::Named};
use iced::{
    widget::{float, Column, Container, Image, Row, Scrollable, Stack, Svg, Text},
    Color, Element, Event, Length, Subscription, Task, Vector,
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
    context_menu: Option<ContextMenuState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Main,
    Add,
}

#[derive(Debug, Clone, Copy)]
enum ContextMenuAction {
    Launch,
    Remove,
}

#[derive(Debug, Clone, Copy)]
struct ContextMenuItem {
    label: &'static str,
    action: ContextMenuAction,
}

const CONTEXT_MENU_ITEMS: [ContextMenuItem; 2] = [
    ContextMenuItem {
        label: "Launch",
        action: ContextMenuAction::Launch,
    },
    ContextMenuItem {
        label: "Remove from Home",
        action: ContextMenuAction::Remove,
    },
];

#[derive(Debug, Clone)]
struct ContextMenuState {
    target_index: usize,
    selected_index: usize,
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
                context_menu: None,
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
                self.context_menu = None;
                Task::none()
            }
            Message::Input(action) => self.handle_navigation(action),
            Message::ToggleMode => match self.mode {
                Mode::Main => {
                    self.mode = Mode::Add;
                    self.selected_index = 0;
                    self.context_menu = None;
                    Task::perform(async { scan_system_apps() }, Message::ScannedApps)
                }
                Mode::Add => {
                    self.mode = Mode::Main;
                    self.selected_index = 0;
                    self.context_menu = None;
                    Task::none()
                }
            },
            Message::ScannedApps(apps) => {
                self.available_apps = apps;
                self.context_menu = None;
                Task::none()
            }
            Message::AppSelected(idx) => {
                if let Some(app) = self.entries.get(idx) {
                    launch_app(&app.exec);
                }
                self.context_menu = None;
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
                self.context_menu = None;
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
                self.context_menu = None;
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
                Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => match key {
                    keyboard::Key::Named(Named::ArrowUp) => Some(Message::Input(Action::Up)),
                    keyboard::Key::Named(Named::ArrowDown) => Some(Message::Input(Action::Down)),
                    keyboard::Key::Named(Named::ArrowLeft) => Some(Message::Input(Action::Left)),
                    keyboard::Key::Named(Named::ArrowRight) => Some(Message::Input(Action::Right)),
                    keyboard::Key::Named(Named::Enter) => Some(Message::Input(Action::Select)),
                    keyboard::Key::Named(Named::Escape) => Some(Message::Input(Action::Back)),
                    keyboard::Key::Named(Named::Tab) => Some(Message::ToggleMode),
                    keyboard::Key::Named(Named::Delete) => Some(Message::Input(Action::Remove)),
                    keyboard::Key::Named(Named::ContextMenu) => {
                        Some(Message::Input(Action::ContextMenu))
                    }
                    keyboard::Key::Named(Named::F10) if modifiers.shift() => {
                        Some(Message::Input(Action::ContextMenu))
                    }
                    _ => None,
                },
                _ => None,
            }
        });

        Subscription::batch(vec![gamepad, keyboard])
    }

    fn handle_navigation(&mut self, action: Action) -> Task<Message> {
        if let Some(task) = self.handle_context_menu_navigation(action) {
            return task;
        }

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
            Action::Remove => {
                if self.mode == Mode::Main {
                    let idx = self.selected_index;
                    return Task::perform(async move { idx }, Message::RemoveApp);
                }
            }
            Action::ContextMenu => {
                if self.mode == Mode::Main && !self.entries.is_empty() {
                    self.context_menu = Some(ContextMenuState {
                        target_index: self.selected_index,
                        selected_index: 0,
                    });
                }
            }
        }
        Task::none()
    }

    fn handle_context_menu_navigation(&mut self, action: Action) -> Option<Task<Message>> {
        let menu = self.context_menu.as_mut()?;
        let item_count = CONTEXT_MENU_ITEMS.len();

        let task = match action {
            Action::Up => {
                if menu.selected_index == 0 {
                    menu.selected_index = item_count.saturating_sub(1);
                } else {
                    menu.selected_index -= 1;
                }
                Task::none()
            }
            Action::Down => {
                menu.selected_index = (menu.selected_index + 1) % item_count;
                Task::none()
            }
            Action::Select => {
                let target = menu.target_index;
                let action = CONTEXT_MENU_ITEMS[menu.selected_index].action;
                self.context_menu = None;
                match action {
                    ContextMenuAction::Launch => {
                        Task::perform(async move { target }, Message::AppSelected)
                    }
                    ContextMenuAction::Remove => {
                        Task::perform(async move { target }, Message::RemoveApp)
                    }
                }
            }
            Action::Back | Action::ContextMenu => {
                self.context_menu = None;
                Task::none()
            }
            _ => Task::none(),
        };

        Some(task)
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

        let grid = self.render_grid(&self.entries);

        if let Some(menu) = &self.context_menu {
            let backdrop = self.render_context_menu_backdrop();
            let menu_panel = self.render_context_menu_panel(menu);
            let floating_menu = float(menu_panel).translate(|bounds, viewport| {
                let target_x = viewport.x + viewport.width / 2.0 - bounds.width / 2.0;
                let target_y = viewport.y + viewport.height / 2.0 - bounds.height / 2.0;
                Vector::new(target_x - bounds.x, target_y - bounds.y)
            });

            Stack::with_children(vec![grid, backdrop, floating_menu.into()])
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            grid
        }
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

        Scrollable::new(grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn render_context_menu_panel(&self, menu: &ContextMenuState) -> Element<'_, Message> {
        let app_name = self
            .entries
            .get(menu.target_index)
            .map(|app| app.name.as_str())
            .unwrap_or("App");

        let mut items = Column::new().spacing(8);
        for (index, item) in CONTEXT_MENU_ITEMS.iter().enumerate() {
            let is_selected = index == menu.selected_index;
            let row = Container::new(Text::new(item.label).size(18).color(if is_selected {
                Color::WHITE
            } else {
                Color::from_rgb(0.8, 0.8, 0.8)
            }))
            .width(Length::Fill)
            .padding(8)
            .style(move |_theme| {
                if is_selected {
                    iced::widget::container::Style {
                        background: Some(Color::from_rgb(0.2, 0.4, 0.8).into()),
                        text_color: Some(Color::WHITE),
                        ..Default::default()
                    }
                } else {
                    iced::widget::container::Style {
                        background: Some(Color::from_rgb(0.15, 0.15, 0.15).into()),
                        text_color: Some(Color::WHITE),
                        ..Default::default()
                    }
                }
            });

            items = items.push(row);
        }

        let menu_panel = Container::new(
            Column::new()
                .push(
                    Text::new(format!("Options for {app_name}"))
                        .size(20)
                        .color(Color::WHITE),
                )
                .push(items)
                .spacing(12),
        )
        .width(Length::Fixed(300.0))
        .padding(16)
        .style(|_theme| iced::widget::container::Style {
            background: Some(Color::from_rgb(0.12, 0.12, 0.12).into()),
            text_color: Some(Color::WHITE),
            ..Default::default()
        });

        menu_panel.into()
    }

    fn render_context_menu_backdrop(&self) -> Element<'_, Message> {
        Container::new(Text::new(""))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| iced::widget::container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.6).into()),
                text_color: Some(Color::WHITE),
                ..Default::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_menu_open_and_close() {
        let (mut launcher, _) = Launcher::new();
        launcher.entries = vec![AppEntry::new("Demo".to_string(), "demo".to_string(), None)];
        launcher.selected_index = 0;

        let _ = launcher.handle_navigation(Action::ContextMenu);
        assert!(launcher.context_menu.is_some());

        let _ = launcher.handle_navigation(Action::Back);
        assert!(launcher.context_menu.is_none());
    }

    #[test]
    fn test_context_menu_navigation_wraps() {
        let (mut launcher, _) = Launcher::new();
        launcher.entries = vec![AppEntry::new("Demo".to_string(), "demo".to_string(), None)];
        launcher.selected_index = 0;

        let _ = launcher.handle_navigation(Action::ContextMenu);
        let last_index = CONTEXT_MENU_ITEMS.len().saturating_sub(1);
        let _ = launcher.handle_navigation(Action::Up);
        assert_eq!(
            launcher.context_menu.as_ref().unwrap().selected_index,
            last_index
        );

        let _ = launcher.handle_navigation(Action::Down);
        assert_eq!(launcher.context_menu.as_ref().unwrap().selected_index, 0);
    }
}
