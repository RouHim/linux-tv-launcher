use iced::widget::{Column, Container, ProgressBar, Row, Scrollable, Space, Text};
use iced::{Color, Element, Length, Padding};

use crate::messages::Message;
use crate::system_info::GamingSystemInfo;
use crate::ui_theme::*;

const COLOR_OK: Color = Color::from_rgb(0.4, 0.8, 0.4);
const COLOR_WARN: Color = Color::from_rgb(0.9, 0.7, 0.3);

pub fn render_system_info_modal<'a>(info: &'a Option<GamingSystemInfo>) -> Element<'a, Message> {
    let title = Text::new("System Information")
        .font(SANSATION)
        .size(36)
        .color(Color::WHITE);

    let title_container = Container::new(title)
        .padding(Padding {
            top: 20.0,
            right: 20.0,
            bottom: 10.0,
            left: 20.0,
        })
        .width(Length::Fill)
        .center_x(Length::Fill);

    let content: Element<'a, Message> = if let Some(info) = info {
        // === LEFT COLUMN ===
        let left_column = build_left_column(info);

        // === RIGHT COLUMN ===
        let right_column = build_right_column(info);

        // Two-column layout
        let columns = Row::new()
            .push(
                Container::new(left_column)
                    .width(Length::FillPortion(1))
                    .padding(Padding {
                        top: 0.0,
                        right: 20.0,
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
                        left: 20.0,
                    }),
            )
            .spacing(50);

        Scrollable::new(columns)
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
        .size(16)
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

    // Modal box - wider (~86% width)
    let modal_box = Container::new(modal_column)
        .width(Length::FillPortion(6))
        .height(Length::FillPortion(85))
        .padding(25)
        .style(|_| iced::widget::container::Style {
            background: Some(COLOR_PANEL.into()),
            border: iced::Border {
                color: Color::WHITE,
                width: 1.0,
                radius: 12.0.into(),
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

fn build_left_column(info: &GamingSystemInfo) -> Element<'_, Message> {
    let mut column = Column::new().spacing(8);

    // === SYSTEM SECTION ===
    column = column.push(section_header_accent("System"));
    column = column.push(info_row("OS", info.os_name.clone()));
    column = column.push(info_row("Kernel", info.kernel_version.clone()));
    column = column.push(info_row("Session", info.xdg_session_type.clone()));

    column = column.push(section_spacer());

    // === HARDWARE SECTION ===
    column = column.push(section_header_accent("Hardware"));
    column = column.push(info_row("CPU", info.cpu_model.clone()));

    // Memory with progress bar
    let mem_label = format!("{} / {}", info.memory_used, info.memory_total);
    let mem_percent = parse_memory_percent(&info.memory_used, &info.memory_total);
    column = column.push(info_row_with_bar(
        "Memory".to_string(),
        mem_label,
        mem_percent,
    ));

    column = column.push(info_row("GPU", info.gpu_info.clone()));
    column = column.push(info_row("Driver", info.gpu_driver.clone()));
    column = column.push(info_row("Vulkan", info.vulkan_info.clone()));

    column = column.push(section_spacer());

    // === STORAGE SECTION ===
    column = column.push(section_header_accent("Storage"));

    if info.disks.is_empty() {
        column = column.push(
            Text::new("No disks found")
                .font(SANSATION)
                .size(17)
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
            ));
        }
    }

    // ZRAM
    if info.zram.enabled {
        let zram_label = format!("ZRAM: {} ({})", info.zram.size, info.zram.algorithm);
        let zram_value = format!("{} used ({})", info.zram.used, info.zram.usage_percent);
        let zram_percent = parse_percent(&info.zram.usage_percent);
        column = column.push(info_row_with_bar(zram_label, zram_value, zram_percent));
    } else {
        column = column.push(
            Text::new("ZRAM: Not Configured")
                .font(SANSATION)
                .size(17)
                .color(COLOR_TEXT_DIM),
        );
    }

    column.into()
}

