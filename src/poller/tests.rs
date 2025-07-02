use std::str::FromStr;

use chrono::{DateTime, Utc};
use mockall::predicate::*;

use super::*;
use crate::{
    github::{GithubError, MockGithubClient},
    messaging::MockMessagingService,
    storage::{MockRepoStorage, RepoEntity},
};

const OWNER: &str = "owner";
const REPO_NAME: &str = "repo";
const REPO_NAME_WITH_OWNER: &str = "owner/repo";
const CHAT_ID: ChatId = ChatId(123);
const LAST_POLL_TIME: i64 = 1715817600_i64;

// Helper for creating a default repo entity for tests
fn default_repo_entity() -> RepoEntity {
    RepoEntity::from_str(REPO_NAME_WITH_OWNER).unwrap()
}

// Helper to create a tracked_labels set for reuse
fn default_tracked_labels() -> HashSet<String> {
    let mut labels = HashSet::new();
    labels.insert("bug".to_string());
    labels.insert("enhancement".to_string());
    labels
}

fn last_poll_time_system_time() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(LAST_POLL_TIME as u64)
}

#[test]
fn test_filter_new_issues() {
    let last_poll_time_system_time = last_poll_time_system_time();
    let before_last_poll_time = last_poll_time_system_time - Duration::from_secs(1);
    let after_last_poll_time = last_poll_time_system_time + Duration::from_secs(1);

    let issues = vec![
        issues::IssuesRepositoryIssuesNodes {
            created_at: DateTime::<Utc>::from(after_last_poll_time).to_rfc3339(),
            ..Default::default()
        },
        issues::IssuesRepositoryIssuesNodes {
            created_at: DateTime::<Utc>::from(before_last_poll_time).to_rfc3339(),
            ..Default::default()
        },
    ];

    // Only 1 issue should be included
    let new_issues = GithubPoller::filter_new_issues(issues.clone(), &last_poll_time_system_time);

    assert_eq!(new_issues.len(), 1);
}

