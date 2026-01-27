use iced::alignment::Horizontal;
use iced::widget::{scrollable, text, Column, Container, Row, Scrollable, Text};
use iced::{Background, Border, Color, Element, Length, Shadow};
use iced_anim::{spring::Motion, AnimationBuilder};
use std::path::PathBuf;

use crate::category_list::CategoryList;
use crate::icons;
use crate::messages::Message;
use crate::model::{Category, LauncherItem, SystemIcon};
use crate::ui_components::render_icon;
use crate::ui_theme::*;

pub fn get_category_dimensions(category: Category, scale: f32) -> (f32, f32, f32, f32) {
    let (w, h, img_w, img_h) = match category {
        Category::Games => (
            GAME_POSTER_WIDTH + 16.0,
            GAME_POSTER_HEIGHT + 140.0,
            GAME_POSTER_WIDTH,
            GAME_POSTER_HEIGHT,
        ),
        _ => (ICON_ITEM_WIDTH, ICON_ITEM_HEIGHT, ICON_SIZE, ICON_SIZE),
    };

    (w * scale, h * scale, img_w * scale, img_h * scale)
}

pub fn render_section_row<'a>(
    active_category: Category,
    target_category: Category,
    list: &'a CategoryList,
    empty_msg: String,
    default_icon_handle: Option<iced::widget::svg::Handle>,
    scale: f32,
) -> Element<'a, Message> {
    let is_active = active_category == target_category;
    let selected_index = if is_active { list.selected_index } else { 0 };

    let target_color = if is_active {
        Color::WHITE
    } else {
        COLOR_TEXT_DIM
    };
    let title: Element<'a, Message> = AnimationBuilder::new(target_color, move |color| {
        Text::new(target_category.title())
            .font(SANSATION)
            .size(24.0 * scale)
            .color(color)
            .into()
    })
    .animation(Motion::SNAPPY)
    .into();

    let (item_width, item_height, image_width, image_height) =
        get_category_dimensions(target_category, scale);

    let content: Element<'_, Message> = if list.items.is_empty() {
        Container::new(
            Text::new(empty_msg)
                .font(SANSATION)
                .size(16.0 * scale)
                .color(COLOR_TEXT_DIM),
        )
        .height(Length::Fixed(item_height))
        .align_y(iced::alignment::Vertical::Center)
        .padding(20.0 * scale)
        .into()
    } else {
        let mut row = Row::new().spacing(ITEM_SPACING * scale);

        for (i, item) in list.items.iter().enumerate() {
            let is_selected = is_active && (i == selected_index);

            let dims = ItemDimensions {
                image_width,
                image_height,
                item_width,
            };
            row = row.push(render_item(
                item,
                is_selected,
                &dims,
                default_icon_handle.clone(),
                scale,
            ));
        }

        Scrollable::new(row)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
                    .spacing(16.0 * scale)
                    .width(8.0 * scale)
                    .scroller_width(6.0 * scale),
            ))
            .id(list.scroll_id.clone())
            .width(Length::Fill)
            .height(Length::Shrink)
            .style(|_theme, _status| {
                let scroller = scrollable::Scroller {
                    background: Background::Color(COLOR_ACCENT),
                    border: Border {
                        radius: 3.0.into(), // Border radius doesn't always need strict scaling
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
        .spacing(10.0 * scale)
        .padding(10.0 * scale)
        .into()
}

/// Item render dimensions bundled to reduce argument count.
pub struct ItemDimensions {
    pub image_width: f32,
    pub image_height: f32,
    pub item_width: f32,
}

#[allow(clippy::too_many_arguments)]
fn render_item<'a>(
    item: &LauncherItem,
    is_selected: bool,
    dims: &ItemDimensions,
    default_icon_handle: Option<iced::widget::svg::Handle>,
    scale: f32,
) -> Element<'a, Message> {
    let image_width = dims.image_width;
    let image_height = dims.image_height;
    let item_width = dims.item_width;

    let target = if is_selected {
        (1.0f32, 10.0f32)
    } else {
        (0.0f32, 0.0f32)
    };

    // Clone data needed inside the Fn closure (called multiple times during animation)
    let item_name = item.name.clone();
    let item_system_icon = item.system_icon;
    let item_icon = item.icon.clone();
    let default_icon = default_icon_handle.clone();

    AnimationBuilder::new(target, move |(border_alpha, shadow_blur)| {
        // Rebuild entire widget tree inside closure — Element is NOT Clone
        let icon_widget: Element<'_, Message> = if let Some(ref sys_icon) = item_system_icon {
            let icon_size = image_width * 0.6;
            let icon = match sys_icon {
                SystemIcon::PowerOff => icons::power_off_icon(icon_size),
                SystemIcon::Pause => icons::pause_icon(icon_size),
                SystemIcon::ArrowsRotate => icons::arrows_rotate_icon(icon_size),
                SystemIcon::ExitBracket => icons::exit_icon(icon_size),
                SystemIcon::Info => icons::info_icon(icon_size),
            };
            Container::new(icon)
                .width(Length::Fixed(image_width))
                .height(Length::Fixed(image_height))
                .align_x(Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .into()
        } else {
            render_icon(
                item_icon.as_ref().map(PathBuf::from),
                image_width,
                image_height,
                "ICON",
                None,
                default_icon.clone(),
            )
        };

        let icon_container = Container::new(icon_widget).padding(6.0 * scale);

        let label = Text::new(item_name.clone())
            .font(SANSATION)
            .width(Length::Fixed(item_width))
            .wrapping(text::Wrapping::Word)
            .align_x(Horizontal::Center)
            .color(Color::WHITE)
            .size(14.0 * scale);

        let content = Column::new()
            .push(icon_container)
            .push(label)
            .align_x(iced::Alignment::Center)
            .spacing(5.0 * scale);

        Container::new(content)
            .width(Length::Fixed(item_width))
            .height(Length::Shrink)
            .padding(6.0 * scale)
            .align_x(Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(move |_theme| iced::widget::container::Style {
                border: iced::Border {
                    color: Color {
                        r: COLOR_ACCENT.r,
                        g: COLOR_ACCENT.g,
                        b: COLOR_ACCENT.b,
                        a: border_alpha,
                    },
                    width: 1.0 * scale.max(1.0),
                    radius: (4.0 * scale).into(),
                },
                shadow: iced::Shadow {
                    color: Color {
                        r: COLOR_ACCENT.r,
                        g: COLOR_ACCENT.g,
                        b: COLOR_ACCENT.b,
                        a: border_alpha * 0.5,
                    },
                    offset: iced::Vector::ZERO,
                    blur_radius: shadow_blur * scale,
                },
                ..Default::default()
            })
            .into()
    })
    .animation(Motion::SNAPPY)
    .into()
}

pub fn render_status<'a>(
    status_message: &'a Option<String>,
    scale: f32,
) -> Option<Element<'a, Message>> {
    let status = status_message.as_ref()?;
    Some(
        Container::new(
            Text::new(status)
                .font(SANSATION)
                .size(16.0 * scale)
                .color(COLOR_STATUS_TEXT),
        )
        .padding(8.0 * scale)
        .style(|_theme| iced::widget::container::Style {
            background: Some(COLOR_STATUS_BACKGROUND.into()),
            text_color: Some(Color::WHITE),
            ..Default::default()
        })
        .into(),
    )
}

pub fn render_controls_hint<'a>(scale: f32) -> Element<'a, Message> {
    let hint = Text::new("Press  −  for controls")
        .font(SANSATION)
        .size(14.0 * scale)
        .color(COLOR_TEXT_DIM);

    Container::new(hint)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .padding(10.0 * scale)
        .into()
}
