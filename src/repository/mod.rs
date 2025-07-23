#[cfg(test)]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use mockall::automock;
use teloxide::types::ChatId;
use thiserror::Error;

use crate::{
    github::{GithubClient, GithubError},
    pagination::Paginated,
    storage::{RepoEntity, RepoStorage, StorageError},
};

/// Represents errors that can occur in the repository service.
#[derive(Debug, Error)]
pub enum RepositoryServiceError {
    /// An error from the GitHub client.
    #[error("Github client error")]
    GithubClientError(#[from] GithubError),

    /// An error from the storage layer.
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),

    /// An error indicating that a user has exceeded a limit.
    #[error("Limit exceeded for user: {0}")]
    LimitExceeded(String),
}

type Result<T> = std::result::Result<T, RepositoryServiceError>;

/// Represents a normalized label with its name, color, count, and selection
/// status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelNormalized {
    /// The name of the label.
    pub name: String,
    /// The color of the label.
    pub color: String,
    /// The number of issues with this label.
    pub count: i64,
    /// Whether the user is tracking this label.
    pub is_selected: bool,
}

/// A trait for managing repositories.
#[automock]
#[async_trait]
pub trait RepositoryService: Send + Sync {
    /// Check if a repository exists on GitHub.
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool>;

    /// Add a repository to the user's tracked repositories.
    /// Returns `true` if the repository was added, `false` if it was already
    /// present.
    async fn add_repo(&self, chat_id: ChatId, repo: RepoEntity) -> Result<bool>;

    /// Remove a repository from the user's tracked repositories.
    async fn remove_repo(&self, chat_id: ChatId, repo_name_with_owner: &str) -> Result<bool>;

    /// Get all repositories tracked by the user.
    async fn get_user_repos(&self, chat_id: ChatId, page: usize) -> Result<Paginated<RepoEntity>>;

    /// Get labels for a repository from GitHub, normalized and paginated.
    async fn get_repo_github_labels(
        &self,
        chat_id: ChatId,
        repo: &RepoEntity,
        page: usize,
    ) -> Result<Paginated<LabelNormalized>>;

    /// Get labels tracked by the user for a specific repository.
    async fn get_user_repo_labels(&self, chat_id: ChatId, repo: &RepoEntity)
    -> Result<Vec<String>>;

    /// Toggle a label for a repository, adding it if not present or removing it
    /// if already present.
    async fn toggle_label(
        &self,
        chat_id: ChatId,
        repo: &RepoEntity,
        label_name: &str,
    ) -> Result<bool>;
}

/// The default implementation of the `RepositoryService` trait.
pub struct DefaultRepositoryService {
    storage: Arc<dyn RepoStorage>,
    github_client: Arc<dyn GithubClient>,
    max_repos_per_user: usize,
    max_labels_per_repo: usize,
}

impl DefaultRepositoryService {
    /// Creates a new `DefaultRepositoryService`.
    pub fn new(
        storage: Arc<dyn RepoStorage>,
        github_client: Arc<dyn GithubClient>,
        max_repos_per_user: usize,
        max_labels_per_repo: usize,
    ) -> Self {
        Self { storage, github_client, max_repos_per_user, max_labels_per_repo }
    }
}

#[async_trait]
impl RepositoryService for DefaultRepositoryService {
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool> {
        self.github_client.repo_exists(owner, name).await.map_err(RepositoryServiceError::from)
    }

    async fn add_repo(&self, chat_id: ChatId, repo: RepoEntity) -> Result<bool> {
        let user_repo_count = self.storage.count_repos_per_user(chat_id).await?;

        if user_repo_count >= self.max_repos_per_user {
            return Err(RepositoryServiceError::LimitExceeded(format!(
                "User {} has reached the maximum number of repositories: {}",
                chat_id, self.max_repos_per_user
            )));
        }

        self.storage.add_repository(chat_id, repo).await.map_err(RepositoryServiceError::from)
    }

    async fn remove_repo(&self, chat_id: ChatId, repo_name_with_owner: &str) -> Result<bool> {
        self.storage
            .remove_repository(chat_id, repo_name_with_owner)
            .await
            .map_err(RepositoryServiceError::from)
    }

    async fn get_user_repos(&self, chat_id: ChatId, page: usize) -> Result<Paginated<RepoEntity>> {
        let repos =
            self.storage.get_repos_per_user(chat_id).await.map_err(RepositoryServiceError::from);
        Ok(Paginated::new(repos?, page))
    }

    async fn get_repo_github_labels(
        &self,
        chat_id: ChatId,
        repo: &RepoEntity,
        page: usize,
    ) -> Result<Paginated<LabelNormalized>> {
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

        // Filter out labels with no issues
        let selected_labels: Vec<_> = repo_labels
            .into_iter()
            .filter(|label| label.issues.as_ref().is_some_and(|issues| issues.total_count > 0))
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

        Ok(Paginated::new(normalized, page))
    }

    async fn toggle_label(
        &self,
        chat_id: ChatId,
        repo: &RepoEntity,
        label_name: &str,
    ) -> Result<bool> {
        // Check if can add more labels
        let tracked_labels = self.storage.get_tracked_labels(chat_id, repo).await?;
        let is_selected = tracked_labels.contains(label_name);

        if is_selected {
            // If label is already selected, remove it
            self.storage
                .toggle_label(chat_id, repo, label_name)
                .await
                .map_err(RepositoryServiceError::from)?;
        } else {
            // Check if user has reached the maximum number of labels per repo
            if tracked_labels.len() >= self.max_labels_per_repo {
                return Err(RepositoryServiceError::LimitExceeded(format!(
                    "User {} has reached the maximum number of labels per repository: {}",
                    chat_id, self.max_labels_per_repo
                )));
            }
            // If label is not selected, add it
            self.storage
                .toggle_label(chat_id, repo, label_name)
                .await
                .map_err(RepositoryServiceError::from)?;
        }

        Ok(!is_selected) // Return true if label was added, false if removed
    }

    async fn get_user_repo_labels(
        &self,
        chat_id: ChatId,
        repo: &RepoEntity,
    ) -> Result<Vec<String>> {
        let tracked_labels = self.storage.get_tracked_labels(chat_id, repo).await?;

        Ok(tracked_labels.into_iter().collect())
    }
}
