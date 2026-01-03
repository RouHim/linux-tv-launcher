use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Asset;

pub fn get_default_icon() -> Option<Vec<u8>> {
    Asset::get("icon.svg").map(|f| f.data.into_owned())
}
