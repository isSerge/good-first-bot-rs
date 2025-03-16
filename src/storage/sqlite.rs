use crate::storage::{RepoStorage, Repository};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use log::debug;
use sqlx::{Pool, Sqlite, SqlitePool, migrate, query};
use std::collections::{HashMap, HashSet};
use teloxide::types::ChatId;

pub struct SqliteStorage {
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    pub async fn new(database_url: &str) -> Result<Self> {
        debug!("Connecting to SQLite database: {}", database_url);
        let pool = SqlitePool::connect(database_url).await?;

        migrate!("./migrations").run(&pool).await?;
        debug!("SQLite database migrated");

        Ok(Self { pool })
    }
}

#[async_trait]
impl RepoStorage for SqliteStorage {
    async fn add_repository(&self, chat_id: ChatId, repository: Repository) -> Result<()> {
        debug!("Adding repository to SQLite: {:?}", repository);

        let chat_id = chat_id.0;

        query!(
            "INSERT OR IGNORE INTO repositories (chat_id, owner, name, name_with_owner) VALUES (?, ?, ?, ?)",
            chat_id,
            repository.owner,
            repository.name,
            repository.name_with_owner,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove_repository(&self, chat_id: ChatId, name_with_owner: &str) -> Result<bool> {
        debug!("Removing repository from SQLite: {}", name_with_owner);

        let chat_id = chat_id.0;

        let result = query!(
            "DELETE FROM repositories WHERE chat_id = ? AND name_with_owner = ?",
            chat_id,
            name_with_owner,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_repos_per_user(&self, chat_id: ChatId) -> Result<HashSet<Repository>> {
        debug!("Getting repositories for user: {}", chat_id);

        let repos = query!(
            "SELECT owner, name, name_with_owner FROM repositories WHERE chat_id = ?",
            chat_id.0,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(repos
            .into_iter()
            .map(|r| Repository {
                owner: r.owner,
                name: r.name,
                name_with_owner: r.name_with_owner,
            })
            .collect())
    }

    async fn contains(&self, chat_id: ChatId, repository: &Repository) -> Result<bool> {
        debug!("Checking if repository exists in SQLite: {:?}", repository);
        let chat_id = chat_id.0;

        let result = query!(
            "SELECT COUNT(*) as count FROM repositories WHERE chat_id = ? AND name_with_owner = ?",
            chat_id,
            repository.name_with_owner,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.count > 0)
    }

    async fn get_all_repos(&self) -> Result<HashMap<ChatId, HashSet<Repository>>> {
        debug!("Getting all repositories from SQLite");

        let repos = query!("SELECT chat_id, owner, name, name_with_owner FROM repositories",)
            .fetch_all(&self.pool)
            .await?;

        let mut result = HashMap::new();
        for r in repos {
            let chat_id = ChatId(r.chat_id);
            let repo = Repository {
                owner: r.owner,
                name: r.name,
                name_with_owner: r.name_with_owner,
            };
            result
                .entry(chat_id)
                .or_insert_with(HashSet::new)
                .insert(repo);
        }

        Ok(result)
    }

    async fn get_last_poll_time(&self, chat_id: ChatId, repository: &Repository) -> Result<i64> {
        debug!("Getting last poll time for repository: {:?}", repository);
        let chat_id = chat_id.0;

        let result = query!(
            "SELECT last_poll_time FROM poller_states WHERE chat_id = ? AND repository_full_name = ?",
            chat_id,
            repository.name_with_owner,
        )
        .fetch_optional(&self.pool)
        .await?;

        // If the repository is not found, return 0
        Ok(result.map_or(0, |r| r.last_poll_time))
    }

    async fn set_last_poll_time(&self, chat_id: ChatId, repository: &Repository) -> Result<()> {
        debug!("Setting last poll time for repository: {:?}", repository);
        let chat_id = chat_id.0;

        let current_time = Utc::now().timestamp();

        query!(
            "INSERT OR REPLACE INTO poller_states (chat_id, repository_full_name, last_poll_time) VALUES (?, ?, ?)",
            chat_id,
            repository.name_with_owner,
            current_time,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
