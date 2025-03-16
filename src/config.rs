use anyhow::{Context, Result};
use std::env;

#[derive(Debug)]
pub struct Config {
    pub github_token: String,
    pub github_graphql_url: String,
    pub telegram_bot_token: String,
    pub poll_interval: u64,
    pub database_url: String,
}

impl Config {
    #[must_use = "This function returns a Result that should not be ignored"]
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            github_token: env::var("GITHUB_TOKEN")
                .context("GITHUB_TOKEN environment variable is required")?,
            github_graphql_url: env::var("GITHUB_GRAPHQL_URL")
                .ok()
                .unwrap_or_else(|| "https://api.github.com/graphql".to_string()),
            telegram_bot_token: env::var("TELOXIDE_TOKEN")
                .context("TELOXIDE_TOKEN environment variable is required")?,
            poll_interval: env::var("POLL_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            database_url: env::var("DATABASE_URL")
                .ok()
                .unwrap_or_else(|| "sqlite://data/data.db".to_string()),
        })
    }
}
