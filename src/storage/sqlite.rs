use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use async_trait::async_trait;
use chrono::Utc;
use log::debug;
use sqlx::{Pool, Sqlite, SqlitePool, migrate, query};
use teloxide::types::ChatId;

use crate::storage::{RepoEntity, RepoStorage, StorageError, StorageResult};

pub struct SqliteStorage {
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    pub async fn new(database_url: &str) -> StorageResult<Self> {
        debug!("Connecting to SQLite database: {database_url}");
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| StorageError::DbError(format!("Failed to connect to SQLite: {e}")))?;

        migrate!("./migrations").run(&pool).await.map_err(|e| {
            StorageError::DbError(format!("Failed to migrate SQLite database: {e}"))
        })?;
        debug!("SQLite database migrated");

        Ok(Self { pool })
    }
}

#[async_trait]
impl RepoStorage for SqliteStorage {
    async fn add_repository(&self, chat_id: ChatId, repository: RepoEntity) -> StorageResult<()> {
        debug!("Adding repository to SQLite: {:?}", repository);

        let chat_id = chat_id.0;

        query!(
            "INSERT OR IGNORE INTO repositories (chat_id, owner, name, name_with_owner) VALUES \
             (?, ?, ?, ?)",
            chat_id,
            repository.owner,
            repository.name,
            repository.name_with_owner,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DbError(format!("Failed to add repository to SQLite: {e}")))?;

        Ok(())
    }

    async fn remove_repository(
        &self,
        chat_id: ChatId,
        name_with_owner: &str,
    ) -> StorageResult<bool> {
        debug!("Removing repository from SQLite: {}", name_with_owner);

        let chat_id = chat_id.0;

        let result = query!(
            "DELETE FROM repositories WHERE chat_id = ? AND name_with_owner = ?",
            chat_id,
            name_with_owner,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StorageError::DbError(format!("Failed to remove repository from SQLite: {e}"))
        })?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_repos_per_user(&self, chat_id: ChatId) -> StorageResult<HashSet<RepoEntity>> {
        debug!("Getting repositories for user: {}", chat_id);

        let repos = query!(
            "SELECT owner, name, name_with_owner FROM repositories WHERE chat_id = ?",
            chat_id.0,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::DbError(format!("Failed to fetch repos for user {}: {}", chat_id.0, e))
        })?;

        let repos = repos
            .into_iter()
            .map(|r| {
                RepoEntity::from_str(&r.name_with_owner)
                    .map_err(|e| StorageError::DataIntegrityError(r.name_with_owner.clone(), e))
            })
            .collect::<Result<HashSet<_>, _>>()?;

        Ok(repos)
    }

    async fn contains(&self, chat_id: ChatId, repository: &RepoEntity) -> StorageResult<bool> {
        debug!("Checking if repository exists in SQLite: {:?}", repository);
        let chat_id = chat_id.0;

        let result = query!(
            "SELECT COUNT(*) as count FROM repositories WHERE chat_id = ? AND name_with_owner = ?",
            chat_id,
            repository.name_with_owner,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::DbError(format!("Failed to check repository in SQLite: {e}")))?;

        Ok(result.count > 0)
    }

    async fn get_all_repos(&self) -> StorageResult<HashMap<ChatId, HashSet<RepoEntity>>> {
        debug!("Getting all repositories from SQLite");

        let repos = query!("SELECT chat_id, owner, name, name_with_owner FROM repositories",)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                StorageError::DbError(format!("Failed to get all repositories from SQLite: {e}"))
            })?;

        let mut result = HashMap::new();
        for r in repos {
            let chat_id = ChatId(r.chat_id);
            let repo =
                RepoEntity { owner: r.owner, name: r.name, name_with_owner: r.name_with_owner };
            result.entry(chat_id).or_insert_with(HashSet::new).insert(repo);
        }

        Ok(result)
    }

    async fn get_last_poll_time(
        &self,
        chat_id: ChatId,
        repository: &RepoEntity,
    ) -> StorageResult<Option<i64>> {
        debug!("Getting last poll time for repository: {:?}", repository);
        let chat_id = chat_id.0;

        let result = query!(
            "SELECT last_poll_time FROM poller_states WHERE chat_id = ? AND repository_full_name \
             = ?",
            chat_id,
            repository.name_with_owner,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            StorageError::DbError(format!("Failed to get last poll time from SQLite: {e}"))
        })?;

        // If the repository is not found, return None
        Ok(result.map(|r| r.last_poll_time))
    }

    async fn set_last_poll_time(
        &self,
        chat_id: ChatId,
        repository: &RepoEntity,
    ) -> StorageResult<()> {
        debug!("Setting last poll time for repository: {:?}", repository);
        let chat_id = chat_id.0;

        let current_time = Utc::now().timestamp();

        query!(
            "INSERT OR REPLACE INTO poller_states (chat_id, repository_full_name, last_poll_time) \
             VALUES (?, ?, ?)",
            chat_id,
            repository.name_with_owner,
            current_time,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StorageError::DbError(format!("Failed to set last poll time in SQLite: {e}"))
        })?;

        Ok(())
    }
}
