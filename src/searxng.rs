use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;
use ureq::Agent;

const DEFAULT_BASE_URL: &str = "https://search.himmelstein.info";

#[derive(Clone)]
pub struct SearxngClient {
    agent: Agent,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    results: Vec<ImageResult>,
}

#[derive(Debug, Deserialize)]
struct ImageResult {
    img_src: Option<String>,
}

impl SearxngClient {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(base_url: String) -> Self {
        let agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .build()
            .new_agent();
        Self { agent, base_url }
    }

    /// Search for an image by query. Returns the first image URL found, if any.
    pub fn search_image(&self, query: &str) -> Result<Option<String>> {
        let url = format!("{}/search", self.base_url);
        let mut resp = self
            .agent
            .get(&url)
            .query("q", query)
            .query("format", "json")
            .query("categories", "images")
            .call()
            .context("Failed to search images on SearXNG")?;

        let search_resp: SearchResponse = resp
            .body_mut()
            .read_json()
            .context("Failed to parse SearXNG response")?;

        // Return the first result with a valid img_src
        for result in search_resp.results {
            if let Some(img_src) = result.img_src {
                if !img_src.is_empty() {
                    return Ok(Some(img_src));
                }
            }
        }

        Ok(None)
    }
}

impl Default for SearxngClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SearxngClient::new();
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn test_custom_base_url() {
        let client = SearxngClient::with_base_url("https://example.com".to_string());
        assert_eq!(client.base_url, "https://example.com");
    }
}
