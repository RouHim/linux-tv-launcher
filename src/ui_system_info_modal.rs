use iced::widget::{Column, Container, Row, Scrollable, Text};
use iced::{Color, Element, Length};

use crate::messages::Message;
use crate::system_info::GamingSystemInfo;
use crate::ui_theme::*;

pub fn render_system_info_modal<'a>(info: &'a Option<GamingSystemInfo>) -> Element<'a, Message> {
    let title = Text::new("System Information")
        .font(SANSATION)
        .size(28)
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(20)
        .width(Length::Fill)
        .center_x(Length::Fill);

    let content: Element<'a, Message> = if let Some(info) = info {
        let mut column = Column::new().spacing(15);

        column = column.push(info_row("OS", info.os_name.clone()));
        column = column.push(info_row("Kernel", info.kernel_version.clone()));
        column = column.push(info_row("CPU", info.cpu_model.clone()));
        column = column.push(info_row(
            "Memory",
            format!("{} / {}", info.memory_used, info.memory_total),
        ));
        column = column.push(info_row("GPU", info.gpu_info.clone()));
        column = column.push(info_row("Driver", info.gpu_driver.clone()));
        column = column.push(info_row("Vulkan", info.vulkan_info.clone()));
        column = column.push(info_row("Session", info.xdg_session_type.clone()));
        column = column.push(info_row("Wine", info.wine_version.clone()));

        // Proton versions list
        column = column.push(
            Text::new("Proton Versions")
                .font(SANSATION)
                .size(18)
                .color(COLOR_TEXT_SOFT),
        );

        let mut proton_col = Column::new().spacing(5).padding(10);
        for proton in &info.proton_versions {
            proton_col = proton_col.push(
                Text::new(proton)
                    .font(SANSATION)
                    .size(16)
                    .color(COLOR_TEXT_BRIGHT),
            );
        }
        column = column.push(Container::new(proton_col).padding(20)); // Indent

        Scrollable::new(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        Container::new(
            Text::new("Loading System Information...")
                .font(SANSATION)
                .size(20)
                .color(COLOR_TEXT_DIM),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    };

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
        .push(content)
        .push(hint_container)
        .spacing(10);

    // Modal box
    let modal_box = Container::new(modal_column)
        .width(Length::Fixed(800.0))
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

    // Overlay container
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

fn info_row<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    Row::new()
        .push(
            Container::new(
                Text::new(label)
                    .font(SANSATION)
                    .size(16)
                    .color(COLOR_TEXT_SOFT),
            )
            .width(Length::Fixed(120.0)),
        )
        .push(
            Text::new(value)
                .font(SANSATION)
                .size(16)
                .color(COLOR_TEXT_BRIGHT)
                .width(Length::Fill),
        )
        .spacing(10)
        .into()
}
