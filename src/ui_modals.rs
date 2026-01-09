use iced::alignment::Horizontal;
use iced::widget::{Column, Container, Row, Scrollable, Text};
use iced::{Color, Element, Length};

use crate::model::Category;
use crate::ui::Message;
use crate::ui_theme::*;

pub fn render_context_menu<'a>(selected_index: usize, category: Category) -> Element<'a, Message> {
    let menu_items: Vec<&str> = match category {
        Category::Apps => vec!["Launch", "Remove Entry", "Quit Launcher", "Close"],
        Category::Games | Category::System => vec!["Launch", "Quit Launcher", "Close"],
    };
    let mut column = Column::new().spacing(10).padding(20);

    for (i, item) in menu_items.iter().enumerate() {
        let is_selected = i == selected_index;
        let text = Text::new(*item)
            .font(SANSATION)
            .size(20)
            .color(if is_selected {
                Color::WHITE
            } else {
                COLOR_TEXT_MUTED
            })
            .align_x(Horizontal::Center);

        let container = Container::new(text)
            .padding(10)
            .width(Length::Fill)
            .style(move |_| {
                if is_selected {
                    iced::widget::container::Style {
                        background: Some(COLOR_ACCENT.into()),
                        text_color: Some(Color::WHITE),
                        ..Default::default()
                    }
                } else {
                    iced::widget::container::Style {
                        text_color: Some(COLOR_TEXT_MUTED),
                        ..Default::default()
                    }
                }
            });

        column = column.push(container);
    }

    let menu_box = Container::new(column)
        .width(Length::Fixed(300.0))
        .style(|_| iced::widget::container::Style {
            background: Some(COLOR_MENU_BACKGROUND.into()),
            border: iced::Border {
                color: Color::WHITE,
                width: 1.0,
                radius: 10.0.into(),
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

pub fn render_help_modal<'a>() -> Element<'a, Message> {
    let title = Text::new("Controller Bindings")
        .font(SANSATION)
        .size(28)
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(20)
        .width(Length::Fill)
        .center_x(Length::Fill);

    // Gamepad bindings
    let gamepad_bindings = vec![
        ("A / South", "Select / Confirm"),
        ("B / East", "Back / Cancel"),
        ("X / West", "Context Menu"),
        ("D-Pad / Left Stick", "Navigate"),
        ("LB / LT", "Previous Category"),
        ("RB / RT", "Next Category"),
        ("− / Select", "Show/Hide Controls"),
    ];

    // Keyboard bindings
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

    let mut content_column = Column::new().spacing(8);

    // Gamepad section header
    content_column = content_column.push(
        Text::new("Gamepad")
            .font(SANSATION)
            .size(18)
            .color(COLOR_TEXT_SOFT),
    );

    // Gamepad bindings
    for (button, action) in gamepad_bindings {
        let row = Row::new()
            .push(
                Container::new(
                    Text::new(button)
                        .font(SANSATION)
                        .size(16)
                        .color(COLOR_TEXT_BRIGHT),
                )
                .width(Length::Fixed(200.0)),
            )
            .push(
                Text::new(action)
                    .font(SANSATION)
                    .size(16)
                    .color(COLOR_TEXT_MUTED),
            )
            .spacing(20);
        content_column = content_column.push(row);
    }

    // Spacer
    content_column = content_column.push(Container::new(Text::new("")).height(Length::Fixed(16.0)));

    // Keyboard section header
    content_column = content_column.push(
        Text::new("Keyboard")
            .font(SANSATION)
            .size(18)
            .color(COLOR_TEXT_SOFT),
    );

    // Keyboard bindings
    for (key, action) in keyboard_bindings {
        let row = Row::new()
            .push(
                Container::new(
                    Text::new(key)
                        .font(SANSATION)
                        .size(16)
                        .color(COLOR_TEXT_BRIGHT),
                )
                .width(Length::Fixed(200.0)),
            )
            .push(
                Text::new(action)
                    .font(SANSATION)
                    .size(16)
                    .color(COLOR_TEXT_MUTED),
            )
            .spacing(20);
        content_column = content_column.push(row);
    }

    let scrollable_content = Scrollable::new(content_column)
        .width(Length::Fill)
        .height(Length::Fill);

    // Hint at bottom
    let hint = Text::new("Press B or − to close")
        .font(SANSATION)
        .size(14)
        .color(COLOR_TEXT_HINT);

    let hint_container = Container::new(hint)
        .padding(10)
        .width(Length::Fill)
        .center_x(Length::Fill);

    let modal_column = Column::new()
        .push(title_container)
        .push(scrollable_content)
        .push(hint_container)
        .spacing(10);

    // Modal box
    let modal_box = Container::new(modal_column)
        .width(Length::Fixed(500.0))
        .height(Length::FillPortion(70))
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

    // Overlay container with semi-transparent background
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
