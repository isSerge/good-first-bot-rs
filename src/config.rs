use std::env::{self, VarError};

const DEFAULT_DATABASE_URL: &str = "sqlite:data/data.db";
const DEFAULT_GITHUB_GRAPHQL_URL: &str = "https://api.github.com/graphql";
const DEFAULT_POLL_INTERVAL: u64 = 10;
const DEFAULT_REPOS_PER_USER: usize = 20;
const DEFAULT_LABELS_PER_REPO: usize = 10;
const DEFAULT_MAX_CONCURRENCY: usize = 10;
const DEFAULT_RATE_LIMIT_THRESHOLD: u64 = 10;

/// Represents the application configuration.
#[derive(Debug)]
pub struct Config {
    /// The GitHub API token.
    pub github_token: String,
    /// The URL of the GitHub GraphQL API.
    pub github_graphql_url: String,
    /// The Telegram bot token.
    pub telegram_bot_token: String,
    /// The interval in seconds to poll for new issues.
    pub poll_interval: u64,
    /// The URL of the database.
    pub database_url: String,
    /// The maximum number of repositories a user can track.
    pub max_repos_per_user: usize,
    /// The maximum number of labels a user can track per repository.
    pub max_labels_per_repo: usize,
    /// The maximum number of concurrent requests to make to the GitHub API.
    pub max_concurrency: usize,
    /// The threshold before the bot should pause operations.
    pub rate_limit_threshold: u64,
}

impl Config {
    /// Creates a new `Config` instance from environment variables.
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
            max_repos_per_user: env::var("MAX_REPOS_PER_USER")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_REPOS_PER_USER),
            max_labels_per_repo: env::var("MAX_LABELS_PER_REPO")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LABELS_PER_REPO),
            max_concurrency: env::var("MAX_CONCURRENCY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_MAX_CONCURRENCY),
            rate_limit_threshold: env::var("RATE_LIMIT_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_RATE_LIMIT_THRESHOLD),
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
                ("DATABASE_URL", Some("sqlite:test/test.db")),
                ("MAX_REPOS_PER_USER", Some("50")),
                ("MAX_LABELS_PER_REPO", Some("20")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.github_token, "test github token");
                assert_eq!(config.github_graphql_url, "https://api.github.com/graphql");
                assert_eq!(config.telegram_bot_token, "test telegram bot token");
                assert_eq!(config.poll_interval, 100);
                assert_eq!(config.database_url, "sqlite:test/test.db");
                assert_eq!(config.max_repos_per_user, 50);
                assert_eq!(config.max_labels_per_repo, 20);
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
                ("DATABASE_URL", Some("sqlite:test/test.db")),
                ("GITHUB_TOKEN", None),
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
                ("DATABASE_URL", Some("sqlite:test/test.db")),
                ("TELOXIDE_TOKEN", None),
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
                ("DATABASE_URL", None),
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
                ("GITHUB_GRAPHQL_URL", None),
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
                ("POLL_INTERVAL", None),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.poll_interval, DEFAULT_POLL_INTERVAL);
            },
        );
    }

    #[test]
    fn test_missing_limits_default() {
        with_vars(
            [
                ("GITHUB_TOKEN", Some("test github token")),
                ("GITHUB_GRAPHQL_URL", Some("https://api.github.com/graphql")),
                ("TELOXIDE_TOKEN", Some("test telegram bot token")),
                ("POLL_INTERVAL", Some("100")),
                ("MAX_REPOS_PER_USER", None),
                ("MAX_LABELS_PER_REPO", None),
                ("MAX_CONCURRENCY", None),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.max_repos_per_user, DEFAULT_REPOS_PER_USER);
                assert_eq!(config.max_labels_per_repo, DEFAULT_LABELS_PER_REPO);
                assert_eq!(config.max_concurrency, DEFAULT_MAX_CONCURRENCY);
            },
        );
    }
}
