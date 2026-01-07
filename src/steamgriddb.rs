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
        let agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(10))
            .timeout_write(Duration::from_secs(10))
            .build();
        Self { agent, api_key }
    }

    pub fn search_game(&self, query: &str) -> Result<Option<u64>> {
        let encoded_query = urlencoding::encode(query);
        let url = format!("{}/search/autocomplete/{}", API_BASE_URL, encoded_query);
        let resp: SearchResponse = self
            .agent
            .get(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .call()
            .context("Failed to search game")?
            .into_json()?;

        if resp.success && !resp.data.is_empty() {
            Ok(Some(resp.data[0].id))
        } else {
            Ok(None)
        }
    }

    pub fn get_images_for_game(&self, game_id: u64) -> Result<Vec<GridData>> {
        let url = format!("{}/grids/game/{}", API_BASE_URL, game_id);
        // We prefer 600x900 vertical grids
        let resp: GridResponse = self
            .agent
            .get(&url)
            .query("dimensions", "600x900")
            // removed styles filter to get more results
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .call()
            .context("Failed to fetch grids")?
            .into_json()?;

        if resp.success {
            Ok(resp.data)
        } else {
            Ok(Vec::new())
        }
    }
}
