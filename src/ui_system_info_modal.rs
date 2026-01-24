use iced::widget::{Column, Container, ProgressBar, Row, Scrollable, Space, Text};
use iced::{Color, Element, Length, Padding};

use crate::messages::Message;
use crate::system_info::GamingSystemInfo;
use crate::ui_theme::*;

pub fn render_system_info_modal<'a>(
    info: &'a Option<GamingSystemInfo>,
    scale: f32,
) -> Element<'a, Message> {
    let title = Text::new("System Information")
        .font(SANSATION)
        .size(scaled(36.0, scale))
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(Padding {
            top: scaled(BASE_PADDING_MEDIUM, scale),
            right: scaled(BASE_PADDING_MEDIUM, scale),
            bottom: scaled(BASE_PADDING_SMALL, scale),
            left: scaled(BASE_PADDING_MEDIUM, scale),
        })
        .width(Length::Fill)
        .center_x(Length::Fill);

    let content: Element<'a, Message> = if let Some(info) = info {
        let left_column = build_left_column(info, scale);
        let right_column = build_right_column(info, scale);

        let columns = Row::new()
            .push(
                Container::new(left_column)
                    .width(Length::FillPortion(1))
                    .padding(Padding {
                        top: 0.0,
                        right: scaled(BASE_PADDING_MEDIUM, scale),
                        bottom: 0.0,
                        left: 0.0,
                    }),
            )
            .push(
                Container::new(right_column)
                    .width(Length::FillPortion(1))
                    .padding(Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: scaled(BASE_PADDING_MEDIUM, scale),
                    }),
            )
            .spacing(scaled(50.0, scale));

        Scrollable::new(columns)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        Container::new(
            Text::new("Loading System Information...")
                .font(SANSATION)
                .size(scaled(BASE_FONT_XLARGE, scale))
                .color(COLOR_TEXT_DIM),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    };

    let hint = Text::new("Press B or − to close")
        .font(SANSATION)
        .size(scaled(BASE_FONT_MEDIUM, scale))
        .color(COLOR_TEXT_HINT);

    let hint_container = Container::new(hint)
        .padding(scaled(BASE_PADDING_SMALL, scale))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let modal_column = Column::new()
        .push(title_container)
        .push(content)
        .push(hint_container)
        .spacing(scaled(BASE_PADDING_SMALL, scale));

    let border_radius = scaled(12.0, scale);
    let modal_box = Container::new(modal_column)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(scaled(25.0, scale))
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
        .padding(scaled(MODAL_OVERLAY_PADDING, scale))
        .style(|_| iced::widget::container::Style {
            background: Some(COLOR_OVERLAY_STRONG.into()),
            ..Default::default()
        })
        .into()
}

fn build_left_column(info: &GamingSystemInfo, scale: f32) -> Element<'_, Message> {
    let mut column = Column::new().spacing(scaled(8.0, scale));

    column = column.push(section_header_accent("System", scale));
    column = column.push(info_row(
        "RhincoTV",
        env!("CARGO_PKG_VERSION").to_string(),
        scale,
    ));
    column = column.push(info_row("OS", info.os_name.clone(), scale));
    column = column.push(info_row("Kernel", info.kernel_version.clone(), scale));
    column = column.push(info_row("Session", info.xdg_session_type.clone(), scale));

    column = column.push(section_spacer(scale));

    column = column.push(section_header_accent("Hardware", scale));
    column = column.push(info_row("CPU", info.cpu_model.clone(), scale));

    let mem_label = format!("{} / {}", info.memory_used, info.memory_total);
    let mem_percent = parse_memory_percent(&info.memory_used, &info.memory_total);
    column = column.push(info_row_with_bar(
        "Memory".to_string(),
        mem_label,
        mem_percent,
        scale,
    ));

    column = column.push(info_row("GPU", info.gpu_info.clone(), scale));
    column = column.push(info_row("Driver", info.gpu_driver.clone(), scale));
    column = column.push(info_row("Vulkan", info.vulkan_info.clone(), scale));

    column = column.push(section_spacer(scale));

    column = column.push(section_header_accent("Storage", scale));

    if info.disks.is_empty() {
        column = column.push(
            Text::new("No disks found")
                .font(SANSATION)
                .size(scaled(17.0, scale))
                .color(COLOR_TEXT_DIM),
        );
    } else {
        for disk in &info.disks {
            let disk_label = format!("{}: {} / {}", disk.mount_point, disk.used, disk.size);
            let disk_percent = parse_percent(&disk.usage_percent);
            column = column.push(info_row_with_bar(
                disk_label,
                disk.usage_percent.clone(),
                disk_percent,
                scale,
            ));
        }
    }

    if info.zram.enabled {
        let zram_label = format!("ZRAM: {} ({})", info.zram.size, info.zram.algorithm);
        let zram_value = format!("{} used ({})", info.zram.used, info.zram.usage_percent);
        let zram_percent = parse_percent(&info.zram.usage_percent);
        column = column.push(info_row_with_bar(
            zram_label,
            zram_value,
            zram_percent,
            scale,
        ));
    } else {
        column = column.push(
            Text::new("ZRAM: Not Configured")
                .font(SANSATION)
                .size(scaled(17.0, scale))
                .color(COLOR_TEXT_DIM),
        );
    }

    column.into()
}

