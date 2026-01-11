use iced::alignment::Horizontal;
use iced::widget::Id;
use iced::widget::{Column, Container, Grid, Scrollable, Text};
use iced::{Color, Element, Length};

use crate::desktop_apps::DesktopApp;
use crate::ui::Message;
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
