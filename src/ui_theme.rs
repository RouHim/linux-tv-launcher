use iced::{Color, Font};

// Custom font
pub const SANSATION: Font = Font::with_name("Sansation");

// Dimensions
pub const GAME_POSTER_WIDTH: f32 = 200.0;
pub const GAME_POSTER_HEIGHT: f32 = 300.0;
pub const ICON_SIZE: f32 = 128.0;
pub const ICON_ITEM_WIDTH: f32 = 150.0;
pub const ICON_ITEM_HEIGHT: f32 = 280.0;

// --- Design System Primitives (from docs/color-schema.md) ---
pub const COLOR_ABYSS_DARK: Color = Color::from_rgb(0.04, 0.06, 0.09); // #0B1016
pub const COLOR_DEEP_SLATE: Color = Color::from_rgb(0.09, 0.13, 0.19); // #162231
pub const COLOR_CYAN_GLOW: Color = Color::from_rgb(0.30, 0.79, 0.94); // #4CC9F0
pub const COLOR_SOFT_WHITE: Color = Color::from_rgb(0.94, 0.96, 0.97); // #F0F4F8
pub const COLOR_MUTED_STEEL: Color = Color::from_rgb(0.58, 0.64, 0.72); // #94A3B8

// --- Semantic Mappings ---

// Backgrounds
pub const COLOR_BACKGROUND: Color = COLOR_ABYSS_DARK;
pub const COLOR_PANEL: Color = COLOR_DEEP_SLATE;
pub const COLOR_MENU_BACKGROUND: Color = COLOR_DEEP_SLATE;
pub const COLOR_STATUS_BACKGROUND: Color = COLOR_ABYSS_DARK;

// Typography
pub const COLOR_TEXT_BRIGHT: Color = COLOR_SOFT_WHITE;
pub const COLOR_TEXT_SOFT: Color = COLOR_SOFT_WHITE;
pub const COLOR_TEXT_MUTED: Color = COLOR_MUTED_STEEL;
pub const COLOR_TEXT_HINT: Color = COLOR_MUTED_STEEL;
pub const COLOR_TEXT_DIM: Color = Color::from_rgb(0.40, 0.44, 0.50); // Darker steel

// Accents & Interactions
pub const COLOR_ACCENT: Color = COLOR_CYAN_GLOW;

// Overlays (derived from primitives)
pub const COLOR_ACCENT_OVERLAY: Color = Color::from_rgba(0.30, 0.79, 0.94, 0.3); // Cyan Glow @ 30%
pub const COLOR_OVERLAY: Color = Color::from_rgba(0.04, 0.06, 0.09, 0.7); // Abyss Dark @ 70%
pub const COLOR_OVERLAY_STRONG: Color = Color::from_rgba(0.04, 0.06, 0.09, 0.85); // Abyss Dark @ 85%

// Status Colors
pub const COLOR_STATUS_TEXT: Color = Color::from_rgb(0.9, 0.8, 0.4);

// General Status Colors (Semantic)
pub const COLOR_SUCCESS: Color = COLOR_BATTERY_GOOD;
pub const COLOR_WARNING: Color = COLOR_BATTERY_MODERATE;
pub const COLOR_ERROR: Color = COLOR_BATTERY_LOW;

// Battery Colors
pub const COLOR_BATTERY_GOOD: Color = Color::from_rgb(0.3, 0.69, 0.31);
pub const COLOR_BATTERY_MODERATE: Color = Color::from_rgb(1.0, 0.6, 0.0);
pub const COLOR_BATTERY_LOW: Color = Color::from_rgb(0.96, 0.26, 0.21);
pub const COLOR_BATTERY_CHARGING: Color = Color::from_rgb(0.13, 0.59, 0.95);

// Layout Constants
pub const MAIN_CONTENT_VERTICAL_PADDING: f32 = 80.0;
pub const ITEM_SPACING: f32 = 10.0;
pub const APP_PICKER_WIDTH_RATIO: f32 = 0.8;
pub const APP_PICKER_PADDING: f32 = 80.0;
pub const DEFAULT_VIEWPORT_HEIGHT: f32 = 600.0;
pub const REFERENCE_WINDOW_HEIGHT: f32 = 1080.0;
pub const MIN_UI_SCALE: f32 = 0.5;
pub const MAX_UI_SCALE: f32 = 3.0;

// --- Base Font Sizes (at 1080p reference) ---
pub const BASE_FONT_TINY: f32 = 12.0;
pub const BASE_FONT_SMALL: f32 = 14.0;
pub const BASE_FONT_MEDIUM: f32 = 16.0;

pub const BASE_FONT_LARGE: f32 = 18.0;
pub const BASE_FONT_XLARGE: f32 = 20.0;

pub const BASE_FONT_TITLE: f32 = 24.0;

pub const BASE_FONT_HEADER: f32 = 28.0;
pub const BASE_FONT_DISPLAY: f32 = 32.0;

// --- Base Padding/Spacing (at 1080p reference) ---
pub const BASE_PADDING_TINY: f32 = 6.0;
pub const BASE_PADDING_SMALL: f32 = 10.0;

pub const BASE_PADDING_MEDIUM: f32 = 20.0;

pub const BASE_PADDING_LARGE: f32 = 40.0;
pub const CATEGORY_ROW_SPACING: f32 = 40.0;

// --- Modal Dimensions ---
pub const MODAL_WIDTH_CONTEXT_MENU: f32 = 300.0;
pub const MODAL_WIDTH_SMALL: f32 = 400.0;
pub const MODAL_WIDTH_MEDIUM: f32 = 560.0;
pub const MODAL_WIDTH_SYSTEM_UPDATE: f32 = 500.0;
pub const MODAL_WIDTH_LARGE: f32 = 600.0;
pub const MODAL_HEIGHT_SMALL: f32 = 300.0;
pub const MODAL_HEIGHT_MEDIUM: f32 = 360.0;
pub const MODAL_OVERLAY_PADDING: f32 = 100.0;
pub const MODAL_HELP_PADDING: f32 = 200.0;

#[inline]
pub fn scaled(base: f32, scale: f32) -> f32 {
    base * scale
}

#[inline]
pub fn scaled_fixed(base: f32, scale: f32) -> iced::Length {
    iced::Length::Fixed(base * scale)
}

// Timing Constants (in seconds)
pub const BATTERY_CHECK_INTERVAL_SECS: u64 = 60;
pub const RESTART_DELAY_SECS: u64 = 2;
