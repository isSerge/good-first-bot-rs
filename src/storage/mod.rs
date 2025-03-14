mod repository;

use log::debug;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use teloxide::types::ChatId;
use tokio::sync::Mutex;

pub use repository::Repository;

/// Represents a storage system for tracking repositories per chat.
/// Uses an in-memory HashMap wrapped in an async Mutex for thread-safe access.
#[derive(Default)]
pub struct Storage {
    data: Arc<Mutex<HashMap<ChatId, HashSet<Repository>>>>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a repository to the storage.
    pub async fn add_repository(&self, chat_id: ChatId, repository: Repository) {
        debug!("Adding repository to storage: {:?}", repository);
        let mut data = self.data.lock().await;
        data.entry(chat_id)
            .or_insert_with(HashSet::new)
            .insert(repository);
    }

    /// Remove a repository from the storage.
    #[must_use = "This function returns a Result that should not be ignored"]
    pub async fn remove_repository(&self, chat_id: ChatId, repo_name: &str) -> bool {
        debug!("Removing repository from storage: {}", repo_name);
        let mut data = self.data.lock().await;
        if let Some(repos) = data.get_mut(&chat_id) {
            let initial_len = repos.len();
            repos.retain(|r| r.full_name != repo_name);
            repos.len() != initial_len
        } else {
            false
        }
    }

    pub async fn get_repos_per_user(&self, chat_id: ChatId) -> HashSet<Repository> {
        debug!("Getting repositories for user: {}", chat_id);
        self.data
            .lock()
            .await
            .get(&chat_id)
            .cloned()
            .unwrap_or_default()
    }

    #[must_use = "This function returns a boolean that should not be ignored"]
    pub async fn contains(&self, chat_id: ChatId, repository: &Repository) -> bool {
        debug!("Checking if repository is in storage: {:?}", repository);
        let data = self.data.lock().await;
        data.get(&chat_id)
            .map(|repos| repos.contains(repository))
            .unwrap_or(false)
    }

    pub async fn get_all_repos(&self) -> HashMap<ChatId, HashSet<Repository>> {
        debug!("Getting all repositories");
        self.data.lock().await.clone()
    }
}
