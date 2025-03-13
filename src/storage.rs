use std::collections::HashMap;
use std::sync::Arc;
use teloxide::types::ChatId;
use tokio::sync::Mutex;

pub type Storage = Arc<Mutex<HashMap<ChatId, Vec<String>>>>;