#[tokio::test]
async fn test_poll_user_repo_new_issues() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mut mock_messaging_service = MockMessagingService::new();

    let last_poll_time_system_time = last_poll_time_system_time();
    let before_last_poll_time_system_time = last_poll_time_system_time - Duration::from_secs(1);
    let after_last_poll_time_system_time = last_poll_time_system_time + Duration::from_secs(1);

    // Create two issues: one new (after the last poll) and one old (before the last
    // poll)
    let issue_new = issues::IssuesRepositoryIssuesNodes {
        created_at: DateTime::<Utc>::from(after_last_poll_time_system_time).to_rfc3339(),
        ..Default::default()
    };
    let issue_old = issues::IssuesRepositoryIssuesNodes {
        created_at: DateTime::<Utc>::from(before_last_poll_time_system_time).to_rfc3339(),
        ..Default::default()
    };
    let issues = vec![issue_new.clone(), issue_old.clone()];
    let mut tracked_labels = HashSet::new();
    tracked_labels.insert("label name".to_string());
    let tracked_labels_clone = tracked_labels.clone();

    mock_github_client
        .expect_repo_issues_by_label()
        .with(
            eq(OWNER),
            eq(REPO_NAME),
            function(move |labels: &HashSet<String>| *labels == tracked_labels_clone),
        )
        .returning(move |_, _, _| Ok(issues.clone()));

    mock_repo_storage
        .expect_get_tracked_labels()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(move |_, _| Ok(tracked_labels.clone()));

    mock_repo_storage
        .expect_get_last_poll_time()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(move |_, _| Ok(Some(LAST_POLL_TIME)));

    mock_messaging_service
        .expect_send_new_issues_msg()
        .withf(move |chat_id_param, repo_name_param, issues_list| {
            *chat_id_param == CHAT_ID
                && repo_name_param == REPO_NAME_WITH_OWNER
                && issues_list.len() == 1
                && issues_list[0].created_at == issue_new.created_at
        })
        .returning(|_, _, _| Ok(()));

    mock_repo_storage
        .expect_set_last_poll_time()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(|_, _| Ok(()));

    let poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
        10,
    );

    let repo = RepoEntity::from_str(REPO_NAME_WITH_OWNER).unwrap();

    // Act
    let result = poller.poll_user_repo(CHAT_ID, repo).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_poll_user_repo_no_issues() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mut mock_messaging_service = MockMessagingService::new();

    let last_poll_time_system_time = last_poll_time_system_time();
    let before_last_poll_time_system_time = last_poll_time_system_time - Duration::from_secs(1);

    let issue_old = issues::IssuesRepositoryIssuesNodes {
        created_at: DateTime::<Utc>::from(before_last_poll_time_system_time).to_rfc3339(),
        ..Default::default()
    };
    // Create two issues: both are old (before the last poll)
    let issues = vec![issue_old.clone(), issue_old];
    let mut tracked_labels = HashSet::new();
    tracked_labels.insert("label name".to_string());
    let labels_clone = tracked_labels.clone();

    mock_repo_storage
        .expect_get_tracked_labels()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(move |_, _| Ok(tracked_labels.clone()));

    mock_github_client
        .expect_repo_issues_by_label()
        .with(
            eq(OWNER),
            eq(REPO_NAME),
            function(move |labels: &HashSet<String>| *labels == labels_clone),
        )
        .returning(move |_, _, _| Ok(issues.clone()));

    mock_repo_storage
        .expect_get_last_poll_time()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(move |_, _| Ok(Some(LAST_POLL_TIME)));

    mock_repo_storage
        .expect_set_last_poll_time()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(|_, _| Ok(()));

    // Messaging service should not be called since there are no new issues
    mock_messaging_service.expect_send_new_issues_msg().times(0);
    mock_repo_storage.expect_set_last_poll_time().times(0);

    let poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
        10,
    );

    let repo = RepoEntity::from_str(REPO_NAME_WITH_OWNER).unwrap();

    // Act
    let result = poller.poll_user_repo(CHAT_ID, repo).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_poll_user_repo_no_tracked_labels_skips() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new(); // Not called
    let mut mock_repo_storage = MockRepoStorage::new();
    let mut mock_messaging_service = MockMessagingService::new(); // Not called

    mock_repo_storage
        .expect_get_tracked_labels()
        .with(eq(CHAT_ID), eq(default_repo_entity()))
        .times(1)
        .returning(|_, _| Ok(HashSet::new())); // Return empty set

    // These should not be called if there are no tracked labels
    mock_repo_storage.expect_get_last_poll_time().times(0);
    mock_github_client.expect_repo_issues_by_label().times(0);
    mock_messaging_service.expect_send_new_issues_msg().times(0);
    mock_repo_storage.expect_set_last_poll_time().times(0);

    let poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
        10,
    );

    // Act
    let result = poller.poll_user_repo(CHAT_ID, default_repo_entity()).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_poll_user_repo_github_unauthorized_error() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mut mock_messaging_service = MockMessagingService::new(); // Should not be called

    let tracked_labels = default_tracked_labels();

    mock_repo_storage
        .expect_get_tracked_labels()
        .with(eq(CHAT_ID), eq(default_repo_entity()))
        .returning({
            let tl = tracked_labels.clone();
            move |_, _| Ok(tl.clone())
        })
        .times(1);

    mock_repo_storage
        .expect_get_last_poll_time()
        .with(eq(CHAT_ID), eq(default_repo_entity()))
        .returning(|_, _| Ok(Some(LAST_POLL_TIME)))
        .times(1);

    mock_github_client
        .expect_repo_issues_by_label()
        .with(eq(OWNER), eq(REPO_NAME), eq(tracked_labels.clone()))
        .returning(|_, _, _| Err(GithubError::Unauthorized))
        .times(1);

    // No messaging or LPT update expected
    mock_messaging_service.expect_send_new_issues_msg().times(0);
    mock_repo_storage.expect_set_last_poll_time().times(0);

    let poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
        10,
    );

    // Act
    let result = poller.poll_user_repo(CHAT_ID, default_repo_entity()).await;

    // Assert
    assert!(result.is_err());
    match result.unwrap_err() {
        PollerError::Github(GithubError::Unauthorized) => { /* Expected */ }
        other => panic!("Expected PollerError::Github(Unauthorized), got {:?}", other),
    }
}

#[tokio::test]
async fn test_poll_user_repo_github_rate_limited() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mut mock_messaging_service = MockMessagingService::new();

    let tracked_labels = default_tracked_labels();

    mock_repo_storage
        .expect_get_tracked_labels()
        .returning_st(move |_, _| Ok(tracked_labels.clone()));
    mock_repo_storage
        .expect_get_last_poll_time()
        .returning_st(move |_, _| Ok(Some(LAST_POLL_TIME)));

    mock_github_client
        .expect_repo_issues_by_label()
        .returning_st(|_, _, _| Err(GithubError::RateLimited));

    mock_messaging_service.expect_send_new_issues_msg().times(0);
    mock_repo_storage.expect_set_last_poll_time().times(0); // LPT not updated

    let poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
        10,
    );

    // Act
    let result = poller.poll_user_repo(CHAT_ID, default_repo_entity()).await;

    // Assert
    assert!(result.is_ok()); // Non-fatal for this repo, logs and continues
}

