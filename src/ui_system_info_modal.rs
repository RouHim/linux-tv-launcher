use iced::widget::{Column, Container, Row, Scrollable, Space, Text};
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
        // Wine versions list
        column = column.push(
            Text::new("Wine Versions")
                .font(SANSATION)
                .size(18)
                .color(COLOR_TEXT_SOFT),
        );

        let mut wine_col = Column::new().spacing(5).padding(10);
        if info.wine_versions.is_empty() {
            wine_col = wine_col.push(
                Text::new("Not Installed")
                    .font(SANSATION)
                    .size(16)
                    .color(COLOR_TEXT_DIM),
            );
        } else {
            for (name, version) in &info.wine_versions {
                wine_col = wine_col.push(
                    Text::new(format!("{}: {}", name, version))
                        .font(SANSATION)
                        .size(16)
                        .color(COLOR_TEXT_BRIGHT),
                );
            }
        }
        column = column.push(Container::new(wine_col).padding(20)); // Indent

        // Proton versions list
        column = column.push(
            Text::new("Proton Versions")
                .font(SANSATION)
                .size(18)
                .color(COLOR_TEXT_SOFT),
        );

        let mut proton_col = Column::new().spacing(5).padding(10);
        if info.proton_versions.is_empty() {
            proton_col = proton_col.push(
                Text::new("None Found")
                    .font(SANSATION)
                    .size(16)
                    .color(COLOR_TEXT_DIM),
            );
        } else {
            for (name, version) in &info.proton_versions {
                proton_col = proton_col.push(
                    Text::new(format!("{}: {}", name, version))
                        .font(SANSATION)
                        .size(16)
                        .color(COLOR_TEXT_BRIGHT),
                );
            }
        }
        column = column.push(Container::new(proton_col).padding(20)); // Indent

        // Storage section
        column = column.push(
            Text::new("Storage")
                .font(SANSATION)
                .size(18)
                .color(COLOR_TEXT_SOFT),
        );

        let mut storage_col = Column::new().spacing(5).padding(10);
        if info.disks.is_empty() {
            storage_col = storage_col.push(
                Text::new("No disks found")
                    .font(SANSATION)
                    .size(16)
                    .color(COLOR_TEXT_DIM),
            );
        } else {
            for disk in &info.disks {
                storage_col = storage_col.push(
                    Text::new(format!(
                        "{}: {} / {} ({})",
                        disk.mount_point, disk.used, disk.size, disk.usage_percent
                    ))
                    .font(SANSATION)
                    .size(16)
                    .color(COLOR_TEXT_BRIGHT),
                );
            }
        }
        column = column.push(Container::new(storage_col).padding(20));

        // ZRAM section
        column = column.push(
            Text::new("ZRAM")
                .font(SANSATION)
                .size(18)
                .color(COLOR_TEXT_SOFT),
        );

        let zram_text = if info.zram.enabled {
            format!(
                "Enabled - {} ({}) - {} used ({})",
                info.zram.size, info.zram.algorithm, info.zram.used, info.zram.usage_percent
            )
        } else {
            "Not Configured".to_string()
        };

        let zram_col = Column::new().spacing(5).padding(10).push(
            Text::new(zram_text)
                .font(SANSATION)
                .size(16)
                .color(if info.zram.enabled {
                    COLOR_TEXT_BRIGHT
                } else {
                    COLOR_TEXT_DIM
                }),
        );
        column = column.push(Container::new(zram_col).padding(20));

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
        .width(Length::FillPortion(2))
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

    let layout = Row::new()
        .push(Space::new().width(Length::FillPortion(1)))
        .push(modal_box)
        .push(Space::new().width(Length::FillPortion(1)))
        .width(Length::Fill);

    // Overlay container
    Container::new(layout)
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
