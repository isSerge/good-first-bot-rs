mod repo_entity;
pub mod sqlite;

use std::collections::{HashMap, HashSet};

use anyhow::{Error, Result};
use async_trait::async_trait;
use mockall::automock;
pub use repo_entity::RepoEntity;
use teloxide::types::ChatId;

#[automock]
#[async_trait]
pub trait RepoStorage: Send + Sync {
    /// Add a repository to the storage.
    async fn add_repository(&self, chat_id: ChatId, repository: RepoEntity) -> Result<(), Error>;

    /// Remove a repository from the storage.
    async fn remove_repository(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: &str,
    ) -> Result<bool, Error>;

    /// Get all repositories for a user.
    async fn get_repos_per_user(&self, chat_id: ChatId) -> Result<HashSet<RepoEntity>, Error>;

    /// Check if a repository exists in the storage.
    async fn contains(&self, chat_id: ChatId, repository: &RepoEntity) -> Result<bool, Error>;

    /// Get all repositories from the storage.
    async fn get_all_repos(&self) -> Result<HashMap<ChatId, HashSet<RepoEntity>>, Error>;

    /// Get the last poll time for a repository.
    async fn get_last_poll_time(
        &self,
        chat_id: ChatId,
        repository: &RepoEntity,
    ) -> Result<Option<i64>, Error>;

    /// Set the last poll time for a repository.
    async fn set_last_poll_time(
        &self,
        chat_id: ChatId,
        repository: &RepoEntity,
    ) -> Result<(), Error>;
}
