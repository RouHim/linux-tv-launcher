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

    pub fn update_cols(&mut self, window_width: f32) {
        let available_width = window_width * APP_PICKER_WIDTH_RATIO - APP_PICKER_PADDING;
        let item_space = ICON_ITEM_WIDTH + ITEM_SPACING;
        let cols = (available_width / item_space).floor() as usize;
        self.cols = cols.max(1);
    }

    pub fn snap_to_selection(&self) -> Task<Message> {
        let row = self.selected_index / self.cols;
        let item_height_with_spacing = ICON_ITEM_HEIGHT + ITEM_SPACING;

        let item_top = row as f32 * item_height_with_spacing;
        let item_bottom = item_top + ICON_ITEM_HEIGHT;

        let viewport_top = self.scroll_offset;
        // Use reported viewport height, or fallback estimate if not yet reported (e.g. initial render)
        let viewport_height = if self.viewport_height > 0.0 {
            self.viewport_height
        } else {
            DEFAULT_VIEWPORT_HEIGHT
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
) -> Element<'a, Message> {
    let title = Text::new("Add Application")
        .font(SANSATION)
        .size(28)
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(20)
        .width(Length::Fill)
        .center_x(Length::Fill);

    let content: Element<'_, Message> = if available_apps.is_empty() {
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

        for (i, app) in available_apps.iter().enumerate() {
            let is_selected = i == state.selected_index;
            grid = grid.push(render_picker_item(app, is_selected));
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
        .width(Length::Fill)
        .height(Length::Fill)
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
        .padding(100)
        .style(|_| iced::widget::container::Style {
            background: Some(COLOR_OVERLAY_STRONG.into()),
            ..Default::default()
        })
        .into()
}

fn render_picker_item<'a>(app: &'a DesktopApp, is_selected: bool) -> Element<'a, Message> {
    let icon_widget = render_icon(
        app.icon_path.clone(),
        ICON_SIZE,
        ICON_SIZE,
        "?",
        Some(48),
        None,
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
