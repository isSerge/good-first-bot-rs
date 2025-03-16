use crate::storage::{RepoStorage, Repository};
use anyhow::Result;
use async_trait::async_trait;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use teloxide::types::ChatId;
use tokio::sync::Mutex;

/// Represents a storage system for tracking repositories per chat.
/// Uses an in-memory HashMap wrapped in an async Mutex for thread-safe access.
#[derive(Default)]
pub struct InMemoryStorage {
    data: Arc<Mutex<HashMap<ChatId, HashSet<Repository>>>>,
}

#[async_trait]
impl RepoStorage for InMemoryStorage {
    /// Add a repository to the storage.
    async fn add_repository(&self, chat_id: ChatId, repository: Repository) -> Result<()> {
        debug!("Adding repository to storage: {:?}", repository);
        let mut data = self.data.lock().await;
        data.entry(chat_id)
            .or_insert_with(HashSet::new)
            .insert(repository);
        Ok(())
    }

    /// Remove a repository from the storage.
    async fn remove_repository(&self, chat_id: ChatId, repo_name: &str) -> Result<bool> {
        debug!("Removing repository from storage: {}", repo_name);
        let mut data = self.data.lock().await;
        if let Some(repos) = data.get_mut(&chat_id) {
            let initial_len = repos.len();
            repos.retain(|r| r.full_name != repo_name);
            Ok(repos.len() != initial_len)
        } else {
            Ok(false)
        }
    }

    async fn get_repos_per_user(&self, chat_id: ChatId) -> Result<HashSet<Repository>> {
        debug!("Getting repositories for user: {}", chat_id);
        let data = self
            .data
            .lock()
            .await
            .get(&chat_id)
            .cloned()
            .unwrap_or_default();
        Ok(data)
    }

    async fn contains(&self, chat_id: ChatId, repository: &Repository) -> Result<bool> {
        debug!("Checking if repository is in storage: {:?}", repository);
        let data = self.data.lock().await;
        let contains = data
            .get(&chat_id)
            .map(|repos| repos.contains(repository))
            .unwrap_or(false);

        Ok(contains)
    }

    async fn get_all_repos(&self) -> Result<HashMap<ChatId, HashSet<Repository>>> {
        debug!("Getting all repositories");
        let data = self.data.lock().await.clone();
        Ok(data)
    }
}
