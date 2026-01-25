use iced::alignment::Horizontal;
use iced::widget::{button, Column, Container, Row, Text};
use iced::{Color, Element, Length};

use crate::auth_flow::{AuthFlow, AuthFlowState};
use crate::messages::Message;
use crate::ui_theme::*;
use crate::virtual_keyboard::VirtualKeyboard;

pub fn render_auth_dialog<'a>(
    flow: &AuthFlow,
    keyboard: &'a VirtualKeyboard,
    scale: f32,
) -> Element<'a, Message> {
    let title = Text::new("Authorization Required")
        .font(SANSATION)
        .size(scaled(BASE_FONT_HEADER, scale))
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(scaled(BASE_PADDING_MEDIUM, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let message_text = Text::new(flow.message.clone())
        .font(SANSATION)
        .size(scaled(BASE_FONT_LARGE, scale))
        .color(COLOR_TEXT_BRIGHT)
        .align_x(Horizontal::Center);

    let message_container = Container::new(message_text)
        .padding(scaled(BASE_PADDING_SMALL, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let mut content_column = Column::new()
        .spacing(scaled(BASE_PADDING_SMALL, scale))
        .push(title_container)
        .push(message_container);

    match &flow.state {
        AuthFlowState::AwaitingPassword { prompt } => {
            let prompt_text = Text::new(prompt.clone())
                .font(SANSATION)
                .size(scaled(BASE_FONT_MEDIUM, scale))
                .color(COLOR_TEXT_MUTED)
                .align_x(Horizontal::Center);

            let prompt_container = Container::new(prompt_text)
                .padding(scaled(BASE_PADDING_TINY, scale))
                .width(Length::Fill)
                .center_x(Length::Fill);

            let password_box = Container::new(
                Text::new(keyboard.display_value())
                    .font(SANSATION)
                    .size(scaled(BASE_FONT_TITLE, scale))
                    .color(COLOR_TEXT_BRIGHT)
                    .align_x(Horizontal::Center),
            )
            .padding(scaled(BASE_PADDING_SMALL, scale))
            .width(scaled_fixed(MODAL_WIDTH_MEDIUM, scale))
            .center_x(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(COLOR_PANEL.into()),
                border: iced::Border {
                    color: Color::WHITE,
                    width: 1.0,
                    radius: scaled(6.0, scale).into(),
                },
                ..Default::default()
            });

            let keyboard_view = keyboard.view(scale).map(Message::AuthKeyboard);

            content_column = content_column
                .push(prompt_container)
                .push(Container::new(password_box).center_x(Length::Fill))
                .push(Container::new(keyboard_view).center_x(Length::Fill))
                .push(action_hint("Select OK to submit", scale))
                .push(button_row_password(scale));
        }
        AuthFlowState::Verifying => {
            content_column = content_column
                .push(action_hint("Verifying...", scale))
                .push(button_row_cancel(scale));
        }
        AuthFlowState::Failed { message } => {
            let error_text = Text::new(message.clone())
                .font(SANSATION)
                .size(scaled(BASE_FONT_MEDIUM, scale))
                .color(COLOR_ERROR)
                .align_x(Horizontal::Center);

            let error_container = Container::new(error_text)
                .padding(scaled(BASE_PADDING_SMALL, scale))
                .width(Length::Fill)
                .center_x(Length::Fill);

            content_column = content_column
                .push(error_container)
                .push(action_hint("Press B to cancel", scale))
                .push(button_row_cancel(scale));
        }
        AuthFlowState::Success => {
            content_column = content_column.push(action_hint("Authorized", scale));
        }
    }

    let border_radius = scaled(10.0, scale);
    let modal_box = Container::new(content_column)
        .width(scaled_fixed(MODAL_WIDTH_LARGE, scale))
        .height(Length::Shrink)
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

fn action_hint<'a>(text_value: &'a str, scale: f32) -> Element<'a, Message> {
    Text::new(text_value)
        .font(SANSATION)
        .size(scaled(BASE_FONT_SMALL, scale))
        .color(COLOR_TEXT_HINT)
        .into()
}

fn button_row_password<'a>(scale: f32) -> Element<'a, Message> {
    action_buttons(
        "Submit",
        Message::AuthSubmit,
        "Cancel",
        Message::AuthCancel,
        scale,
    )
}

fn button_row_cancel<'a>(scale: f32) -> Element<'a, Message> {
    let row = Row::new()
        .spacing(scaled(BASE_PADDING_MEDIUM, scale))
        .push(modal_button("Cancel", Message::AuthCancel, scale));

    Container::new(row)
        .padding(scaled(BASE_PADDING_SMALL, scale))
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

fn action_buttons<'a>(
    left_label: &'a str,
    left_msg: Message,
    right_label: &'a str,
    right_msg: Message,
    scale: f32,
) -> Element<'a, Message> {
    let row = Row::new()
        .spacing(scaled(BASE_PADDING_MEDIUM, scale))
        .push(modal_button(left_label, left_msg, scale))
        .push(modal_button(right_label, right_msg, scale));

    Container::new(row)
        .padding(scaled(BASE_PADDING_SMALL, scale))
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

fn modal_button<'a>(label: &'a str, message: Message, scale: f32) -> Element<'a, Message> {
    let text: Text<'a> = Text::new(label)
        .font(SANSATION)
        .size(scaled(BASE_FONT_LARGE, scale))
        .color(Color::WHITE)
        .align_x(Horizontal::Center);

    let border_radius = scaled(8.0, scale);
    let content: Container<'a, Message> = Container::new(text)
        .padding(scaled(12.0, scale))
        .width(scaled_fixed(160.0, scale))
        .center_x(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(COLOR_ACCENT.into()),
            text_color: Some(Color::WHITE),
            border: iced::Border {
                color: Color::WHITE,
                width: 1.0,
                radius: border_radius.into(),
            },
            ..Default::default()
        });

    button(content)
        .padding(0)
        .style(|_, _| button::Style::default())
        .on_press(message)
        .into()
}
