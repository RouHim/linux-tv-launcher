use iced::alignment::Horizontal;
use iced::widget::Id;
use iced::widget::{operation, Column, Container, Grid, Scrollable, Text};
use iced::{Color, Element, Length, Task};

use crate::desktop_apps::DesktopApp;
use crate::input::Action;
use crate::messages::Message;
use crate::ui_components::render_icon;
use crate::ui_theme::*;

pub struct AppPickerState {
    pub selected_index: usize,
    pub cols: usize,
    pub scrollable_id: Id,
    pub scroll_offset: f32,
    pub viewport_height: f32,
}

impl AppPickerState {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            cols: 6,
            scrollable_id: Id::unique(),
            scroll_offset: 0.0,
            viewport_height: 0.0,
        }
    }

    pub fn update_cols(&mut self, window_width: f32, scale: f32) {
        let available_width =
            window_width * APP_PICKER_WIDTH_RATIO - scaled(APP_PICKER_PADDING, scale);
        let item_space = scaled(ICON_ITEM_WIDTH, scale) + scaled(ITEM_SPACING, scale);
        let cols = (available_width / item_space).floor() as usize;
        self.cols = cols.max(1);
    }

    pub fn snap_to_selection(&self, scale: f32) -> Task<Message> {
        let row = self.selected_index / self.cols;
        let item_height_with_spacing =
            scaled(ICON_ITEM_HEIGHT, scale) + scaled(ITEM_SPACING, scale);

        let item_top = row as f32 * item_height_with_spacing;
        let item_bottom = item_top + scaled(ICON_ITEM_HEIGHT, scale);

        let viewport_top = self.scroll_offset;
        let viewport_height = if self.viewport_height > 0.0 {
            self.viewport_height
        } else {
            scaled(DEFAULT_VIEWPORT_HEIGHT, scale)
        };
        let viewport_bottom = viewport_top + viewport_height;

        let target_y = if item_top < viewport_top {
            Some(item_top)
        } else if item_bottom > viewport_bottom {
            Some(item_bottom - viewport_height + scaled(10.0, scale))
        } else {
            None
        };

        if let Some(y) = target_y {
            operation::scroll_to(
                self.scrollable_id.clone(),
                iced::widget::scrollable::AbsoluteOffset {
                    x: 0.0,
                    y: y.max(0.0),
                },
            )
        } else {
            Task::none()
        }
    }

    pub fn navigate(&mut self, action: Action, list_len: usize) {
        if list_len == 0 {
            return;
        }
        self.selected_index = Self::grid_navigate(self.selected_index, action, self.cols, list_len);
    }

    fn grid_navigate(current: usize, action: Action, cols: usize, len: usize) -> usize {
        match action {
            Action::Up if current >= cols => current - cols,
            Action::Down if current + cols < len => current + cols,
            Action::Left if current > 0 => current - 1,
            Action::Right if current + 1 < len => current + 1,
            _ => current,
        }
    }
}

pub fn render_app_picker<'a>(
    state: &'a AppPickerState,
    available_apps: &'a [DesktopApp],
    scale: f32,
) -> Element<'a, Message> {
    let title = Text::new("Add Application")
        .font(SANSATION)
        .size(scaled(BASE_FONT_HEADER, scale))
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(scaled(BASE_PADDING_MEDIUM, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let content: Element<'_, Message> = if available_apps.is_empty() {
        Container::new(
            Text::new("No applications found")
                .font(SANSATION)
                .size(scaled(BASE_FONT_LARGE, scale))
                .color(COLOR_TEXT_MUTED),
        )
        .padding(scaled(BASE_PADDING_LARGE, scale))
        .center_x(Length::Fill)
        .into()
    } else {
        let mut grid = Grid::new()
            .columns(state.cols)
            .spacing(scaled(ITEM_SPACING, scale))
            .height(Length::Shrink);

        for (i, app) in available_apps.iter().enumerate() {
            let is_selected = i == state.selected_index;
            grid = grid.push(render_picker_item(app, is_selected, scale));
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
        .size(scaled(BASE_FONT_SMALL, scale))
        .color(COLOR_TEXT_HINT);

    let hint_container = Container::new(hint)
        .padding(scaled(BASE_PADDING_SMALL, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let picker_column = Column::new()
        .push(title_container)
        .push(content)
        .push(hint_container)
        .spacing(scaled(BASE_PADDING_SMALL, scale));

    let border_radius = scaled(10.0, scale);
    let picker_box = Container::new(picker_column)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(COLOR_PANEL.into()),
            border: iced::Border {
                color: Color::WHITE,
                width: 1.0,
                radius: border_radius.into(),
            },
            ..Default::default()
        });

    Container::new(picker_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(scaled(MODAL_OVERLAY_PADDING, scale))
        .style(|_| iced::widget::container::Style {
            background: Some(COLOR_OVERLAY_STRONG.into()),
            ..Default::default()
        })
        .into()
}

fn render_picker_item<'a>(
    app: &'a DesktopApp,
    is_selected: bool,
    scale: f32,
) -> Element<'a, Message> {
    let icon_size = scaled(ICON_SIZE, scale);
    let icon_widget = render_icon(
        app.icon_path.clone(),
        icon_size,
        icon_size,
        "?",
        Some((48.0 * scale) as u32),
        None,
    );

    let icon_container = Container::new(icon_widget).padding(scaled(BASE_PADDING_TINY, scale));

    let item_width = scaled(ICON_ITEM_WIDTH, scale);
    let label = Text::new(app.name.clone())
        .font(SANSATION)
        .width(Length::Fixed(item_width))
        .align_x(Horizontal::Center)
        .color(Color::WHITE)
        .size(scaled(BASE_FONT_TINY, scale));

    let content = Column::new()
        .push(icon_container)
        .push(label)
        .align_x(iced::Alignment::Center)
        .spacing(scaled(5.0, scale));

    let item_height = scaled(ICON_ITEM_HEIGHT, scale);
    let border_radius = scaled(4.0, scale);
    Container::new(content)
        .width(Length::Fixed(item_width))
        .height(Length::Fixed(item_height))
        .padding(scaled(BASE_PADDING_TINY, scale))
        .align_x(Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_theme| {
            if is_selected {
                iced::widget::container::Style {
                    border: iced::Border {
                        color: COLOR_ACCENT,
                        width: 2.0,
                        radius: border_radius.into(),
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
