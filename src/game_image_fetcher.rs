use crate::image_cache::ImageCache;
use crate::searxng::SearxngClient;
use crate::steamgriddb::SteamGridDbClient;
use std::path::PathBuf;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct GameImageFetcher {
    cache: ImageCache,
    sgdb_client: SteamGridDbClient,
    searxng_client: SearxngClient,
    width: u32,
    height: u32,
}

impl GameImageFetcher {
    pub fn new(
        cache_dir: PathBuf,
        sgdb_client: SteamGridDbClient,
        searxng_client: SearxngClient,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            cache: ImageCache { cache_dir },
            sgdb_client,
            searxng_client,
            width,
            height,
        }
    }

    pub fn fetch(
        &self,
        game_id: Uuid,
        game_name: &str,
        source_image_url: Option<&str>,
    ) -> anyhow::Result<Option<(Uuid, PathBuf)>> {
        if let Some(path) = self.cache.find_existing_image(game_name) {
            info!("Cache hit for '{}': {:?}", game_name, path);
            return Ok(Some((game_id, path)));
        }

        if let Some(path) = self.try_source_image(game_name, source_image_url) {
            return Ok(Some((game_id, path)));
        }

        if let Some(path) = self.try_sgdb_image(game_name) {
            return Ok(Some((game_id, path)));
        }

        if let Some(path) = self.try_searxng_image(game_name) {
            return Ok(Some((game_id, path)));
        }

        warn!("Could not find any cover art for '{}'", game_name);
        Ok(None)
    }

    fn try_source_image(&self, game_name: &str, source_image_url: Option<&str>) -> Option<PathBuf> {
        let url = source_image_url?;
        info!("Trying Heroic image URL for '{}': {}", game_name, url);
        match self
            .cache
            .save_image(game_name, url, self.width, self.height)
        {
            Ok(path) => {
                info!(
                    "Successfully saved Heroic image for '{}' to {:?}",
                    game_name, path
                );
                Some(path)
            }
            Err(e) => {
                warn!(
                    "Failed to download Heroic image for '{}': {}, trying SteamGridDB...",
                    game_name, e
                );
                None
            }
        }
    }

    fn try_sgdb_image(&self, game_name: &str) -> Option<PathBuf> {
        info!("Fetching image for '{}' from SteamGridDB...", game_name);
        match self.sgdb_client.search_game(game_name) {
            Ok(Some(sgdb_id)) => {
                info!("Found SteamGridDB ID for '{}': {}", game_name, sgdb_id);
                match self.sgdb_client.get_images_for_game(sgdb_id) {
                    Ok(images) => {
                        if let Some(first_image) = images.first() {
                            info!("Downloading image for '{}': {}", game_name, first_image.url);
                            match self.cache.save_image(
                                game_name,
                                &first_image.url,
                                self.width,
                                self.height,
                            ) {
                                Ok(path) => {
                                    info!(
                                        "Successfully saved SteamGridDB image for '{}' to {:?}",
                                        game_name, path
                                    );
                                    Some(path)
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to save SteamGridDB image for '{}': {}, trying SearXNG...",
                                        game_name, e
                                    );
                                    None
                                }
                            }
                        } else {
                            warn!(
                                "No images found on SteamGridDB for '{}' (ID: {}), trying SearXNG...",
                                game_name, sgdb_id
                            );
                            None
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to get SteamGridDB images for '{}': {}, trying SearXNG...",
                            game_name, e
                        );
                        None
                    }
                }
            }
            Ok(None) => {
                warn!(
                    "Game not found on SteamGridDB: '{}', trying SearXNG...",
                    game_name
                );
                None
            }
            Err(e) => {
                warn!(
                    "Failed to search SteamGridDB for '{}': {}, trying SearXNG...",
                    game_name, e
                );
                None
            }
        }
    }

    fn try_searxng_image(&self, game_name: &str) -> Option<PathBuf> {
        let search_query = format!("{} game cover", game_name);
        info!("Searching SearXNG for '{}' cover art...", game_name);
        match self.searxng_client.search_image(&search_query) {
            Ok(Some(url)) => {
                info!("Found SearXNG image for '{}': {}", game_name, url);
                match self
                    .cache
                    .save_image(game_name, &url, self.width, self.height)
                {
                    Ok(path) => {
                        info!(
                            "Successfully saved SearXNG image for '{}' to {:?}",
                            game_name, path
                        );
                        Some(path)
                    }
                    Err(e) => {
                        error!("Failed to save SearXNG image for '{}': {}", game_name, e);
                        None
                    }
                }
            }
            Ok(None) => {
                warn!("No images found on SearXNG for '{}'", game_name);
                None
            }
            Err(e) => {
                error!("Failed to search SearXNG for '{}': {}", game_name, e);
                None
            }
        }
    }
}
