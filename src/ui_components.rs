use iced::widget::{Container, Image, Svg, Text};
use iced::{Color, ContentFit, Element, Length};
use std::path::{Path, PathBuf};

use crate::ui_theme::SANSATION;

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
