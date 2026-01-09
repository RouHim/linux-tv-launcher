use iced::widget::{Container, Image, Svg, Text};
use iced::{Color, ContentFit, Element, Length};
use std::path::Path;

use crate::ui_theme::SANSATION;

pub enum IconPath<'a> {
    Str(&'a str),
    Path(&'a Path),
}

impl IconPath<'_> {
    pub fn is_svg(&self) -> bool {
        match self {
            IconPath::Str(path) => path.ends_with(".svg"),
            IconPath::Path(path) => path.extension() == Some(std::ffi::OsStr::new("svg")),
        }
    }
}

pub fn render_icon<'a, Message>(
    icon_path: Option<IconPath<'_>>,
    width: f32,
    height: f32,
    fallback_text: &'static str,
    fallback_size: Option<u32>,
    default_icon_handle: Option<iced::widget::svg::Handle>,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    if let Some(icon_path) = icon_path {
        let is_svg = icon_path.is_svg();
        match icon_path {
            IconPath::Str(path) => {
                if is_svg {
                    return Svg::from_path(path)
                        .width(Length::Fixed(width))
                        .height(Length::Fixed(height))
                        .into();
                }

                return Image::new(path)
                    .width(Length::Fixed(width))
                    .height(Length::Fixed(height))
                    .content_fit(ContentFit::Contain)
                    .into();
            }
            IconPath::Path(path) => {
                if is_svg {
                    return Svg::from_path(path.to_path_buf())
                        .width(Length::Fixed(width))
                        .height(Length::Fixed(height))
                        .into();
                }

                return Image::new(path.to_path_buf())
                    .width(Length::Fixed(width))
                    .height(Length::Fixed(height))
                    .content_fit(ContentFit::Contain)
                    .into();
            }
        }
    }

    if let Some(handle) = default_icon_handle {
        Svg::new(handle)
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .into()
    } else {
        let text = match fallback_size {
            Some(size) => Text::new(fallback_text)
                .font(SANSATION)
                .size(size)
                .color(Color::WHITE),
            None => Text::new(fallback_text).font(SANSATION).color(Color::WHITE),
        };

        Container::new(text)
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
