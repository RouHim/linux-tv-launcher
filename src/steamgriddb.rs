use anyhow::{Context, Result};
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
struct GameData {
    id: u64,
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

    pub fn search_game(&self, query: &str) -> Result<Option<u64>> {
        let encoded_query = urlencoding::encode(query);
        let url = format!("{}/search/autocomplete/{}", API_BASE_URL, encoded_query);
        let mut resp = self
            .agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .call()
            .context("Failed to search game")?;

        let search_resp: SearchResponse = resp
            .body_mut()
            .read_json()
            .context("Failed to parse search response")?;

        if search_resp.success && !search_resp.data.is_empty() {
            Ok(Some(search_resp.data[0].id))
        } else {
            Ok(None)
        }
    }

    pub fn get_images_for_game(&self, game_id: u64) -> Result<Vec<GridData>> {
        let url = format!("{}/grids/game/{}", API_BASE_URL, game_id);
        // We prefer 600x900 vertical grids
        let mut resp = self
            .agent
            .get(&url)
            .query("dimensions", "600x900")
            // removed styles filter to get more results
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .call()
            .context("Failed to fetch grids")?;

        let grid_resp: GridResponse = resp
            .body_mut()
            .read_json()
            .context("Failed to parse grid response")?;

        if grid_resp.success {
            Ok(grid_resp.data)
        } else {
            Ok(Vec::new())
        }
    }
}
