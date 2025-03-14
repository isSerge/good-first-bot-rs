use anyhow::{Result, Context};
use std::env;

#[derive(Debug)]
pub struct Config {
    pub github_token: String,
    pub github_graphql_url: String,
    pub telegram_bot_token: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            github_token: env::var("GITHUB_TOKEN")
                .context("GITHUB_TOKEN environment variable is required")?,
            github_graphql_url: env::var("GITHUB_GRAPHQL_URL")
                .unwrap_or_else(|_| "https://api.github.com/graphql".into()),
            telegram_bot_token: env::var("TELOXIDE_TOKEN")
                .context("TELOXIDE_TOKEN environment variable is required")?,
        })
    }
}
