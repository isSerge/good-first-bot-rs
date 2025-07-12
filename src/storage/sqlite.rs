use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use async_trait::async_trait;
use chrono::Utc;
use serde_json;
use sqlx::{Pool, Sqlite, SqlitePool, migrate, query};
use teloxide::types::ChatId;

use crate::storage::{RepoEntity, RepoStorage, StorageError, StorageResult};

const INITIAL_DEFAULT_LABELS_JSON: &str =
    r#"["good first issue","beginner-friendly","help wanted"]"#;

pub struct SqliteStorage {
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    pub async fn new(database_url: &str) -> StorageResult<Self> {
        tracing::debug!("Connecting to SQLite database: {database_url}");
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| StorageError::DbError(format!("Failed to connect to SQLite: {e}")))?;

        migrate!("./migrations").run(&pool).await.map_err(|e| {
            StorageError::DbError(format!("Failed to migrate SQLite database: {e}"))
        })?;
        tracing::debug!("SQLite database migrated");

        Ok(Self { pool })
    }
}

#[async_trait]
impl RepoStorage for SqliteStorage {
    async fn add_repository(&self, chat_id: ChatId, repository: RepoEntity) -> StorageResult<bool> {
        tracing::debug!("Adding repository to SQLite: {:?}", repository);

        let chat_id = chat_id.0;

        let result = query!(
            "INSERT OR IGNORE INTO repositories (chat_id, owner, name, name_with_owner, \
             tracked_labels) VALUES (?, ?, ?, ?, ?)",
            chat_id,
            repository.owner,
            repository.name,
            repository.name_with_owner,
            INITIAL_DEFAULT_LABELS_JSON,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DbError(format!("Failed to add repository to SQLite: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    async fn remove_repository(
        &self,
        chat_id: ChatId,
        name_with_owner: &str,
    ) -> StorageResult<bool> {
        tracing::debug!("Removing repository from SQLite: {}", name_with_owner);

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

    async fn get_repos_per_user(&self, chat_id: ChatId) -> StorageResult<Vec<RepoEntity>> {
        tracing::debug!("Getting repositories for user: {}", chat_id);

        let repos = query!(
            "SELECT owner, name, name_with_owner 
            FROM repositories 
            WHERE chat_id = ?
            ORDER BY LOWER(name_with_owner) ASC",
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
                RepoEntity::from_str(&r.name_with_owner).map_err(|e| {
                    StorageError::DataIntegrityError(r.name_with_owner.clone(), e.into())
                })
            })
            .collect::<Result<Vec<RepoEntity>, _>>()?;

        Ok(repos)
    }

    async fn get_all_repos(&self) -> StorageResult<HashMap<ChatId, HashSet<RepoEntity>>> {
        tracing::debug!("Getting all repositories from SQLite");

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
        tracing::debug!("Getting last poll time for repository: {:?}", repository);
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
        tracing::debug!("Setting last poll time for repository: {:?}", repository);
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

    async fn get_tracked_labels(
        &self,
        chat_id: ChatId,
        repository: &RepoEntity,
    ) -> StorageResult<HashSet<String>> {
        tracing::debug!("Getting tracked labels for repository: {}", repository.name_with_owner);
        let chat_id_i64 = chat_id.0;

        let raw_result = query!(
            "SELECT tracked_labels FROM repositories WHERE chat_id = ? AND name_with_owner = ?",
            chat_id_i64,
            repository.name_with_owner,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            StorageError::DbError(format!("Failed to get tracked labels from SQLite: {e}"))
        })?;

        let labels_str = raw_result.tracked_labels.unwrap_or("[]".to_string());
        let labels: HashSet<String> = serde_json::from_str(&labels_str).map_err(|e| {
            StorageError::DataIntegrityError(repository.name_with_owner.clone(), e.into())
        })?;
        tracing::debug!(
            "Tracked labels for repository {}: {:?}",
            repository.name_with_owner,
            labels
        );

        Ok(labels)
    }

    async fn toggle_label(
        &self,
        chat_id: ChatId,
        repository: &RepoEntity,
        label_name: &str,
    ) -> StorageResult<bool> {
        tracing::debug!("Toggling label for repository: {}", repository.name_with_owner);
        let chat_id_i64 = chat_id.0;

        let mut tracked_labels = self.get_tracked_labels(chat_id, repository).await?;

        if tracked_labels.contains(label_name) {
            tracked_labels.remove(label_name);
        } else {
            tracked_labels.insert(label_name.to_string());
        }

        let labels_str = serde_json::to_string(&tracked_labels).map_err(|e| {
            StorageError::DataIntegrityError(repository.name_with_owner.clone(), e.into())
        })?;

        query!(
            "UPDATE repositories SET tracked_labels = ? WHERE chat_id = ? AND name_with_owner = ?",
            labels_str,
            chat_id_i64,
            repository.name_with_owner,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DbError(format!("Failed to toggle label in SQLite: {e}")))?;

        Ok(tracked_labels.contains(label_name))
    }

    async fn count_repos_per_user(&self, chat_id: ChatId) -> StorageResult<usize> {
        tracing::debug!("Counting repositories for user: {}", chat_id);

        let result =
            query!("SELECT COUNT(*) as count FROM repositories WHERE chat_id = ?", chat_id.0,)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    StorageError::DbError(format!(
                        "Failed to count repositories in SQLite for user {}: {}",
                        chat_id.0, e
                    ))
                })?;

        Ok(result.count.try_into().unwrap_or(0))
    }
}
