mod repository;
pub mod sqlite;

use anyhow::{Error, Result};
use async_trait::async_trait;
pub use repository::Repository;
use std::collections::{HashMap, HashSet};
use teloxide::types::ChatId;

#[async_trait]
pub trait RepoStorage: Send + Sync {
    #[must_use = "This function returns a boolean that should not be ignored"]
    async fn add_repository(&self, chat_id: ChatId, repository: Repository) -> Result<(), Error>;
    #[must_use = "This function returns a boolean that should not be ignored"]
    async fn remove_repository(&self, chat_id: ChatId, repo_name: &str) -> Result<bool, Error>;
    #[must_use = "This function returns a boolean that should not be ignored"]
    async fn get_repos_per_user(&self, chat_id: ChatId) -> Result<HashSet<Repository>, Error>;
    #[must_use = "This function returns a boolean that should not be ignored"]
    async fn contains(&self, chat_id: ChatId, repository: &Repository) -> Result<bool, Error>;
    #[must_use = "This function returns a boolean that should not be ignored"]
    async fn get_all_repos(&self) -> Result<HashMap<ChatId, HashSet<Repository>>, Error>;
}
