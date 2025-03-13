mod repository;

use std::collections::HashMap;
use std::sync::Arc;
use teloxide::types::ChatId;
use tokio::sync::Mutex;

pub use repository::Repository;
pub struct Storage {
    data: Arc<Mutex<HashMap<ChatId, Vec<Repository>>>>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_repository(&self, chat_id: ChatId, repository: Repository) {
        let mut data = self.data.lock().await;
        data.entry(chat_id)
            .or_insert_with(Vec::new)
            .push(repository);
    }

    pub async fn remove_repository(&self, chat_id: ChatId, repo_name: &str) -> bool {
        let mut data = self.data.lock().await;
        if let Some(repos) = data.get_mut(&chat_id) {
            let initial_len = repos.len();
            repos.retain(|r| r.name_with_owner != repo_name);
            repos.len() != initial_len
        } else {
            false
        }
    }

    pub async fn get_repositories(&self, chat_id: ChatId) -> Vec<Repository> {
        self.data
            .lock()
            .await
            .get(&chat_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn contains(&self, chat_id: &ChatId, repository: &Repository) -> bool {
        let data = self.data.lock().await;
        data.get(chat_id)
            .map(|repos| repos.contains(repository))
            .unwrap_or(false)
    }
}
