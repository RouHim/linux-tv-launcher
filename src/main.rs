use tracing_subscriber::EnvFilter;

mod app;
mod assets;
mod game_sources;
mod gamepad;
mod input;
mod launcher;
mod model;
mod storage;
mod system_update;

pub use app::Launcher;

fn main() -> iced::Result {
    let mut env_filter = EnvFilter::from_default_env();
    if let Ok(directive) = "linux_tv_launcher=info".parse() {
        env_filter = env_filter.add_directive(directive);
    }

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    iced::application(Launcher::new, Launcher::update, Launcher::view)
        .title(|launcher: &Launcher| launcher.title())
        .subscription(Launcher::subscription)
        .run()
}
