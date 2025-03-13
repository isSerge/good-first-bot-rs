use std::collections::HashMap;
use std::sync::Arc;
use teloxide::types::ChatId;
use tokio::sync::Mutex;

pub type Storage = Arc<Mutex<HashMap<ChatId, Vec<Repository>>>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Repository {
    pub url: String,
    pub name_with_owner: String,
}

impl Repository {
    pub fn new(name_with_owner: String, url: String) -> Self {
        Self {
            name_with_owner,
            url,
        }
    }
}
