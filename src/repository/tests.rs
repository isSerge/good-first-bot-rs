use std::{str::FromStr, sync::Arc, collections::HashSet};

use mockall::predicate::eq;

use super::*;
use crate::{
    github::{GithubError, MockGithubClient, labels},
    storage::{MockRepoStorage, RepoEntity},
};

#[tokio::test]
async fn test_repo_exists() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    mock_github_client.expect_repo_exists().returning(|_, _| Ok(true));
    let mock_repo_storage = MockRepoStorage::new();
    let repository_service =
        DefaultRepositoryService::new(Arc::new(mock_repo_storage), Arc::new(mock_github_client));

    // Act
    let repo_exists = repository_service.repo_exists("owner", "repo").await;

    // Assert
    assert!(repo_exists.is_ok());
    assert!(repo_exists.unwrap());
}

#[tokio::test]
async fn test_contains_repo() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage.expect_contains().returning(|_, _| Ok(true));
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(MockGithubClient::new()),
    );

    let repo = RepoEntity::from_str("owner/repo").unwrap();

    // Act
    let contains = repository_service.contains_repo(ChatId(1), &repo).await;

    // Assert
    assert!(contains.is_ok());
    assert!(contains.unwrap());
}

#[tokio::test]
async fn test_add_repo() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage.expect_add_repository().returning(|_, _| Ok(()));
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(MockGithubClient::new()),
    );

    // Act
    let repo = RepoEntity::from_str("owner/repo").unwrap();
    let result = repository_service.add_repo(ChatId(1), repo).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_remove_repo() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage.expect_remove_repository().returning(|_, _| Ok(true));

    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(MockGithubClient::new()),
    );

    // Act
    let result = repository_service.remove_repo(ChatId(1), "owner/repo").await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_user_repos() {
    // Arrange
    let repo1 = RepoEntity::from_str("owner/repo").unwrap();
    let repo2 = RepoEntity::from_str("owner/repo2").unwrap();
    let mut repos = vec![];
    repos.push(repo1);
    repos.push(repo2);

    let repos_clone = repos.clone();

    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage.expect_get_repos_per_user().returning(move |_| Ok(repos.clone()));

    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(MockGithubClient::new()),
    );

    // Act
    let result = repository_service.get_user_repos(ChatId(1)).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), repos_clone);
}

#[tokio::test]
async fn test_get_repo_labels() {
    // Arrange
    let chat_id = ChatId(1);
    let repo = RepoEntity::from_str("owner/repo").unwrap();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mut mock_github_client = MockGithubClient::new();

    let mut tracked_labels = HashSet::new();
    tracked_labels.insert("bug".to_string());

    mock_repo_storage
        .expect_get_tracked_labels()
        .with(eq(chat_id), eq(repo.clone()))
        .times(1)
        .returning(move |_, _| Ok(tracked_labels.clone()));

    mock_github_client.expect_repo_labels().returning(|_, _| {
        Ok(vec![labels::LabelsRepositoryLabelsNodes {
            name: "bug".to_string(),
            color: "44b3e2".to_string(),
            issues: Some(labels::LabelsRepositoryLabelsNodesIssues { total_count: 5 }),
        }])
    });

    let repository_service =
        DefaultRepositoryService::new(Arc::new(mock_repo_storage), Arc::new(mock_github_client));

    // Act
    let result = repository_service.get_repo_labels(chat_id, &repo).await;

    // Assert
    assert!(result.is_ok());
    let labels = result.unwrap();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].name, "bug");
    assert_eq!(labels[0].count, 5);
}

#[tokio::test]
async fn test_repo_exists_error() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    mock_github_client.expect_repo_exists().returning(|_, _| Err(GithubError::Unauthorized));

    let repository_service = DefaultRepositoryService::new(
        Arc::new(MockRepoStorage::new()),
        Arc::new(mock_github_client),
    );

    // Act
    let result = repository_service.repo_exists("owner", "repo").await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_contains_repo_error() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage
        .expect_contains()
        .returning(|_, _| Err(StorageError::DbError("error".to_string())));

    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(MockGithubClient::new()),
    );

    // Act
    let result = repository_service
        .contains_repo(ChatId(1), &RepoEntity::from_str("owner/repo").unwrap())
        .await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_add_repo_error() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage
        .expect_add_repository()
        .returning(|_, _| Err(StorageError::DbError("error".to_string())));

    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(MockGithubClient::new()),
    );

    // Act
    let result =
        repository_service.add_repo(ChatId(1), RepoEntity::from_str("owner/repo").unwrap()).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_remove_repo_error() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage
        .expect_remove_repository()
        .returning(|_, _| Err(StorageError::DbError("error".to_string())));

    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(MockGithubClient::new()),
    );

    // Act
    let result = repository_service.remove_repo(ChatId(1), "owner/repo").await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_user_repos_error() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage
        .expect_get_repos_per_user()
        .returning(|_| Err(StorageError::DbError("error".to_string())));

    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(MockGithubClient::new()),
    );

    // Act
    let result = repository_service.get_user_repos(ChatId(1)).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_repo_labels_error() {
    // Arrange
    let chat_id = ChatId(1);
    let repo = RepoEntity::from_str("owner/repo").unwrap();
    let mut mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage
        .expect_get_tracked_labels()
        .with(eq(chat_id), eq(repo.clone()))
        .returning(|_, _| Ok(HashSet::new()));
    mock_github_client.expect_repo_labels().returning(|_, _| Err(GithubError::Unauthorized));

    let repository_service =
        DefaultRepositoryService::new(Arc::new(mock_repo_storage), Arc::new(mock_github_client));

    // Act
    let result = repository_service.get_repo_labels(chat_id, &repo).await;

    // Assert
    assert!(result.is_err());
}
