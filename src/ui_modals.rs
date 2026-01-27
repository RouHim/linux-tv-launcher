use iced::alignment::Horizontal;
use iced::widget::{Column, Container, Row, Scrollable, Text};
use iced::{Color, Element, Length};
use iced_anim::{spring::Motion, AnimationBuilder};

use crate::messages::Message;
use crate::model::Category;
use crate::ui_theme::*;

pub fn render_context_menu<'a>(
    selected_index: usize,
    category: Category,
    scale: f32,
) -> Element<'a, Message> {
    let menu_items: Vec<&str> = match category {
        Category::Apps => vec!["Launch", "Remove Entry", "Quit Launcher", "Close"],
        Category::Games | Category::System => vec!["Launch", "Quit Launcher", "Close"],
    };
    let mut column = Column::new()
        .spacing(scaled(BASE_PADDING_SMALL, scale))
        .padding(scaled(BASE_PADDING_MEDIUM, scale));

    for (i, item) in menu_items.iter().enumerate() {
        let is_selected = i == selected_index;
        let target_bg = if is_selected {
            COLOR_ACCENT
        } else {
            Color::TRANSPARENT
        };
        let target_text = if is_selected {
            Color::WHITE
        } else {
            COLOR_TEXT_MUTED
        };

        let item_text = item.to_string();

        let animated_item: Element<'a, Message> =
            AnimationBuilder::new((target_bg, target_text), move |(bg_color, txt_color)| {
                let text = Text::new(item_text.clone())
                    .font(SANSATION)
                    .size(scaled(BASE_FONT_XLARGE, scale))
                    .color(txt_color)
                    .align_x(Horizontal::Center);

                Container::new(text)
                    .padding(scaled(BASE_PADDING_SMALL, scale))
                    .width(Length::Fill)
                    .style(move |_| iced::widget::container::Style {
                        background: Some(bg_color.into()),
                        text_color: Some(txt_color),
                        ..Default::default()
                    })
                    .into()
            })
            .animation(Motion::SNAPPY)
            .into();

        column = column.push(animated_item);
    }

    let border_radius = scaled(10.0, scale);
    let menu_box = Container::new(column)
        .width(scaled_fixed(MODAL_WIDTH_CONTEXT_MENU, scale))
        .style(move |_| iced::widget::container::Style {
            background: Some(COLOR_MENU_BACKGROUND.into()),
            border: iced::Border {
                color: Color::WHITE,
                width: 1.0,
                radius: border_radius.into(),
            },
            ..Default::default()
        });

    Container::new(menu_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(COLOR_OVERLAY.into()),
            ..Default::default()
        })
        .into()
}

