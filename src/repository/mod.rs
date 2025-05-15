#[cfg(test)]
mod tests;

use std::{collections::HashSet, sync::Arc};

use async_trait::async_trait;
use mockall::automock;
use teloxide::types::ChatId;
use thiserror::Error;

use crate::{
    github::GithubClient,
    storage::{RepoEntity, RepoStorage, StorageError},
};

#[derive(Debug, Error)]
pub enum RepositoryServiceError {
    #[error("Failed to check if repository exists")]
    RepoExistCheckFailed,
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
}

type Result<T> = std::result::Result<T, RepositoryServiceError>;

#[automock]
#[async_trait]
pub trait RepositoryService: Send + Sync {
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool>;
    async fn contains_repo(&self, chat_id: ChatId, repo: &RepoEntity) -> Result<bool>;
    async fn add_repo(&self, chat_id: ChatId, repo: RepoEntity) -> Result<()>;
    async fn remove_repo(&self, chat_id: ChatId, repo_name_with_owner: &str) -> Result<bool>;
    async fn get_user_repos(&self, chat_id: ChatId) -> Result<HashSet<RepoEntity>>;
}

pub struct DefaultRepositoryService {
    storage: Arc<dyn RepoStorage>,
    github_client: Arc<dyn GithubClient>,
}

impl DefaultRepositoryService {
    pub fn new(storage: Arc<dyn RepoStorage>, github_client: Arc<dyn GithubClient>) -> Self {
        Self { storage, github_client }
    }
}

#[async_trait]
impl RepositoryService for DefaultRepositoryService {
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool> {
        self.github_client
            .repo_exists(owner, name)
            .await
            .map_err(|_| RepositoryServiceError::RepoExistCheckFailed)
    }

    async fn contains_repo(&self, chat_id: ChatId, repo: &RepoEntity) -> Result<bool> {
        self.storage.contains(chat_id, repo).await.map_err(RepositoryServiceError::from)
    }

    async fn add_repo(&self, chat_id: ChatId, repo: RepoEntity) -> Result<()> {
        self.storage.add_repository(chat_id, repo).await.map_err(RepositoryServiceError::from)
    }

    async fn remove_repo(&self, chat_id: ChatId, repo_name_with_owner: &str) -> Result<bool> {
        self.storage
            .remove_repository(chat_id, repo_name_with_owner)
            .await
            .map_err(RepositoryServiceError::from)
    }

    async fn get_user_repos(&self, chat_id: ChatId) -> Result<HashSet<RepoEntity>> {
        self.storage.get_repos_per_user(chat_id).await.map_err(RepositoryServiceError::from)
    }
}