fn build_right_column(info: &GamingSystemInfo, scale: f32) -> Element<'_, Message> {
    let mut column = Column::new().spacing(scaled(8.0, scale));

    column = column.push(section_header_accent("Gaming Tools", scale));

    let (gamemode_text, gamemode_ok) = if info.gamemode.available {
        if info.gamemode.active {
            ("Installed (Active)", true)
        } else {
            ("Installed (Inactive)", true)
        }
    } else {
        ("Not Installed", false)
    };
    column = column.push(info_row_with_status(
        "GameMode".to_string(),
        gamemode_text.to_string(),
        gamemode_ok,
        scale,
    ));

    if info.wine_versions.is_empty() {
        column = column.push(info_row_colored(
            "Wine",
            "Not Installed".to_string(),
            COLOR_TEXT_DIM,
            scale,
        ));
    } else {
        for (name, version) in &info.wine_versions {
            column = column.push(info_row(name, version.clone(), scale));
        }
    }

    if !info.proton_versions.is_empty() {
        column = column.push(Space::new().height(scaled_fixed(5.0, scale)));
        column = column.push(
            Text::new("Proton Versions")
                .font(SANSATION)
                .size(scaled(15.0, scale))
                .color(COLOR_TEXT_SOFT),
        );
        for (name, version) in &info.proton_versions {
            column = column.push(
                Text::new(format!("  {} — {}", name, version))
                    .font(SANSATION)
                    .size(scaled(BASE_FONT_MEDIUM, scale))
                    .color(COLOR_TEXT_BRIGHT),
            );
        }
    }

    column = column.push(section_spacer(scale));

    column = column.push(section_header_accent("Kernel Tweaks", scale));

    let governor_ok = info.cpu_governor == "performance";
    column = column.push(info_row_with_status(
        "CPU Governor".to_string(),
        info.cpu_governor.clone(),
        governor_ok,
        scale,
    ));

    let map_count_display = format_large_number(info.kernel_tweaks.vm_max_map_count);
    column = column.push(info_row_with_status(
        "max_map_count".to_string(),
        map_count_display,
        info.kernel_tweaks.vm_max_map_count_ok,
        scale,
    ));

    let swappiness_str = info.kernel_tweaks.swappiness.to_string();
    column = column.push(info_row_with_status(
        "Swappiness".to_string(),
        swappiness_str,
        info.kernel_tweaks.swappiness_ok,
        scale,
    ));

    column = column.push(info_row_with_status(
        "Clocksource".to_string(),
        info.kernel_tweaks.clocksource.clone(),
        info.kernel_tweaks.clocksource_ok,
        scale,
    ));

    column = column.push(section_spacer(scale));

    column = column.push(section_header_accent("Controllers", scale));
    if info.controllers.is_empty() {
        column = column.push(
            Text::new("No controllers detected")
                .font(SANSATION)
                .size(scaled(17.0, scale))
                .color(COLOR_TEXT_DIM),
        );
    } else {
        for controller in &info.controllers {
            column = column.push(
                Text::new(format!("{}  ({})", controller.name, controller.device_path))
                    .font(SANSATION)
                    .size(scaled(17.0, scale))
                    .color(COLOR_TEXT_BRIGHT),
            );
        }
    }

    column.into()
}

fn section_header_accent(title: &str, scale: f32) -> Element<'_, Message> {
    Text::new(title)
        .font(SANSATION)
        .size(scaled(BASE_FONT_XLARGE, scale))
        .color(COLOR_ACCENT)
        .into()
}

fn section_spacer(scale: f32) -> Element<'static, Message> {
    Space::new().height(scaled_fixed(15.0, scale)).into()
}

