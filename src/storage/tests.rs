use std::str::FromStr;

use teloxide::types::ChatId;

use super::{RepoEntity, RepoStorage, sqlite::SqliteStorage};

async fn create_in_memory_storage() -> SqliteStorage {
    SqliteStorage::new("sqlite::memory:").await.unwrap()
}

#[tokio::test]
async fn test_add_and_get_repository() {
    let storage = create_in_memory_storage().await;
    let chat_id = ChatId(1);
    let repo = RepoEntity::from_str("owner/repo").unwrap();

    let result = storage.add_repository(chat_id, repo.clone()).await.unwrap();

    assert!(result);

    let repos = storage.get_repos_per_user(chat_id).await.unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0], repo);
}

#[tokio::test]
async fn test_add_same_repository() {
    let storage = create_in_memory_storage().await;
    let chat_id = ChatId(1);
    let repo = RepoEntity::from_str("owner/repo").unwrap();

    let result = storage.add_repository(chat_id, repo.clone()).await.unwrap();

    assert!(result);

    // Adding the same repository again should return false
    let result = storage.add_repository(chat_id, repo.clone()).await.unwrap();
    assert!(!result);

    let repos = storage.get_repos_per_user(chat_id).await.unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0], repo);
}

#[tokio::test]
async fn test_remove_repository() {
    let storage = create_in_memory_storage().await;
    let chat_id = ChatId(1);
    let repo = RepoEntity::from_str("owner/repo").unwrap();

    storage.add_repository(chat_id, repo.clone()).await.unwrap();
    let removed = storage.remove_repository(chat_id, &repo.name_with_owner).await.unwrap();
    assert!(removed);

    let repos = storage.get_repos_per_user(chat_id).await.unwrap();
    assert!(repos.is_empty());
}

#[tokio::test]
async fn test_get_all_repos() {
    let storage = create_in_memory_storage().await;
    let chat_id1 = ChatId(1);
    let chat_id2 = ChatId(2);
    let repo1 = RepoEntity::from_str("owner/repo1").unwrap();
    let repo2 = RepoEntity::from_str("owner/repo2").unwrap();

    storage.add_repository(chat_id1, repo1.clone()).await.unwrap();
    storage.add_repository(chat_id2, repo2.clone()).await.unwrap();

    let all_repos = storage.get_all_repos().await.unwrap();
    assert_eq!(all_repos.len(), 2);
    assert!(all_repos.get(&chat_id1).unwrap().contains(&repo1));
    assert!(all_repos.get(&chat_id2).unwrap().contains(&repo2));
}

#[tokio::test]
async fn test_poll_time() {
    let storage = create_in_memory_storage().await;
    let chat_id = ChatId(1);
    let repo = RepoEntity::from_str("owner/repo").unwrap();

    storage.add_repository(chat_id, repo.clone()).await.unwrap();
    storage.set_last_poll_time(chat_id, &repo).await.unwrap();
    let last_poll_time = storage.get_last_poll_time(chat_id, &repo).await.unwrap();

    assert!(last_poll_time.is_some());
}

#[tokio::test]
async fn test_toggle_labels() {
    let storage = create_in_memory_storage().await;
    let chat_id = ChatId(1);
    let repo = RepoEntity::from_str("owner/repo").unwrap();

    storage.add_repository(chat_id, repo.clone()).await.unwrap();

    let is_selected = storage.toggle_label(chat_id, &repo, "bug").await.unwrap();
    assert!(is_selected);

    let labels = storage.get_tracked_labels(chat_id, &repo).await.unwrap();
    assert!(labels.contains("bug"));

    let is_selected = storage.toggle_label(chat_id, &repo, "bug").await.unwrap();
    assert!(!is_selected);

    let labels = storage.get_tracked_labels(chat_id, &repo).await.unwrap();
    assert!(!labels.contains("bug"));
}

#[tokio::test]
async fn test_count_repos_per_user() {
    let storage = create_in_memory_storage().await;
    let chat_id = ChatId(1);
    let repo1 = RepoEntity::from_str("owner/repo1").unwrap();
    let repo2 = RepoEntity::from_str("owner/repo2").unwrap();

    storage.add_repository(chat_id, repo1.clone()).await.unwrap();
    storage.add_repository(chat_id, repo2.clone()).await.unwrap();

    let count = storage.count_repos_per_user(chat_id).await.unwrap();
    assert_eq!(count, 2);
}
