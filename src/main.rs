use tracing_subscriber::EnvFilter;

mod app;
mod assets;
mod gamepad;
mod input;
mod launcher;
mod model;
mod storage;
mod xdg_utils;

pub use app::Launcher;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("linux_tv_launcher=info".parse().unwrap()),
        )
        .init();

    // Debug: Scan apps at startup
    println!("--- DEBUG: Scanning apps ---");
    let apps = xdg_utils::scan_system_apps();
    for app in apps.iter().take(10) {
        println!("App: {}, Icon: {:?}", app.name, app.icon);
    }
    println!("--- DEBUG: End scan ---");

    iced::application(Launcher::new, Launcher::update, Launcher::view)
        .title(|launcher: &Launcher| launcher.title())
        .subscription(Launcher::subscription)
        .run()
}
