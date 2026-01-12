use iced::{Color, Element};
use iced_fonts::fontawesome;

pub fn power_off_icon<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    fontawesome::power_off()
        .size(size)
        .color(Color::WHITE)
        .into()
}

pub fn pause_icon<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    fontawesome::pause().size(size).color(Color::WHITE).into()
}

pub fn arrows_rotate_icon<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    fontawesome::arrows_rotate()
        .size(size)
        .color(Color::WHITE)
        .into()
}

pub fn exit_icon<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    fontawesome::arrow_right_from_bracket()
        .size(size)
        .color(Color::WHITE)
        .into()
}

pub fn info_icon<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    fontawesome::info().size(size).color(Color::WHITE).into()
}

pub fn gamepad_icon<'a, Message: 'a>(size: f32, color: Color) -> Element<'a, Message> {
    fontawesome::gamepad().size(size).color(color).into()
}

pub fn battery_full_icon<'a, Message: 'a>(size: f32, color: Color) -> Element<'a, Message> {
    fontawesome::battery_full().size(size).color(color).into()
}

pub fn battery_three_quarters_icon<'a, Message: 'a>(
    size: f32,
    color: Color,
) -> Element<'a, Message> {
    fontawesome::battery_three_quarters()
        .size(size)
        .color(color)
        .into()
}

pub fn battery_half_icon<'a, Message: 'a>(size: f32, color: Color) -> Element<'a, Message> {
    fontawesome::battery_half().size(size).color(color).into()
}

pub fn battery_quarter_icon<'a, Message: 'a>(size: f32, color: Color) -> Element<'a, Message> {
    fontawesome::battery_quarter()
        .size(size)
        .color(color)
        .into()
}

pub fn battery_empty_icon<'a, Message: 'a>(size: f32, color: Color) -> Element<'a, Message> {
    fontawesome::battery_empty().size(size).color(color).into()
}

pub fn bolt_icon<'a, Message: 'a>(size: f32, color: Color) -> Element<'a, Message> {
    fontawesome::bolt().size(size).color(color).into()
}

pub fn plug_icon<'a, Message: 'a>(size: f32, color: Color) -> Element<'a, Message> {
    fontawesome::plug().size(size).color(color).into()
}
