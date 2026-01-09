use tracing_subscriber::EnvFilter;

mod assets;
mod desktop_apps;
mod focus_manager;
mod game_image_fetcher;
mod game_sources;
mod gamepad;
mod icons;
mod image_cache;
mod input;
mod launcher;
mod model;
mod searxng;
mod steamgriddb;
mod storage;
mod system_update;
mod ui;
mod ui_app_picker;
mod ui_components;
mod ui_modals;
mod ui_theme;

use iced_fonts::FONTAWESOME_FONT_BYTES;
use ui::Launcher;

fn main() -> iced::Result {
    let mut env_filter = EnvFilter::from_default_env();
    if let Ok(directive) = "linux_tv_launcher=info".parse() {
        env_filter = env_filter.add_directive(directive);
    }

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    iced::application(Launcher::new, Launcher::update, Launcher::view)
        .title(|launcher: &Launcher| launcher.title())
        .subscription(Launcher::subscription)
        .font(assets::get_sansation_font().expect("Sansation font embedded"))
        .font(FONTAWESOME_FONT_BYTES)
        .window(iced::window::Settings {
            decorations: false,
            fullscreen: true,
            level: iced::window::Level::AlwaysOnTop,
            ..Default::default()
        })
        .run()
}