pub fn render_help_modal<'a>(scale: f32) -> Element<'a, Message> {
    let title = Text::new("Controller Bindings")
        .font(SANSATION)
        .size(scaled(BASE_FONT_HEADER, scale))
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(scaled(BASE_PADDING_MEDIUM, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let gamepad_bindings = vec![
        ("A / South", "Select / Confirm"),
        ("B / East", "Back / Cancel"),
        ("X / West", "Context Menu"),
        ("Y / North", "Add App (in Apps)"),
        ("D-Pad / Left Stick", "Navigate"),
        ("LB / LT", "Previous Category"),
        ("RB / RT", "Next Category"),
        ("− / Select", "Show/Hide Controls"),
    ];

    let keyboard_bindings = vec![
        ("Arrow Keys", "Navigate"),
        ("Enter", "Select / Confirm"),
        ("Escape", "Back / Cancel"),
        ("Tab", "Next Category"),
        ("C", "Context Menu"),
        ("+ / A", "Add App (in Apps)"),
        ("−", "Show/Hide Controls"),
        ("F4", "Quit Launcher"),
    ];

    let mut content_column = Column::new().spacing(scaled(8.0, scale));

    content_column = content_column.push(
        Text::new("Gamepad")
            .font(SANSATION)
            .size(scaled(BASE_FONT_LARGE, scale))
            .color(COLOR_TEXT_SOFT),
    );

    for (button, action) in gamepad_bindings {
        let row = Row::new()
            .push(
                Container::new(
                    Text::new(button)
                        .font(SANSATION)
                        .size(scaled(BASE_FONT_MEDIUM, scale))
                        .color(COLOR_TEXT_BRIGHT),
                )
                .width(scaled_fixed(200.0, scale)),
            )
            .push(
                Text::new(action)
                    .font(SANSATION)
                    .size(scaled(BASE_FONT_MEDIUM, scale))
                    .color(COLOR_TEXT_MUTED),
            )
            .spacing(scaled(BASE_PADDING_MEDIUM, scale));
        content_column = content_column.push(row);
    }

    content_column =
        content_column.push(Container::new(Text::new("")).height(scaled_fixed(16.0, scale)));

    content_column = content_column.push(
        Text::new("Keyboard")
            .font(SANSATION)
            .size(scaled(BASE_FONT_LARGE, scale))
            .color(COLOR_TEXT_SOFT),
    );

    for (key, action) in keyboard_bindings {
        let row = Row::new()
            .push(
                Container::new(
                    Text::new(key)
                        .font(SANSATION)
                        .size(scaled(BASE_FONT_MEDIUM, scale))
                        .color(COLOR_TEXT_BRIGHT),
                )
                .width(scaled_fixed(200.0, scale)),
            )
            .push(
                Text::new(action)
                    .font(SANSATION)
                    .size(scaled(BASE_FONT_MEDIUM, scale))
                    .color(COLOR_TEXT_MUTED),
            )
            .spacing(scaled(BASE_PADDING_MEDIUM, scale));
        content_column = content_column.push(row);
    }

    let scrollable_content = Scrollable::new(content_column)
        .width(Length::Fill)
        .height(Length::Fill);

    let hint = Text::new("Press B or − to close")
        .font(SANSATION)
        .size(scaled(BASE_FONT_SMALL, scale))
        .color(COLOR_TEXT_HINT);

    let hint_container = Container::new(hint)
        .padding(scaled(BASE_PADDING_SMALL, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let modal_column = Column::new()
        .push(title_container)
        .push(scrollable_content)
        .push(hint_container)
        .spacing(scaled(BASE_PADDING_SMALL, scale));

    let border_radius = scaled(10.0, scale);
    let modal_box = Container::new(modal_column)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(scaled(BASE_PADDING_MEDIUM, scale))
        .style(move |_| iced::widget::container::Style {
            background: Some(COLOR_PANEL.into()),
            border: iced::Border {
                color: Color::WHITE,
                width: 1.0,
                radius: border_radius.into(),
            },
            ..Default::default()
        });

    Container::new(modal_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(scaled(MODAL_HELP_PADDING, scale))
        .style(|_| iced::widget::container::Style {
            background: Some(COLOR_OVERLAY_STRONG.into()),
            ..Default::default()
        })
        .into()
}

pub fn render_app_not_found_modal<'a>(
    item_name: &str,
    selected_index: usize,
    scale: f32,
) -> Element<'a, Message> {
    let title = Text::new("App Not Found")
        .font(SANSATION)
        .size(scaled(26.0, scale))
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(scaled(BASE_PADDING_SMALL, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let message = Text::new(format!(
        "{} is no longer installed. Remove it from your list?",
        item_name
    ))
    .font(SANSATION)
    .size(scaled(BASE_FONT_LARGE, scale))
    .color(COLOR_TEXT_BRIGHT)
    .align_x(Horizontal::Center);

    let message_container = Container::new(message)
        .padding(scaled(BASE_PADDING_SMALL, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let options = ["Remove", "Cancel"];

    let options_row = Row::with_children(
        options
            .iter()
            .enumerate()
            .map(|(index, &label)| modal_button(label, index == selected_index, scale)),
    )
    .spacing(scaled(BASE_PADDING_MEDIUM, scale));

    let options_container = Container::new(options_row)
        .padding(scaled(BASE_PADDING_SMALL, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let modal_column = Column::new()
        .push(title_container)
        .push(message_container)
        .push(options_container)
        .spacing(scaled(BASE_PADDING_SMALL, scale));

    let border_radius = scaled(10.0, scale);
    let modal_box = Container::new(modal_column)
        .width(scaled_fixed(MODAL_WIDTH_MEDIUM, scale))
        .padding(scaled(BASE_PADDING_MEDIUM, scale))
        .style(move |_| iced::widget::container::Style {
            background: Some(COLOR_PANEL.into()),
            border: iced::Border {
                color: Color::WHITE,
                width: 1.0,
                radius: border_radius.into(),
            },
            ..Default::default()
        });

    Container::new(modal_box)
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

fn modal_button<'a>(label: &'a str, is_selected: bool, scale: f32) -> Element<'a, Message> {
    let text = Text::new(label)
        .font(SANSATION)
        .size(scaled(BASE_FONT_LARGE, scale))
        .color(if is_selected {
            Color::WHITE
        } else {
            COLOR_TEXT_MUTED
        })
        .align_x(Horizontal::Center);

    let border_radius = scaled(8.0, scale);
    Container::new(text)
        .padding(scaled(12.0, scale))
        .width(scaled_fixed(140.0, scale))
        .center_x(Length::Fill)
        .style(move |_| {
            if is_selected {
                iced::widget::container::Style {
                    background: Some(COLOR_ACCENT.into()),
                    text_color: Some(Color::WHITE),
                    border: iced::Border {
                        color: Color::WHITE,
                        width: 1.0,
                        radius: border_radius.into(),
                    },
                    ..Default::default()
                }
            } else {
                iced::widget::container::Style {
                    background: Some(COLOR_PANEL.into()),
                    text_color: Some(COLOR_TEXT_MUTED),
                    border: iced::Border {
                        color: COLOR_TEXT_MUTED,
                        width: 1.0,
                        radius: border_radius.into(),
                    },
                    ..Default::default()
                }
            }
        })
        .into()
}
