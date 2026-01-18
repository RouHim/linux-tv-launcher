use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::time::Duration;
use ureq::Agent;

const API_BASE_URL: &str = "https://www.steamgriddb.com/api/v2";

#[derive(Clone)]
pub struct SteamGridDbClient {
    agent: Agent,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    success: bool,
    data: Vec<GameData>,
}

#[derive(Debug, Deserialize)]
struct GameResponse {
    success: bool,
    data: GameData,
}

#[derive(Debug, Deserialize)]
struct GameData {
    id: u64,
    name: String,
}

#[derive(Debug, Deserialize)]
struct GridResponse {
    success: bool,
    data: Vec<GridData>,
}

#[derive(Debug, Deserialize)]
pub struct GridData {
    pub url: String,
}

impl SteamGridDbClient {
    pub fn new(api_key: String) -> Self {
        let agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .new_agent();
        Self { agent, api_key }
    }

    fn get<T: DeserializeOwned>(&self, path: &str, params: &[(&str, &str)]) -> Result<T> {
        let url = format!("{}{}", API_BASE_URL, path);
        let mut req = self
            .agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.api_key));

        for (k, v) in params {
            req = req.query(k, v);
        }

        let mut resp = req.call().context("Failed to contact SteamGridDB")?;
        resp.body_mut()
            .read_json()
            .context("Failed to parse SGDB response")
    }

    pub fn search_game(&self, query: &str) -> Result<Option<u64>> {
        let encoded_query = urlencoding::encode(query);
        let path = format!("/search/autocomplete/{}", encoded_query);

        let search_resp: SearchResponse = match self.get(&path, &[]) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("SGDB Search failed for '{}': {}", query, e);
                return Ok(None);
            }
        };

        if !search_resp.success || search_resp.data.is_empty() {
            tracing::warn!("SGDB Search for '{}' returned no results", query);
            return Ok(None);
        }

        // 1. Try Exact Match (Case Insensitive)
        if let Some(exact) = search_resp
            .data
            .iter()
            .find(|g| g.name.eq_ignore_ascii_case(query))
        {
            return Ok(Some(exact.id));
        }

        // 2. Fallback to first result
        Ok(Some(search_resp.data[0].id))
    }

    pub fn get_game_by_steam_appid(&self, appid: &str) -> Result<Option<u64>> {
        let appid = appid.trim();
        if appid.is_empty() {
            return Ok(None);
        }

        let path = format!("/games/steam/{}", appid);
        let search_resp: GameResponse = match self.get(&path, &[]) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("SGDB AppID lookup failed for '{}': {}", appid, e);
                return Ok(None);
            }
        };

        if search_resp.success {
            Ok(Some(search_resp.data.id))
        } else {
            Ok(None)
        }
    }

    pub fn get_images_for_game(&self, game_id: u64) -> Result<Vec<GridData>> {
        let path = format!("/grids/game/{}", game_id);
        // We prefer 600x900 vertical grids
        let grid_resp: GridResponse = match self.get(&path, &[("dimensions", "600x900")]) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("SGDB Grid fetch failed for game_id {}: {}", game_id, e);
                return Ok(Vec::new());
            }
        };

        if grid_resp.success {
            Ok(grid_resp.data)
        } else {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn get_api_key() -> Option<String> {
        env::var("STEAMGRIDDB_API_KEY").ok()
    }

    #[test]
    fn test_search_game_integration() {
        let api_key = match get_api_key() {
            Some(key) => key,
            None => {
                println!("Skipping test_search_game_integration: STEAMGRIDDB_API_KEY not set");
                return;
            }
        };

        let client = SteamGridDbClient::new(api_key);
        // Search for a known game
        let result = client.search_game("Celeste");
        assert!(result.is_ok());
        let game_id = result.unwrap();
        assert!(game_id.is_some());
        println!("Found Celeste ID: {:?}", game_id);
    }

    #[test]
    fn test_get_game_by_steam_appid_integration() {
        let api_key = match get_api_key() {
            Some(key) => key,
            None => {
                println!("Skipping test_get_game_by_steam_appid_integration: STEAMGRIDDB_API_KEY not set");
                return;
            }
        };

        let client = SteamGridDbClient::new(api_key);
        // Celeste Steam AppID is 504230
        let result = client.get_game_by_steam_appid("504230");
        assert!(result.is_ok());
        let game_id = result.unwrap();
        assert!(game_id.is_some());
        println!("Found Celeste by AppID: {:?}", game_id);
    }

    #[test]
    fn test_limbo_integration() {
        let api_key = match get_api_key() {
            Some(key) => key,
            None => {
                println!("Skipping test_limbo_integration: STEAMGRIDDB_API_KEY not set");
                return;
            }
        };

        let client = SteamGridDbClient::new(api_key);

        // 1. Search by AppID (Limbo = 48000)
        println!("Testing Limbo Lookup by AppID (48000)...");
        let result_id = client.get_game_by_steam_appid("48000");
        assert!(result_id.is_ok());
        let game_id = result_id.unwrap().expect("Should find Limbo by AppID");
        println!("Found Limbo ID: {}", game_id);

        // 2. Fetch Images
        println!("Fetching images for Limbo (ID: {})...", game_id);
        let images = client.get_images_for_game(game_id);
        assert!(images.is_ok());
        let grids = images.unwrap();
        assert!(!grids.is_empty(), "Should find at least one grid for Limbo");
        println!("Found {} grids for Limbo", grids.len());
        println!("First grid URL: {}", grids[0].url);

        // 3. Search by Name (Fallback test)
        println!("Testing Limbo Lookup by Name...");
        let search_res = client.search_game("Limbo");
        assert!(search_res.is_ok());
        let search_id = search_res.unwrap().expect("Should find Limbo by Name");
        assert_eq!(
            game_id, search_id,
            "Name search ID should match AppID search ID"
        );
    }
}
