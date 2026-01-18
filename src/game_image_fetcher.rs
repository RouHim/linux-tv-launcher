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
        steam_appid: Option<&str>,
    ) -> anyhow::Result<Option<(Uuid, PathBuf)>> {
        let path = self
            .cache
            .find_existing_image(game_name)
            .or_else(|| self.try_source_image(game_name, source_image_url))
            .or_else(|| {
                let res = self.try_sgdb_by_steam_id(game_name, steam_appid);
                if res.is_none() && steam_appid.is_some() {
                    tracing::warn!(
                        "SGDB AppID lookup failed for '{}', falling back to name search",
                        game_name
                    );
                }
                res
            })
            .or_else(|| {
                let res = self.try_sgdb_image(game_name);
                if res.is_none() {
                    tracing::warn!(
                        "SGDB Name lookup failed for '{}', falling back to SearxNG",
                        game_name
                    );
                }
                res
            })
            .or_else(|| self.try_searxng_image(game_name));

        Ok(path.map(|p| (game_id, p)))
    }

    fn try_source_image(&self, game_name: &str, source_image_url: Option<&str>) -> Option<PathBuf> {
        let url = source_image_url?;
        self.cache
            .save_image(game_name, url, self.width, self.height)
            .ok()
    }

    fn try_sgdb_by_steam_id(&self, game_name: &str, steam_appid: Option<&str>) -> Option<PathBuf> {
        let appid = steam_appid.map(str::trim).filter(|id| !id.is_empty())?;
        match self.sgdb_client.get_game_by_steam_appid(appid) {
            Ok(Some(sgdb_id)) => self.download_sgdb_image(game_name, sgdb_id),
            _ => None,
        }
    }

    fn try_sgdb_image(&self, game_name: &str) -> Option<PathBuf> {
        match self.sgdb_client.search_game(game_name) {
            Ok(Some(sgdb_id)) => self.download_sgdb_image(game_name, sgdb_id),
            _ => None,
        }
    }

    fn download_sgdb_image(&self, game_name: &str, sgdb_id: u64) -> Option<PathBuf> {
        match self.sgdb_client.get_images_for_game(sgdb_id) {
            Ok(images) => images.first().and_then(|image| {
                self.cache
                    .save_image(game_name, &image.url, self.width, self.height)
                    .ok()
            }),
            Err(_e) => None,
        }
    }

    fn try_searxng_image(&self, game_name: &str) -> Option<PathBuf> {
        let search_query = format!("{} game cover", game_name);
        let url = self
            .searxng_client
            .search_image(&search_query)
            .ok()
            .flatten()?;
        self.cache
            .save_image(game_name, &url, self.width, self.height)
            .ok()
    }
}
