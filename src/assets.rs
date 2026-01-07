use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Asset;

pub fn get_default_icon() -> Option<Vec<u8>> {
    Asset::get("icon.svg").map(|f| f.data.into_owned())
}

pub fn get_shutdown_icon() -> Option<Vec<u8>> {
    Asset::get("shutdown.svg").map(|f| f.data.into_owned())
}

pub fn get_suspend_icon() -> Option<Vec<u8>> {
    Asset::get("suspend.svg").map(|f| f.data.into_owned())
}

pub fn get_exit_icon() -> Option<Vec<u8>> {
    Asset::get("exit.svg").map(|f| f.data.into_owned())
}
