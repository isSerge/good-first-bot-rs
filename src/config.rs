use std::env::{self, VarError};

const DEFAULT_DATABASE_URL: &str = "sqlite://data/data.db";
const DEFAULT_GITHUB_GRAPHQL_URL: &str = "https://api.github.com/graphql";
const DEFAULT_POLL_INTERVAL: u64 = 10;

#[derive(Debug)]
pub struct Config {
    pub github_token: String,
    pub github_graphql_url: String,
    pub telegram_bot_token: String,
    pub poll_interval: u64,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, VarError> {
        Ok(Self {
            github_token: env::var("GITHUB_TOKEN")?,
            github_graphql_url: env::var("GITHUB_GRAPHQL_URL")
                .unwrap_or_else(|_| DEFAULT_GITHUB_GRAPHQL_URL.to_string()),
            telegram_bot_token: env::var("TELOXIDE_TOKEN")?,
            poll_interval: env::var("POLL_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_POLL_INTERVAL),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use temp_env::with_vars;

    use super::*;

    #[test]
    fn test_from_env() {
        with_vars(
            [
                ("GITHUB_TOKEN", Some("test github token")),
                ("GITHUB_GRAPHQL_URL", Some("https://api.github.com/graphql")),
                ("TELOXIDE_TOKEN", Some("test telegram bot token")),
                ("POLL_INTERVAL", Some("100")),
                ("DATABASE_URL", Some("sqlite://test/test.db")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.github_token, "test github token");
                assert_eq!(config.github_graphql_url, "https://api.github.com/graphql");
                assert_eq!(config.telegram_bot_token, "test telegram bot token");
                assert_eq!(config.poll_interval, 100);
                assert_eq!(config.database_url, "sqlite://test/test.db");
            },
        );
    }

    #[test]
    fn test_missing_github_token_error() {
        with_vars(
            [
                ("GITHUB_GRAPHQL_URL", Some("https://api.github.com/graphql")),
                ("TELOXIDE_TOKEN", Some("test telegram bot token")),
                ("POLL_INTERVAL", Some("100")),
                ("DATABASE_URL", Some("sqlite://test/test.db")),
            ],
            || {
                unsafe {
                    env::remove_var("GITHUB_TOKEN");
                }
                let config = Config::from_env();
                assert!(config.is_err());
            },
        );
    }

    #[test]
    fn test_missing_telegram_bot_token_error() {
        with_vars(
            [
                ("GITHUB_GRAPHQL_URL", Some("https://api.github.com/graphql")),
                ("GITHUB_TOKEN", Some("test github token")),
                ("POLL_INTERVAL", Some("100")),
                ("DATABASE_URL", Some("sqlite://test/test.db")),
            ],
            || {
                let config = Config::from_env();
                assert!(config.is_err());
            },
        );
    }

    #[test]
    fn test_missing_database_url_default() {
        with_vars(
            [
                ("GITHUB_GRAPHQL_URL", Some("https://api.github.com/graphql")),
                ("GITHUB_TOKEN", Some("test github token")),
                ("TELOXIDE_TOKEN", Some("test telegram bot token")),
                ("POLL_INTERVAL", Some("100")),
            ],
            || {
                let config = Config::from_env().unwrap();

                assert_eq!(config.database_url, DEFAULT_DATABASE_URL);
            },
        );
    }

    #[test]
    fn test_missing_github_graphql_url_default() {
        with_vars(
            [
                ("GITHUB_TOKEN", Some("test github token")),
                ("TELOXIDE_TOKEN", Some("test telegram bot token")),
                ("POLL_INTERVAL", Some("100")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.github_graphql_url, DEFAULT_GITHUB_GRAPHQL_URL);
            },
        );
    }

    #[test]
    fn test_missing_poll_interval_default() {
        with_vars(
            [
                ("GITHUB_TOKEN", Some("test github token")),
                ("GITHUB_GRAPHQL_URL", Some("https://api.github.com/graphql")),
                ("TELOXIDE_TOKEN", Some("test telegram bot token")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.poll_interval, DEFAULT_POLL_INTERVAL);
            },
        );
    }
}
