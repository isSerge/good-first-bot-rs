use std::{collections::HashSet, str::FromStr, sync::Arc};

use mockall::predicate::eq;

use super::*;
use crate::{
    github::{GithubError, MockGithubClient, labels},
    storage::{MockRepoStorage, RepoEntity},
};

const MAX_REPOS_PER_USER: usize = 10;
const MAX_LABELS_PER_REPO: usize = 5;

#[tokio::test]
async fn test_repo_exists() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    mock_github_client.expect_repo_exists().returning(|_, _| Ok(true));
    let mock_repo_storage = MockRepoStorage::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let repo_exists = repository_service.repo_exists("owner", "repo").await;

    // Assert
    assert!(repo_exists.is_ok());
    assert!(repo_exists.unwrap());
}

#[tokio::test]
async fn test_add_repo() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage.expect_count_repos_per_user().returning(|_| Ok(5));
    mock_repo_storage.expect_add_repository().returning(|_, _| Ok(true));
    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let repo = RepoEntity::from_str("owner/repo").unwrap();
    let result = repository_service.add_repo(ChatId(1), repo).await;

    println!("Add repo result: {:?}", result);

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_remove_repo() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage.expect_remove_repository().returning(|_, _| Ok(true));
    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
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
    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let page = 1;
    let result = repository_service.get_user_repos(ChatId(1), page).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Paginated::new(repos_clone, page));
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

    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let result = repository_service.get_repo_github_labels(chat_id, &repo, 1).await;

    // Assert
    assert!(result.is_ok());
    let labels = result.unwrap();
    assert_eq!(labels.items.len(), 1);
    assert_eq!(labels.items[0].name, "bug");
    assert_eq!(labels.items[0].count, 5);
}

#[tokio::test]
async fn test_repo_exists_error() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    mock_github_client.expect_repo_exists().returning(|_, _| Err(GithubError::Unauthorized));
    let mock_repo_storage = MockRepoStorage::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let result = repository_service.repo_exists("owner", "repo").await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_add_repo_error() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage.expect_count_repos_per_user().returning(|_| Ok(MAX_REPOS_PER_USER));
    mock_repo_storage
        .expect_add_repository()
        .returning(|_, _| Err(StorageError::DbError("error".to_string())));
    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
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
    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
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
    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let result = repository_service.get_user_repos(ChatId(1), 1).await;

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

    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let result = repository_service.get_repo_github_labels(chat_id, &repo, 1).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_add_repo_limit_exceeded() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage.expect_count_repos_per_user().returning(|_| Ok(MAX_REPOS_PER_USER));
    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let result =
        repository_service.add_repo(ChatId(1), RepoEntity::from_str("owner/repo").unwrap()).await;

    // Assert
    assert!(result.is_err());
    if let RepositoryServiceError::LimitExceeded(msg) = result.unwrap_err() {
        assert_eq!(
            msg,
            format!(
                "User {} has reached the maximum number of repositories: {}",
                ChatId(1),
                MAX_REPOS_PER_USER
            )
        );
    } else {
        panic!("Expected LimitExceeded error");
    }
}

#[tokio::test]
async fn test_toggle_label_limit_exceeded() {
    // Arrange
    let mut mock_repo_storage = MockRepoStorage::new();

    // Should already have MAX_LABELS_PER_REPO labels
    let mut tracked_labels = HashSet::new();
    for i in 0..MAX_LABELS_PER_REPO {
        tracked_labels.insert(format!("label_{}", i));
    }

    mock_repo_storage
        .expect_get_tracked_labels()
        .with(eq(ChatId(1)), eq(RepoEntity::from_str("owner/repo").unwrap()))
        .returning(move |_, _| Ok(tracked_labels.clone()));

    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let result = repository_service
        .toggle_label(ChatId(1), &RepoEntity::from_str("owner/repo").unwrap(), "bug")
        .await;

    // Assert
    assert!(result.is_err());
    if let RepositoryServiceError::LimitExceeded(msg) = result.unwrap_err() {
        assert_eq!(
            msg,
            format!(
                "User {} has reached the maximum number of labels per repository: {}",
                ChatId(1),
                MAX_LABELS_PER_REPO
            )
        );
    } else {
        panic!("Expected LimitExceeded error");
    }
}

#[tokio::test]
async fn test_toggle_label_add() {
    // Arrange
    let chat_id = ChatId(1);
    let repo = RepoEntity::from_str("owner/repo").unwrap();
    let label_name = "bug";

    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage
        .expect_get_tracked_labels()
        .with(eq(chat_id), eq(repo.clone()))
        .returning(|_, _| Ok(HashSet::new())); // No labels tracked yet

    mock_repo_storage
        .expect_toggle_label()
        .with(eq(chat_id), eq(repo.clone()), eq(label_name))
        .returning(|_, _, _| Ok(true)); // Label added

    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let result = repository_service.toggle_label(chat_id, &repo, label_name).await;

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap()); // Expecting true, as the label was added
}

#[tokio::test]
async fn test_toggle_label_remove() {
    // Arrange
    let chat_id = ChatId(1);
    let repo = RepoEntity::from_str("owner/repo").unwrap();
    let label_name = "bug";

    let mut tracked_labels = HashSet::new();
    tracked_labels.insert(label_name.to_string());

    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage
        .expect_get_tracked_labels()
        .with(eq(chat_id), eq(repo.clone()))
        .returning(move |_, _| Ok(tracked_labels.clone())); // "bug" is already tracked

    mock_repo_storage
        .expect_toggle_label()
        .with(eq(chat_id), eq(repo.clone()), eq(label_name))
        .returning(|_, _, _| Ok(false)); // Label removed

    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let result = repository_service.toggle_label(chat_id, &repo, label_name).await;

    // Assert
    assert!(result.is_ok());
    assert!(!result.unwrap()); // Expecting false, as the label was removed
}

#[tokio::test]
async fn test_get_user_repo_labels() {
    // Arrange
    let chat_id = ChatId(1);
    let repo = RepoEntity::from_str("owner/repo").unwrap();
    let mut tracked_labels = HashSet::new();
    tracked_labels.insert("bug".to_string());
    tracked_labels.insert("enhancement".to_string());

    let mut mock_repo_storage = MockRepoStorage::new();
    mock_repo_storage
        .expect_get_tracked_labels()
        .with(eq(chat_id), eq(repo.clone()))
        .returning(move |_, _| Ok(tracked_labels.clone()));

    let mock_github_client = MockGithubClient::new();
    let repository_service = DefaultRepositoryService::new(
        Arc::new(mock_repo_storage),
        Arc::new(mock_github_client),
        MAX_REPOS_PER_USER,
        MAX_LABELS_PER_REPO,
    );

    // Act
    let result = repository_service.get_user_repo_labels(chat_id, &repo).await;

    // Assert
    assert!(result.is_ok());
    let labels = result.unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"bug".to_string()));
    assert!(labels.contains(&"enhancement".to_string()));
}