#[tokio::test]
async fn test_poll_user_repo_github_graphql_error() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mut mock_messaging_service = MockMessagingService::new();

    let tracked_labels = default_tracked_labels();
    let repo_entity = default_repo_entity();

    mock_repo_storage
        .expect_get_tracked_labels()
        .returning_st(move |_, _| Ok(tracked_labels.clone()));
    mock_repo_storage
        .expect_get_last_poll_time()
        .returning_st(move |_, _| Ok(Some(LAST_POLL_TIME)));

    mock_github_client.expect_repo_issues_by_label().returning_st(|_, _, _| {
        Err(GithubError::GraphQLApiError("Could not resolve to a Repository".to_string()))
    });

    mock_messaging_service.expect_send_new_issues_msg().times(0);
    mock_repo_storage.expect_set_last_poll_time().times(0);

    let poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
        10,
    );

    // Act
    let result = poller.poll_user_repo(CHAT_ID, repo_entity).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_poll_user_repo_set_lpt_fails() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mut mock_messaging_service = MockMessagingService::new();

    let tracked_labels = default_tracked_labels();
    let after_last_poll_time_system_time = last_poll_time_system_time() + Duration::from_secs(1);
    let issue_new = issues::IssuesRepositoryIssuesNodes {
        created_at: DateTime::<Utc>::from(after_last_poll_time_system_time).to_rfc3339(),
        id: "new_id_lpt_fail".to_string(),
        ..Default::default()
    };
    let issues_from_github = vec![issue_new.clone()];

    mock_repo_storage
        .expect_get_tracked_labels()
        .returning_st(move |_, _| Ok(tracked_labels.clone()));
    mock_repo_storage.expect_get_last_poll_time().returning_st(|_, _| Ok(Some(LAST_POLL_TIME)));
    mock_github_client
        .expect_repo_issues_by_label()
        .returning_st(move |_, _, _| Ok(issues_from_github.clone()));
    mock_messaging_service.expect_send_new_issues_msg().returning_st(|_, _, _| Ok(())); // Message sent fine

    mock_repo_storage
        .expect_set_last_poll_time()
        .with(eq(CHAT_ID), eq(default_repo_entity()))
        .times(1)
        .returning_st(|_, _| Err(StorageError::DbError("Failed to write LPT".to_string())));

    let poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
        10,
    );

    // Act
    let result = poller.poll_user_repo(CHAT_ID, default_repo_entity()).await;

    // Assert
    assert!(result.is_ok()); // Non-fatal, logs error but continues
}

#[tokio::test]
async fn test_poll_user_repo_get_tracked_labels_storage_error() {
    let mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mock_messaging_service = MockMessagingService::new();

    mock_repo_storage
        .expect_get_tracked_labels()
        .with(eq(CHAT_ID), eq(default_repo_entity()))
        .times(1)
        .returning_st(|_, _| Err(StorageError::DbError("DB init fail".to_string())));

    let poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
        10,
    );
    let result = poller.poll_user_repo(CHAT_ID, default_repo_entity()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        PollerError::Storage(StorageError::DbError(msg)) if msg == "DB init fail" => {}
        other => panic!("Expected PollerError::Storage(DbError(...)), got {:?}", other),
    }
}

#[tokio::test]
async fn test_poll_user_repo_get_last_poll_time_storage_error() {
    let mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mock_messaging_service = MockMessagingService::new();
    let tracked_labels = default_tracked_labels();

    mock_repo_storage
        .expect_get_tracked_labels()
        .returning_st(move |_, _| Ok(tracked_labels.clone()));
    mock_repo_storage
        .expect_get_last_poll_time()
        .with(eq(CHAT_ID), eq(default_repo_entity()))
        .times(1)
        .returning_st(|_, _| Err(StorageError::DbError("LPT read fail".to_string())));

    let poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
        10,
    );
    let result = poller.poll_user_repo(CHAT_ID, default_repo_entity()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        PollerError::Storage(StorageError::DbError(msg)) if msg == "LPT read fail" => {}
        other => panic!("Expected PollerError::Storage(DbError(...)), got {:?}", other),
    }
}
