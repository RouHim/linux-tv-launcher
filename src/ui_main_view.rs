use iced::alignment::Horizontal;
use iced::widget::{scrollable, Column, Container, Row, Scrollable, Text};
use iced::{Background, Border, Color, Element, Length, Shadow};
use std::path::PathBuf;

use crate::category_list::CategoryList;
use crate::icons;
use crate::messages::Message;
use crate::model::{Category, LauncherItem, SystemIcon};
use crate::ui_components::render_icon;
use crate::ui_theme::*;

pub fn get_category_dimensions(category: Category) -> (f32, f32, f32, f32) {
    match category {
        Category::Games => (
            GAME_POSTER_WIDTH + 16.0,
            GAME_POSTER_HEIGHT + 140.0,
            GAME_POSTER_WIDTH,
            GAME_POSTER_HEIGHT,
        ),
        _ => (ICON_ITEM_WIDTH, ICON_ITEM_HEIGHT, ICON_SIZE, ICON_SIZE),
    }
}

pub fn render_section_row<'a>(
    active_category: Category,
    target_category: Category,
    list: &'a CategoryList,
    empty_msg: String,
    default_icon_handle: Option<iced::widget::svg::Handle>,
) -> Element<'a, Message> {
    let is_active = active_category == target_category;
    let selected_index = if is_active { list.selected_index } else { 0 };

    let title = Text::new(target_category.title())
        .font(SANSATION)
        .size(24)
        .color(if is_active {
            Color::WHITE
        } else {
            COLOR_TEXT_DIM
        });

    let (item_width, item_height, image_width, image_height) =
        get_category_dimensions(target_category);

    let content: Element<'_, Message> = if list.items.is_empty() {
        Container::new(Text::new(empty_msg).font(SANSATION).color(COLOR_TEXT_DIM))
            .height(Length::Fixed(item_height))
            .center_y(Length::Fixed(item_height))
            .padding(20)
            .into()
    } else {
        let mut row = Row::new().spacing(10);

        for (i, item) in list.items.iter().enumerate() {
            let is_selected = is_active && (i == selected_index);

            row = row.push(render_item(
                item,
                is_selected,
                image_width,
                image_height,
                item_width,
                item_height,
                default_icon_handle.clone(),
            ));
        }

        Scrollable::new(row)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
                    .spacing(16.0) // Add space between content and scrollbar
                    .width(8.0)
                    .scroller_width(6.0),
            ))
            .id(list.scroll_id.clone())
            .width(Length::Fill)
            .height(Length::Shrink)
            .style(|_theme, _status| {
                let scroller = scrollable::Scroller {
                    background: Background::Color(COLOR_ACCENT),
                    border: Border {
                        radius: 3.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                };
                let rail = scrollable::Rail {
                    background: Some(Background::Color(COLOR_PANEL)),
                    border: Border {
                        radius: 4.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                    scroller,
                };
                scrollable::Style {
                    container: iced::widget::container::Style::default(),
                    vertical_rail: rail,
                    horizontal_rail: rail,
                    gap: None,
                    auto_scroll: scrollable::AutoScroll {
                        background: Background::Color(COLOR_PANEL),
                        border: Border::default(),
                        shadow: Shadow::default(),
                        icon: Color::WHITE,
                    },
                }
            })
            .into()
    };

    Column::new()
        .push(title)
        .push(content)
        .spacing(10)
        .padding(10)
        .into()
}

fn render_item<'a>(
    item: &LauncherItem,
    is_selected: bool,
    image_width: f32,
    image_height: f32,
    item_width: f32,
    _item_height: f32,
    default_icon_handle: Option<iced::widget::svg::Handle>,
) -> Element<'a, Message> {
    let icon_widget: Element<'a, Message> = if let Some(sys_icon) = &item.system_icon {
        match sys_icon {
            SystemIcon::PowerOff => icons::power_off_icon(image_width),
            SystemIcon::Pause => icons::pause_icon(image_width),
            SystemIcon::ArrowsRotate => icons::arrows_rotate_icon(image_width),
            SystemIcon::ExitBracket => icons::exit_icon(image_width),
            SystemIcon::Info => icons::info_icon(image_width),
        }
    } else {
        render_icon(
            item.icon.as_ref().map(PathBuf::from),
            image_width,
            image_height,
            "ICON",
            None,
            default_icon_handle,
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

pub fn render_status<'a>(status_message: &'a Option<String>) -> Option<Element<'a, Message>> {
    let status = status_message.as_ref()?;
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

pub fn render_controls_hint<'a>() -> Element<'a, Message> {
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