fn build_right_column(info: &GamingSystemInfo) -> Element<'_, Message> {
    let mut column = Column::new().spacing(8);

    // === GAMING TOOLS SECTION ===
    column = column.push(section_header_accent("Gaming Tools"));

    // GameMode with status indicator
    let (gamemode_text, gamemode_ok) = if info.gamemode.available {
        if info.gamemode.active {
            ("Installed (Active)", true)
        } else {
            ("Installed (Inactive)", false)
        }
    } else {
        ("Not Installed", false)
    };
    column = column.push(info_row_with_status(
        "GameMode".to_string(),
        gamemode_text.to_string(),
        gamemode_ok,
    ));

    // Wine versions
    if info.wine_versions.is_empty() {
        column = column.push(info_row_colored(
            "Wine",
            "Not Installed".to_string(),
            COLOR_TEXT_DIM,
        ));
    } else {
        for (name, version) in &info.wine_versions {
            column = column.push(info_row(name, version.clone()));
        }
    }

    // Proton versions
    if !info.proton_versions.is_empty() {
        column = column.push(Space::new().height(Length::Fixed(5.0)));
        column = column.push(
            Text::new("Proton Versions")
                .font(SANSATION)
                .size(15)
                .color(COLOR_TEXT_SOFT),
        );
        for (name, version) in &info.proton_versions {
            column = column.push(
                Text::new(format!("  {} — {}", name, version))
                    .font(SANSATION)
                    .size(16)
                    .color(COLOR_TEXT_BRIGHT),
            );
        }
    }

    column = column.push(section_spacer());

    // === KERNEL TWEAKS SECTION ===
    column = column.push(section_header_accent("Kernel Tweaks"));

    // CPU Governor
    let governor_ok = info.cpu_governor == "performance";
    column = column.push(info_row_with_status(
        "CPU Governor".to_string(),
        info.cpu_governor.clone(),
        governor_ok,
    ));

    // vm.max_map_count
    let map_count_display = format_large_number(info.kernel_tweaks.vm_max_map_count);
    column = column.push(info_row_with_status(
        "max_map_count".to_string(),
        map_count_display,
        info.kernel_tweaks.vm_max_map_count_ok,
    ));

    // Swappiness
    let swappiness_str = info.kernel_tweaks.swappiness.to_string();
    column = column.push(info_row_with_status(
        "Swappiness".to_string(),
        swappiness_str,
        info.kernel_tweaks.swappiness_ok,
    ));

    // Clocksource
    column = column.push(info_row_with_status(
        "Clocksource".to_string(),
        info.kernel_tweaks.clocksource.clone(),
        info.kernel_tweaks.clocksource_ok,
    ));

    column = column.push(section_spacer());

    // === CONTROLLERS SECTION ===
    column = column.push(section_header_accent("Controllers"));
    if info.controllers.is_empty() {
        column = column.push(
            Text::new("No controllers detected")
                .font(SANSATION)
                .size(17)
                .color(COLOR_TEXT_DIM),
        );
    } else {
        for controller in &info.controllers {
            column = column.push(
                Text::new(format!("{}  ({})", controller.name, controller.device_path))
                    .font(SANSATION)
                    .size(17)
                    .color(COLOR_TEXT_BRIGHT),
            );
        }
    }

    column.into()
}

// === HELPER FUNCTIONS ===

fn section_header_accent(title: &str) -> Element<'_, Message> {
    Text::new(title)
        .font(SANSATION)
        .size(20)
        .color(COLOR_ACCENT)
        .into()
}

fn section_spacer() -> Element<'static, Message> {
    Space::new().height(Length::Fixed(15.0)).into()
}

fn info_row(label: &str, value: String) -> Element<'_, Message> {
    Row::new()
        .push(
            Container::new(
                Text::new(label)
                    .font(SANSATION)
                    .size(17)
                    .color(COLOR_TEXT_SOFT),
            )
            .width(Length::Fixed(130.0)),
        )
        .push(
            Text::new(value)
                .font(SANSATION)
                .size(17)
                .color(COLOR_TEXT_BRIGHT),
        )
        .spacing(10)
        .into()
}

fn info_row_colored(label: &str, value: String, color: Color) -> Element<'_, Message> {
    Row::new()
        .push(
            Container::new(
                Text::new(label)
                    .font(SANSATION)
                    .size(17)
                    .color(COLOR_TEXT_SOFT),
            )
            .width(Length::Fixed(130.0)),
        )
        .push(Text::new(value).font(SANSATION).size(17).color(color))
        .spacing(10)
        .into()
}

fn info_row_with_status(label: String, value: String, ok: bool) -> Element<'static, Message> {
    let indicator = status_indicator(ok);
    let color = if ok { COLOR_OK } else { COLOR_WARN };

    Row::new()
        .push(indicator)
        .push(
            Container::new(
                Text::new(label)
                    .font(SANSATION)
                    .size(17)
                    .color(COLOR_TEXT_SOFT),
            )
            .width(Length::Fixed(120.0)),
        )
        .push(Text::new(value).font(SANSATION).size(17).color(color))
        .spacing(8)
        .into()
}

fn status_indicator(ok: bool) -> Element<'static, Message> {
    let (symbol, color) = if ok {
        ("●", COLOR_OK)
    } else {
        ("○", COLOR_WARN)
    };

    Text::new(symbol)
        .font(SANSATION)
        .size(16)
        .color(color)
        .into()
}

fn info_row_with_bar(label: String, value: String, percent: f32) -> Element<'static, Message> {
    let bar_color = if percent > 90.0 {
        COLOR_WARN
    } else {
        COLOR_ACCENT
    };

    let bar = ProgressBar::new(0.0..=100.0, percent).style(move |_theme| {
        iced::widget::progress_bar::Style {
            background: Color::from_rgb(0.2, 0.2, 0.2).into(),
            bar: bar_color.into(),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 3.0.into(),
            },
        }
    });

    Column::new()
        .push(
            Row::new()
                .push(
                    Text::new(label)
                        .font(SANSATION)
                        .size(17)
                        .color(COLOR_TEXT_SOFT),
                )
                .push(Space::new().width(Length::Fill))
                .push(
                    Text::new(value)
                        .font(SANSATION)
                        .size(17)
                        .color(COLOR_TEXT_BRIGHT),
                ),
        )
        .push(
            Container::new(bar)
                .width(Length::Fill)
                .height(Length::Fixed(6.0)),
        )
        .spacing(3)
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
    // Parse memory strings like "8.2 GB" into bytes for percentage calculation
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
