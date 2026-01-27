use iced::widget::{Column, Container, Row, Text};
use iced::{Color, Element, Length};

use crate::input::Action;
use crate::messages::Message;
use crate::ui_state::{AppUpdatePhase, AppUpdateState};
use crate::ui_theme::*;

const SPINNER_CHARS: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub fn render_app_update_modal<'a>(state: &'a AppUpdateState, scale: f32) -> Element<'a, Message> {
    let spinner = SPINNER_CHARS[state.spinner_tick % SPINNER_CHARS.len()];

    let title = Text::new("App Update")
        .font(SANSATION)
        .size(scaled(BASE_FONT_HEADER, scale))
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(scaled(BASE_PADDING_MEDIUM, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let (status_label, status_color) = match state.phase {
        AppUpdatePhase::Prompt => (
            format!("Update available: v{}", state.release.version),
            COLOR_TEXT_BRIGHT,
        ),
        AppUpdatePhase::Updating => (
            "Downloading and installing...".to_string(),
            COLOR_TEXT_BRIGHT,
        ),
        AppUpdatePhase::Completed => ("Update complete. Restarting...".to_string(), COLOR_SUCCESS),
        AppUpdatePhase::Failed => ("Update failed".to_string(), COLOR_ERROR),
    };

    let status_row = Row::new()
        .spacing(scaled(16.0, scale))
        .align_y(iced::Alignment::Center)
        .push(
            Text::new(match state.phase {
                AppUpdatePhase::Updating => spinner.to_string(),
                AppUpdatePhase::Completed => "✓".to_string(),
                AppUpdatePhase::Failed => "✗".to_string(),
                AppUpdatePhase::Prompt => "↑".to_string(),
            })
            .font(SANSATION)
            .size(scaled(BASE_FONT_DISPLAY, scale))
            .color(status_color),
        )
        .push(
            Text::new(status_label)
                .font(SANSATION)
                .size(scaled(22.0, scale))
                .color(status_color),
        );

    let mut body_column = Column::new()
        .spacing(scaled(16.0, scale))
        .align_x(iced::Alignment::Center);

    body_column = body_column.push(status_row);

    if state.phase == AppUpdatePhase::Prompt {
        let body = if state.release.body.trim().is_empty() {
            "No release notes provided.".to_string()
        } else {
            state.release.body.clone()
        };

        body_column = body_column.push(
            Container::new(
                Text::new(body)
                    .font(SANSATION)
                    .size(scaled(BASE_FONT_MEDIUM, scale))
                    .color(COLOR_TEXT_MUTED),
            )
            .width(Length::Fill)
            .padding(scaled(BASE_PADDING_SMALL, scale)),
        );
    }

    if let Some(message) = &state.status_message {
        body_column = body_column.push(
            Container::new(
                Text::new(message)
                    .font(SANSATION)
                    .size(scaled(BASE_FONT_MEDIUM, scale))
                    .color(COLOR_TEXT_MUTED),
            )
            .padding(scaled(BASE_PADDING_SMALL, scale))
            .center_x(Length::Fill),
        );
    }

    let hint_text = match state.phase {
        AppUpdatePhase::Prompt => "Press Enter/A to update, or Esc/B to skip",
        AppUpdatePhase::Updating => "Updating...",
        AppUpdatePhase::Completed => "Restarting...",
        AppUpdatePhase::Failed => "Press B or Esc to close",
    };

    let hint = Text::new(hint_text)
        .font(SANSATION)
        .size(scaled(BASE_FONT_SMALL, scale))
        .color(COLOR_TEXT_HINT);

    let hint_container = Container::new(hint)
        .padding(scaled(BASE_PADDING_SMALL, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let modal_column = Column::new()
        .push(title_container)
        .push(
            Container::new(body_column)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .push(hint_container)
        .spacing(scaled(BASE_PADDING_SMALL, scale));

    let border_radius = scaled(10.0, scale);
    let modal_box = Container::new(modal_column)
        .width(scaled_fixed(MODAL_WIDTH_LARGE, scale))
        .height(scaled_fixed(MODAL_HEIGHT_MEDIUM, scale))
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
            background: Some(Color::TRANSPARENT.into()),
            ..Default::default()
        })
        .into()
}

pub fn handle_app_update_navigation(state: &AppUpdateState, action: Action) -> Option<Message> {
    match state.phase {
        AppUpdatePhase::Prompt => match action {
            Action::Select => Some(Message::StartAppUpdate),
            Action::Back | Action::ShowHelp => Some(Message::CloseAppUpdateModal),
            _ => None,
        },
        AppUpdatePhase::Updating => None,
        AppUpdatePhase::Failed => match action {
            Action::Back | Action::ShowHelp | Action::Select => Some(Message::CloseAppUpdateModal),
            _ => None,
        },
        _ => None,
    }
}
