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

// Timing Constants (in seconds)
pub const BATTERY_CHECK_INTERVAL_SECS: u64 = 60;
pub const RESTART_DELAY_SECS: u64 = 2;
