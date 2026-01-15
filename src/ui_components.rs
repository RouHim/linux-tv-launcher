use chrono::{DateTime, Local};
use gilrs::PowerInfo;
use iced::widget::{Container, Image, Row, Stack, Svg, Text};
use iced::{Alignment, Color, ContentFit, Element, Length};
use std::path::{Path, PathBuf};

use crate::gamepad::GamepadBattery;
use crate::icons;
use crate::ui_theme::{
    COLOR_BATTERY_CHARGING, COLOR_BATTERY_GOOD, COLOR_BATTERY_LOW, COLOR_BATTERY_MODERATE,
    COLOR_TEXT_BRIGHT, SANSATION,
};

fn is_svg(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
}

pub fn render_icon<'a, Message>(
    icon_path: Option<PathBuf>,
    width: f32,
    height: f32,
    fallback_text: &'static str,
    fallback_size: Option<u32>,
    default_icon_handle: Option<iced::widget::svg::Handle>,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    if let Some(path) = icon_path {
        return if is_svg(&path) {
            Svg::from_path(path)
                .width(Length::Fixed(width))
                .height(Length::Fixed(height))
                .into()
        } else {
            Image::new(path)
                .width(Length::Fixed(width))
                .height(Length::Fixed(height))
                .content_fit(ContentFit::Contain)
                .into()
        };
    }

    if let Some(handle) = default_icon_handle {
        return Svg::new(handle)
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .into();
    }

    let mut text = Text::new(fallback_text).font(SANSATION).color(Color::WHITE);
    if let Some(size) = fallback_size {
        text = text.size(size);
    }

    Container::new(text)
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

pub fn render_gamepad_batteries<'a, Message>(batteries: &[GamepadBattery]) -> Element<'a, Message>
where
    Message: 'a,
{
    let mut row = Row::new().spacing(24).align_y(Alignment::Center);

    for battery in batteries.iter().take(4) {
        if let Some((battery_icon, _color)) = get_battery_visuals(battery.power_info) {
            // Gamepad icon
            let gp_icon = icons::gamepad_icon(22.0, Color::WHITE);

            row = row.push(
                Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(gp_icon)
                    .push(battery_icon),
            );
        }
    }

    row.into()
}

fn get_battery_visuals<'a, Message>(power: PowerInfo) -> Option<(Element<'a, Message>, Color)>
where
    Message: 'a,
{
    match power {
        PowerInfo::Charged => {
            let color = COLOR_BATTERY_GOOD;
            let icon = Stack::new()
                .push(icons::battery_full_icon(18.0, color))
                .push(icons::bolt_icon(12.0, color));
            Some((icon.into(), color))
        }
        PowerInfo::Charging(lvl) => {
            let color = COLOR_BATTERY_CHARGING;
            let base = battery_level_icon(lvl, color);
            let icon = Stack::new().push(base).push(icons::bolt_icon(12.0, color));
            Some((icon.into(), color))
        }
        PowerInfo::Discharging(lvl) => {
            let color = if lvl > 60 {
                COLOR_BATTERY_GOOD
            } else if lvl > 30 {
                COLOR_BATTERY_MODERATE
            } else {
                COLOR_BATTERY_LOW
            };
            let icon = battery_level_icon(lvl, color);
            Some((icon, color))
        }
        PowerInfo::Wired => Some((icons::plug_icon(18.0, Color::WHITE), Color::WHITE)),
        PowerInfo::Unknown => None,
    }
}

fn battery_level_icon<'a, Message>(lvl: u8, color: Color) -> Element<'a, Message>
where
    Message: 'a,
{
    let size = 18.0;
    let icon = match lvl {
        91..=u8::MAX => icons::battery_full_icon(size, color),
        61..=90 => icons::battery_three_quarters_icon(size, color),
        41..=60 => icons::battery_half_icon(size, color),
        16..=40 => icons::battery_quarter_icon(size, color),
        _ => icons::battery_empty_icon(size, color),
    };
    icon.into()
}

pub fn render_clock<'a, Message>(time: &DateTime<Local>) -> Element<'a, Message>
where
    Message: 'a,
{
    Text::new(time.format("%H:%M").to_string())
        .font(SANSATION)
        .size(32)
        .color(COLOR_TEXT_BRIGHT)
        .into()
}
