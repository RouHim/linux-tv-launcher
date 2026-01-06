use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

pub struct ImageCache {
    pub cache_dir: PathBuf,
}

impl ImageCache {
    pub fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "rouven", "linux-tv-launcher")
            .context("Failed to determine project directories")?;
        let cache_dir = dirs.cache_dir().join("grids");
        fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;
        Ok(Self { cache_dir })
    }

    pub fn get_image_path(&self, game_name: &str, extension: &str) -> PathBuf {
        let safe_name = self.sanitize_name(game_name);
        self.cache_dir.join(format!("{}.{}", safe_name, extension))
    }

    fn sanitize_name(&self, name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
            .collect()
    }

    pub fn find_existing_image(&self, game_name: &str) -> Option<PathBuf> {
        let safe_name = self.sanitize_name(game_name);
        let extensions = ["png", "jpg", "jpeg", "webp"];
        for ext in extensions {
            let path = self.cache_dir.join(format!("{}.{}", safe_name, ext));
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    pub fn save_image(&self, game_name: &str, url: &str, width: u32, height: u32) -> Result<PathBuf> {
        let extension = url
            .split('.')
            .next_back()
            .unwrap_or("png");

        let path = self.get_image_path(game_name, extension);
        if path.exists() {
            return Ok(path);
        }

        let resp = ureq::get(url).call().context("Failed to download image")?;
        let mut bytes = Vec::new();
        resp.into_reader()
            .read_to_end(&mut bytes)
            .context("Failed to read response body")?;

        let img = image::load_from_memory(&bytes).context("Failed to load image from memory")?;
        // Resize to requested dimensions, maintaining aspect ratio.
        let resized = img.resize(width, height, image::imageops::FilterType::Triangle);

        resized.save(&path).context("Failed to save resized image")?;

        Ok(path)
    }
}
