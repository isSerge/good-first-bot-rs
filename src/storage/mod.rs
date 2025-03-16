mod repository;
pub mod sqlite;

use anyhow::{Error, Result};
use async_trait::async_trait;
pub use repository::Repository;
use std::collections::{HashMap, HashSet};
use teloxide::types::ChatId;

#[async_trait]
pub trait RepoStorage: Send + Sync {
    /// Add a repository to the storage.
    #[must_use = "This function returns a result that should not be ignored"]
    async fn add_repository(&self, chat_id: ChatId, repository: Repository) -> Result<(), Error>;

    /// Remove a repository from the storage.
    #[must_use = "This function returns a result that should not be ignored"]
    async fn remove_repository(&self, chat_id: ChatId, repo_name: &str) -> Result<bool, Error>;

    /// Get all repositories for a user.
    #[must_use = "This function returns a result that should not be ignored"]
    async fn get_repos_per_user(&self, chat_id: ChatId) -> Result<HashSet<Repository>, Error>;

    /// Check if a repository exists in the storage.
    async fn contains(&self, chat_id: ChatId, repository: &Repository) -> Result<bool, Error>;

    /// Get all repositories from the storage.
    #[must_use = "This function returns a result that should not be ignored"]
    async fn get_all_repos(&self) -> Result<HashMap<ChatId, HashSet<Repository>>, Error>;

    /// Get the last poll time for a repository.
    #[must_use = "This function returns a result that should not be ignored"]
    async fn get_last_poll_time(
        &self,
        chat_id: ChatId,
        repository: &Repository,
    ) -> Result<i64, Error>;

    /// Set the last poll time for a repository.
    #[must_use = "This function returns a result that should not be ignored"]
    async fn set_last_poll_time(
        &self,
        chat_id: ChatId,
        repository: &Repository,
    ) -> Result<(), Error>;
}
