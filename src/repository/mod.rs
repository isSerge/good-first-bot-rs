#[cfg(test)]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use mockall::automock;
use teloxide::types::ChatId;
use thiserror::Error;

use crate::{
    github::{GithubClient, GithubError},
    storage::{RepoEntity, RepoStorage, StorageError},
};

#[derive(Debug, Error)]
pub enum RepositoryServiceError {
    #[error("Github client error")]
    GithubClientError(#[from] GithubError),
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
}

type Result<T> = std::result::Result<T, RepositoryServiceError>;

/// Represents a normalized label with its name, color, count, and selection
/// status.
pub struct LabelNormalized {
    pub name: String,
    pub color: String,
    pub count: i64,
    pub is_selected: bool,
}

#[automock]
#[async_trait]
pub trait RepositoryService: Send + Sync {
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool>;
    async fn contains_repo(&self, chat_id: ChatId, repo: &RepoEntity) -> Result<bool>;
    async fn add_repo(&self, chat_id: ChatId, repo: RepoEntity) -> Result<()>;
    async fn remove_repo(&self, chat_id: ChatId, repo_name_with_owner: &str) -> Result<bool>;
    async fn get_user_repos(&self, chat_id: ChatId) -> Result<Vec<RepoEntity>>;
    async fn get_repo_labels(
        &self,
        chat_id: ChatId,
        repo: &RepoEntity,
    ) -> Result<Vec<LabelNormalized>>;
    async fn toggle_label(
        &self,
        chat_id: ChatId,
        repo: &RepoEntity,
        label_name: &str,
    ) -> Result<bool>;
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
        self.github_client.repo_exists(owner, name).await.map_err(RepositoryServiceError::from)
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

    async fn get_user_repos(&self, chat_id: ChatId) -> Result<Vec<RepoEntity>> {
        self.storage.get_repos_per_user(chat_id).await.map_err(RepositoryServiceError::from)
    }

    async fn get_repo_labels(
        &self,
        chat_id: ChatId,
        repo: &RepoEntity,
    ) -> Result<Vec<LabelNormalized>> {
        // Get tracked labels from storage
        let tracked_labels = self.storage.get_tracked_labels(chat_id, repo).await?;

        // Get repo labels from GitHub
        let mut repo_labels = self.github_client.repo_labels(&repo.owner, &repo.name).await?;

        // Sort repo labels by issue count
        repo_labels.sort_by(|a, b| {
            let count_a = a.issues.as_ref().map_or(0, |issues| issues.total_count);
            let count_b = b.issues.as_ref().map_or(0, |issues| issues.total_count);
            count_b.cmp(&count_a)
        });

        // TODO: consider using pagination to get all labels with issues
        // Take up to 20 labels with more than 0 issues
        let selected_labels: Vec<_> = repo_labels
            .into_iter()
            .filter(|label| label.issues.as_ref().is_some_and(|issues| issues.total_count > 0))
            .take(20)
            .collect();

        let normalized = selected_labels
            .into_iter()
            .map(|label| {
                let name = label.name.clone();
                let color = label.color.clone();
                let count = label.issues.map_or(0, |issues| issues.total_count);
                let is_selected = tracked_labels.contains(&label.name);
                LabelNormalized { name, color, count, is_selected }
            })
            .collect();

        Ok(normalized)
    }

    async fn toggle_label(
        &self,
        chat_id: ChatId,
        repo: &RepoEntity,
        label_name: &str,
    ) -> Result<bool> {
        let is_selected = self.storage.toggle_label(chat_id, repo, label_name).await?;

        Ok(is_selected)
    }
}
