mod repo_entity;
pub mod sqlite;

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use mockall::automock;
pub use repo_entity::RepoEntity;
use teloxide::types::ChatId;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DbError(String),
    #[error("Data integrity error: Stored repository '{0}' is invalid: {1}")]
    DataIntegrityError(String, #[source] repo_entity::RepoEntityError),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[automock]
#[async_trait]
pub trait RepoStorage: Send + Sync {
    /// Add a repository to the storage.
    async fn add_repository(&self, chat_id: ChatId, repository: RepoEntity) -> StorageResult<()>;

    /// Remove a repository from the storage.
    async fn remove_repository(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: &str,
    ) -> StorageResult<bool>;

    /// Get all repositories for a user.
    async fn get_repos_per_user(&self, chat_id: ChatId) -> StorageResult<HashSet<RepoEntity>>;

    /// Check if a repository exists in the storage.
    async fn contains(&self, chat_id: ChatId, repository: &RepoEntity) -> StorageResult<bool>;

    /// Get all repositories from the storage.
    async fn get_all_repos(&self) -> StorageResult<HashMap<ChatId, HashSet<RepoEntity>>>;

    /// Get the last poll time for a repository.
    async fn get_last_poll_time(
        &self,
        chat_id: ChatId,
        repository: &RepoEntity,
    ) -> StorageResult<Option<i64>>;

    /// Set the last poll time for a repository.
    async fn set_last_poll_time(
        &self,
        chat_id: ChatId,
        repository: &RepoEntity,
    ) -> StorageResult<()>;
}
