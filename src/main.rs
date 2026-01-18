mod assets;
mod category_list;
mod desktop_apps;
mod focus_manager;
mod game_image_fetcher;
mod game_sources;
mod gamepad;
mod gopher64;
mod icons;
mod image_cache;
mod input;
mod launcher;
mod messages;
mod model;
mod osk;
mod searxng;
mod steamgriddb;
mod storage;
mod sys_utils;
mod system_info;
mod system_update;
mod system_update_state;
mod ui;
mod ui_app_picker;
mod ui_components;
mod ui_main_view;
mod ui_modals;
mod ui_state;
mod ui_system_info_modal;
mod ui_system_update_modal;
mod ui_theme;
mod updater;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    let mut settings = iced::Settings::default();
    if let Some(sansation) = assets::get_sansation_font() {
        settings.fonts.push(sansation.into());
    }
    settings
        .fonts
        .push(iced_fonts::FONTAWESOME_FONT_BYTES.into());

    iced::application(ui::Launcher::new, ui::Launcher::update, ui::Launcher::view)
        .title(ui::Launcher::title)
        .subscription(ui::Launcher::subscription)
        .settings(settings)
        .window(iced::window::Settings {
            decorations: false,
            fullscreen: true,
            ..Default::default()
        })
        .run()
}