fn info_row(label: &str, value: String, scale: f32) -> Element<'_, Message> {
    Row::new()
        .push(
            Container::new(
                Text::new(label)
                    .font(SANSATION)
                    .size(scaled(17.0, scale))
                    .color(COLOR_TEXT_SOFT),
            )
            .width(scaled_fixed(130.0, scale)),
        )
        .push(
            Text::new(value)
                .font(SANSATION)
                .size(scaled(17.0, scale))
                .color(COLOR_TEXT_BRIGHT),
        )
        .spacing(scaled(BASE_PADDING_SMALL, scale))
        .into()
}

fn info_row_colored(label: &str, value: String, color: Color, scale: f32) -> Element<'_, Message> {
    Row::new()
        .push(
            Container::new(
                Text::new(label)
                    .font(SANSATION)
                    .size(scaled(17.0, scale))
                    .color(COLOR_TEXT_SOFT),
            )
            .width(scaled_fixed(100.0, scale)),
        )
        .push(
            Text::new(value)
                .font(SANSATION)
                .size(scaled(17.0, scale))
                .color(color),
        )
        .spacing(scaled(BASE_PADDING_SMALL, scale))
        .into()
}

fn info_row_with_status(
    label: String,
    value: String,
    ok: bool,
    scale: f32,
) -> Element<'static, Message> {
    let indicator = status_indicator(ok, scale);
    let color = if ok { COLOR_SUCCESS } else { COLOR_WARNING };

    Row::new()
        .push(indicator)
        .push(
            Container::new(
                Text::new(label)
                    .font(SANSATION)
                    .size(scaled(17.0, scale))
                    .color(COLOR_TEXT_SOFT),
            )
            .width(scaled_fixed(200.0, scale)),
        )
        .push(
            Text::new(value)
                .font(SANSATION)
                .size(scaled(17.0, scale))
                .color(color),
        )
        .spacing(scaled(8.0, scale))
        .into()
}

fn status_indicator(ok: bool, scale: f32) -> Element<'static, Message> {
    let (symbol, color) = if ok {
        ("●", COLOR_SUCCESS)
    } else {
        ("○", COLOR_WARNING)
    };

    Text::new(symbol)
        .font(SANSATION)
        .size(scaled(BASE_FONT_MEDIUM, scale))
        .color(color)
        .into()
}

fn info_row_with_bar(
    label: String,
    value: String,
    percent: f32,
    scale: f32,
) -> Element<'static, Message> {
    let bar_color = if percent > 90.0 {
        COLOR_WARNING
    } else {
        COLOR_ACCENT
    };

    let border_radius = scaled(3.0, scale);
    let bar = ProgressBar::new(0.0..=100.0, percent).style(move |_theme| {
        iced::widget::progress_bar::Style {
            background: COLOR_ABYSS_DARK.into(),
            bar: bar_color.into(),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: border_radius.into(),
            },
        }
    });

    Column::new()
        .push(
            Row::new()
                .push(
                    Text::new(label)
                        .font(SANSATION)
                        .size(scaled(17.0, scale))
                        .color(COLOR_TEXT_SOFT),
                )
                .push(Space::new().width(Length::Fill))
                .push(
                    Text::new(value)
                        .font(SANSATION)
                        .size(scaled(17.0, scale))
                        .color(COLOR_TEXT_BRIGHT),
                ),
        )
        .push(
            Container::new(bar)
                .width(Length::Fill)
                .height(scaled_fixed(6.0, scale)),
        )
        .spacing(scaled(3.0, scale))
        .into()
}

fn format_large_number(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn parse_percent(s: &str) -> f32 {
    s.trim_end_matches('%').parse::<f32>().unwrap_or(0.0)
}

fn parse_memory_percent(used: &str, total: &str) -> f32 {
    let used_bytes = parse_memory_to_bytes(used);
    let total_bytes = parse_memory_to_bytes(total);

    if total_bytes > 0.0 {
        ((used_bytes / total_bytes) * 100.0) as f32
    } else {
        0.0
    }
}

fn parse_memory_to_bytes(s: &str) -> f64 {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 2 {
        return 0.0;
    }

    let value: f64 = parts[0].parse().unwrap_or(0.0);
    let unit = parts[1].to_uppercase();

    match unit.as_str() {
        "B" => value,
        "KB" => value * 1024.0,
        "MB" => value * 1024.0 * 1024.0,
        "GB" => value * 1024.0 * 1024.0 * 1024.0,
        "TB" => value * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 0.0,
    }
}
