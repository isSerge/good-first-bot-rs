use std::{str::FromStr, sync::Arc};

use super::*;
use crate::{
    github::MockGithubClient,
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
    let mut repos = HashSet::new();
    repos.insert(repo1);
    repos.insert(repo2);

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
async fn test_repo_exists_error() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    mock_github_client.expect_repo_exists().returning(|_, _| Err(anyhow::anyhow!("error")));

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
    mock_repo_storage.expect_contains().returning(|_, _| Err(anyhow::anyhow!("error")));

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
    mock_repo_storage.expect_add_repository().returning(|_, _| Err(anyhow::anyhow!("error")));

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
    mock_repo_storage.expect_remove_repository().returning(|_, _| Err(anyhow::anyhow!("error")));

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
    mock_repo_storage.expect_get_repos_per_user().returning(|_| Err(anyhow::anyhow!("error")));

    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(MockGithubClient::new()),
    );

    // Act
    let result = repository_service.get_user_repos(ChatId(1)).await;

    // Assert
    assert!(result.is_err());
}
