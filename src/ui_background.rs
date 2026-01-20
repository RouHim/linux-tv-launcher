use iced::mouse::Cursor;
use iced::widget::canvas::{self, Canvas, Geometry, Path};
use iced::{Color, Element, Length, Point, Rectangle, Theme};
use std::rc::Rc;

use crate::ui_theme::{COLOR_BACKGROUND, COLOR_SOFT_WHITE};

#[derive(Debug, Clone)]
pub struct WhaleSharkBackground {
    cache: Rc<canvas::Cache>,
}

impl Default for WhaleSharkBackground {
    fn default() -> Self {
        Self {
            cache: Rc::new(canvas::Cache::new()),
        }
    }
}

impl WhaleSharkBackground {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view<'a, Message: 'a>(&self) -> Element<'a, Message> {
        Canvas::new(self.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<Message> canvas::Program<Message> for WhaleSharkBackground {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            // 1. Draw base background
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), COLOR_BACKGROUND);

            // 2. Draw Whale Shark pattern (dots)
            // Settings for the pattern
            let cell_size = 40.0;
            let rows = (bounds.height / cell_size).ceil() as u32;
            let cols = (bounds.width / cell_size).ceil() as u32;

            for y in 0..rows {
                for x in 0..cols {
                    let seed = hash(x, y);

                    // Only draw a dot if the seed is above a threshold (density control)
                    // Whale sharks are spotty, let's say 85% chance of a spot in a cell
                    if seed > 0.15 {
                        // Randomize position within the cell
                        let offset_x = (hash(x + 1000, y) - 0.5) * (cell_size * 0.6);
                        let offset_y = (hash(x, y + 1000) - 0.5) * (cell_size * 0.6);

                        let center_x = (x as f32 * cell_size) + (cell_size / 2.0) + offset_x;
                        let center_y = (y as f32 * cell_size) + (cell_size / 2.0) + offset_y;

                        // Randomize size: 1.5px to 4.5px radius
                        let radius = 1.5 + (hash(x + 500, y + 500) * 3.0);

                        // Randomize opacity: 0.01 to 0.05
                        let alpha = 0.01 + (hash(x + 200, y + 200) * 0.04);

                        let color = Color {
                            a: alpha,
                            ..COLOR_SOFT_WHITE
                        };

                        let dot = Path::circle(Point::new(center_x, center_y), radius);
                        frame.fill(&dot, color);
                    }
                }
            }
        });

        vec![geometry]
    }
}

// Simple deterministic pseudo-random hash
fn hash(x: u32, y: u32) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x45D9F3B);
    h = h.wrapping_add((y as u64).wrapping_mul(0x119DE1F3));
    h = (h ^ (h >> 13)).wrapping_mul(0x5DEECE66D);
    let val = h ^ (h >> 16);
    // Normalize to 0.0 - 1.0 using u32 range
    (val as u32) as f32 / u32::MAX as f32
}
