use crate::image_cache::ImageCache;
use crate::searxng::SearxngClient;
use crate::steamgriddb::SteamGridDbClient;
use std::path::PathBuf;
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

        Ok(None)
    }

    fn try_source_image(&self, game_name: &str, source_image_url: Option<&str>) -> Option<PathBuf> {
        let url = source_image_url?;
        self.cache
            .save_image(game_name, url, self.width, self.height)
            .ok()
    }

    fn try_sgdb_image(&self, game_name: &str) -> Option<PathBuf> {
        match self.sgdb_client.search_game(game_name) {
            Ok(Some(sgdb_id)) => match self.sgdb_client.get_images_for_game(sgdb_id) {
                Ok(images) => {
                    if let Some(first_image) = images.first() {
                        self.cache
                            .save_image(game_name, &first_image.url, self.width, self.height)
                            .ok()
                    } else {
                        None
                    }
                }
                Err(_e) => None,
            },
            Ok(None) => None,
            Err(_e) => None,
        }
    }

    fn try_searxng_image(&self, game_name: &str) -> Option<PathBuf> {
        let search_query = format!("{} game cover", game_name);
        match self.searxng_client.search_image(&search_query) {
            Ok(Some(url)) => self
                .cache
                .save_image(game_name, &url, self.width, self.height)
                .ok(),
            Ok(None) => None,
            Err(_e) => None,
        }
    }
}
