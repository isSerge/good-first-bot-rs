use crate::github::GithubClient;
use crate::storage::{RepoStorage, Repository};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use teloxide::types::ChatId;

#[async_trait]
pub trait RepositoryService: Send + Sync {
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool>;
    async fn storage_contains(&self, chat_id: ChatId, repo: &Repository) -> Result<bool>;
    async fn add_repo(&self, chat_id: ChatId, repo: Repository) -> Result<()>;
    async fn remove_repo(&self, chat_id: ChatId, repo: Repository) -> Result<bool>;
    async fn get_user_repos(&self, chat_id: ChatId) -> Result<HashSet<Repository>>;
}

pub struct DefaultRepositoryService {
    storage: Arc<dyn RepoStorage>,
    github_client: Arc<GithubClient>,
}

impl DefaultRepositoryService {
    pub fn new(storage: Arc<dyn RepoStorage>, github_client: Arc<GithubClient>) -> Self {
        Self {
            storage,
            github_client,
        }
    }
}

#[async_trait]
impl RepositoryService for DefaultRepositoryService {
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool> {
        self.github_client.repo_exists(owner, name).await
    }

    async fn storage_contains(&self, chat_id: ChatId, repo: &Repository) -> Result<bool> {
        self.storage.contains(chat_id, repo).await
    }

    async fn add_repo(&self, chat_id: ChatId, repo: Repository) -> Result<()> {
        self.storage.add_repository(chat_id, repo).await
    }

    async fn remove_repo(&self, chat_id: ChatId, repo: Repository) -> Result<bool> {
        self.storage
            .remove_repository(chat_id, &repo.name_with_owner)
            .await
    }

    async fn get_user_repos(&self, chat_id: ChatId) -> Result<HashSet<Repository>> {
        self.storage.get_repos_per_user(chat_id).await
    }
}
