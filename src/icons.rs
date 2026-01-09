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
