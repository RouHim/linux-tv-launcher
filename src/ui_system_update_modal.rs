use iced::widget::{Column, Container, ProgressBar, Row, Text};
use iced::{Color, Element, Length};

use crate::messages::Message;
use crate::system_update_state::{SystemUpdateState, UpdateStatus};
use crate::ui_theme::*;

pub fn render_system_update_modal<'a>(state: &SystemUpdateState) -> Element<'a, Message> {
    let spinner_chars = ["◐", "◓", "◑", "◒"];
    let spinner = spinner_chars[state.spinner_tick % 4];

    let mut progress_bar_value: Option<f32> = None;

    let (icon_text, status_message, status_color) = match &state.status {
        UpdateStatus::Starting => (
            spinner.to_string(),
            "Preparing update...".to_string(),
            COLOR_TEXT_BRIGHT,
        ),
        UpdateStatus::SyncingDatabases => (
            spinner.to_string(),
            "Syncing databases...".to_string(),
            COLOR_TEXT_BRIGHT,
        ),
        UpdateStatus::CheckingUpdates => (
            spinner.to_string(),
            "Checking for updates...".to_string(),
            COLOR_TEXT_BRIGHT,
        ),
        UpdateStatus::Downloading { package } => {
            let msg = if let Some(pkg) = package {
                format!("Downloading: {}", pkg)
            } else {
                "Downloading packages...".to_string()
            };
            (spinner.to_string(), msg, COLOR_TEXT_BRIGHT)
        }
        UpdateStatus::Building { package } => (
            spinner.to_string(),
            format!("Building: {}", package),
            COLOR_TEXT_BRIGHT,
        ),
        UpdateStatus::Installing {
            current,
            total,
            package,
        } => {
            if *total > 0 {
                progress_bar_value = Some((*current as f32 / *total as f32) * 100.0);
            }
            (
                spinner.to_string(),
                format!("Installing {}/{} \n{}", current, total, package),
                COLOR_TEXT_BRIGHT,
            )
        }
        UpdateStatus::Completed { restart_required } => {
            if *restart_required {
                (
                    "✓".to_string(),
                    "Update complete. Restart required.".to_string(),
                    Color::from_rgb(0.2, 0.8, 0.2),
                )
            } else {
                (
                    "✓".to_string(),
                    "Update complete!".to_string(),
                    Color::from_rgb(0.2, 0.8, 0.2),
                )
            }
        }
        UpdateStatus::NoUpdates => (
            "✓".to_string(),
            "System is up to date".to_string(),
            Color::from_rgb(0.2, 0.8, 0.2),
        ),
        UpdateStatus::Failed(_) => (
            "✗".to_string(),
            "Update failed".to_string(),
            Color::from_rgb(0.8, 0.2, 0.2),
        ),
    };

    let title = Text::new("System Update")
        .font(SANSATION)
        .size(28)
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(20)
        .width(Length::Fill)
        .center_x(Length::Fill);

    let mut status_column = Column::new().spacing(20).align_x(iced::Alignment::Center);

    let status_row = Row::new()
        .spacing(20)
        .align_y(iced::Alignment::Center)
        .push(
            Text::new(icon_text)
                .font(SANSATION)
                .size(40)
                .color(status_color),
        )
        .push(
            Text::new(status_message)
                .font(SANSATION)
                .size(24)
                .color(status_color),
        );

    status_column = status_column.push(status_row);

    if let Some(value) = progress_bar_value {
        let bar = ProgressBar::new(0.0..=100.0, value).style(|_theme| {
            iced::widget::progress_bar::Style {
                background: COLOR_PANEL.into(),
                bar: COLOR_ACCENT.into(),
                border: iced::Border {
                    color: Color::WHITE,
                    width: 1.0,
                    radius: 5.0.into(),
                },
            }
        });

        status_column = status_column.push(
            Container::new(bar)
                .width(Length::Fixed(400.0))
                .height(Length::Fixed(10.0)),
        );
    }

    let content_container = Container::new(status_column)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    // Detail / Error message if failed
    let detail_text = if let UpdateStatus::Failed(msg) = &state.status {
        if msg.to_lowercase().contains("manual intervention") {
            Some(msg.clone())
        } else {
            Some(format!("{}\n\nManual intervention is required.", msg))
        }
    } else {
        None
    };

    let mut modal_column = Column::new().push(title_container).push(content_container);

    if let Some(msg) = detail_text {
        modal_column = modal_column.push(
            Container::new(
                Text::new(msg)
                    .font(SANSATION)
                    .size(16)
                    .color(COLOR_TEXT_MUTED),
            )
            .padding(10)
            .center_x(Length::Fill),
        );
    }

    // Hint at bottom
    let hint_text = match &state.status {
        UpdateStatus::Completed { restart_required } if *restart_required => {
            "Press Enter/A to Restart, or Esc/B to Postpone"
        }
        status if status.is_finished() => "Press B or Esc to close",
        UpdateStatus::Installing { .. } => "Installing... (Cannot cancel)",
        _ => "Press B or Esc to Cancel",
    };

    let hint = Text::new(hint_text)
        .font(SANSATION)
        .size(14)
        .color(COLOR_TEXT_HINT);

    let hint_container = Container::new(hint)
        .padding(10)
        .width(Length::Fill)
        .center_x(Length::Fill);

    modal_column = modal_column.push(hint_container);

    // Modal box
    let modal_box = Container::new(modal_column)
        .width(Length::Fixed(500.0))
        .height(Length::Fixed(300.0))
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
